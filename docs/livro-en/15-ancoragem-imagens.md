# Chapter 15 — Image anchoring

> **Principle:** a well-chosen screenshot is an assertion. It tells a human "looks right" and an AI "compare current state with this" — and in KDP, it *is* the product.

## Why anchored images — text describes, image proves

In a rich-UI app most regressions are visual — a button that vanished, a kanban column that broke, a dialog that won't open. An agent that only reads text may think everything is fine when the screen is empty. Anchored images close that gap: they are the **snapshot test a human grasps in a glance** and an AI can compare pixel-wise.

## How the docs already do it — the Mintlify pattern

Mintlify files in `docs/` wrap every image in `<Frame>` with descriptive `alt`:

```mdx
<Frame>
  <img src="/images/workspaces-preview-no-script.png"
       alt="Preview panel showing prompt to set up a dev server script" />
</Frame>
```

The fullest case is `docs/browser-testing.mdx`: a 3-step walkthrough illustrated by four screenshots — no-script prompt, script dialog, Start button, log panel, browser annotated with 7 numbered controls. Text and image anchor each other; each number in the image is explained in the list. `docs/mobile-testing.md` does the same for device testing.

Rules that emerge:

- Every image has `alt` describing **what should be seen**.
- UI images have a name identifying state (`preview-no-script` vs `preview-dev-server-running`).
- Path is `/images/...` — relative to the docs site, versioned. In the book, `docs/images/livro/`.
- Consistent resolution (1440x900 in the book) for stable comparison.

## What is already anchored — 12 images

The book has 12 versioned anchors in `docs/images/livro/`:

| Group | Files | Ch. |
| --- | --- | --- |
| **Real app** | `ancora-board-principal.png` (989 KB) | 04 |
|  | `ancora-workspace-aberta.png` (775 KB) | 04 |
|  | `ancora-settings.png` (254 KB) | 03 |
|  | `ancora-criar-card-*.png` (3 files) | 05 |
|  | `ancora-workspace-chat-bar.png` (353 KB) | 05 |
| **AssinaFacil previews** | `saas-landing.png` (53 KB) | 08 |
|  | `saas-planos.png` (44 KB) | 08 |
|  | `saas-checkout.png` (41 KB) | 08 |
|  | `saas-minhas-assinaturas.png` (37 KB) | 08 |
|  | `saas-landing-mobile.png` (23 KB) | 08 |

The first 7 are real screenshots; the 5 AssinaFacil ones are PIL-generated previews (commit `5371b672`, reproducible) — placeholders until the ch. 08 cards reach Done and are replaced by real Preview screenshots.

## The full anchoring plan — what remains

- **Board:** `livro/board-empty.png` (empty board + create button).
- **Workspace:** `livro/workspace-diff.png` (Changes), `livro/workspace-terminal.png` (xterm), `livro/workspace-preview.png` (browser toolbar 1-7).
- **Approvals:** `livro/approvals-inbox.png` (TUI + 1 tool permission), `livro/review-request.png` (VK-REVIEW-REQUEST banner).

Capture at 1440x900 with the same seed data (same project/branch) for stable comparison. For KDP print, export at 300 DPI (ch. 09).

## How to capture — 3 paths

### 1. Real Preview screenshot (most faithful)

Run `pnpm run dev`, open the AssinaFacil workspace, **Preview** tab. On macOS: Cmd+Shift+4 → drag → move to `docs/images/livro/saas-landing.png`.

### 2. Via workspace chat (becomes Attachment, visible to agent)

Drag the image into the workspace chat — the app POSTs to `POST /api/attachments/upload` (`crates/server/src/routes/attachments.rs:83`, 20 MB). The agent receives it as visual context (ch. 05).

### 3. PIL generation (fastest — for previews before code exists)

```python
from PIL import Image
im = Image.new("RGB", (1440, 900), "#f8fafc")
# ... draw hero, cards, table (see commit 5371b672)
im.save("docs/images/livro/saas-landing.png")
```

No browser, reproducible, ideal for writing the chapter before coding.

## How the AI uses the anchor

1. **Post-change visual validation.** After touching `packages/web-core/src/`, the agent boots the dev server, screenshots the route and diffs against the anchor. Unexpected delta → fix before committing.
2. **Spec by image.** The card attaches the desired anchor (e.g., plan dialog mock). The agent has, besides the text spec, the visual target — and knows it's done when the screen matches.

## Chapter checklist

- [ ] Every new visual feature has an anchor in `docs/images/livro/` with a predictable name.
- [ ] Every image has descriptive `alt` and, in Mintlify docs, is inside `<Frame>`.
- [ ] The plan covers: board, workspace (5 tabs), approvals, dialogs — versioned.
- [ ] Screenshots at consistent resolution/seed for stable comparison.
- [ ] PIL previews replaced by real screenshots when the card reaches Done (ch. 08).
- [ ] For KDP print, images exported at 300 DPI, CMYK, 0.125 inch bleed (ch. 09).
