use std::{collections::HashMap, path::Path, sync::Arc};

use async_trait::async_trait;
use derivative::Derivative;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use workspace_utils::{command_ext::GroupSpawnNoWindowExt, msg_store::MsgStore};

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
        ActionType, CommandRunResult, NormalizedEntry, NormalizedEntryError, NormalizedEntryType,
        ToolResult, ToolStatus,
        utils::{
            EntryIndexProvider, patch,
            patch::{add_normalized_entry, replace_normalized_entry},
            shell_command_parsing::CommandCategory,
        },
    },
    model_selector::{ModelInfo, ModelSelectorConfig, PermissionPolicy},
    profile::ExecutorConfig,
};

#[derive(Deserialize, Debug)]
#[serde(tag = "event", rename_all = "snake_case")]
enum AgyEvent {
    Init {
        conversation_id: Option<String>,
    },
    StepUpdate {
        step_update: AgyStepUpdate,
    },
    Result {
        result: AgyResult,
    },
    #[serde(other)]
    Other,
}

#[derive(Deserialize, Debug)]
struct AgyStepUpdate {
    step_type: Option<String>,
    step_index: Option<usize>,
    state: Option<String>,
    tool_name: Option<String>,
    tool_info: Option<AgyToolInfo>,
    text_delta: Option<String>,
    conversation_id: Option<String>,
}

#[derive(Deserialize, Debug)]
struct AgyToolInfo {
    name: Option<String>,
    parameters: Option<serde_json::Value>,
    output: Option<String>,
}

#[derive(Deserialize, Debug)]
struct AgyResult {
    conversation_id: Option<String>,
    status: Option<String>,
    response: Option<String>,
    error: Option<String>,
}

/// Antigravity (agy) — Google's agentic CLI.
#[derive(Derivative, Clone, Serialize, Deserialize, TS, JsonSchema)]
#[derivative(Debug, PartialEq)]
pub struct Antigravity {
    #[serde(default)]
    pub append_prompt: AppendPrompt,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
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
    /// Resolve the `agy` binary: PATH first, then standard locations.
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
        if let Some(home) = dirs::home_dir() {
            let candidate = home.join(".local").join("bin").join("agy");
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
        }
        "agy".to_string()
    }

    fn build_command_builder(
        &self,
        prompt: &str,
        conversation_id: Option<&str>,
    ) -> Result<CommandBuilder, CommandBuildError> {
        let mut builder = CommandBuilder::new(&Self::agy_binary());

        builder = builder.extend_params(["--print", prompt]);
        builder = builder.extend_params(["--output-format", "stream-json"]);

        if let Some(conv_id) = conversation_id {
            builder = builder.extend_params(["--conversation", conv_id]);
        }

        if let Some(model) = &self.model {
            builder = builder.extend_params(["--model", model.as_str()]);
        }

        let effort = self.effort.as_deref().or_else(|| {
            if let Some(model) = &self.model {
                if model.contains("3.7") || model.contains("gemini-3.7") {
                    Some("high")
                } else {
                    None
                }
            } else {
                Some("high")
            }
        });

        if let Some(effort) = effort {
            if !effort.is_empty() {
                builder = builder.extend_params(["--effort", effort]);
            }
        }

        if self.yolo.unwrap_or(true) {
            builder = builder.extend_params(["--dangerously-skip-permissions"]);
        }

        apply_overrides(builder, &self.cmd)
    }

    /// Live-query `agy models` (a real subcommand: "List available models")
    /// for the currently available model list. `agy` prints one
    /// `id\tdisplay name` row per line to stdout (status/progress text goes
    /// to stderr, so stdout is clean to parse) — e.g. `gemini-3.7-flash-high
    /// \tGemini 3.7 Flash (High)`. Effort variants come back as distinct
    /// rows already, so no separate `reasoning_options` need attaching here.
    ///
    /// Returns empty when `agy` isn't installed or the command fails —
    /// never a stale guess. This replaced a hardcoded model list that had
    /// drifted (still listing retired `gemini-2.5-*` ids) — models here
    /// change too often to hand-maintain.
    async fn discover_models_live() -> Vec<ModelInfo> {
        let mut cmd = tokio::process::Command::new(Self::agy_binary());
        cmd.arg("models")
            // Without this, `agy` inherits this process's stdin — which
            // isn't a TTY here — and some CLIs block waiting for input that
            // will never arrive instead of treating it as closed. Confirmed
            // live: without `Stdio::null()` the spawned `agy models` child
            // hangs forever (never even hits the network) when launched
            // from the server, despite running instantly from a terminal.
            .stdin(std::process::Stdio::null());

        let output = tokio::time::timeout(std::time::Duration::from_secs(15), cmd.output()).await;

        let Ok(Ok(output)) = output else {
            return Vec::new();
        };
        if !output.status.success() {
            return Vec::new();
        }
        let Ok(stdout) = String::from_utf8(output.stdout) else {
            return Vec::new();
        };

        stdout
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }
                let mut parts = line.splitn(2, '\t');
                let id = parts.next()?.trim();
                if id.is_empty() {
                    return None;
                }
                let name = parts.next().map(str::trim).unwrap_or(id);
                Some(ModelInfo {
                    id: id.to_string(),
                    name: name.to_string(),
                    provider_id: None,
                    reasoning_options: vec![],
                })
            })
            .collect()
    }

    async fn spawn_print_mode(
        &self,
        current_dir: &Path,
        prompt: &str,
        conversation_id: Option<&str>,
        env: &ExecutionEnv,
    ) -> Result<SpawnedChild, ExecutorError> {
        let combined_prompt = self.append_prompt.combine_prompt(prompt);
        let command = self
            .build_command_builder(&combined_prompt, conversation_id)?
            .build_initial()?;
        let (program_path, args) = command.into_resolved().await?;

        let mut cmd = tokio::process::Command::new(&program_path);
        cmd.args(&args)
            .current_dir(current_dir)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        env.clone()
            .with_profile(&self.cmd)
            .apply_to_command(&mut cmd);

        let child = cmd.group_spawn_no_window()?;

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
        if let Some(reasoning_id) = &executor_config.reasoning_id {
            self.effort = Some(reasoning_id.clone());
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
        self.spawn_print_mode(current_dir, prompt, None, env).await
    }

    async fn spawn_follow_up(
        &self,
        current_dir: &Path,
        prompt: &str,
        session_id: &str,
        _reset_to_message_id: Option<&str>,
        env: &ExecutionEnv,
    ) -> Result<SpawnedChild, ExecutorError> {
        self.spawn_print_mode(current_dir, prompt, Some(session_id), env)
            .await
    }

    fn normalize_logs(
        &self,
        msg_store: Arc<MsgStore>,
        _worktree_path: &Path,
    ) -> Vec<tokio::task::JoinHandle<()>> {
        use futures::StreamExt;

        let entry_index = EntryIndexProvider::start_from(&msg_store);
        let handle = tokio::spawn(async move {
            let mut stdout_lines = msg_store.stdout_lines_stream();
            let mut current_assistant: Option<(usize, String)> = None;
            let mut active_tools: HashMap<usize, usize> = HashMap::new();
            let mut session_id_reported = false;

            while let Some(Ok(line)) = stdout_lines.next().await {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // Check if line is a JSON stream event
                if let Ok(event) = serde_json::from_str::<AgyEvent>(trimmed) {
                    match event {
                        AgyEvent::Init { conversation_id } => {
                            if let Some(conv_id) = conversation_id {
                                msg_store.push_session_id(conv_id);
                                session_id_reported = true;
                            }
                        }
                        AgyEvent::StepUpdate { step_update } => {
                            if !session_id_reported
                                && let Some(conv_id) = step_update.conversation_id
                            {
                                msg_store.push_session_id(conv_id);
                                session_id_reported = true;
                            }

                            let step_type = step_update.step_type.as_deref().unwrap_or("");
                            let step_idx = step_update.step_index.unwrap_or(0);
                            let state = step_update.state.as_deref().unwrap_or("DONE");

                            if step_type == "agent_response" {
                                if let Some(delta) = step_update.text_delta {
                                    if !delta.is_empty() {
                                        let entry = match &mut current_assistant {
                                            Some((_, content)) => {
                                                content.push_str(&delta);
                                                NormalizedEntry {
                                                    timestamp: None,
                                                    entry_type:
                                                        NormalizedEntryType::AssistantMessage,
                                                    content: content.clone(),
                                                    metadata: None,
                                                }
                                            }
                                            None => NormalizedEntry {
                                                timestamp: None,
                                                entry_type: NormalizedEntryType::AssistantMessage,
                                                content: delta.clone(),
                                                metadata: None,
                                            },
                                        };

                                        match &mut current_assistant {
                                            Some((index, _)) => {
                                                replace_normalized_entry(&msg_store, *index, entry);
                                            }
                                            None => {
                                                let index = add_normalized_entry(
                                                    &msg_store,
                                                    &entry_index,
                                                    entry,
                                                );
                                                current_assistant = Some((index, delta));
                                            }
                                        }
                                    }
                                }
                            } else if step_type == "tool" {
                                // Close current assistant message turn
                                current_assistant = None;

                                let tool_name =
                                    step_update.tool_name.unwrap_or_else(|| "tool".to_string());
                                let tool_info = step_update.tool_info;
                                let params = tool_info.as_ref().and_then(|i| i.parameters.clone());
                                let output = tool_info
                                    .as_ref()
                                    .and_then(|i| i.output.clone())
                                    .unwrap_or_default();

                                let status = if state == "ACTIVE" {
                                    ToolStatus::Created
                                } else {
                                    ToolStatus::Success
                                };

                                let action_type = match tool_name.as_str() {
                                    "view_file" | "read_file" => {
                                        let path = params
                                            .as_ref()
                                            .and_then(|p| {
                                                p.get("AbsolutePath").or_else(|| p.get("path"))
                                            })
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        ActionType::FileRead { path }
                                    }
                                    "grep_search" | "find_by_name" | "search_web" => {
                                        let query = params
                                            .as_ref()
                                            .and_then(|p| {
                                                p.get("Query").or_else(|| {
                                                    p.get("query").or_else(|| p.get("Pattern"))
                                                })
                                            })
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        ActionType::Search { query }
                                    }
                                    "run_command" => {
                                        let command = params
                                            .as_ref()
                                            .and_then(|p| {
                                                p.get("CommandLine").or_else(|| p.get("command"))
                                            })
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        ActionType::CommandRun {
                                            command,
                                            result: Some(CommandRunResult {
                                                exit_status: None,
                                                output: if output.is_empty() {
                                                    None
                                                } else {
                                                    Some(output.clone())
                                                },
                                            }),
                                            category: CommandCategory::Other,
                                        }
                                    }
                                    "replace_file_content" | "write_to_file" => {
                                        let path = params
                                            .as_ref()
                                            .and_then(|p| {
                                                p.get("TargetFile").or_else(|| p.get("path"))
                                            })
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        ActionType::FileEdit {
                                            path,
                                            changes: vec![],
                                        }
                                    }
                                    _ => ActionType::Tool {
                                        tool_name: tool_name.clone(),
                                        arguments: params,
                                        result: Some(ToolResult::markdown(output.clone())),
                                    },
                                };

                                let entry = NormalizedEntry {
                                    timestamp: None,
                                    entry_type: NormalizedEntryType::ToolUse {
                                        tool_name,
                                        action_type,
                                        status,
                                    },
                                    content: output,
                                    metadata: None,
                                };

                                if let Some(&existing_idx) = active_tools.get(&step_idx) {
                                    replace_normalized_entry(&msg_store, existing_idx, entry);
                                } else {
                                    let idx = add_normalized_entry(&msg_store, &entry_index, entry);
                                    if state == "ACTIVE" {
                                        active_tools.insert(step_idx, idx);
                                    }
                                }
                            }
                        }
                        AgyEvent::Result { result } => {
                            if !session_id_reported && let Some(conv_id) = result.conversation_id {
                                msg_store.push_session_id(conv_id);
                                session_id_reported = true;
                            }

                            if let Some(response) = result.response {
                                if !response.trim().is_empty() {
                                    let entry = NormalizedEntry {
                                        timestamp: None,
                                        entry_type: NormalizedEntryType::AssistantMessage,
                                        content: response.clone(),
                                        metadata: None,
                                    };
                                    match &mut current_assistant {
                                        Some((index, _)) => {
                                            replace_normalized_entry(&msg_store, *index, entry);
                                        }
                                        None => {
                                            let index = add_normalized_entry(
                                                &msg_store,
                                                &entry_index,
                                                entry,
                                            );
                                            current_assistant = Some((index, response));
                                        }
                                    }
                                }
                            }

                            if let Some(error) = result.error {
                                if !error.trim().is_empty() {
                                    let entry = NormalizedEntry {
                                        timestamp: None,
                                        entry_type: NormalizedEntryType::ErrorMessage {
                                            error_type: NormalizedEntryError::Other,
                                        },
                                        content: error,
                                        metadata: None,
                                    };
                                    add_normalized_entry(&msg_store, &entry_index, entry);
                                }
                            }
                        }
                        AgyEvent::Other => {}
                    }
                    continue;
                }

                // If line looks like a JSON event that failed parsing, do NOT dump raw json to chat
                if trimmed.starts_with('{') && trimmed.contains("\"event\"") {
                    continue;
                }

                // Plain text fallback (e.g. CLI banner or non-stream output)
                let cleaned = strip_ansi_escapes::strip_str(&line);
                if cleaned.trim().is_empty() && current_assistant.is_none() {
                    continue;
                }
                let entry = NormalizedEntry {
                    timestamp: None,
                    entry_type: NormalizedEntryType::AssistantMessage,
                    content: match &mut current_assistant {
                        Some((_, content)) => {
                            content.push('\n');
                            content.push_str(&cleaned);
                            content.clone()
                        }
                        None => cleaned.clone(),
                    },
                    metadata: None,
                };
                match &mut current_assistant {
                    Some((index, _)) => {
                        replace_normalized_entry(&msg_store, *index, entry);
                    }
                    None => {
                        let index = add_normalized_entry(&msg_store, &entry_index, entry);
                        current_assistant = Some((index, cleaned));
                    }
                }
            }
        });

        vec![handle]
    }

    fn default_mcp_config_path(&self) -> Option<std::path::PathBuf> {
        dirs::home_dir().map(|home| {
            home.join(".gemini")
                .join("antigravity-cli")
                .join("settings.json")
        })
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
            reasoning_id: self.effort.clone(),
            permission_policy: Some(if self.yolo.unwrap_or(true) {
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
        let mut models = Self::discover_models_live().await;

        // If user already configured a model, ensure it's in the list
        if let Some(configured_model) = &self.model {
            if !models.iter().any(|m| m.id == *configured_model) {
                models.insert(
                    0,
                    ModelInfo {
                        id: configured_model.clone(),
                        name: configured_model.clone(),
                        provider_id: None,
                        reasoning_options: vec![],
                    },
                );
            }
        }

        let default_model = self
            .model
            .clone()
            .or_else(|| models.first().map(|m| m.id.clone()));

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
