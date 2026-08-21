/**
 * mem0-vk — semantic recall quality test, against the REAL embedding
 * backend (docker-compose's `embeddings` sidecar: sentence-transformers
 * all-MiniLM-L6-v2, local CPU, no API key/cost).
 *
 * context-drift.test.ts validates the mem0-vk RANKING MECHANISM (sort,
 * dedupe, cap) using a deterministic bag-of-words stub embedding — it
 * cannot catch a real embedding model regressing, because the stub isn't
 * the model. This file exercises the actual model, in two ways:
 *
 *   1. Sanity check: same signal/noise shape as context-drift.test.ts, but
 *      through the real model — a baseline that should be easy.
 *   2. Adversarial paraphrase: the query shares almost NO literal words
 *      with the signal fact it should retrieve, and the noise pool
 *      includes "lexical decoys" — topics that share surface vocabulary
 *      with the query but are semantically unrelated. A naive keyword-
 *      overlap ranking would fail this; a real semantic embedding should
 *      not. This is the check a bag-of-words stub can never stand in for.
 *
 * Extraction is left unconfigured (see harness.ts's `realEmbedding` mode),
 * so stored content is the raw input text verbatim — no LLM calls, no
 * cost, and the test controls exactly what gets embedded.
 *
 * Requires: Qdrant (`docker compose up -d qdrant`) AND the embeddings
 * sidecar (`docker compose up -d embeddings`) reachable — both already run
 * as part of the normal mem0-vk stack (`docker compose up -d --build`).
 *
 * Run:  npm run build && npm run test:semantic
 */
import {
  QDRANT_URL,
  REAL_EMBED_URL,
  REAL_EMBED_DIM,
  startApp,
  waitUp,
  isReachable,
  makeChecker,
} from "./harness.js";

const APP_PORT = 18143;
const STUB_PORT = 18144; // unused (realEmbedding mode), kept for startApp's shape
const BASE = `http://127.0.0.1:${APP_PORT}`;
const COLLECTION = `test-semantic-${Date.now()}`;
const SEARCH_LIMIT = 5;

const { check, summary } = makeChecker();

async function store(content: string, userId: string) {
  const r = await fetch(`${BASE}/api/memories`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ content, user_id: userId }),
  });
  return r.json() as Promise<{ ok: boolean; ids: string[]; stored: string[] }>;
}

async function search(query: string, userId: string, limit: number) {
  const r = await fetch(`${BASE}/api/search`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ query, user_id: userId, limit }),
  });
  return r.json() as Promise<{
    vector: { id: string; score: number; payload?: { content?: string } }[];
  }>;
}

async function main() {
  console.log(
    `mem0-vk semantic-recall test (real embedding)\n  qdrant:      ${QDRANT_URL}\n  embeddings:  ${REAL_EMBED_URL}\n  app:         ${BASE}\n  collection:  ${COLLECTION}\n`
  );

  const qdrantOk = await isReachable(`${QDRANT_URL}/collections`);
  check("Qdrant reachable", qdrantOk);
  const embedOk = await isReachable(REAL_EMBED_URL.replace(/\/v1$/, "/health"));
  check("Embeddings sidecar reachable", embedOk);
  if (!qdrantOk || !embedOk) {
    console.error(
      "\nRequired service(s) down — aborting. Start them with:\n  docker compose up -d qdrant embeddings"
    );
    process.exit(2);
  }

  const { proc, stop } = startApp({
    appPort: APP_PORT,
    stubPort: STUB_PORT,
    collection: COLLECTION,
    realEmbedding: true,
  });
  // Async on purpose — see test.ts for why `process.on('exit', ...)` can't
  // run the DELETE reliably; every call site below awaits cleanup() itself.
  const cleanup = async () => {
    stop();
    await fetch(`${QDRANT_URL}/collections/${COLLECTION}`, { method: "DELETE" }).catch(() => {});
  };
  process.on("exit", () => stop());

  try {
    await waitUp(`${BASE}/health`);
  } catch (e: any) {
    console.error(`\nApp failed to start: ${e.message}`);
    await cleanup();
    process.exit(2);
  }

  const health = await (await fetch(`${BASE}/health`)).json();
  check(
    "app reports real embedding dim",
    health.dim === REAL_EMBED_DIM,
    `dim=${health.dim}`
  );

  // ── Scenario A: sanity check (marker-based, same shape as context-drift) ──
  {
    const USER = "test-semantic-sanity";
    const marker = "TASK-9c1";
    const signal = await store(`${marker}: refresh auth token on 401`, USER);
    check("scenario A: signal stored", signal.ids?.length === 1, JSON.stringify(signal));
    const signalId = signal.ids?.[0];

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
    ];
    for (const t of noiseTopics) await store(t, USER);

    const res = await search(`${marker} auth token refresh`, USER, SEARCH_LIMIT);
    const retrieved = res.vector ?? [];
    const topIsSignal = retrieved[0]?.id === signalId;
    check(
      "scenario A: signal ranks first with real embedding",
      topIsSignal,
      `top: ${JSON.stringify(retrieved[0])}`
    );

    await fetch(`${BASE}/api/memories/${USER}`, { method: "DELETE" }).catch(() => {});
  }

  // ── Scenario B: adversarial paraphrase, zero literal word overlap ────────
  {
    const USER = "test-semantic-paraphrase";

    // The fact to recall — written with specific, deliberate vocabulary.
    const signal = await store(
      "The nightly backup job compresses the SQLite database and uploads it to encrypted cold storage.",
      USER
    );
    check("scenario B: signal stored", signal.ids?.length === 1, JSON.stringify(signal));
    const signalId = signal.ids?.[0];

    // Lexical decoys: share surface words with the QUERY below ("save",
    // "file", "disk", "protect", "archive") but are semantically unrelated
    // to the signal fact. A keyword-overlap ranker would rank these highly;
    // a real semantic embedding should not.
    const decoys = [
      "Users can save their draft comment before submitting a review.",
      "The onboarding wizard writes a config file to disk on first run.",
      "Branch protection rules require at least one approving review.",
      "The changelog is archived under docs/CHANGELOG-old.md.",
      "Uploading a large attachment shows a progress bar in the chat box.",
    ];
    for (const d of decoys) await store(d, USER);

    // Generic unrelated noise, no shared vocabulary with either side.
    const noise = [
      "The tui cockpit renders kanban columns in a terminal grid.",
      "Antigravity is one of the ten supported coding agents.",
      "Pipelines can pause on a manual-review stage before merging.",
    ];
    for (const n of noise) await store(n, USER);

    // Paraphrase query: zero literal overlap with the stored sentence
    // ("nightly"/"backup"/"compresses"/"SQLite"/"uploads"/"encrypted"/"cold
    // storage" vs. "save"/"safeguard"/"database records"/"protected
    // offsite storage" below) — pure semantic match required.
    const query =
      "How do we safeguard the database records by copying them somewhere protected offsite each night?";
    const res = await search(query, USER, SEARCH_LIMIT);
    const retrieved = res.vector ?? [];
    const report = {
      query,
      retrieved: retrieved.map((r) => ({
        id: r.id,
        score: Number(r.score?.toFixed(4)),
        signal: r.id === signalId,
        content: r.payload?.content,
      })),
    };
    console.log("\nScenario B structured diff:\n" + JSON.stringify(report, null, 2));

    const topIsSignal = retrieved[0]?.id === signalId;
    check(
      "scenario B: paraphrased signal outranks lexical decoys (real semantic match)",
      topIsSignal,
      `top: ${JSON.stringify(retrieved[0]?.payload?.content)}`
    );

    await fetch(`${BASE}/api/memories/${USER}`, { method: "DELETE" }).catch(() => {});
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
