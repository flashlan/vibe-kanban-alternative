use std::{path::Path, sync::Arc};

use async_trait::async_trait;
use derivative::Derivative;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use workspace_utils::msg_store::MsgStore;

use crate::{
    approvals::ExecutorApprovalService,
    command::{CmdOverrides, CommandBuildError, CommandBuilder, apply_overrides},
    env::ExecutionEnv,
    executor_discovery::ExecutorDiscoveredOptions,
    executors::{
        AppendPrompt, AvailabilityInfo, BaseCodingAgent, ExecutorError, SpawnedChild,
        StandardCodingAgentExecutor, gemini::AcpAgentHarness,
    },
    logs::utils::patch,
    model_selector::{ModelInfo, ModelProvider, ModelSelectorConfig, PermissionPolicy},
    profile::ExecutorConfig,
};

#[derive(Derivative, Clone, Serialize, Deserialize, TS, JsonSchema)]
#[derivative(Debug, PartialEq)]
pub struct QwenCode {
    #[serde(default)]
    pub append_prompt: AppendPrompt,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "mode")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yolo: Option<bool>,
    #[serde(flatten)]
    pub cmd: CmdOverrides,
    #[serde(skip)]
    #[ts(skip)]
    #[derivative(Debug = "ignore", PartialEq = "ignore")]
    pub approvals: Option<Arc<dyn ExecutorApprovalService>>,
}

impl QwenCode {
    /// Normalise a stored/selected model id before it reaches the Qwen ACP
    /// session.
    ///
    /// Handles two real-world corruptions:
    /// - Literal quotes picked up from UI editing (e.g. `qwen38-q4"`).
    /// - A `provider/model` prefix (e.g. `openai/qwen38-q4`) that the frontend
    ///   sends when a model carries a `provider_id`. The Qwen ACP
    ///   `session/set_model` expects the **bare** id as written in
    ///   `~/.qwen/settings.json`, so we strip the leading `provider/` segment.
    fn sanitize_model_id(raw: &str) -> String {
        let trimmed = raw.trim();
        let unquoted = trimmed
            .trim_matches(|c| c == '"' || c == '\'')
            .trim();
        // Strip a single leading `provider/` segment if present.
        let bare = unquoted
            .split_once('/')
            .map(|(_, rest)| {
                // Only treat as a prefix when there's no further slash in the
                // remainder (real Qwen ids never contain `/`).
                if rest.contains('/') {
                    unquoted
                } else {
                    rest
                }
            })
            .unwrap_or(unquoted)
            .trim();
        bare.to_string()
    }

    fn build_command_builder(&self) -> Result<CommandBuilder, CommandBuildError> {
        let mut builder = CommandBuilder::new("qwen");

        if let Some(model) = &self.model {
            builder = builder.extend_params(["--model", Self::sanitize_model_id(model).as_str()]);
        }

        if self.yolo.unwrap_or(false) {
            builder = builder.extend_params(["--yolo"]);
        }
        builder = builder.extend_params(["--acp"]);
        apply_overrides(builder, &self.cmd)
    }
}

#[async_trait]
impl StandardCodingAgentExecutor for QwenCode {
    fn apply_overrides(&mut self, executor_config: &ExecutorConfig) {
        if let Some(model_id) = executor_config.model_id.as_ref() {
            let clean = Self::sanitize_model_id(model_id);
            if !clean.is_empty() {
                self.model = Some(clean);
            }
        }

        if let Some(agent_id) = executor_config.agent_id.as_ref() {
            self.agent = Some(agent_id.clone());
        }
        if let Some(permission_policy) = executor_config.permission_policy.clone() {
            self.yolo = Some(matches!(
                permission_policy,
                crate::model_selector::PermissionPolicy::Auto
            ));
        }
    }

    fn use_approvals(&mut self, approvals: Arc<dyn ExecutorApprovalService>) {
        self.approvals = Some(approvals);
    }

    async fn spawn(
        &self,
        current_dir: &Path,
        prompt: &str,
        env: &ExecutionEnv,
    ) -> Result<SpawnedChild, ExecutorError> {
        let qwen_command = self.build_command_builder()?.build_initial()?;
        let combined_prompt = self.append_prompt.combine_prompt(prompt);
        let mut harness = AcpAgentHarness::with_session_namespace("qwen_sessions");
        if let Some(model) = &self.model {
            harness = harness.with_model(Self::sanitize_model_id(model));
        }
        if let Some(agent) = &self.agent {
            harness = harness.with_mode(agent);
        }
        let approvals = if self.yolo.unwrap_or(false) {
            None
        } else {
            self.approvals.clone()
        };
        harness
            .spawn_with_command(
                current_dir,
                combined_prompt,
                qwen_command,
                env,
                &self.cmd,
                approvals,
            )
            .await
    }

    async fn spawn_follow_up(
        &self,
        current_dir: &Path,
        prompt: &str,
        session_id: &str,
        _reset_to_message_id: Option<&str>,
        env: &ExecutionEnv,
    ) -> Result<SpawnedChild, ExecutorError> {
        let qwen_command = self.build_command_builder()?.build_follow_up(&[])?;
        let combined_prompt = self.append_prompt.combine_prompt(prompt);
        let mut harness = AcpAgentHarness::with_session_namespace("qwen_sessions");
        if let Some(model) = &self.model {
            harness = harness.with_model(Self::sanitize_model_id(model));
        }
        if let Some(agent) = &self.agent {
            harness = harness.with_mode(agent);
        }
        let approvals = if self.yolo.unwrap_or(false) {
            None
        } else {
            self.approvals.clone()
        };
        harness
            .spawn_follow_up_with_command(
                current_dir,
                combined_prompt,
                session_id,
                qwen_command,
                env,
                &self.cmd,
                approvals,
            )
            .await
    }

    fn normalize_logs(
        &self,
        msg_store: Arc<MsgStore>,
        worktree_path: &Path,
    ) -> Vec<tokio::task::JoinHandle<()>> {
        crate::executors::acp::normalize_logs(msg_store, worktree_path)
    }

    // MCP configuration methods
    fn default_mcp_config_path(&self) -> Option<std::path::PathBuf> {
        dirs::home_dir().map(|home| home.join(".qwen").join("settings.json"))
    }

    fn get_availability_info(&self) -> AvailabilityInfo {
        let mcp_config_found = self
            .default_mcp_config_path()
            .map(|p| p.exists())
            .unwrap_or(false);

        let installation_indicator_found = dirs::home_dir()
            .map(|home| home.join(".qwen").join("installation_id").exists())
            .unwrap_or(false);

        if mcp_config_found || installation_indicator_found {
            AvailabilityInfo::InstallationFound
        } else {
            AvailabilityInfo::NotFound
        }
    }

    fn get_preset_options(&self) -> ExecutorConfig {
        use crate::model_selector::*;
        ExecutorConfig {
            executor: BaseCodingAgent::QwenCode,
            variant: None,
            model_id: self.model.clone(),
            agent_id: self.agent.clone(),
            reasoning_id: None,
            permission_policy: Some(if self.yolo.unwrap_or(false) {
                PermissionPolicy::Auto
            } else {
                PermissionPolicy::Supervised
            }),
        }
    }

    async fn discover_options(
        &self,
        _workdir: Option<&std::path::Path>,
        _repo_path: Option<&std::path::Path>,
    ) -> Result<futures::stream::BoxStream<'static, json_patch::Patch>, ExecutorError> {
        let (providers, models, default_model) = Self::load_settings_models();

        let options = ExecutorDiscoveredOptions {
            model_selector: ModelSelectorConfig {
                providers,
                models,
                default_model,
                permissions: vec![PermissionPolicy::Auto, PermissionPolicy::Supervised],
                ..Default::default()
            },
            ..Default::default()
        };
        Ok(Box::pin(futures::stream::once(async move {
            patch::executor_discovered_options(options)
        })))
    }
}

impl QwenCode {
    /// Read `~/.qwen/settings.json` (the same file the Qwen Code CLI reads)
    /// and surface the configured model providers/models to the selector.
    ///
    /// Each model carries its `provider_id` so the frontend can group and
    /// select it correctly. The Qwen ACP `session/set_model` accepts the bare
    /// model id, so [`sanitize_model_id`] strips any `provider/model` prefix
    /// before the value reaches the ACP session.
    fn load_settings_models() -> (Vec<ModelProvider>, Vec<ModelInfo>, Option<String>) {
        let path = dirs::home_dir()
            .map(|h| h.join(".qwen").join("settings.json"))
            .expect("no home dir");
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return (Vec::new(), Vec::new(), None),
        };

        let parsed: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Failed to parse Qwen settings {}: {}", path.display(), e);
                return (Vec::new(), Vec::new(), None);
            }
        };

        let mut providers = Vec::new();
        let mut models = Vec::new();

        if let Some(provs) = parsed
            .get("modelProviders")
            .and_then(|p| p.as_object())
        {
            for (provider_id, entries) in provs {
                let Some(arr) = entries.as_array() else {
                    continue;
                };
                providers.push(ModelProvider {
                    id: provider_id.clone(),
                    name: provider_id.clone(),
                });
                for entry in arr {
                    let Some(id) = entry.get("id").and_then(|v| v.as_str()) else {
                        continue;
                    };
                    let name = entry
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or(id)
                        .to_string();
                    models.push(ModelInfo {
                        id: id.to_string(),
                        name,
                        provider_id: Some(provider_id.clone()),
                        reasoning_options: Vec::new(),
                    });
                }
            }
        }

        let default_model = parsed
            .get("model")
            .and_then(|m| m.get("name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        (providers, models, default_model)
    }
}
