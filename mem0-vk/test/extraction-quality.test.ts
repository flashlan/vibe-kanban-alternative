/**
 * mem0-vk — entity/relation extraction quality test, against the LIVE
 * running container's REAL extraction LLM (whatever provider/key is
 * configured — groq, openrouter, llama, openai; see `GET /api/config`).
 *
 * Unlike the other test files, this one does NOT spawn an isolated child
 * process: extraction quality depends on real API keys this test must never
 * see or need (the live container already has them loaded). It hits
 * `localhost:8000` directly, scoped to a dedicated `user_id` so it never
 * touches real project memories (same isolation Qdrant payload-filter
 * already proven by test.ts's "isolation: other user sees 0" check), and
 * cleans that user_id up afterward.
 *
 * This is a QUALITY smoke test, not a correctness oracle: an LLM's exact
 * extraction wording is not deterministic. Assertions are deliberately
 * lenient (substring/keyword presence, not exact match) — the point is to
 * catch a badly broken extraction path (empty results, garbage JSON, wrong
 * ballpark), not to grade prose quality turn by turn.
 *
 * KNOWN, OBSERVED non-determinism: the free-tier extraction model (e.g.
 * groq's qwen3.6-27b) sometimes returns a syntactically valid `facts` array
 * but EMPTY `entities`/`relations` arrays for a given input — confirmed by
 * running this test repeatedly against a live groq-backed container: one run
 * had 3/3 cases produce entities/relations, another had only 1/3, with only
 * one of those failures actually logged server-side (`[llm] ... no JSON
 * object, failing over` — src/index.ts's `llmChat`); the rest were silent
 * (valid JSON, just an empty graph). This is a real characteristic of the
 * model, not a code bug, so per-case entities/relations checks are
 * majority-based (see MIN_GRAPH_SUCCESS_RATIO below) rather than all-or-
 * nothing — a single flaky call must not fail the whole suite, but the
 * extraction path going systematically graph-empty still will.
 *
 * Skips (exit 0, not a failure) if the live container is unreachable or has
 * no extraction provider configured — this depends on infra/secrets outside
 * the test's control, unlike context-drift.test.ts / semantic-recall.test.ts
 * which are fully self-contained.
 *
 * Requires: the mem0-vk stack running with a real extraction key
 * (`docker compose up -d --build` in mem0-vk/, with GROQ_API_KEY or
 * equivalent set in mem0-vk/.env).
 *
 * Run:  npm run test:extraction
 */
import { isReachable, makeChecker } from "./harness.js";

const BASE = process.env.MEM0_VK_URL || "http://localhost:8000";
const USER = `test-extraction-quality-${Date.now()}`;

const { check, summary } = makeChecker();

// Fraction of cases that must come back with a non-empty entities/relations
// graph. Below this, the extraction path is broken systematically (not just
// one flaky call) — see the file header's KNOWN non-determinism note for why
// this isn't 1.0.
const MIN_GRAPH_SUCCESS_RATIO = 0.5;

interface Case {
  name: string;
  content: string;
  /** At least this many of these keywords must appear (case-insensitive)
   * somewhere across the extracted entities' names/descriptions or
   * relations' subject/predicate/object. */
  expectAnyOf: string[];
  minMatches: number;
}

const cases: Case[] = [
  {
    name: "auth rewrite",
    content:
      "The auth module was rewritten to use JWT tokens instead of session cookies. The backend team made this change to fix a session-replay vulnerability.",
    expectAnyOf: ["jwt", "auth", "session", "token", "backend"],
    minMatches: 2,
  },
  {
    name: "qdrant storage",
    content:
      "Qdrant stores vector embeddings for the mem0 memory system. Each point holds a payload with content, user_id, and a timestamp.",
    expectAnyOf: ["qdrant", "vector", "embedding", "mem0", "payload"],
    minMatches: 2,
  },
  {
    name: "tui cockpit",
    content:
      "The TUI cockpit is a separate Rust crate that lets a single developer drive coding agents from the terminal without opening a browser.",
    expectAnyOf: ["tui", "rust", "cockpit", "terminal", "agent"],
    minMatches: 2,
  },
];

async function store(content: string) {
  const r = await fetch(`${BASE}/api/memories`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ content, user_id: USER }),
  });
  return r.json() as Promise<{
    ok: boolean;
    stored: string[];
    ids: string[];
    entities: number;
    relations: number;
    graph: boolean;
  }>;
}

async function fetchStoredPayloads(): Promise<
  Map<string, { content: string; entities: string[]; relations: string[] }>
> {
  const r = await fetch(`${BASE}/api/memories/${USER}`);
  const j = await r.json();
  const byId = new Map<
    string,
    { content: string; entities: string[]; relations: string[] }
  >();
  for (const m of j.memories ?? []) {
    byId.set(m.id, {
      content: m.payload?.content ?? "",
      entities: m.payload?.entities ?? [],
      relations: m.payload?.relations ?? [],
    });
  }
  return byId;
}

async function main() {
  console.log(`mem0-vk extraction-quality test\n  app:  ${BASE}\n  user: ${USER}\n`);

  const reachable = await isReachable(`${BASE}/health`);
  if (!reachable) {
    console.log(
      `mem0-vk not reachable at ${BASE} — skipping (this test needs the live stack running).`
    );
    process.exit(0);
  }

  const cfg = await (await fetch(`${BASE}/api/config`)).json();
  const provider = cfg.provider as string | undefined;
  const hasKey = provider ? Boolean(cfg.providers?.[provider]?.has_key) : false;
  if (!provider || !hasKey) {
    console.log(
      `No extraction provider/key configured (provider=${provider}, has_key=${hasKey}) — skipping. Set an extraction key in mem0-vk/.env to run this test.`
    );
    process.exit(0);
  }
  console.log(`Extraction provider: ${provider} (${cfg.providers[provider].model})\n`);

  // Async on purpose — `process.exit()` tears the process down before an
  // in-flight promise started inside a synchronous `process.on('exit', ...)`
  // handler can complete, so every call site below awaits cleanup() itself
  // rather than relying on an exit listener (see test.ts for the same note).
  const cleanup = async () => {
    await fetch(`${BASE}/api/memories/${USER}`, { method: "DELETE" }).catch(() => {});
  };

  // Extraction can split one input into several facts (memoryStore in
  // src/index.ts loops `for (const fact of facts)`), each its own Qdrant
  // point — but every fact from the same store() call shares the same
  // entities/relations payload. Track ids per case (not content-prefix
  // matching, which breaks once extraction paraphrases the input) so recall
  // below can look any of them up.
  const idsByCase = new Map<string, string[]>();
  let totalIdsExpected = 0;
  let graphSuccessCount = 0;

  for (const c of cases) {
    const res = await store(c.content);
    check(
      `${c.name}: extraction produced facts`,
      Array.isArray(res.stored) && res.stored.length > 0,
      JSON.stringify(res)
    );
    const hasGraph = (res.entities ?? 0) > 0 || (res.relations ?? 0) > 0;
    if (hasGraph) graphSuccessCount++;
    console.log(
      `${c.name}: entities=${res.entities} relations=${res.relations}${hasGraph ? "" : " (empty this run — see KNOWN non-determinism note)"}`
    );
    idsByCase.set(c.name, res.ids ?? []);
    totalIdsExpected += res.ids?.length ?? 0;
    // Pace calls: free-tier providers (groq/openrouter) have low per-minute
    // token budgets — mirrors reExtractGraph's pacing in src/index.ts.
    await new Promise((r) => setTimeout(r, 1200));
  }

  check(
    `at least ${Math.ceil(cases.length * MIN_GRAPH_SUCCESS_RATIO)}/${cases.length} cases produced a non-empty graph`,
    graphSuccessCount >= Math.ceil(cases.length * MIN_GRAPH_SUCCESS_RATIO),
    `graphSuccessCount=${graphSuccessCount}/${cases.length}`
  );

  const stored = await fetchStoredPayloads();
  check(
    "recall returns every point store() reported",
    stored.size === totalIdsExpected,
    `recalled=${stored.size} expected=${totalIdsExpected}`
  );

  for (const c of cases) {
    const ids = idsByCase.get(c.name) ?? [];
    const point = ids.map((id) => stored.get(id)).find((p) => p != null);
    const hasGraph = (point?.entities.length ?? 0) > 0 || (point?.relations.length ?? 0) > 0;
    if (!hasGraph) {
      console.log(`\n${c.name}: skipping keyword-match check — no graph output this run.`);
      continue;
    }
    const haystack = `${point!.entities.join(" ")} ${point!.relations.join(" ")}`.toLowerCase();
    const matched = c.expectAnyOf.filter((kw) => haystack.includes(kw));
    console.log(
      `\n${c.name}:\n  entities:  ${JSON.stringify(point!.entities)}\n  relations: ${JSON.stringify(point!.relations)}\n  matched keywords: ${JSON.stringify(matched)} (need >= ${c.minMatches})`
    );
    check(
      `${c.name}: extracted graph mentions >= ${c.minMatches} expected keyword(s)`,
      matched.length >= c.minMatches,
      `matched=${JSON.stringify(matched)} entities=${JSON.stringify(point!.entities)} relations=${JSON.stringify(point!.relations)}`
    );
  }

  const { passed, failed, failures } = summary();
  console.log(`\n${passed}/${passed + failed} checks passed`);
  if (failures.length) {
    console.log("Failed:");
    for (const f of failures) console.log(`  - ${f}`);
  }
  await cleanup();
  process.exit(failed === 0 ? 0 : 1);
}

main().catch((err) => {
  console.error("\nFATAL:", err);
  process.exit(2);
});
