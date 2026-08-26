# Chapter 4 — Interface tour

> **Goal:** know where everything lives before creating your first card.

## The app as a map

Everything in Vibe Kanban happens in two places:

1. **Project board** — where you plan (cards and columns).
2. **Workspace view** — where you execute (conversation with the agent + diffs + preview).

The **global sidebar** (left bar, present on every screen) connects the two. It is described in `docs/workspaces/interface.mdx:54`.

## The global sidebar — read in 10 seconds

```
Projetos
 └─ My SaaS (root project)
    ├─ Tasks          ← the cards of this project
    └─ Workspaces     ← all workspaces of this project (Active / Running / Idle / Archived)
```

- **Projects** at the top, with **+** to create a project.
- Each project has **Tasks** (the cards) and, if root, **Workspaces** aggregated.
- Workspaces appear as leaves grouped into **Active / Running / Idle / Needs Attention / Archived**. A blue dot means the dev server is running; a badge means a linked PR; a raised-hand icon means `Needs Attention` (pending approval).
- For a flat list with search/filters, open the **Workspaces dashboard** (`/workspaces`).

Shortcut that saves time: `Cmd/Ctrl + K` → type the project or workspace name — the command bar (`docs/workspaces/command-bar.mdx`) takes you there without scrolling.

## The board (kanban) — where you plan

Open a project to land on the board (`docs/getting-started.mdx:44`). The board has 4 zones:

1. **App bar** (top) — navigate between projects, Workspaces and **Settings** (gear). It's where you import `projects.toml` and switch agent/IDE.
2. **Columns** — each column is a `project_status` (e.g., Todo → Next steps / In Progress → In progress / In Review → In review / Done → Done). Configurable per project via `projects.toml → statuses` (`docs/cockpit/local-projects.mdx`).
3. **Cards** — issues as cards. Each card shows title, priority, tags and Simple ID (`AF-1`). Each column header has a **+** that creates a card already in that column — and the **New Issue** button does the same.
4. **Right panel** — details of the selected card (or the draft being created). It hosts **Workspaces**, **Sub-Issues** and **Comments** (ch. 05).

**Book anchor — main board:**

![Main board — project "Novo aplicativo SaaS" with columns Next steps / In progress / In review / Done](/images/livro/ancora-board-principal.png)

*The book's board (project "Novo aplicativo SaaS"): 4 PT-BR columns — Próximos passos, Em andamento, Em revisão, Concluído (configurable `project_status` via `projects.toml` → `statuses`, `docs/cockpit/local-projects.mdx`). Each column shows its count; the right panel opens the selected card. Site reference screenshots: `/images/onboarding-projects.png`.*

> **30-second exercise:** count the columns in the anchor above. There are 4 — the same ones you declared in `projects.toml` in ch. 03. Change `statuses` to 3 or 5 and reload — the board reflects it instantly. That's how you feel that the board is just a view of `project_status` in SQLite (`crates/db/src/models/project_status.rs`).

**Book anchor — open workspace:**

![Open workspace — Conversation left, Context (Changes/Logs/Preview) center, Details (Git/Terminal) right](/images/livro/ancora-workspace-aberta.png)

*AssinaFácil workspace open: Conversation with the agent on the left; Context alternating Changes/Logs/Preview in the center; Details with Git/Terminal/Notes on the right — exactly the three panels of `docs/workspaces/interface.mdx:10`.*

## The workspace view — the three panels (where you execute)

Opening a workspace (`docs/workspaces/interface.mdx:10`), the screen splits into:

| Panel | Position | What it's for | When you use it |
| --- | --- | --- | --- |
| **Conversation** | Left (main) | Chat with the agent, switch sessions, send follow-ups | 80% of the time — where you ask and correct |
| **Context** | Right (main, switchable) | **Changes** (diffs) / **Logs** (stdout) / **Preview** (embedded browser) | To review what the agent did and see the app running |
| **Details Sidebar** | Right edge | Git (repo/branch, ahead/behind), Terminal (xterm.js), Notes | For git, commands and quick notes |

You don't need to memorize — the **workspace navbar** (`docs/workspaces/interface.mdx:20`) has buttons to toggle each panel:

- Left: **Archive Workspace**.
- Center-right (panel controls): Toggle Left Sidebar / Chat / Changes / Logs / Preview / Right Sidebar.
- Right (utilities): **Spawn Orchestrator**, **Command Bar** (`Cmd/Ctrl + K`), **Projects Guide**, **Settings**.

One time-saving tip: the **Context Bar** — a draggable floating bar with shortcuts to open in IDE, copy workspace path, start dev server and switch Preview/Changes — described in `docs/workspaces/interface.mdx:239`.

### Conversation panel — your channel to the agent

- Full history with the agent, rich text and plan approval.
- **Session dropdown** in the chat toolbar: switch between sessions, create **New Session** when context grows large (the agent warns; see ch. 14 watchdog at 400k tokens).
- Shortcuts: `Cmd/Ctrl + Enter` sends; `Shift + Cmd/Ctrl + Enter` sends in alternate mode; `Cmd/Ctrl + B/I/U` formats.
- **Attachments:** drag-and-drop an image straight into the chat — the app uploads to `POST /api/attachments/upload` (`crates/server/src/routes/attachments.rs:83`, 20 MB, `image/png|jpeg|gif|webp`) and the agent receives it as visual context. Best way to send a mock or error screenshot (ch. 05 §4 detail).

### Context panel — Changes / Logs / Preview (switch with one click)

- **Changes** (`/images/workspaces-changes-panel.png`): modified-files tree + diffs with syntax highlight + **inline comments** to give the agent feedback ("this diff should touch `plans.ts`, not `landing.tsx`").
- **Logs** (`/images/workspaces-logs-panel.png`): tabs per process, in-log search, real-time stdout/stderr. This is where you see `VK-PIPELINE-STAGE: N` reported live as the agent advances the pipeline (ch. 06) — and `VK-REVIEW-REQUEST` when it needs you.
- **Preview** (`/images/workspaces-preview-panel.png`): embedded browser brought up via **Preview proxy** (Rust) + your **Dev server script** (Node, `projects.toml` or Settings). Supports multiple tabs, desktop/mobile modes and auto URL detection from logs (`docs/browser-testing.mdx:34`). For the SaaS of ch. 08, this is where you see `http://localhost:5173` running.

> **When to use each:** Changes to review code, Logs to understand why `check` broke, Preview to validate visually. The agent writes to all three — you read all three.

### Details sidebar — Git / Terminal / Notes (always at hand)

- **Git** (`/images/workspaces-git-panel.png`): current repo and branch (`vk/xxxx-*`), target branch, uncommitted count, ahead/behind — and a shortcut for **Create PR / Merge / Rebase** (`docs/workspaces/git-operactions.mdx`, ch. 07).
- **Terminal** (`/images/workspaces-terminal.png`): xterm.js right in the workspace environment — run `git status`, `pnpm run check`, `cargo test` there. Persists across panel switches.
- **Notes** (`/images/workspaces-notes.png`): rich-text editor per workspace (auto-save). Use it to note card decisions — the next agent opening the workspace reads them.

## Command bar and shortcuts that matter

`Cmd/Ctrl + K` opens the command bar (`docs/workspaces/command-bar.mdx`) — create workspace, archive, duplicate, switch panels, issue actions, all without the mouse. The 3 shortcuts you'll use daily:

| Shortcut | Where | Does |
| --- | --- | --- |
| `Cmd/Ctrl + K` | Global | Command bar |
| `Cmd/Ctrl + Enter` | Chat | Send message |
| Drag image → chat | Chat | Attach as visual context |

## Chapter checklist

- [ ] I can point on the board: app bar, columns, cards, right panel — and create a card via the column **+** or New Issue.
- [ ] I can point in the workspace: Conversation, Context (Changes/Logs/Preview) and Details (Git/Terminal/Notes).
- [ ] I can use the global sidebar to navigate projects and workspaces by state (Active/Running/Needs Attention).
- [ ] I can switch Context between Changes/Logs/Preview and open the embedded terminal.
- [ ] I can open the command bar (`Cmd/Ctrl + K`) and send a message in chat.
