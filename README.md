<p align="center">
  <a href="https://dexloom.mintlify.app">
    <picture>
      <source srcset="packages/public/vibe-kanban-logo-dark.svg" media="(prefers-color-scheme: dark)">
      <source srcset="packages/public/vibe-kanban-logo.svg" media="(prefers-color-scheme: light)">
      <img src="packages/public/vibe-kanban-logo.svg" alt="Vibe Kanban Logo">
    </picture>
  </a>
</p>

<p align="center">Get 10X more out of Claude Code, Gemini CLI, Codex, Amp and other coding agents...</p>

> **Fork by [evertonekacy](https://github.com/evertonekacy)** — adds [Gitea/Forgejo support](#gitea--forgejo-support) alongside GitHub, so you can manage PRs on self-hosted instances.

## What's different in this fork

This fork targets self-hosted development workflows where GitHub is not the primary host. Changes are additive — all upstream features remain intact.

- **Gitea / Forgejo PR support** — create PRs, check status, and fetch comments on any Gitea/Forgejo instance via its REST API. Routing is automatic: remotes pointing at `github.com` use the GitHub provider (`gh` CLI); remotes whose host matches the configured Gitea base URL use the Gitea provider. No manual switching needed.
- **Secure token storage** — the Gitea token lives in `~/.vibe-kanban/gitea.toml` (or the `GITEA_TOKEN` env var), never in the app config or the repository.

## Gitea / Forgejo Support

### Setup

1. Create a **personal access token** on your Gitea/Forgejo instance (scope: `Projects: read/write` or broader as needed).
2. Store the token outside the repo:

```toml
# ~/.vibe-kanban/gitea.toml
token = "your-personal-access-token"
```

   Alternatively, set the environment variable `GITEA_TOKEN` (takes precedence if both are set).

3. Open **Settings → Gitea** in the app and fill in:
   - **Base URL** — e.g. `https://gitea.example.com` (trailing slash optional)
   - **Default branch** — e.g. `main` (used when a PR's base branch is unspecified)

### How routing works

The app inspects the `git remote` URL of each project:

| Remote host | Provider used |
|---|---|
| `github.com`, `github.*` (Enterprise) | GitHub (`gh` CLI) |
| matches `gitea_base_url` host | Gitea (REST API) |
| anything else | unsupported (clear error) |

You can have a board with a mix of GitHub and Gitea projects simultaneously — each project is routed independently.

### Limitations

- Gitea PR creation uses the REST API directly (not a CLI), so no `gh`-specific features (like project-based auto-assignment) apply.
- Comments are fetched from both the issue and PR comment endpoints and merged, matching GitHub's unified comment view.
- Forgejo instances are fully compatible (they share the Gitea API surface used here).
<p align="center">
  <a href="https://www.npmjs.com/package/vibe-kanban-alternative"><img alt="npm" src="https://img.shields.io/npm/v/vibe-kanban-alternative?style=flat-square" /></a>
  <a href="https://github.com/flashlan/vibe-kanban-alternative/actions"><img alt="Build status" src="https://img.shields.io/github/actions/workflow/status/flashlan/vibe-kanban-alternative/main.yml" /></a>
  <a href="https://deepwiki.com/BloopAI/vibe-kanban"><img src="https://deepwiki.com/badge.svg" alt="Ask DeepWiki"></a>
</p>

<h1 align="center"><strong>vibe-kanban-alternative</strong></h1>
<p align="center">
  The independent, self-hosted fork of vibe-kanban. One dev, your machine, your
  agents — drive a crew of coding agents from the terminal or your phone.
</p>
<p align="center">
  Built for a <strong>single-developer process</strong>: no team, no cloud, no
  auth. Adds a <a href="#terminal-ui-tui">TUI cockpit</a> and
  <a href="#telegram-integration">Telegram channel orchestration</a> on top of
  upstream <a href="https://github.com/BloopAI/vibe-kanban">vibe-kanban</a>.
</p>

![](packages/public/vibe-kanban-indie-screenshot-overview.png)

# Welcome to vibe-kanban-alternative

## Overview

In a world where software engineers spend most of their time planning and reviewing coding agents, the most impactful way to ship more is to get faster at planning and review.

`vibe-kanban-alternative` is built for this — for a single developer, running entirely on your own machine. Use kanban issues to plan work, then create workspaces where coding agents can execute.

- **Plan with kanban issues** — create, prioritise, and assign issues on a kanban board
- **Run coding agents in workspaces** — each workspace gives an agent a branch, a terminal, and a dev server
- **Review diffs and leave inline comments** — send feedback directly to the agent without leaving the UI
- **Preview your app** — built-in browser with devtools, inspect mode, and device emulation
- **Switch between 10+ coding agents** — Claude Code, Codex, Gemini CLI, GitHub Copilot, Amp, Cursor, OpenCode, Droid, CCR, and Qwen Code
- **Create pull requests and merge** — open PRs with AI-generated descriptions, review on GitHub, and merge

![](packages/public/vibe-kanban-screenshot-workspace.png)

One command. Describe the work, review the diff, ship it.

```bash
npx vibe-kanban-alternative
```


## Installation

This fork is published on the public npm registry as **`vibe-kanban-alternative`**.

Prerequisites:

- [Node.js](https://nodejs.org/) (>=20) and npm
- A supported coding agent installed and authenticated (the app drives it). A full list is in the [docs](https://dexloom.mintlify.app/supported-coding-agents).

Run it directly with no install:

```bash
npx vibe-kanban-alternative
```

Or install globally:

```bash
npm install -g vibe-kanban-alternative
vibe-kanban-alternative
```

The package ships the web app plus prebuilt `vibe-kanban`, `vibe-kanban-mcp` and
`vibe-kanban-review` binaries and the `npx-cli` launcher, so a Rust toolchain is
not required to run it. On first launch it opens the board at
`http://localhost:3001`.

## Terminal UI (TUI)

`vibe-tui` is a terminal cockpit for the backend — list workspaces and sessions, watch live agent transcripts, manage a kanban board for local projects, and approve/deny/answer the things agents block on, all without leaving the terminal. It's also the always-available manual override for the [Telegram automation](#telegram-integration) below.

Run it against a running backend (it discovers the backend via its port file, or set `VIBE_BACKEND_URL`):

```bash
cargo run -p tui
```

Keys (press `?` in-app for the full list):

| Context | Keys |
|---|---|
| Global | `a` approvals inbox · `?` help · `q` quit |
| List | `↑↓`/`jk` move · `⇥` switch pane · `⏎` open · `n` new task · `b` board · `r` refresh |
| Detail | `⇥`/`←→` move focus between panes (processes · git · transcript) · `↑↓`/`jk` navigate the focused pane · `n`/`p` process · `f` follow · `i` message agent · `s` stop · `esc` back |
| Git pane (in detail) | focus it with `⇥`, then `↑↓` select repo · `m` merge · `R` rebase · `P` create PR · `u` push — shows branch→target, ↑ahead/↓behind, ±diff, PR state per repo |
| Approvals inbox | `↑↓` move · `y` approve · `d` deny · `⏎` answer · `esc` back |
| Board | `←→` column · `↑↓` card · `[ ]` move card · `n` new · `e` edit · `d` delete · `w` workspace · `p` project · `⏎` detail |

## Telegram Integration

`vibe-telegram-bridge` is a **send-only** daemon that streams coding-agent escalations to a Telegram supergroup, so a blocked agent (waiting on a tool-permission prompt or a clarifying question) can be unblocked remotely — by a human replying in Telegram, or by a PM agent acting through the MCP approval tools. The bridge never reads Telegram and never polls the bot token, so it coexists with the sombrax-telegram listener without a 409 conflict.

It is configured by `~/.vibe-kanban/telegram.toml` (see `automation/telegram.toml.example`):

```toml
# ~/.vibe-kanban/telegram.toml
enabled = true
bot_token = "123456:ABC..."        # optional; falls back to $TELEGRAM_BOT_TOKEN
                                   # or ~/.claude/channels/telegram/.env
chat_id = "-1001234567890"         # your supergroup (must have Topics enabled)
general_thread_id = "1"            # optional General topic
per_worktree_topics = true         # spawn a forum topic per Claude Code worktree
# topic_executors = ["CLAUDE_CODE"]  # which executors get a topic
# topic_name_template = "vk: {name}" # {name}/{branch} substituted
```

```bash
cargo run -p telegram-bridge
```

When `enabled = false` (or no config is present), the daemon exits cleanly. With `per_worktree_topics = true`, the bridge watches the backend's `/api/events` stream and creates a dedicated forum topic for each opted-in worktree, routing that worktree's escalations into it; the `workspace_id → message_thread_id` map is persisted in `~/.vibe-kanban/telegram-topics.json` so restarts reuse existing topics.

The app surfaces a read-only **Settings → Telegram** panel (status + a "Send test message" button); it reads `telegram.toml` and the bridge's heartbeat file but does not edit the config — the TOML is hand-edited.

For the full architecture (TUI, bridge, MCP approval tools, and the PM agent), see [`automation/README.md`](automation/README.md).

## Claude Code Plugins (SombraX)

The automation layer is driven from Claude Code by three plugins published in the **`sombrax_plugins`** marketplace: **vibe-kanban-alternative** (orchestration skills, the agent crew, and the bundled `vibe-kanban` MCP server), **sombrax-telegram** (the inbound Telegram channel listener that pairs with the send-only bridge above), and **sombrax-codex** (Codex CLI helpers for independent plan and code review).

Add the marketplace once, then install the plugins you want — from inside Claude Code:

```text
# Add the marketplace (once)
/plugin marketplace add dexloom/sombrax_plugins

# Install the plugins
/plugin install vibe-kanban-alternative@sombrax-plugins
/plugin install sombrax-telegram@sombrax-plugins
/plugin install sombrax-codex@sombrax-plugins
```

The plugins are optional — Indie's web UI, board, and workspaces work without them. Install them when you want to drive the board and the agent crew from Claude Code. See [Claude Code plugins & skills](https://dexloom.mintlify.app/integrations/claude-code-plugins) for the full breakdown.

## Documentation

Head to the [documentation site](https://dexloom.mintlify.app) for the latest guides, including [What's different in Indie](https://dexloom.mintlify.app/indie/whats-different) and the [Solo Cockpit](https://dexloom.mintlify.app/cockpit/index).

## Self-Hosting

Indie runs entirely on your own machine — no cloud, no account. See the [self-hosting guide](https://dexloom.mintlify.app/self-hosting/local-development) to run it locally or [behind Docker](https://dexloom.mintlify.app/self-hosting/deploy-docker).

## Support

For feature requests and bugs, please open an issue on [`flashlan/vibe-kanban-alternative`](https://github.com/flashlan/vibe-kanban-alternative/issues).

## Contributing

`vibe-kanban-alternative` is an independent, single-developer fork. Please raise ideas and changes as issues on [`flashlan/vibe-kanban-alternative`](https://github.com/flashlan/vibe-kanban-alternative/issues) before opening a PR, so we can discuss implementation details and alignment with the roadmap.

## Development

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) (>=20)
- [pnpm](https://pnpm.io/) (>=8)

Additional development tools:
```bash
cargo install cargo-watch
cargo install sqlx-cli
```

Install dependencies:
```bash
pnpm i
```

### Running the dev server

```bash
pnpm run dev
```

This will start the backend and web app. A blank DB will be copied from the `dev_assets_seed` folder.

### Building the web app

To build just the web app:

```bash
cd packages/local-web
pnpm run build
```

### Build from source (macOS)

1. Run `./local-build.sh`
2. Test with `cd npx-cli && node bin/cli.js`

### Environment Variables

The following environment variables can be configured at build time or runtime:

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `PORT` | Runtime | Auto-assign | **Production**: Server port. **Dev**: Frontend port (backend uses PORT+1) |
| `BACKEND_PORT` | Runtime | `0` (auto-assign) | Backend server port (dev mode only, overrides PORT+1) |
| `FRONTEND_PORT` | Runtime | `3001` | Frontend dev server port (dev mode only, overrides PORT) |
| `HOST` | Runtime | `127.0.0.1` | Backend server host |
| `MCP_HOST` | Runtime | Value of `HOST` | MCP server connection host (use `127.0.0.1` when `HOST=0.0.0.0` on Windows) |
| `MCP_PORT` | Runtime | Value of `BACKEND_PORT` | MCP server connection port |
| `DISABLE_WORKTREE_CLEANUP` | Runtime | Not set | Disable all git worktree cleanup including orphan and expired workspace cleanup (for debugging) |
| `VK_ALLOWED_ORIGINS` | Runtime | Not set | Comma-separated list of origins that are allowed to make backend API requests (e.g., `https://my-vibekanban-frontend.com`) |

**Build-time variables** must be set when running `pnpm run build`. **Runtime variables** are read when the application starts.

#### Self-Hosting with a Reverse Proxy or Custom Domain

When running Vibe Kanban behind a reverse proxy (e.g., nginx, Caddy, Traefik) or on a custom domain, you must set the `VK_ALLOWED_ORIGINS` environment variable. Without this, the browser's Origin header won't match the backend's expected host, and API requests will be rejected with a 403 Forbidden error.

Set it to the full origin URL(s) where your frontend is accessible:

```bash
# Single origin
VK_ALLOWED_ORIGINS=https://vk.example.com

# Multiple origins (comma-separated)
VK_ALLOWED_ORIGINS=https://vk.example.com,https://vk-staging.example.com
```

### Remote Deployment

When running Vibe Kanban on a remote server (e.g., via systemctl, Docker, or cloud hosting), you can configure your editor to open projects via SSH:

1. **Access via tunnel**: Use Cloudflare Tunnel, ngrok, or similar to expose the web UI
2. **Configure remote SSH** in Settings → Editor Integration:
   - Set **Remote SSH Host** to your server hostname or IP
   - Set **Remote SSH User** to your SSH username (optional)
3. **Prerequisites**:
   - SSH access from your local machine to the remote server
   - SSH keys configured (passwordless authentication)
   - VSCode Remote-SSH extension

When configured, the "Open in VSCode" buttons will generate URLs like `vscode://vscode-remote/ssh-remote+user@host/path` that open your local editor and connect to the remote server.

See the [documentation](https://dexloom.mintlify.app/settings/general) for detailed setup instructions.
