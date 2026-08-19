use std::{path::Path, sync::Arc};

use async_trait::async_trait;
use derivative::Derivative;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use workspace_utils::msg_store::MsgStore;

pub use super::acp::AcpAgentHarness;
use crate::{
    approvals::ExecutorApprovalService,
    command::{CmdOverrides, CommandBuildError, CommandBuilder, apply_overrides},
    env::ExecutionEnv,
    executor_discovery::ExecutorDiscoveredOptions,
    executors::{
        AppendPrompt, AvailabilityInfo, BaseCodingAgent, ExecutorError, SpawnedChild,
        StandardCodingAgentExecutor,
    },
    logs::utils::patch,
    model_selector::{ModelInfo, ModelSelectorConfig, PermissionPolicy},
    profile::ExecutorConfig,
};

/// Antigravity (agy) — Google's agentic CLI, the ACP-capable successor to the
/// Gemini CLI. Modeled on the Gemini executor: both speak the Agent Client
/// Protocol, so the same harness drives them.
#[derive(Derivative, Clone, Serialize, Deserialize, TS, JsonSchema)]
#[derivative(Debug, PartialEq)]
pub struct Antigravity {
    #[serde(default)]
    pub append_prompt: AppendPrompt,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yolo: Option<bool>,
    #[serde(flatten)]
    pub cmd: CmdOverrides,
    #[serde(skip)]
    #[ts(skip)]
    #[derivative(Debug = "ignore", PartialEq = "ignore")]
    pub approvals: Option<Arc<dyn ExecutorApprovalService>>,
}

impl Antigravity {
    /// Resolve the `agy` binary: PATH first, then the standard install location.
    fn agy_binary() -> String {
        if let Ok(path) = std::env::var("AGY_BIN") {
            return path;
        }
        if let Ok(output) = std::process::Command::new("which").arg("agy").output()
            && output.status.success()
        {
            if let Ok(s) = String::from_utf8(output.stdout) {
                let s = s.trim();
                if !s.is_empty() {
                    return s.to_string();
                }
            }
        }
        // Standard install: ~/.local/bin/agy
        if let Some(home) = dirs::home_dir() {
            let candidate = home.join(".local").join("bin").join("agy");
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
        }
        "agy".to_string()
    }

    fn build_command_builder(&self) -> Result<CommandBuilder, CommandBuildError> {
        // ACP mode: `agy agentapi`. The base command is resolved at runtime.
        let mut builder = CommandBuilder::new(&Self::agy_binary());

        builder = builder.extend_params(["agentapi"]);

        if let Some(model) = &self.model {
            builder = builder.extend_params(["--model", model.as_str()]);
        }

        apply_overrides(builder, &self.cmd)
    }

    /// Run `agy models` and parse `id\tname` lines into `ModelInfo`.
    async fn fetch_cli_models() -> Vec<ModelInfo> {
        let mut cmd = tokio::process::Command::new(Self::agy_binary());
        cmd.arg("models");
        if let Some(home) = dirs::home_dir() {
            cmd.env("HOME", &home);
        }
        let Ok(output) = cmd.output().await else {
            return vec![];
        };
        let Ok(text) = String::from_utf8(output.stdout) else {
            return vec![];
        };

        let mut models = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty()
                || trimmed.starts_with("Fetching")
                || trimmed.starts_with("Error")
            {
                continue;
            }
            let (id, name) = match trimmed.split_once('\t') {
                Some((id, name)) => (id.trim().to_string(), name.trim().to_string()),
                None => continue,
            };
            if id.is_empty() {
                continue;
            }
            models.push(ModelInfo {
                id: id.clone(),
                name: if name.is_empty() { id } else { name },
                provider_id: None,
                reasoning_options: vec![],
            });
        }
        models
    }
}

#[async_trait]
impl StandardCodingAgentExecutor for Antigravity {
    fn apply_overrides(&mut self, executor_config: &ExecutorConfig) {
        if let Some(model_id) = &executor_config.model_id {
            self.model = Some(model_id.clone());
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
        let harness = AcpAgentHarness::new();
        let combined_prompt = self.append_prompt.combine_prompt(prompt);
        let command = self.build_command_builder()?.build_initial()?;
        let approvals = if self.yolo.unwrap_or(false) {
            None
        } else {
            self.approvals.clone()
        };
        harness
            .spawn_with_command(
                current_dir,
                combined_prompt,
                command,
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
        let harness = AcpAgentHarness::new();
        let combined_prompt = self.append_prompt.combine_prompt(prompt);
        let command = self.build_command_builder()?.build_follow_up(&[])?;
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
                command,
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
        super::acp::normalize_logs_with_suppressed_stderr_patterns(
            msg_store,
            worktree_path,
            &[
                "was started but never ended. Skipping metrics.",
                "YOLO mode is enabled. All tool calls will be automatically approved.",
            ],
        )
    }

    fn default_mcp_config_path(&self) -> Option<std::path::PathBuf> {
        dirs::home_dir().map(|home| home.join(".gemini").join("antigravity-cli").join("settings.json"))
    }

    fn get_availability_info(&self) -> AvailabilityInfo {
        let settings_found = self
            .default_mcp_config_path()
            .map(|p| p.exists())
            .unwrap_or(false);

        let installation_indicator_found = dirs::home_dir()
            .map(|home| home.join(".gemini").join("antigravity-cli").exists())
            .unwrap_or(false);

        if settings_found || installation_indicator_found {
            AvailabilityInfo::InstallationFound
        } else {
            AvailabilityInfo::NotFound
        }
    }

    fn get_preset_options(&self) -> ExecutorConfig {
        use crate::model_selector::*;
        ExecutorConfig {
            executor: BaseCodingAgent::Antigravity,
            variant: None,
            model_id: self.model.clone(),
            agent_id: None,
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
        // Pull the real model list from the installed CLI (`agy models`), so the
        // dropdown always matches what the CLI actually supports — no hardcoding.
        let models = Self::fetch_cli_models().await;

        let default_model = models
            .iter()
            .find(|m| m.id.contains("flash"))
            .or_else(|| models.first())
            .map(|m| m.id.clone());

        let options = ExecutorDiscoveredOptions {
            model_selector: ModelSelectorConfig {
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
