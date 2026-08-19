use std::{path::Path, sync::Arc};

use async_trait::async_trait;
use derivative::Derivative;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use workspace_utils::{
    command_ext::GroupSpawnNoWindowExt,
    msg_store::MsgStore,
};

use crate::{
    approvals::ExecutorApprovalService,
    command::{CmdOverrides, CommandBuildError, CommandBuilder, apply_overrides},
    env::ExecutionEnv,
    executor_discovery::ExecutorDiscoveredOptions,
    executors::{
        AppendPrompt, AvailabilityInfo, BaseCodingAgent, ExecutorError, SpawnedChild,
        StandardCodingAgentExecutor,
    },
    logs::{
        utils::{
            patch,
            patch::{add_normalized_entry, replace_normalized_entry},
            EntryIndexProvider,
        },
        NormalizedEntry, NormalizedEntryType,
    },
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
        // Antigravity has no ACP mode in the CLI — it runs single-shot via
        // `agy --print <prompt>`. The prompt is piped to stdin.
        let mut builder = CommandBuilder::new(&Self::agy_binary());

        builder = builder.extend_params(["--print"]);

        if let Some(model) = &self.model {
            builder = builder.extend_params(["--model", model.as_str()]);
        }

        if self.yolo.unwrap_or(false) {
            builder = builder.extend_params(["--dangerously-skip-permissions"]);
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

    /// Spawn `agy --print --model X`, pipe the prompt to stdin, and stream the
    /// response. Antigravity's CLI has no ACP mode, so this is single-shot.
    async fn spawn_print_mode(
        &self,
        current_dir: &Path,
        prompt: &str,
        env: &ExecutionEnv,
    ) -> Result<SpawnedChild, ExecutorError> {
        let command = self.build_command_builder()?.build_initial()?;
        let (program_path, args) = command.into_resolved().await?;
        let combined_prompt = self.append_prompt.combine_prompt(prompt);

        let mut cmd = tokio::process::Command::new(&program_path);
        cmd.args(&args)
            .current_dir(current_dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        env.clone()
            .with_profile(&self.cmd)
            .apply_to_command(&mut cmd);

        let mut child = cmd.group_spawn_no_window()?;

        // Pipe the prompt to the child's stdin, then close it so `agy --print`
        // reads until EOF.
        if let Some(mut stdin) = child.inner().stdin.take() {
            let prompt = combined_prompt.clone();
            tokio::spawn(async move {
                use tokio::io::AsyncWriteExt;
                let _ = stdin.write_all(prompt.as_bytes()).await;
                let _ = stdin.flush().await;
                // dropping stdin closes the pipe
            });
        }

        let _ = &mut child;
        Ok(SpawnedChild {
            child,
            exit_signal: None,
            cancel: None,
        })
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
        self.spawn_print_mode(current_dir, prompt, env).await
    }

    async fn spawn_follow_up(
        &self,
        current_dir: &Path,
        prompt: &str,
        _session_id: &str,
        _reset_to_message_id: Option<&str>,
        env: &ExecutionEnv,
    ) -> Result<SpawnedChild, ExecutorError> {
        self.spawn_print_mode(current_dir, prompt, env).await
    }

    fn normalize_logs(
        &self,
        msg_store: Arc<MsgStore>,
        _worktree_path: &Path,
    ) -> Vec<tokio::task::JoinHandle<()>> {
        // Antigravity's `--print` mode emits plain text (not ACP JSON), so we
        // accumulate stdout lines into a streaming AssistantMessage entry.
        use futures::StreamExt;

        let entry_index = EntryIndexProvider::start_from(&msg_store);
        let handle = tokio::spawn(async move {
            let mut stdout_lines = msg_store.stdout_lines_stream();
            let mut current: Option<(usize, String)> = None;

            while let Some(Ok(line)) = stdout_lines.next().await {
                let cleaned = strip_ansi_escapes::strip_str(&line);
                if cleaned.trim().is_empty() && current.is_none() {
                    continue;
                }
                let entry = NormalizedEntry {
                    timestamp: None,
                    entry_type: NormalizedEntryType::AssistantMessage,
                    content: match &mut current {
                        Some((_, content)) => {
                            content.push('\n');
                            content.push_str(&cleaned);
                            content.clone()
                        }
                        None => cleaned.clone(),
                    },
                    metadata: None,
                };
                match &mut current {
                    Some((index, _)) => {
                        replace_normalized_entry(&msg_store, *index, entry);
                    }
                    None => {
                        let index = add_normalized_entry(&msg_store, &entry_index, entry);
                        current = Some((index, cleaned));
                    }
                }
            }
        });

        vec![handle]
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
