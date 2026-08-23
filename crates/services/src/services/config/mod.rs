use std::path::PathBuf;

use thiserror::Error;

pub mod editor;
mod versions;

pub use editor::EditorOpenError;

pub const DEFAULT_PR_DESCRIPTION_PROMPT: &str = r#"Update the PR that was just created with a better title and description.
The PR number is #{pr_number} and the URL is {pr_url}.

Analyze the changes in this branch and write:
1. A concise, descriptive title that summarizes the changes, postfixed with "(Vibe Kanban)"
2. A detailed description that explains:
   - What changes were made
   - Why they were made (based on the task context)
   - Any important implementation details
   - At the end, include a note: "This PR was written using [Vibe Kanban](https://vibekanban.com)"

Update the PR using gh pr edit."#;

pub const DEFAULT_COMMIT_REMINDER_PROMPT: &str = "There are uncommitted changes. Please stage and commit them now with a descriptive commit message.";

/// Resolved by `get_rules` (`pre` field). Always-on guardrails to keep in
/// mind throughout a card's work — scoping (which repo's memory to use) and
/// the recall step. Extracted from what used to be duplicated verbatim in
/// every bundled pipeline's `memory` stage.
pub const DEFAULT_GENERAL_RULES_PRE: &str = "PROJECT MEMORY — general rules for using mem0 across this card's work. SCOPING: call `get_context` first — `workspace_repos` lists every repo attached to this workspace (usually one). Use `workspace_repos[0].repo_name` as the PRIMARY `user_id` for `memory_search`/`memory_save` calls — that's this workspace's main project repo. If `workspace_repos` has more than one entry and the fact or query you're handling genuinely concerns a different repo in that list (e.g. a card doing cross-repo integration work), ALSO call `memory_search`/`memory_save` with that OTHER repo's `repo_name` as `user_id` — memory stays scoped per-repo, but a secondary repo you're actively integrating with isn't left blind. Never use a repo slug outside `workspace_repos`. (1) BEFORE you begin, recall memories relevant to the code you will touch: call `memory_search` with `user_id` set to the relevant repo slug(s) and a query naming the files/modules/area this card changes. Returns at most 5 hits per call — if they don't cover what you need, refine the query (narrower terms, a different file/module) and call it again rather than assuming one call is enough. Apply only memories that genuinely match this work; ignore irrelevant ones. If a hit names a specific module, file, or entity and you need to know how it fits into the surrounding code — what depends on it, what it depends on — rather than just that one isolated fact, call `memory_graph_traverse` from that entity name; it returns real graph structure, not just similar-sounding text. If a hit looks old, or inconsistent with the code you are actually seeing, call `memory_check_staleness` with that entity name before trusting or acting on it — a stale memory (describing code already removed or changed) is worse than no memory at all. Never let a memory override the card's explicit instructions. (2) WHILE you work, note any new DURABLE fact about the project (a decision, convention, dependency, or root cause) that a future session would need — and which repo it belongs to.";

/// Resolved by `get_rules` (`post` field). Closing checklist to run once
/// the work is verified.
pub const DEFAULT_GENERAL_RULES_POST: &str = "AFTER the work is verified, call `memory_save` with `user_id` set to that fact's repo slug for each durable fact noted while working. The content MUST be self-contained, factual, and verified — NEVER save speculation, guesses, half-finished work, or ephemeral state (commit hashes, timestamps, log lines). A false or unverified memory poisons every future agent, so when in doubt, do not save.";

/// Prompt for the "Generate spec" intake flow. A coding agent runs once,
/// non-interactively, in a throwaway worktree containing the project's repos,
/// and turns a rough one-line brief into a development-ready technical task.
/// The `{brief}` placeholder is substituted with the user's brief.
///
/// Hard requirements baked in: the agent must NOT ask questions (it is
/// single-shot), must stay read-only (no edits/commits/implementation), and
/// must end with a single fenced ```json block carrying `title` + `description`
/// so the backend can parse it deterministically.
pub const DEFAULT_SPEC_INTAKE_PROMPT: &str = r#"You are acting as a product manager. Turn the rough task brief below into a clear, development-ready technical task that a developer (or a planning step) can pick up cold.

ROUGH BRIEF:
{brief}

You are running NON-INTERACTIVELY and READ-ONLY:
- You CANNOT ask the user questions. Where the brief is ambiguous, make a sensible decision and record it under "Decisions made" as [assumed].
- Do NOT edit files, create files, run git, commit, or implement anything. You may read/grep/glob the repos in your working directory ONLY to ground your assumptions (confirm a named file/flag/endpoint/table really exists and means what the brief implies). Keep this light — a few lookups, not a full exploration.
- Produce the WHAT and the acceptance criteria, NOT the step-by-step implementation plan.

Read the brief for what's missing: open design decisions phrased as questions, vague verbs with no definition of done ("refactor", "improve"), bundled concerns, integration assumptions, and unstated scope edges. Resolve them in the spec.

Write a medium-length spec (about one screen) using exactly these sections, dropping any section that has nothing substantive:

## Outcome — what's different when this is done
Observable behavior/state, not implementation. 2–5 bullets.

## Scope
**In scope:** bullets. **Explicitly out of scope:** the tempting-but-not-now items.

## Technical requirements
Concrete, grounded, checkable constraints. Name real files/flags/endpoints you verified; mark anything unverified as [unverified]. 3–8 bullets.

## Decisions made
Every open decision you resolved + a few words of why. Mark defaults [assumed].

## Testing & acceptance criteria
How we'll know it works — concrete and checkable ("running X produces Y"). Cover the obvious edge cases.

## Risks, dependencies & open assumptions
Anything that could derail it, what it depends on, and every still-unconfirmed assumption.

OUTPUT CONTRACT (critical):
Your FINAL message must be EXACTLY one fenced code block tagged `json` and NOTHING before or after it, of the form:
```json
{"title": "<one-line title, terse and scannable, no 'Task:' prefix>", "description": "<the full markdown spec: the sections above>"}
```
The `description` value is a JSON string, so escape newlines as \n and quotes as \". Do not wrap the JSON in prose. Do not emit any text after the closing fence."#;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("Validation error: {0}")]
    ValidationError(String),
}

pub type Config = versions::v10::Config;
pub type PipelineStep = versions::v10::PipelineStep;
pub type NotificationConfig = versions::v10::NotificationConfig;
pub type EditorConfig = versions::v10::EditorConfig;
pub type ThemeMode = versions::v10::ThemeMode;
pub type SoundFile = versions::v10::SoundFile;
pub type EditorType = versions::v10::EditorType;
pub type GitHubConfig = versions::v10::GitHubConfig;
pub type GiteaConfig = versions::v10::GiteaConfig;
pub type UiLanguage = versions::v10::UiLanguage;
pub type ShowcaseState = versions::v10::ShowcaseState;
pub type SendMessageShortcut = versions::v10::SendMessageShortcut;

/// Will always return config, trying old schemas or eventually returning default
pub async fn load_config_from_file(config_path: &PathBuf) -> Config {
    match std::fs::read_to_string(config_path) {
        Ok(raw_config) => Config::from(raw_config),
        Err(_) => {
            tracing::info!("No config file found, creating one");
            Config::default()
        }
    }
}

/// Saves the config to the given path
pub async fn save_config_to_file(
    config: &Config,
    config_path: &PathBuf,
) -> Result<(), ConfigError> {
    let raw_config = serde_json::to_string_pretty(config)?;
    std::fs::write(config_path, raw_config)?;
    Ok(())
}
