# Chapter 1 — Introduction: what this manual solves

## Who this book is for

For a developer who just installed Vibe Kanban Alternative and wants to **use the interface to actually build software** — not to study the app's architecture. By the end of Part I you will know how to:

- install and configure the app in your `projects.toml`;
- navigate the interface (board, workspaces, panels);
- create and move **cards** across the kanban columns;
- understand what **pipelines** are and how they move your card on their own;
- use **git without fear** inside Vibe Kanban (workspaces, worktrees, branches, PRs);
- build a project from scratch — **a complete SaaS** — using only the interface.

**Part II** is for when you want to customize Vibe Kanban itself. The focus now is **using the application to develop**.

> **Read Chapter 02 before installing.** It introduces the current vibe coding vocabulary — *Engineering Loop*, *Spec Development*, *multi-agent*, *YOLO mode*, *context engineering* — with examples from this repository. The practical chapters (03 onward) assume you already recognize these terms.

## What Vibe Kanban Alternative is, in one page

Vibe Kanban Alternative is a **self-hosted kanban for a solo developer to drive AI coding agents**. Each board card is a task ("fix login", "create the SaaS plans page"). Each task becomes a **workspace** — an isolated folder with its own git branch — where an agent (Claude Code, OpenCode, Codex, Gemini, Cursor, Copilot, etc.) writes code for you. You watch progress on the board, review diffs and merge.

The concepts you will use every day:

| Concept | What it is, in one sentence |
| --- | --- |
| **Issue / Card** | Unit of work. Title + description + status + priority + tags. The description becomes the agent's prompt. |
| **Board / Columns** | The per-project kanban board. Each column is a `project_status` (e.g., Todo → In Progress → Done). You drag cards between columns. |
| **Workspace** | Isolated environment of a task: a git worktree + `vk/xxxx-name` branch + agent session. |
| **Pipeline** | A recipe in TOML (`assets/pipelines/*.toml`) that tells the agent what to do and in what order — and how to report progress (`VK-PIPELINE-STAGE: N`). |
| **Session** | A conversation with an agent inside a workspace. A workspace can have several sessions. |
| **Setup/Cleanup/Dev scripts** | Per-repository/per-project commands Vibe Kanban runs automatically when creating/opening/closing a workspace. |

## The guided project of this book

Starting in Chapter 08 you will build a real SaaS — **AssinaFácil**, a fictional subscription-management SaaS — entirely through the Vibe Kanban interface. Each Part I chapter leaves a card ready for the next, so by the end you have a board with the product's complete history.

## How to read

- Follow Part I in order on the first read; each chapter ends with a **checklist** you can tick off on your own board.
- Paths like `docs/getting-started.mdx` or `crates/server/src/main.rs` really exist in this branch (`vk/1f98-livre-vibo-kanba`) — open them and check.
- Screenshots cited live in `/images/` (site docs) and `docs/images/livro/` (book anchors, ch. 15).
