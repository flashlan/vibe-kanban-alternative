# Chapter 7 — Git, workspaces and worktrees

> **Principle:** every card gets an isolated git worktree. You never touch `main` directly, and parallel agents never collide.

## The worktree-manager cycle

Each workspace maps to a git worktree created by `crates/worktree-manager` and tracked by `crates/workspace-manager` / `crates/git`. The cycle:

```
create workspace → git worktree (branch vk/xxxx-name) → agent works → you review → squash-merge to target
```

The original repo stays untouched until the merge. This is why multi-agent (ch. 02 §4) is conflict-free.

## The three panels, for git

The **Details sidebar → Git** (`docs/workspaces/interface.mdx:10`) shows:

- Current repo and branch (`vk/xxxx-name`).
- Target branch (where it will merge).
- Uncommitted count, ahead/behind.

Shortcuts there: **Create PR**, **Merge**, **Rebase**.

## Changes → Preview → Terminal: the review loop

1. **Changes** — review the diffs; leave inline comments for the agent ("this belongs in `plans.ts`").
2. **Preview** — open the dev server in the embedded browser; validate visually.
3. **Terminal** — run `git`, `pnpm`, `cargo` in the workspace environment (xterm.js).

Feedback is inline: the agent reads your comment and adjusts in the same workspace.

## Duplicate / Archive / Pin

- **Duplicate** a workspace to fork an experiment from a known-good state.
- **Archive** when done (stops the dev server, keeps the branch).
- **Pin** to keep a workspace at the top of the sidebar.

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| `AddrInUse` on dev port | Another instance holds it; `lsof -nP -i :3002` + check `cwd`; don't kill the wrong one (ch. 02 §5) |
| `VK-REVIEW-REQUEST` stuck | Agent finished but you didn't answer; open the approval in TUI/Telegram/`respond_to_approval` |
| Worktree won't create | Target branch missing locally; fetch it first |
| Merge conflict | Branches diverged; rebase the workspace onto target, re-run `check` |

## Chapter checklist

- [ ] I know each workspace is a git worktree with its own `vk/xxxx` branch.
- [ ] I can review via Changes → Preview → Terminal without leaving the UI.
- [ ] I can Create PR / Merge / Rebase from the Git panel.
- [ ] I know Duplicate / Archive / Pin and when to use them.
- [ ] I can diagnose `AddrInUse` and a stuck `VK-REVIEW-REQUEST`.
