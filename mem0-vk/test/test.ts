/**
 * mem0-vk — HTTP smoke test (REST + MCP Streamable-HTTP).
 *
 * Spawns:
 *   1. a stub LLM/embedding server (OpenAI-format) in-process — no external deps
 *   2. the built server (dist/index.js) with a unique Qdrant collection
 *
 * Requires a reachable Qdrant at $QDRANT_URL (default http://localhost:6333).
 *
 * Run:  npm run build && npm test
 */
import { QDRANT_URL, DIM, startStub, startApp, waitUp, makeChecker } from "./harness.js";

// ── Config ────────────────────────────────────────────────────────────────────
const APP_PORT = 18123;
const STUB_PORT = 18124;
const BASE = `http://127.0.0.1:${APP_PORT}`;
const COLLECTION = `test-${Date.now()}`;
const USER = "test-http";

const { check, summary } = makeChecker();

// ── MCP client (stateless — one request per round-trip) ──────────────────────
class McpClient {
  private nextId = 1;
  async call(method: string, params: unknown): Promise<any> {
    const id = this.nextId++;
    const resp = await fetch(`${BASE}/mcp`, {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json, text/event-stream" },
      body: JSON.stringify({ jsonrpc: "2.0", id, method, params }),
    });
    const ct = resp.headers.get("content-type") || "";
    const text = await resp.text();
    if (!resp.ok) throw new Error(`MCP ${method} → HTTP ${resp.status}: ${text.slice(0, 300)}`);
    if (ct.includes("application/json")) return JSON.parse(text);
    // SSE: take the first "data:" line
    const line = text.split("\n").find((l) => l.startsWith("data:"));
    if (!line) throw new Error(`MCP ${method} → no SSE data: ${text.slice(0, 300)}`);
    return JSON.parse(line.slice(5).trim());
  }
}

// ── Test run ──────────────────────────────────────────────────────────────────
async function main() {
  console.log(`mem0-vk HTTP test\n  qdrant:    ${QDRANT_URL}\n  app:       ${BASE}\n  collection: ${COLLECTION}\n`);

  // 0. Qdrant reachable?
  let qdrantOk = false;
  try {
    const r = await fetch(`${QDRANT_URL}/collections`);
    qdrantOk = r.ok;
  } catch {}
  check("Qdrant reachable", qdrantOk);
  if (!qdrantOk) {
    console.error("\nQdrant is down — aborting (start it with: docker compose up -d qdrant)");
    process.exit(2);
  }

  const stub = await startStub(STUB_PORT);
  const { proc, stop } = startApp({ appPort: APP_PORT, stubPort: STUB_PORT, collection: COLLECTION });
  // Async on purpose: dropping the test collection is an HTTP DELETE, and
  // `process.exit()` tears the process down before any in-flight promise
  // started inside a synchronous `process.on('exit', ...)` handler can
  // complete — every call site below MUST `await cleanup()` before exiting.
  // The 'exit' listener stays registered only as a sync-only fallback (kills
  // the spawned child) for paths that terminate without reaching an await
  // (e.g. an uncaught throw); it deliberately does not re-attempt the DELETE.
  const cleanup = async () => {
    stop();
    stub.close();
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

  // ── REST ────────────────────────────────────────────────────────────────────
  console.log("REST:");

  const health = await (await fetch(`${BASE}/health`)).json();
  check("GET /health", health.ok === true && health.collection === COLLECTION && health.dim === DIM, JSON.stringify(health));

  const idx = await (await fetch(`${BASE}/`)).json();
  check("GET / index lists endpoints", Boolean(idx.endpoints?.mcp && idx.endpoints?.rest), JSON.stringify(idx).slice(0, 200));

  const store400 = await fetch(`${BASE}/api/memories`, {
    method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ user_id: USER }),
  });
  check("POST /api/memories 400 on missing content", store400.status === 400);

  const store = await fetch(`${BASE}/api/memories`, {
    method: "POST", headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ content: "The test project uses Qdrant on port 6333 and Hono for the API.", user_id: USER }),
  });
  const storeJson = await store.json();
  check("POST /api/memories 201", store.status === 201 && storeJson.ok === true);
  check("store returned 2 facts", Array.isArray(storeJson.stored) && storeJson.stored.length === 2, JSON.stringify(storeJson.stored));
  const memoryId: string | undefined = storeJson.ids?.[0];
  check("store returned a UUID id", Boolean(memoryId && /^[0-9a-f-]{36}$/i.test(memoryId)), memoryId);
  check("store extracted entities", (storeJson.entities ?? 0) >= 1, `entities=${storeJson.entities}`);
  check("store extracted relations", (storeJson.relations ?? 0) >= 1, `relations=${storeJson.relations}`);

  const search400 = await fetch(`${BASE}/api/search`, {
    method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ user_id: USER }),
  });
  check("POST /api/search 400 on missing query", search400.status === 400);

  const search = await (await fetch(`${BASE}/api/search`, {
    method: "POST", headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ query: "Qdrant port", user_id: USER, limit: 5 }),
  })).json();
  check("POST /api/search finds facts", Array.isArray(search.vector) && search.vector.length >= 1, JSON.stringify(search.vector?.map((v: any) => v.payload?.content)));
  check("search reports embedding backend", search.embedding_backend === "local", `backend=${search.embedding_backend}`);

  const recall = await (await fetch(`${BASE}/api/memories/${USER}`)).json();
  check("GET /api/memories/:user_id count=2", recall.count === 2, `count=${recall.count}`);
  check("recall has prompt_block header", typeof recall.prompt_block === "string" && recall.prompt_block.startsWith("--- project memories (2)"), recall.prompt_block?.slice(0, 80));
  check("prompt_block lists fact lines", (recall.prompt_block.match(/^- /gm) || []).length === 2);

  const other = await (await fetch(`${BASE}/api/memories/other-user`)).json();
  check("isolation: other user sees 0", other.count === 0, `count=${other.count}`);

  if (memoryId) {
    const patch404 = await fetch(`${BASE}/api/memories/not-a-uuid`, {
      method: "PATCH", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ content: "x" }),
    });
    check("PATCH unknown id → 404", patch404.status === 404, `status=${patch404.status}`);

    const patch = await fetch(`${BASE}/api/memories/${memoryId}`, {
      method: "PATCH", headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ content: "Updated: the project now uses Qdrant 1.19 and Hono v4." }),
    });
    const patchJson = await patch.json();
    check("PATCH /api/memories/:id ok", patch.ok === true, JSON.stringify(patchJson));

    const recall2 = await (await fetch(`${BASE}/api/memories/${USER}`)).json();
    check("recall count still 2 after update", recall2.count === 2, `count=${recall2.count}`);
  }

  // DELETE point by UUID
  if (memoryId) {
    const del = await (await fetch(`${BASE}/api/memories/${memoryId}`, { method: "DELETE" })).json();
    check("DELETE by UUID → scope=point", del.scope === "point" && del.deleted === 1, JSON.stringify(del));
  }
  const recall3 = await (await fetch(`${BASE}/api/memories/${USER}`)).json();
  check("recall count=1 after point delete", recall3.count === 1, `count=${recall3.count}`);

  // DELETE all by user_id (non-UUID path)
  const delAll = await (await fetch(`${BASE}/api/memories/${USER}`, { method: "DELETE" })).json();
  check("DELETE by user_id → scope=user", delAll.scope === "user" && delAll.deleted === 1, JSON.stringify(delAll));
  const recall4 = await (await fetch(`${BASE}/api/memories/${USER}`)).json();
  check("recall count=0 after delete-all", recall4.count === 0, `count=${recall4.count}`);

  // ── MCP ─────────────────────────────────────────────────────────────────────
  console.log("MCP:");
  const mcp = new McpClient();

  const init1 = await mcp.call("initialize", {
    protocolVersion: "2025-03-26",
    capabilities: {},
    clientInfo: { name: "test", version: "1" },
  });
  check("MCP initialize #1", Boolean(init1.result?.serverInfo?.name === "mem0-vk"), JSON.stringify(init1).slice(0, 200));

  const init2 = await mcp.call("initialize", {
    protocolVersion: "2025-03-26",
    capabilities: {},
    clientInfo: { name: "test", version: "1" },
  });
  check("MCP initialize #2 (stateless — no 'already connected')", Boolean(init2.result?.serverInfo), JSON.stringify(init2).slice(0, 200));

  const tools = await mcp.call("tools/list", {});
  const toolNames: string[] = tools.result?.tools?.map((t: any) => t.name) || [];
  check("MCP tools/list = 5 tools", toolNames.length === 5, toolNames.join(","));
  for (const n of ["memory_store", "memory_search", "memory_recall", "memory_update", "memory_forget"]) {
    check(`MCP has tool ${n}`, toolNames.includes(n));
  }

  const mStore = await mcp.call("tools/call", {
    name: "memory_store",
    arguments: { content: "MCP test: the API uses Hono and the vector store is Qdrant.", user_id: USER },
  });
  const mStoreText = mStore.result?.content?.[0]?.text || "";
  check("MCP memory_store", mStoreText.includes("Stored 2 fact(s)"), mStoreText.slice(0, 200));
  const mStoreId = (mStoreText.match(/\[([0-9a-f-]{36})\]/i) || []) [1];

  const mSearch = await mcp.call("tools/call", {
    name: "memory_search",
    arguments: { query: "Hono Qdrant", user_id: USER, limit: 5 },
  });
  const mSearchText = mSearch.result?.content?.[0]?.text || "";
  check("MCP memory_search hits", mSearchText.includes("Vector hits:") && !mSearchText.includes("No vector hits"), mSearchText.slice(0, 200));

  const mRecall = await mcp.call("tools/call", {
    name: "memory_recall",
    arguments: { user_id: USER },
  });
  const mRecallText = mRecall.result?.content?.[0]?.text || "";
  check("MCP memory_recall prompt_block", mRecallText.startsWith("--- project memories (2)") && mRecallText.includes("- stub fact"), mRecallText.slice(0, 160));

  if (mStoreId) {
    const mUpdate = await mcp.call("tools/call", {
      name: "memory_update",
      arguments: { memory_id: mStoreId, content: "Updated MCP fact: the API uses Hono v4." },
    });
    check("MCP memory_update", (mUpdate.result?.content?.[0]?.text || "").includes("updated"), (mUpdate.result?.content?.[0]?.text || "").slice(0, 160));
  }

  const mForgetAll = await mcp.call("tools/call", {
    name: "memory_forget",
    arguments: { user_id: USER },
  });
  check("MCP memory_forget (user)", (mForgetAll.result?.content?.[0]?.text || "").includes("deleted"), (mForgetAll.result?.content?.[0]?.text || "").slice(0, 120));

  // ── Summary ─────────────────────────────────────────────────────────────────
  const { passed, failed, failures } = summary();
  console.log(`\n${passed}/${passed + failed} checks passed`);
  if (failures.length) {
    console.log("Failed:");
    for (const f of failures) console.log(`  - ${f}`);
  }
  await cleanup();
  process.exit(failed === 0 ? 0 : 1);
}

main().catch(async (err) => {
  console.error("\nFATAL:", err);
  process.exit(2);
});
