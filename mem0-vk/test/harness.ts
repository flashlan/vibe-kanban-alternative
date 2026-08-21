/**
 * mem0-vk test harness — shared boot/teardown for the server + stub
 * LLM/embedding backend. Used by both test.ts (smoke test) and
 * context-drift.test.ts (recall/drift regression test).
 */
import http from "http";
import { spawn, ChildProcess } from "child_process";
import { setTimeout as sleep } from "timers/promises";

export const QDRANT_URL = process.env.QDRANT_URL || "http://localhost:6333";
export const DIM = 768;

// Deterministic bag-of-words embedding: each lowercased word hashes to a
// fixed bucket, so texts sharing words end up with higher cosine similarity
// and texts sharing none end up near-orthogonal — a positional char
// histogram (the original stub) doesn't have that property, which made
// relevance-ranking tests (context-drift.test.ts) meaningless. Same text
// always returns the same vector; search ordering is stable across runs.
export function stubEmbedding(text: string): number[] {
  const v = new Array(DIM).fill(0);
  const words = text.toLowerCase().match(/[a-z0-9]+/g) || [];
  for (const w of words) {
    let h = 0;
    for (let i = 0; i < w.length; i++) h = (h * 31 + w.charCodeAt(i)) >>> 0;
    v[h % DIM] += 1;
  }
  const norm = Math.sqrt(v.reduce((s, x) => s + x * x, 0)) || 1;
  return v.map((x) => x / norm);
}

export function startStub(stubPort: number): Promise<http.Server> {
  const server = http.createServer((req, res) => {
    if (req.method === "POST" && req.url === "/v1/embeddings") {
      let body = "";
      req.on("data", (c) => (body += c));
      req.on("end", () => {
        const { input } = JSON.parse(body);
        const texts = Array.isArray(input) ? input : [input];
        res.writeHead(200, { "Content-Type": "application/json" });
        res.end(
          JSON.stringify({
            object: "list",
            data: texts.map((t: string, i: number) => ({
              object: "embedding",
              index: i,
              embedding: stubEmbedding(t),
            })),
            model: "stub",
          })
        );
      });
      return;
    }
    if (req.method === "POST" && req.url === "/v1/chat/completions") {
      let body = "";
      req.on("data", (c) => (body += c));
      req.on("end", () => {
        const { messages } = JSON.parse(body);
        const user = messages?.[1]?.content || "";
        const payload = {
          facts: [`stub fact one for: ${user.slice(0, 40)}`, `stub fact two for: ${user.slice(0, 40)}`],
          entities: [
            { name: "Mem0VK", type: "project", description: "memory server under test" },
            { name: "Qdrant", type: "tech", description: "vector store" },
          ],
          relations: [{ subject: "Mem0VK", predicate: "uses", object: "Qdrant" }],
        };
        res.writeHead(200, { "Content-Type": "application/json" });
        res.end(
          JSON.stringify({
            id: "stub",
            object: "chat.completion",
            choices: [{ index: 0, message: { role: "assistant", content: JSON.stringify(payload) }, finish_reason: "stop" }],
          })
        );
      });
      return;
    }
    res.writeHead(404, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ error: "not found" }));
  });

  return new Promise((resolve, reject) => {
    server.listen(stubPort, "127.0.0.1", () => resolve(server));
    server.on("error", reject);
  });
}

export function startApp(opts: {
  appPort: number;
  stubPort: number;
  collection: string;
  graphUrl?: string;
}): { proc: ChildProcess; stop: () => void } {
  const proc = spawn("node", ["dist/index.js"], {
    cwd: new URL("..", import.meta.url).pathname,
    env: {
      ...process.env,
      PORT: String(opts.appPort),
      HOST: "127.0.0.1",
      QDRANT_URL,
      MEM0_COLLECTION: opts.collection,
      EMBED_DIM: String(DIM),
      EMBED_LOCAL_URL: `http://127.0.0.1:${opts.stubPort}/v1`,
      EMBED_LOCAL_MODEL: "stub",
      MEM0_LLM_PROVIDER: "llama",
      MEM0_LLAMA_URL: `http://127.0.0.1:${opts.stubPort}/v1`,
      MEM0_LLAMA_MODEL: "stub",
      GRAPH_URL: opts.graphUrl ?? "",
    },
    stdio: ["ignore", "pipe", "pipe"],
  });
  proc.stderr?.on("data", () => {});
  proc.stdout?.on("data", () => {});
  return {
    proc,
    stop: () => {
      try {
        proc.kill("SIGTERM");
      } catch {}
    },
  };
}

export async function waitUp(url: string, timeoutMs = 15000): Promise<void> {
  const t0 = Date.now();
  while (Date.now() - t0 < timeoutMs) {
    try {
      const r = await fetch(url);
      if (r.ok) return;
    } catch {}
    await sleep(200);
  }
  throw new Error(`timeout waiting for ${url}`);
}

export function makeChecker() {
  let passed = 0;
  let failed = 0;
  const failures: string[] = [];
  function check(name: string, ok: boolean, detail?: string) {
    if (ok) {
      passed++;
      console.log(`  ✓ ${name}`);
    } else {
      failed++;
      failures.push(name);
      console.log(`  ✗ ${name}${detail ? `\n      ${detail}` : ""}`);
    }
  }
  return {
    check,
    summary: () => ({ passed, failed, failures }),
  };
}
