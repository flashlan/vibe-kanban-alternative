# Chapter 8 — Practical project: Building a SaaS with Vibe Kanban

> **Principle:** the best way to learn the interface is to ship something real. We build **AssinaFácil**, a fictional subscription-management SaaS, entirely through cards and workspaces.

## The setup spec (reuse from ch. 02)

A monorepo with three packages: `app-web` (Vite), `web-core` (shared), `ui` (design system). The agent runs `pnpm run dev`; Preview shows "Hello AssinaFácil".

## The SaaS walkthrough

| # | Card | What it delivers | Anchor |
| --- | --- | --- | --- |
| 1 | Setup monorepo | `pnpm run dev` boots; Preview shows "Hello AssinaFácil" | — |
| 2 | Landing page | Hero + CTA working | `saas-landing.png` (desktop 1440×900) + `saas-landing-mobile.png` (390×780) |
| 3 | Auth — login/signup | Forms with validation; mocked state | (future anchor `saas-auth.png`) |
| 4 | Plans & checkout | 3-plan table; Subscribe → /checkout mock | `saas-planos.png` + `saas-checkout.png` |
| 5 | Logged area | Mocked list; Cancel changes state | `saas-minhas-assinaturas.png` |
| 6 | Webhooks | `POST /webhooks` changes entitlement; test via workspace Terminal | — |

Each anchor is captured when the card reaches Done (drag the image into the workspace chat — `crates/server/src/routes/attachments.rs:83` — or save directly; ch. 15).

**Anchors of AssinaFácil (previews generated):**

![Landing — AssinaFácil (hero + MRR + features)](/images/livro/saas-landing.png)

![Plans — 3 columns, Pro highlighted](/images/livro/saas-planos.png)

![Checkout — form + summary](/images/livro/saas-checkout.png)

![My subscriptions — logged table with actions](/images/livro/saas-minhas-assinaturas.png)

*Previews generated in PIL for the book — replace with real Preview screenshots when the cards reach Done; keep 1440×900 for stable comparison.*

## The 6-card epic

Write the epic **AssinaFácil — MVP** with the 6 sub-issues above. Dispatch 1 and 2 in parallel workspaces (ch. 02 §4). Each card: Todo → In Progress → In Review → Done with Preview validated. Merge/PR of each workspace when Done.

## When something goes wrong (shortcuts)

- **Preview blank** — dev server script missing; check Logs.
- **`VK-REVIEW-REQUEST`** — agent needs you; answer in UI/TUI/Telegram.
- **Port conflict** — `AddrInUse`; see ch. 02 §5.
- **Needs Attention** — an approval is pending; the hand is raised in the sidebar.

## Chapter checklist

- [ ] Epic + 5 sub-issues created; at least 2 workspaces ran in parallel.
- [ ] Each card did Todo → In Progress → In Review → Done with Preview validated.
- [ ] Screenshots `docs/images/livro/saas-*.png` captured.
- [ ] Merge/PR of each workspace completed; final board in Done.
