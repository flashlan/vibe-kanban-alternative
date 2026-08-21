use std::{collections::HashMap, path::Path, sync::Arc};

use async_trait::async_trait;
use derivative::Derivative;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use workspace_utils::{
    command_ext::GroupSpawnNoWindowExt, diff::create_unified_diff, msg_store::MsgStore,
    path::make_path_relative,
};

use crate::{
    approvals::ExecutorApprovalService,
    command::{CmdOverrides, CommandBuildError, CommandBuilder, CommandParts, apply_overrides},
    env::ExecutionEnv,
    executor_discovery::ExecutorDiscoveredOptions,
    executors::{
        AppendPrompt, AvailabilityInfo, BaseCodingAgent, ExecutorError, SpawnedChild,
        StandardCodingAgentExecutor,
    },
    logs::{
        ActionType, CommandRunResult, FileChange, NormalizedEntry, NormalizedEntryError,
        NormalizedEntryType, TokenUsageInfo, ToolResult, ToolStatus,
        stderr_processor::normalize_stderr_logs,
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
        #[serde(default)]
        #[allow(dead_code)]
        init: Option<AgyInitData>,
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

#[derive(Deserialize, Debug, Default)]
#[allow(dead_code)]
struct AgyInitData {
    cwd: Option<String>,
    tools: Option<Vec<String>>,
    permission_mode: Option<String>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct AgyStepUpdate {
    step_type: Option<String>,
    step_index: Option<usize>,
    state: Option<String>,
    tool_name: Option<String>,
    tool_info: Option<AgyToolInfo>,
    text_delta: Option<String>,
    conversation_id: Option<String>,
    duration_seconds: Option<f64>,
    usage: Option<AgyUsage>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct AgyToolInfo {
    name: Option<String>,
    parameters: Option<serde_json::Value>,
    output: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct AgyUsage {
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    thinking_tokens: Option<i64>,
    cache_read_tokens: Option<i64>,
    total_tokens: Option<i64>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct AgyResult {
    conversation_id: Option<String>,
    status: Option<String>,
    response: Option<String>,
    error: Option<String>,
    duration_seconds: Option<f64>,
    num_turns: Option<usize>,
    usage: Option<AgyUsage>,
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
        builder = builder.extend_params(["--print-timeout", "30m"]);

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

fn map_tool_action(
    tool_name: &str,
    params: Option<&serde_json::Value>,
    output: &str,
    worktree: &str,
) -> ActionType {
    match tool_name {
        "view_file" | "read_file" => {
            let raw_path = params
                .and_then(|p| {
                    p.get("AbsolutePath")
                        .or_else(|| p.get("path"))
                        .or_else(|| p.get("file_path"))
                })
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let path = make_path_relative(raw_path, worktree);
            ActionType::FileRead { path }
        }
        "grep_search" | "find_by_name" => {
            let query = params
                .and_then(|p| {
                    p.get("Query")
                        .or_else(|| p.get("query"))
                        .or_else(|| p.get("Pattern"))
                        .or_else(|| p.get("pattern"))
                })
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            ActionType::Search { query }
        }
        "search_web" => {
            let query = params
                .and_then(|p| p.get("query").or_else(|| p.get("Query")))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            ActionType::Search { query }
        }
        "read_url_content" | "open_browser_url" | "read_browser_page" => {
            let url = params
                .and_then(|p| p.get("Url").or_else(|| p.get("url")))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            ActionType::WebFetch { url }
        }
        "list_dir" => {
            let raw_path = params
                .and_then(|p| {
                    p.get("DirectoryPath")
                        .or_else(|| p.get("path"))
                        .or_else(|| p.get("dir_path"))
                })
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let path = make_path_relative(raw_path, worktree);
            ActionType::FileRead { path }
        }
        "run_command" => {
            let command = params
                .and_then(|p| p.get("CommandLine").or_else(|| p.get("command")))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let category = CommandCategory::from_command(&command);
            ActionType::CommandRun {
                command,
                result: Some(CommandRunResult {
                    exit_status: None,
                    output: if output.is_empty() {
                        None
                    } else {
                        Some(output.to_string())
                    },
                }),
                category,
            }
        }
        "replace_file_content" => {
            let raw_path = params
                .and_then(|p| {
                    p.get("TargetFile")
                        .or_else(|| p.get("path"))
                        .or_else(|| p.get("file_path"))
                })
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let target = params
                .and_then(|p| p.get("TargetContent").or_else(|| p.get("target_content")))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let replacement = params
                .and_then(|p| {
                    p.get("ReplacementContent")
                        .or_else(|| p.get("replacement_content"))
                })
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let changes = if !target.is_empty() || !replacement.is_empty() {
                vec![FileChange::Edit {
                    unified_diff: create_unified_diff(raw_path, target, replacement),
                    has_line_numbers: false,
                }]
            } else {
                vec![]
            };
            ActionType::FileEdit {
                path: make_path_relative(raw_path, worktree),
                changes,
            }
        }
        "write_to_file" => {
            let raw_path = params
                .and_then(|p| {
                    p.get("TargetFile")
                        .or_else(|| p.get("path"))
                        .or_else(|| p.get("file_path"))
                })
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let content = params
                .and_then(|p| {
                    p.get("CodeContent")
                        .or_else(|| p.get("content"))
                        .or_else(|| p.get("code"))
                })
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let changes = vec![FileChange::Write {
                content: content.to_string(),
            }];
            ActionType::FileEdit {
                path: make_path_relative(raw_path, worktree),
                changes,
            }
        }
        _ => ActionType::Tool {
            tool_name: tool_name.to_string(),
            arguments: params.cloned(),
            result: Some(ToolResult::markdown(output.to_string())),
        },
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
        worktree_path: &Path,
    ) -> Vec<tokio::task::JoinHandle<()>> {
        use futures::StreamExt;

        let worktree = worktree_path.to_string_lossy().to_string();
        let entry_index = EntryIndexProvider::start_from(&msg_store);

        let h_stderr = normalize_stderr_logs(msg_store.clone(), entry_index.clone());

        let h_stdout = tokio::spawn(async move {
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
                        AgyEvent::Init {
                            conversation_id,
                            init: _,
                        } => {
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

                            // Token usage tracking
                            if let Some(usage) = &step_update.usage
                                && let Some(total) = usage.total_tokens
                            {
                                let entry = NormalizedEntry {
                                    timestamp: None,
                                    entry_type: NormalizedEntryType::TokenUsageInfo(
                                        TokenUsageInfo {
                                            total_tokens: total as u32,
                                            model_context_window: 1_000_000,
                                        },
                                    ),
                                    content: format!(
                                        "Tokens used: {} / Context window: 1000000",
                                        total
                                    ),
                                    metadata: None,
                                };
                                add_normalized_entry(&msg_store, &entry_index, entry);
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
                                let params = tool_info.as_ref().and_then(|i| i.parameters.as_ref());
                                let output = tool_info
                                    .as_ref()
                                    .and_then(|i| i.output.as_deref())
                                    .unwrap_or_default();

                                let status = match state {
                                    "ACTIVE" => ToolStatus::Created,
                                    "ERROR" | "FAILED" => ToolStatus::Failed,
                                    _ => ToolStatus::Success,
                                };

                                let action_type =
                                    map_tool_action(&tool_name, params, output, &worktree);

                                let entry = NormalizedEntry {
                                    timestamp: None,
                                    entry_type: NormalizedEntryType::ToolUse {
                                        tool_name,
                                        action_type,
                                        status,
                                    },
                                    content: output.to_string(),
                                    metadata: None,
                                };

                                if let Some(&existing_idx) = active_tools.get(&step_idx) {
                                    replace_normalized_entry(&msg_store, existing_idx, entry);
                                    if state != "ACTIVE" {
                                        active_tools.remove(&step_idx);
                                    }
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

                            // Token usage tracking on final result
                            if let Some(usage) = &result.usage
                                && let Some(total) = usage.total_tokens
                            {
                                let entry = NormalizedEntry {
                                    timestamp: None,
                                    entry_type: NormalizedEntryType::TokenUsageInfo(
                                        TokenUsageInfo {
                                            total_tokens: total as u32,
                                            model_context_window: 1_000_000,
                                        },
                                    ),
                                    content: format!(
                                        "Tokens used: {} / Context window: 1000000",
                                        total
                                    ),
                                    metadata: None,
                                };
                                add_normalized_entry(&msg_store, &entry_index, entry);
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
                            } else if let Some(status) = result.status {
                                if status.eq_ignore_ascii_case("FAILURE")
                                    || status.eq_ignore_ascii_case("ERROR")
                                {
                                    let entry = NormalizedEntry {
                                        timestamp: None,
                                        entry_type: NormalizedEntryType::ErrorMessage {
                                            error_type: NormalizedEntryError::Other,
                                        },
                                        content: format!("Execution ended with status: {}", status),
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

        vec![h_stderr, h_stdout]
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

/// Antigravity Headed — runs `agy` interactively in a detached tmux session.
#[derive(Derivative, Clone, Serialize, Deserialize, TS, JsonSchema)]
#[derivative(Debug, PartialEq)]
pub struct AntigravityHeaded {
    #[serde(flatten)]
    #[ts(flatten)]
    #[schemars(flatten)]
    pub inner: Antigravity,

    /// Open a terminal-emulator window attached to the session when a headed run
    /// starts. When disabled, the agent still runs in a detached tmux session
    /// (`tmux attach -t vk-<id>`) but no window is opened.
    #[serde(default = "default_open_terminal")]
    #[schemars(
        title = "Open terminal window",
        description = "Open a terminal window attached to the session on start. When off, the agent runs in a background tmux session you can attach to later."
    )]
    pub open_terminal: bool,
}

fn default_open_terminal() -> bool {
    true
}

impl AntigravityHeaded {
    pub fn open_terminal_enabled(&self) -> bool {
        self.open_terminal
    }

    /// Build the command that launches the `agy` interactive TUI for a headed run.
    pub fn build_interactive_command(
        &self,
        prompt: &str,
        conversation_id: Option<&str>,
    ) -> Result<CommandParts, CommandBuildError> {
        let mut builder = CommandBuilder::new(&Antigravity::agy_binary());

        if let Some(conv_id) = conversation_id {
            builder = builder.extend_params(["--conversation", conv_id]);
        }

        if let Some(model) = &self.inner.model {
            builder = builder.extend_params(["--model", model.as_str()]);
        }

        let effort = self.inner.effort.as_deref().or_else(|| {
            if let Some(model) = &self.inner.model {
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

        if self.inner.yolo.unwrap_or(true) {
            builder = builder.extend_params(["--dangerously-skip-permissions"]);
        }

        if !prompt.is_empty() {
            builder = builder.extend_params(["--prompt", prompt]);
        }

        let builder = apply_overrides(builder, &self.inner.cmd)?;
        builder.build_initial()
    }
}

#[async_trait]
impl StandardCodingAgentExecutor for AntigravityHeaded {
    fn apply_overrides(&mut self, executor_config: &ExecutorConfig) {
        self.inner.apply_overrides(executor_config);
    }

    fn use_approvals(&mut self, approvals: Arc<dyn ExecutorApprovalService>) {
        self.inner.use_approvals(approvals);
    }

    async fn spawn(
        &self,
        current_dir: &Path,
        prompt: &str,
        env: &ExecutionEnv,
    ) -> Result<SpawnedChild, ExecutorError> {
        self.inner.spawn(current_dir, prompt, env).await
    }

    async fn spawn_follow_up(
        &self,
        current_dir: &Path,
        prompt: &str,
        session_id: &str,
        reset_to_message_id: Option<&str>,
        env: &ExecutionEnv,
    ) -> Result<SpawnedChild, ExecutorError> {
        self.inner
            .spawn_follow_up(current_dir, prompt, session_id, reset_to_message_id, env)
            .await
    }

    fn normalize_logs(
        &self,
        msg_store: Arc<MsgStore>,
        current_dir: &Path,
    ) -> Vec<tokio::task::JoinHandle<()>> {
        self.inner.normalize_logs(msg_store, current_dir)
    }

    async fn discover_options(
        &self,
        workdir: Option<&Path>,
        repo_path: Option<&Path>,
    ) -> Result<futures::stream::BoxStream<'static, json_patch::Patch>, ExecutorError> {
        self.inner.discover_options(workdir, repo_path).await
    }

    fn get_preset_options(&self) -> ExecutorConfig {
        let mut cfg = self.inner.get_preset_options();
        cfg.executor = BaseCodingAgent::AntigravityHeaded;
        cfg
    }

    fn default_mcp_config_path(&self) -> Option<std::path::PathBuf> {
        self.inner.default_mcp_config_path()
    }

    fn get_availability_info(&self) -> AvailabilityInfo {
        self.inner.get_availability_info()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_agy_events() {
        let init_json = r#"{"event":"init","conversation_id":"conv-123","init":{"cwd":"/tmp","tools":["run_command"]}}"#;
        let event: AgyEvent = serde_json::from_str(init_json).unwrap();
        match event {
            AgyEvent::Init {
                conversation_id, ..
            } => {
                assert_eq!(conversation_id, Some("conv-123".to_string()));
            }
            _ => panic!("Expected Init event"),
        }

        let step_json = r#"{"event":"step_update","step_update":{"conversation_id":"conv-123","step_index":1,"state":"ACTIVE","step_type":"tool","tool_name":"run_command","tool_info":{"name":"run_command","parameters":{"CommandLine":"cargo test"}}}}"#;
        let event: AgyEvent = serde_json::from_str(step_json).unwrap();
        match event {
            AgyEvent::StepUpdate { step_update } => {
                assert_eq!(step_update.tool_name, Some("run_command".to_string()));
                assert_eq!(step_update.state, Some("ACTIVE".to_string()));
            }
            _ => panic!("Expected StepUpdate event"),
        }

        let result_json = r#"{"event":"result","result":{"conversation_id":"conv-123","status":"SUCCESS","response":"All done!","usage":{"total_tokens":1500}}}"#;
        let event: AgyEvent = serde_json::from_str(result_json).unwrap();
        match event {
            AgyEvent::Result { result } => {
                assert_eq!(result.status, Some("SUCCESS".to_string()));
                assert_eq!(result.response, Some("All done!".to_string()));
                assert_eq!(result.usage.and_then(|u| u.total_tokens), Some(1500));
            }
            _ => panic!("Expected Result event"),
        }
    }

    #[test]
    fn test_map_tool_action_replace_file_content() {
        let params = serde_json::json!({
            "TargetFile": "/workspace/src/main.rs",
            "TargetContent": "fn old() {}",
            "ReplacementContent": "fn new() {}"
        });
        let action = map_tool_action("replace_file_content", Some(&params), "Done", "/workspace");
        match action {
            ActionType::FileEdit { path, changes } => {
                assert_eq!(path, "src/main.rs");
                assert_eq!(changes.len(), 1);
                match &changes[0] {
                    FileChange::Edit { unified_diff, .. } => {
                        assert!(unified_diff.contains("-fn old() {}"));
                        assert!(unified_diff.contains("+fn new() {}"));
                    }
                    _ => panic!("Expected FileChange::Edit"),
                }
            }
            _ => panic!("Expected ActionType::FileEdit"),
        }
    }

    #[test]
    fn test_map_tool_action_run_command() {
        let params = serde_json::json!({
            "CommandLine": "git status"
        });
        let action = map_tool_action("run_command", Some(&params), "clean", "/workspace");
        match action {
            ActionType::CommandRun {
                command,
                category,
                result,
            } => {
                assert_eq!(command, "git status");
                assert_eq!(category, CommandCategory::Other);
                assert_eq!(result.and_then(|r| r.output), Some("clean".to_string()));
            }
            _ => panic!("Expected ActionType::CommandRun"),
        }
    }

    #[test]
    fn test_map_tool_action_list_dir() {
        let params = serde_json::json!({
            "DirectoryPath": "/workspace/src"
        });
        let action = map_tool_action("list_dir", Some(&params), "main.rs", "/workspace");
        match action {
            ActionType::FileRead { path } => {
                assert_eq!(path, "src");
            }
            _ => panic!("Expected ActionType::FileRead"),
        }
    }

    #[test]
    fn test_antigravity_headed_build_command() {
        let headed = AntigravityHeaded {
            inner: Antigravity {
                append_prompt: AppendPrompt::default(),
                model: Some("gemini-2.5-pro".to_string()),
                effort: Some("high".to_string()),
                yolo: Some(true),
                cmd: CmdOverrides::default(),
                approvals: None,
            },
            open_terminal: true,
        };

        assert!(headed.open_terminal_enabled());
        let cmd = headed
            .build_interactive_command("Fix login bug", Some("conv-test-123"))
            .unwrap();

        assert!(cmd.args().contains(&"--conversation".to_string()));
        assert!(cmd.args().contains(&"conv-test-123".to_string()));
        assert!(cmd.args().contains(&"--model".to_string()));
        assert!(cmd.args().contains(&"gemini-2.5-pro".to_string()));
        assert!(cmd.args().contains(&"--effort".to_string()));
        assert!(cmd.args().contains(&"high".to_string()));
        assert!(
            cmd.args()
                .contains(&"--dangerously-skip-permissions".to_string())
        );
        assert!(cmd.args().contains(&"--prompt".to_string()));
        assert!(cmd.args().contains(&"Fix login bug".to_string()));
    }
}
