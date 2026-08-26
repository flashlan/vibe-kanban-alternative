# Chapter 5 — Cards and Kanban: the lifecycle in practice

> **Principle:** a card is a contract. The better you write it, the less you correct.

## The create-card dialog, in two parts

The dialog has two zones:

1. **Top** — Title, Status, Priority, Tags (and the agent/model picker).
2. **Bottom** — Description (the spec that becomes the agent's prompt) + Save.

**Book anchor — create card (top):**

![Create card — top: Title, Status, Priority, Tags](/images/livro/ancora-criar-card-topo.png)

*The top of the dialog: a verb-led Title ("Create plans page"), Status (drops into the column), Priority (urgent/high/medium/low) and Tags. These four fields are what the board shows on the card without opening it.*

**Book anchor — create card (bottom):**

![Create card — bottom: Description + Save](/images/livro/ancora-criar-card-base.png)

*The bottom: the Description is where the spec lives (ch. 02 §2). A strong description has what/where/validate/constraints. Save creates the card and its first workspace-ready draft.*

> Exercise: open **New Issue** and write a card titled "Add retry to webhook handler" with priority `high` and tag `reliability`. Notice the card immediately shows those on the board — no agent needed yet.

## Creating a card from a Workspace

You don't have to start from the board. Inside a workspace:

1. Open the **Workspaces** section of the right panel.
2. Click **Create** — a card is born linked to that workspace, pre-filled with the workspace's repo/branch.

**Book anchor — create card / Workspaces section:**

![Create card — Workspaces / Create section](/images/livro/ancora-criar-card-workspace.png)

*The right panel's Workspaces area with a Create button — the card inherits the workspace context, so the agent already knows the branch and repo.*

## The lifecycle: Todo → In Progress → In Review → Done

| Stage | What happens | Who moves it |
| --- | --- | --- |
| **Todo** | Card written, not started | You |
| **In Progress** | Agent dispatched in a workspace; pipeline running | Agent (via pipeline) |
| **In Review** | Agent raised `VK-REVIEW-REQUEST` or finished; awaiting you | You (after review) |
| **Done** | Validated (Preview + check) and merged | You |

A card can have **Sub-Issues** (the epic → sub-issues pattern of ch. 08) and **Comments** for threaded feedback.

## The chat bar, dissected

The workspace chat bar is where you drive the agent daily:

**Book anchor — workspace chat bar:**

![Workspace chat bar: Tasks, template, presets, permissions, attachments](/images/livro/ancora-workspace-chat-bar.png)

| Control | What it does |
| --- | --- |
| **Tasks** | Quick link to the linked card and its sub-issues |
| **Template** | Reuse a prompt template for similar cards |
| **Presets** | Saved agent/model/permission presets (e.g., YOLO mode on) |
| **Permissions** | Approve/deny tool permissions for this session |
| **Attachments** | Drag an image in — becomes agent context (ch. 04) |

## Chapter checklist

- [ ] I can create a card from the board (+) and from a workspace (Workspaces → Create).
- [ ] I know the two parts of the dialog (top fields / bottom description) and why the description is the spec.
- [ ] I can move a card through Todo → In Progress → In Review → Done and say who moves it each time.
- [ ] I can attach an image to chat and know it becomes agent context.
- [ ] I recognize the chat bar controls (Tasks, template, presets, permissions, attachments).
