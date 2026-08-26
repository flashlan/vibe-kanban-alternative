# Chapter 2 — Vibe coding notions: Engineering Loop, Spec Development and multi-agent

> **Goal:** learn the current vibe coding jargon — with real examples from the AssinaFácil SaaS — so the practical chapters (03 onward) don't have to stop at every new word.

## 1. Vibe coding, in one sentence — and why it is not "ask and pray"

Vibe coding is **coding by intent**: you describe the outcome you want in natural language and an agent writes, runs and fixes the code. The term was popularized by Andrej Karpathy in Feb/2025 and stuck because it captures the feeling — you "vibe" the idea, the machine materializes it.

### What vibe coding is **not**

To use the tool well, debunk three wrong expectations:

- **It is not "no-code."** You remain the owner of architecture, review and the merge. The agent is a junior developer working at token speed — fast, but needing a senior (you) to approve and steer.
- **It is not "ask and pray."** A loose prompt in chat with no context produces code that *looks* right and breaks at runtime. Real vibe coding is a flow with **artifacts** that make intent auditable.
- **It is not cost-free magic.** The agent has finite memory (the *context window*) and selective attention. If you don't manage what it sees and what it remembers, it hallucinates, forgets or repeats work. Hence the practices in this chapter (and §6: context, memory, autocompact).

### What vibe coding **is**, in Vibe Kanban

In Vibe Kanban, vibe coding is not a loose prompt in chat. It is a flow with artifacts — **cards (spec), pipelines (recipe), workspaces (isolated worktrees)** — that make intent auditable, repeatable and parallelizable. Without artifacts, the agent hallucinates; with artifacts, it delivers.

```
intent (you) → spec (card) → pipeline (recipe) → workspace (worktree) → loop (agent) → review (you)
```

### Vibe coding maturity levels

| Level | How you work | Where this book lives |
| --- | --- | --- |
| 1 — Loose chat | You ask and copy the result | — (starting point, fragile) |
| 2 — Spec-driven | Each task becomes a card with strong spec + done criteria (ch. 2–5) | Part I |
| 3 — Orchestrated | Several agents in parallel, pipelines, memory and autocompact managing state (ch. 6, 10–14) | Part II |

This book takes you from level 1 to 3. Chapter 2 is the vocabulary; §6 (below) is what separates someone who "asks" from someone who "drives."

This chapter presents the three pillars that sustain this flow and, next, the operational best practices.

## 2. Spec Development — the spec comes before the code

**Spec = what the software should do, before how it does it.** If the spec is weak, the code is born crooked and every later fix is a patch.

### Where the spec lives in this repository

- **The card is the spec.** In `docs/issue-management.mdx:57`, the card description becomes the prompt the agent receives when creating the workspace. The difference between a weak and a strong card is the difference between a lost and a precise agent.
- **Pipelines formalize the spec.** `speckit` and `basic` (`assets/pipelines/*.toml`) generate `SPEC.md` / `IMPLEMENTATION_PLAN.md` before touching code — the agent only codes after the spec is reviewed.
- **ADRs guard the architectural spec.** The root `AGENTS.md` tells agents to consult `docs/ADR/` before proposing alternatives.

### Concrete example: AssinaFácil "Plans page"

**Weak spec** (the agent will guess — and fail):

> Make a plans page.

**Strong spec** (the agent nails it first try):

> **Title:** Create `/plans` page with 3 plans (Free, Pro R$49, Enterprise) and CTA to checkout
>
> **Description:**
> - Comparison table with 3 columns, "Most popular" badge on Pro, "Subscribe" CTA in each column.
> - Files: `packages/web-core/src/pages/plans.tsx` (new), `packages/web-core/src/features/billing/plans.ts` (plan data).
> - Validation: Preview at 1440px and 375px shows the table without breaking; `pnpm run check` passes; anchored screenshot at `docs/images/livro/saas-planos.png` matches.
> - Constraints: use configured Tailwind (`packages/local-web/AGENTS.md`); no new dependency; follow `packages/web-core/src/features/` pattern.

Notice the structure: **what** (3-plan table), **where** (files), **how to validate** (check + screenshot), **constraints** (no new lib). The agent receives this as a prompt and already knows when it's done — when Preview matches the anchor.

### Spec jargon you will see

| Term | Meaning here |
| --- | --- |
| **Spec-Driven Architecture** | The spec dictates who does what: TypeScript on Node handles UI; Rust handles state/processes/git. The `crates/` vs `packages/` vs `shared/` boundary is drawn in the spec, not improvised (ch. 11). |
| **Spec Intake** | Turning a vague brief ("I want a subscription SaaS") into a dev-ready task (`docs/cockpit/spec-intake.mdx`). In the book, the AssinaFácil intake becomes the epic + 5 sub-issues of ch. 8. |
| **Generated contract** | The boundary spec lives in Rust structs with `#[derive(TS)]` and is generated to TypeScript (`shared/types.ts` via `crates/server/src/bin/generate_types.rs` — ch. 12). The code **is** the spec. |
| **Done criteria** | A sentence stating when the card is Done — always with observable validation ("Preview shows X; `pnpm run check` passes"). |

> Practical rule: if you can't write the spec in 5 sentences + 1 done criterion, the agent won't be able to implement it either.

## 3. Engineering Loop — the loop that lets the agent self-correct

The **Engineering Loop** is the cycle that lets the agent **self-correct without you**:

```
write → run checks → read the error in the log → fix → repeat
```

When the loop is short and legible, 90% of errors resolve themselves. When it's long or opaque, the agent stops and escalates — and you are interrupted.

### The three ingredients of the loop

**a) CLI as the agent's interface.** Canonical commands in `package.json` that the agent reads from `AGENTS.md`:

```bash
pnpm run check        # tsc (local-web, web-core, ui) + cargo check + guards
pnpm run lint         # ESLint + cargo clippy -- -D warnings (with --features qa-mode)
pnpm run format       # cargo fmt + Prettier
cargo test --workspace
pnpm run generate-types  # regenerates shared/types.ts (ch. 12)
```

The agent doesn't guess commands; it runs the same ones you would.

**b) Errors that teach.** Guards fail with a message pointing at the fix:

- `scripts/check-migration-frozen.sh` — blocks editing a published migration and says why.
- `scripts/check-legacy-frontend-paths.sh` — blocks importing from an old path and points to the new one.
- `cargo clippy -- -D warnings` — turns a warning into an error; nothing passes as "just a warning."

**c) Logs as protocol.** The agent reports progress by writing to the log; the backend watches the `MsgStore`:

- `VK-PIPELINE-STAGE: N` → persists `workspaces.current_pipeline_stage` (`crates/services/src/services/pipeline_stage.rs`, regex `(?i)VK-PIPELINE-STAGE:\s*(\d+)`).
- `VK-REVIEW-REQUEST: ...` → triggers sound + notification (`crates/services/src/services/review_request.rs`, regex `(?i)VK-REVIEW-REQUEST:\s*(.+)`).

The log is simultaneously human output and machine API — and it works the same for Claude, Codex, OpenCode, Gemini… because they all write to the same `MsgStore` (`crates/executors/src/stdout_dup.rs`).

### Real loop walkthrough (with a real error)

Imagine the "Plans page" card above. The agent writes `plans.tsx` and runs:

```bash
pnpm run check
# → error TS2322: Type 'string' is not assignable to type 'PlanTier' in plans.ts:14
```

The agent reads the error, opens `plans.ts:14`, sees it used `"pro"` instead of `"Pro"` (the `PlanTier` type comes from `shared/types.ts`, generated from Rust — ch. 12), fixes it, and runs again:

```bash
pnpm run check   # → passes
pnpm run lint    # → passes (clippy -D warnings shows no mercy)
```

Only then does it write `VK-PIPELINE-STAGE: 2` and move on. You did nothing — the loop closed on its own because the error was legible and the command was canonical.

If the error were opaque ("failed"), the agent would stop at `Needs Attention` and you'd have to guess. That's why the book insists: **invest in the loop before you invest in the prompt**.

## 4. Multi-agent — several agents in parallel without stepping on each other

One agent alone already helps. Several in parallel change the scale — but only if the repository isolates each one's work.

### Isolation: workspaces = worktrees

Each workspace is a folder in `.vibe-kanban-workspaces/` with its own branch `vk/xxxx-name` created from the `target branch` (`docs/workspaces/creating-workspaces.mdx:12`). Your original repo is untouched.

```
original repo (main)
  ├─ .vibe-kanban-workspaces/
  │   ├─ vk-a1b2-landing-page/   (workspace A, branch vk/a1b2-...)
  │   └─ vk-c3d4-auth/           (workspace B, branch vk/c3d4-...)
  └─ (untouched)
```

- **One card, one workspace — or several.** You can link multiple workspaces to the same card and run Claude, Codex and OpenCode in parallel, each in its own worktree.
- **Multi-agent pipelines.** `swarm-multi-agent.toml` orchestrates subagents on different fronts of the same epic. The **Orchestrator** is the singleton agent directing the whole board (`docs/cockpit/orchestrator.mdx`, `crates/services/src/services/orchestrator_compactor.rs` — with 400k-token / 1h / 10m-cooldown watchdog).

### Practical example: AssinaFácil in parallel

Epic **AssinaFácil — MVP** with sub-issues:

1. Landing page
2. Auth (login/signup)

You dispatch both at once:

- Workspace A (`vk-a1b2-landing`) — agent 1 writes `landing.tsx`.
- Workspace B (`vk-c3d4-auth`) — agent 2 writes `auth.tsx`.

Each runs its own `pnpm run check` in its worktree, no conflict. When both raise `VK-REVIEW-REQUEST`, you review the diffs in **Changes** and the app in each workspace's **Preview**, move the cards to Done, and the pipeline squash-merges into `main` — one at a time, no merge hell.

### Multi-agent jargon

| Term | Meaning here |
| --- | --- |
| **Swarm / crew** | A set of agents in different workspaces of the same project. |
| **Orchestration** | Deciding who does what and in what order — via `get_pipeline` / `report_pipeline_stage` (MCP). |
| **YOLO mode** | Running without asking permission on every tool call (`docs/vibe-guide.mdx:52` — "Use YOLO mode" for async to work; without it you reinvented pair programming, just slower). |
| **Needs Attention** | Workspace state in the sidebar when there's a pending approval — the agent raised its hand (`docs/workspaces/interface.mdx:70`). |

## 5. Quick glossary — the jargon you see everywhere

| Jargon | Practical translation |
| --- | --- |
| **Context engineering** | Choosing what the agent sees (files, logs, rules). The job of `AGENTS.md` and `get_rules`/`memory_search`. |
| **Prompt engineering** | Writing the card description so the agent gets it right (ch. 05: title with verb + done criterion). |
| **Spec intake** | Turning a vague request into a well-specified card before coding. |
| **Approval** | A pause asking permission — tool permission ("can I run `rm`?") or a question ("which priority?") — answered via TUI, Telegram or `respond_to_approval` (MCP). |
| **Setup / Cleanup / Dev scripts** | Per-project/repo commands Vibe Kanban runs when creating/opening/closing a workspace (Settings → Projects & Repositories). |
| **Worktree** | A lightweight repo copy with its own branch — the isolation that enables conflict-free multi-agent. |
| **Target vs Working branch** | Target = where it will merge (e.g., `main`, you define it); Working = where the agent works (`vk/xxxx`, auto-created). |
| **Preview proxy** | The Rust server that embeds the Node dev server inside the Preview panel (ch. 07). |
| **TUI / Telegram bridge** | Control surfaces for a solo dev: `vibe-tui` (`crates/tui`) in the terminal and `vibe-telegram-bridge` (`crates/telegram-bridge`) in Telegram — both talk to the same approvals API (`automation/README.md`). |
| **Squash-merge** | The `quick` pipeline joins the workspace commits into a single commit on the target branch. |
| **ADR** | Architecture Decision Record in `docs/ADR/` — the versioned architectural spec. |

## 6. Best practices — context, memory and autocompact

Level-3 vibe coding is not writing better prompts; it's **managing the agent's cognitive state**. Three levers: what it *sees* (context), what it *remembers* between sessions (memory) and how it *survives* long runs (autocompact). Mastering the three is what separates "asking" from "driving."

### 6.1 Context engineering — context is the AI's source code

**Context engineering** is the discipline of choosing what the agent sees each turn. The agent is only as good as what's in its window; everything outside it, for it, does not exist.

In Vibe Kanban, context is layered (ch. 10):

- `AGENTS.md` root (identity, map, commands) + `docs/AGENTS.md` (docs) + `packages/local-web/AGENTS.md` (UI) — each layer loads only when the agent touches that folder, saving tokens.
- `get_rules` (MCP) brings general rules at the start of each card; `get_pipeline` brings **only the current stage**, not the whole pipeline.
- Cards carry only a pipeline **pointer** (`<!-- vk:pipeline:start -->`), not the heavy prompt — the content enters the window only when the card runs.

**Context manipulation** (what you, human, do):

- **Inject context intentionally.** Attach a screenshot or a log snippet to the chat (`POST /api/attachments/upload`, ch. 05 §4) instead of describing "the error" at length — the image is worth a thousand words of context.
- **Reduce noise.** Use subagents (multi-agent, §4) so side work doesn't pollute the main window; don't cram the whole board into the prompt.
- **Prune dead context.** When the transcript grows large, advance `VK-PIPELINE-STAGE` and let the **OrchestratorCompactor** (§6.3) cut what doesn't matter.

> Golden rule: treat the *context window* as a finite budget. Every line you put in the card is one less line for the agent to reason with.

### 6.2 Autocompact — how the agent doesn't "forget" on long runs

**Autocompact** is the automatic compaction of the transcript when the context window is about to overflow. Without it, on multi-hour runs the agent "loses the beginning" — forgets the spec and starts contradicting what it did.

In Vibe Kanban, the watchdog is the **OrchestratorCompactor** (`crates/services/src/services/orchestrator_compactor.rs`):

- Measures transcript tokens **every 60s**.
- If it passes **400k tokens** (or 1h without compacting, with at least 50k), it types `/compact` in the tmux session.
- Types it **via keystrokes** (`tmux send-keys`), not as pasted text — because slash commands don't work as pasted text in an interactive session.
- **10min cooldown** between sends; **3 consecutive failures** escalate to Telegram (`crates/telegram-bridge`).

For you: you don't need to watch the context size — the watchdog handles it. For your SaaS (ch. 08): apply the same principle — cut dead context before overflowing, not after.

> **Manual compact vs autocompact:** `/compact` is the command the agent (or watchdog) fires; "autocompact" is that firing happening on its own. In agents without a watchdog, you send `/compact` yourself when the transcript grows.

### 6.3 Agent memory — mem0 (semantic + graph)

**Agent memory** is what lets the agent *remember between sessions* verified facts, instead of relearning everything each card. In Vibe Kanban this is **mem0**, exposed as MCP tools (`crates/mcp/src/task_server/tools/mem0.rs`):

| Tool | What it does | When to use |
| --- | --- | --- |
| `memory_search` | Search by **semantic similarity** (not keyword) | "How does the pipeline stage flow work?" before touching code |
| `memory_save` | Persists a **verified, durable** fact | After confirming an architectural decision |
| `memory_graph_traverse` | Traverses dependency edges from an entity | "What consumes the `VK-PIPELINE-STAGE` marker?" |
| `memory_check_staleness` | Checks whether a saved entity still exists in code (diff `commit_sha` → HEAD) | Before trusting an old memory |

**Semantic memory** (`memory_search`): answers by *meaning proximity*. You ask "what's the card-to-production flow?" and it ranks the most relevant passages — even if none contain the word "flow". `AGENTS.md` tip: if results don't cover what you need, **re-search with a sharper query** rather than asking for more hits — iterating beats broad searching.

**Graph memory** (`memory_graph_traverse`): follows the *actual structure* of the code. From a start node (`start`, e.g., `pipeline_stage.rs`), you go `out` (what depends on it), `in` (what it depends on) or `both`, up to `hops` steps (max 3). It's a "who uses this?" map — useful when semantics don't match but the dependency is clear.

**What to save (and what not to):**

- ✅ Save **verified** and durable facts (ADR decisions, contracts, where each crate lives). `memory_save` is *best-effort*: returns `stored=false` if mem0 is unavailable — not an error, just persist later.
- ❌ Don't save speculation, secrets, or ephemeral facts (today's target branch may change tomorrow).
- ❌ Don't save what's in `AGENTS.md` — duplicating context is wasted space.

**Staleness:** `memory_check_staleness` looks at the `commit_sha` captured when the memory was saved and diffs the repo to HEAD. If the entity's text vanishes from removed code, it's **stale**. `checked=false` means "couldn't verify" — treat as *unknown*, never as "confirmed fresh".

### 6.4 Other terms you will see

| Term | Practical translation |
| --- | --- |
| **Context window** | Max size (in tokens) of what the agent sees at once. Your budget (§6.1). |
| **Context engineering** | Choosing what enters that window — the job of `AGENTS.md`, `get_rules` and pruning. |
| **Autocompact** | Automatic transcript compaction when the window overflows (§6.2). |
| **Compact (manual)** | The `/compact` command that cuts the transcript keeping the essentials. |
| **Semantic memory** | Memory by meaning similarity (`memory_search`). |
| **Graph memory** | Memory by dependency edges (`memory_graph_traverse`). |
| **Embeddings** | How mem0 turns text into vectors to compare meaning (internal detail; you just use the search). |
| **Retrieval / RAG** | Recovering memory or docs to inject into context. |
| **Staleness** | Whether a memory still matches current code (`memory_check_staleness`). |
| **Scratch / Notes** | Per-workspace ephemeral draft (Notes panel, ch. 04) — not to be confused with persistent memory. |

### 6.5 Golden rules of best practices

- Treat the *context window* as a budget: inject the essential, cut the noise.
- Compact early — autocompact or `/compact` — on long runs, before overflowing.
- Save in memory **only verified and durable facts**; keep speculation and secrets out.
- Use **graph** to discover neighbors (who uses what) and **semantic** to recall *how* something works.
- **Check staleness** before trusting old memory — `checked=false` is not "fresh."
- Memory and context don't replace `AGENTS.md`: the canonical source lives in the file, not in volatile memory.

## 7. How it all comes together in 2 minutes

A typical flow, with the right vocabulary:

1. You do **spec intake**: write a "Plans page" card with the strong spec from §2 and a done criterion.
2. Create a **workspace** (worktree + working branch `vk/xxxx-plans`) linked to the card, choose the `quick` **pipeline** and dispatch the agent.
3. The agent enters the **Engineering Loop**: writes `plans.tsx` → `pnpm run check` → reads `TS2322` → fixes → `VK-PIPELINE-STAGE: 2`.
4. If it needs you, it raises `VK-REVIEW-REQUEST` or `Needs Attention` — you answer in the interface, the **TUI** (`cargo run -p tui`, key `a`) or **Telegram**.
5. When the done criterion hits (Preview 1440px + 375px ok, check passes, screenshot `saas-planos.png` matches), the card goes to **Done** and the pipeline **squash-merges** into `main`. Meanwhile another agent already runs in parallel on the "Auth" card — **multi-agent**.

In the next chapters you will live this flow in practice, starting with installation (ch. 03).

## Chapter checklist

- [ ] I can explain vibe coding, spec and engineering loop in one sentence each.
- [ ] I can turn "Make a plans page" into a strong spec with where/validate/constraints.
- [ ] I can describe the loop `write → check → read error → fix` with a real example (`TS2322`).
- [ ] I know why worktrees enable conflict-free multi-agent (and what target vs working branch means).
- [ ] I recognize the glossary terms (spec, multi-agent, glossary and best practices) when they appear in ch. 03–08.
- [ ] I know what context engineering is and how Vibe Kanban injects/prunes context (AGENTS.md layers, pipeline pointer, attachments).
- [ ] I know what autocompact is and which watchdog handles it (OrchestratorCompactor, 400k tokens, /compact via keys).
- [ ] I know the difference between semantic memory (memory_search) and graph memory (memory_graph_traverse), and what to save vs not.
- [ ] I can use memory_check_staleness before trusting old memory.
- [ ] I can narrate the 2-minute flow (spec → workspace → loop → review → merge) without opening the book.
