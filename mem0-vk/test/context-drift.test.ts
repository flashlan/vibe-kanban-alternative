/**
 * mem0-vk — context handoff drift/loss regression test.
 *
 * Simulates the mem0 side of a multi-agent pipeline handoff (see
 * docs/ADR/ADR-028-mem0-agentic-recall.md, ADR-029): stage N calls
 * `memory_save` for facts relevant to the current task; stage N+1 calls
 * `memory_search` with a task-scoped query and gets back the top `limit`
 * ranked hits (see crates/mcp/src/task_server/tools/mem0.rs).
 *
 * Two independent things can go wrong at this handoff:
 *   - LOSS: a fact stage N saved for THIS task never makes it into stage
 *     N+1's top-`limit` results (crowded out by unrelated project memory).
 *   - DRIFT: unrelated ("noise") memory outranks the actual signal — the
 *     agent recalls facts that don't belong to the task in front of it.
 *
 * This produces a structured diff (expected signal vs. what was actually
 * retrieved, with scores) and asserts a recall floor, so a future change to
 * the embedding pipeline, ranking, or cap logic that quietly degrades
 * recall shows up as a test failure instead of silent context loss in
 * production. NOTE: the embedding backend here is the same deterministic
 * character-histogram stub used by test.ts — it validates the mem0-vk
 * scoring/ranking MECHANISM, not the semantic quality of whichever real
 * embedding model is configured in production.
 *
 * Run:  npm run build && npm run test:drift
 */
import { QDRANT_URL, DIM, startStub, startApp, waitUp, makeChecker } from "./harness.js";

const APP_PORT = 18133;
const STUB_PORT = 18134;
const BASE = `http://127.0.0.1:${APP_PORT}`;
const USER = "test-drift";
const COLLECTION = `test-drift-${Date.now()}`;
const SEARCH_LIMIT = 5; // matches DEFAULT_SEARCH_LIMIT in crates/mcp/src/task_server/tools/mem0.rs

const { check, summary } = makeChecker();

interface StoreResult {
  ok: boolean;
  ids: string[];
  stored: string[];
}

async function store(content: string): Promise<StoreResult> {
  const r = await fetch(`${BASE}/api/memories`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ content, user_id: USER }),
  });
  return r.json();
}

async function search(query: string, limit: number) {
  const r = await fetch(`${BASE}/api/search`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ query, user_id: USER, limit }),
  });
  return r.json() as Promise<{ vector: { id: string; score: number; payload?: { content?: string } }[] }>;
}

async function main() {
  console.log(`mem0-vk context-drift test\n  qdrant:     ${QDRANT_URL}\n  app:        ${BASE}\n  collection: ${COLLECTION}\n`);

  let qdrantOk = false;
  try {
    qdrantOk = (await fetch(`${QDRANT_URL}/collections`)).ok;
  } catch {}
  check("Qdrant reachable", qdrantOk);
  if (!qdrantOk) {
    console.error("\nQdrant is down — aborting (start it with: docker compose up -d qdrant)");
    process.exit(2);
  }

  const stub = await startStub(STUB_PORT);
  const { proc, stop } = startApp({ appPort: APP_PORT, stubPort: STUB_PORT, collection: COLLECTION });
  const cleanup = () => {
    stop();
    stub.close();
    fetch(`${QDRANT_URL}/collections/${COLLECTION}`, { method: "DELETE" }).catch(() => {});
  };
  process.on("exit", cleanup);

  try {
    await waitUp(`${BASE}/health`);
  } catch (e: any) {
    console.error(`\nApp failed to start: ${e.message}`);
    stop();
    process.exit(2);
  }

  // ── Stage N: save the facts this handoff actually needs ("signal") ─────────
  // The stub extraction LLM echoes the first 40 chars of `content` into each
  // of the 2 facts it emits — keep the marker inside that window.
  const marker = "TASK-7f3";
  const signalStore = await store(`${marker}: refresh auth token on 401`);
  const signalIds = new Set(signalStore.ids ?? []);
  check(
    "stage N: signal facts stored",
    signalIds.size === 2,
    JSON.stringify(signalStore)
  );

  // ── Concurrent project memory: unrelated facts from other tasks/stages ─────
  // ("noise" — what's already accumulated in the repo's shared mem0 memory
  // by the time this handoff happens).
  const noiseTopics = [
    "database migrations run via sqlx",
    "the frontend uses vite and tailwind",
    "ci runs cargo clippy and eslint",
    "pipelines are defined in toml files",
    "the tui cockpit is a separate crate",
    "telegram bridge is send-only",
    "workspaces map to git worktrees",
    "usage dashboard reads execution_processes",
    "pr descriptions are ai-generated",
    "settings persist to a local sqlite db",
    "graph memory uses networkx",
    "qdrant stores one point per fact",
    "embeddings fall back through 4 backends",
    "extraction llm has a failover chain",
    "docker compose builds three services",
  ];
  let noiseCount = 0;
  for (const topic of noiseTopics) {
    const r = await store(topic);
    noiseCount += r.ids?.length ?? 0;
  }
  check("noise pool stored", noiseCount === noiseTopics.length * 2, `noiseCount=${noiseCount}`);

  // ── Stage N+1: task-scoped recall (ADR-028 — scoped query, not a dump) ─────
  const query = `${marker} auth token refresh`;
  const res = await search(query, SEARCH_LIMIT);
  const retrieved = res.vector ?? [];

  const hits = retrieved.filter((r) => signalIds.has(r.id));
  const missed = [...signalIds].filter((id) => !retrieved.some((r) => r.id === id));
  const recall = signalIds.size > 0 ? hits.length / signalIds.size : 0;
  const noiseIntrusions = retrieved.filter((r) => !signalIds.has(r.id));

  const report = {
    query,
    signal_expected: signalIds.size,
    noise_pool: noiseCount,
    search_limit: SEARCH_LIMIT,
    retrieved: retrieved.map((r) => ({
      id: r.id,
      score: Number(r.score?.toFixed(4)),
      signal: signalIds.has(r.id),
      content: r.payload?.content,
    })),
    hits: hits.length,
    missed: missed.length,
    recall,
    noise_intrusions: noiseIntrusions.length,
  };
  console.log("\nStructured diff:\n" + JSON.stringify(report, null, 2));

  // Regression floor: with a distinctive marker shared by query + signal
  // facts, and none of the noise facts, the mechanism should never lose a
  // signal fact to 15 unrelated noise topics within top-5. A future change
  // that breaks ranking/dedup/cap would show up here as recall < 1.
  check("stage N+1 recalls all signal facts (no context loss)", recall === 1, `recall=${recall} missed=${missed.length}`);
  check(
    "no noise fact outranks the marker-scoped signal (no drift)",
    hits.length > 0 && retrieved.slice(0, hits.length).every((r) => signalIds.has(r.id)),
    `top hits: ${retrieved.map((r) => (signalIds.has(r.id) ? "signal" : "noise")).join(",")}`
  );

  const { passed, failed, failures } = summary();
  console.log(`\n${passed}/${passed + failed} checks passed`);
  if (failures.length) {
    console.log("Failed:");
    for (const f of failures) console.log(`  - ${f}`);
  }
  cleanup();
  process.exit(failed === 0 ? 0 : 1);
}

main().catch((err) => {
  console.error("\nFATAL:", err);
  process.exit(2);
});
