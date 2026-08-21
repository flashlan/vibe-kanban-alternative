"""
mem0-vk embeddings + graph container.

FastAPI service providing:
  • /v1/embeddings  — OpenAI-format embeddings via sentence-transformers (CPU)
  • /graph/*        — in-memory NetworkX graph (one graph per user_id)
  • /health         — liveness + loaded model info

The mem0-vk Node server calls this as its "local" embedding backend
(EMBED_LOCAL_URL) and, when GRAPH_URL is set, as the graph store.
"""

from __future__ import annotations

import logging
import os
import re
import threading
from dataclasses import dataclass, field
from typing import Any

import fastapi
from fastapi import FastAPI
from pydantic import BaseModel, Field
from sentence_transformers import SentenceTransformer
from starlette.responses import JSONResponse

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")
log = logging.getLogger("embeddings")

MODEL_NAME = os.environ.get("ST_MODEL", "sentence-transformers/all-MiniLM-L6-v2")
PORT = int(os.environ.get("PORT", "8001"))
# Directory where per-user graphs are persisted as GraphML files
# (`<user_id>.graphml`). Mount a volume here to survive container restarts.
GRAPH_DIR = os.environ.get("GRAPH_DIR", "/data/graphs")

app = FastAPI(title="mem0-vk embeddings + graph", version="1.0.0")

# ── Model ─────────────────────────────────────────────────────────────────────

_model: SentenceTransformer | None = None
_model_lock = threading.Lock()


def get_model() -> SentenceTransformer:
    global _model
    if _model is None:
        with _model_lock:
            if _model is None:
                log.info("loading %s (CPU)…", MODEL_NAME)
                _model = SentenceTransformer(MODEL_NAME, device="cpu")
                log.info("model loaded: dim=%d", _model.get_sentence_embedding_dimension())
    return _model


def dim() -> int:
    return int(get_model().get_sentence_embedding_dimension())


# ── Embeddings (OpenAI-compatible) ────────────────────────────────────────────

class EmbedRequest(BaseModel):
    input: str | list[str]
    model: str | None = None


class EmbeddingData(BaseModel):
    object: str = "embedding"
    index: int
    embedding: list[float]


class EmbedResponse(BaseModel):
    object: str = "list"
    data: list[EmbeddingData]
    model: str
    usage: dict[str, int]


@app.get("/v1/models")
def list_models():
    return {"object": "list", "data": [{"id": MODEL_NAME, "object": "model"}]}


@app.post("/v1/embeddings", response_model=EmbedResponse)
def embeddings(req: EmbedRequest):
    model = get_model()
    texts = [req.input] if isinstance(req.input, str) else req.input
    vectors = model.encode([t for t in texts], normalize_embeddings=True)
    return EmbedResponse(
        data=[EmbeddingData(index=i, embedding=[float(x) for x in v]) for i, v in enumerate(vectors)],
        model=MODEL_NAME,
        usage={"prompt_tokens": 0, "total_tokens": 0},
    )


# ── Graph (NetworkX, in-memory, one DiGraph per user_id) ─────────────────────

class Entity(BaseModel):
    name: str
    type: str = "other"
    description: str = ""


class Relation(BaseModel):
    subject: str
    predicate: str
    object: str


class UpsertRequest(BaseModel):
    user_id: str
    entities: list[Entity] = Field(default_factory=list)
    relations: list[Relation] = Field(default_factory=list)


class QueryRequest(BaseModel):
    user_id: str
    query: str = ""


class RemoveNodeRequest(BaseModel):
    node_id: str


class RemoveUserRequest(BaseModel):
    user_id: str


@dataclass
class GraphStore:
    graphs: dict[str, Any] = field(default_factory=dict)
    lock: threading.Lock = field(default_factory=threading.Lock)

    def get(self, user_id: str):
        import networkx as nx

        with self.lock:
            g = self.graphs.get(user_id)
            if g is None:
                g = self._load(user_id) or nx.DiGraph()
                self.graphs[user_id] = g
            return g

    def _path(self, user_id: str) -> str:
        import re as _re

        safe = _re.sub(r"[^A-Za-z0-9._-]", "_", user_id) or "default"
        return os.path.join(GRAPH_DIR, f"{safe}.graphml")

    def _load(self, user_id: str):
        import networkx as nx

        path = self._path(user_id)
        try:
            if os.path.exists(path):
                log.info("loading graph from %s", path)
                return nx.read_graphml(path)
        except Exception as e:  # noqa: BLE001
            log.error("failed to load graph %s: %s", path, e)
        return None

    def _persist(self, user_id: str) -> None:
        import networkx as nx

        g = self.graphs.get(user_id)
        if g is None:
            return
        try:
            os.makedirs(GRAPH_DIR, exist_ok=True)
            nx.write_graphml(g, self._path(user_id))
        except Exception as e:  # noqa: BLE001
            log.error("failed to persist graph %s: %s", user_id, e)

    def upsert(self, user_id: str, req: "UpsertRequest") -> dict:
        import networkx as nx

        g = self.get(user_id)
        added_nodes = 0
        for e in req.entities:
            name = _norm(e.name)
            if not name:
                continue
            if g.has_node(name):
                g.nodes[name].setdefault("description", "")
                if e.description:
                    g.nodes[name]["description"] = e.description
            else:
                g.add_node(name, type=e.type, description=e.description)
                added_nodes += 1
        added_edges = 0
        for r in req.relations:
            s, o = _norm(r.subject), _norm(r.object)
            if not s or not o:
                continue
            if not g.has_node(s):
                g.add_node(s, type="other", description="")
            if not g.has_node(o):
                g.add_node(o, type="other", description="")
            if not g.has_edge(s, o):
                g.add_edge(s, o, predicate=r.predicate)
                added_edges += 1
            else:
                g[s][o]["predicate"] = r.predicate
        self._persist(user_id)
        return {"ok": True, "user_id": user_id, "nodes_added": added_nodes, "edges_added": added_edges}

    def remove_user(self, user_id: str) -> dict:
        with self.lock:
            g = self.graphs.pop(user_id, None)
            n = g.number_of_nodes() if g else 0
        try:
            path = self._path(user_id)
            if os.path.exists(path):
                os.remove(path)
        except Exception as e:  # noqa: BLE001
            log.error("failed to delete graph file %s: %s", user_id, e)
        return {"ok": True, "removed": g is not None, "nodes": n}


graph = GraphStore()


def _norm(name: str) -> str:
    return re.sub(r"\s+", " ", (name or "").strip())


@app.get("/health")
def health():
    return {"ok": True, "model": MODEL_NAME, "dim": dim(), "users": len(graph.graphs)}


@app.get("/graph/stats")
def graph_stats(user_id: str | None = None):
    if user_id:
        g = graph.get(user_id)
        return {"user_id": user_id, "nodes": g.number_of_nodes(), "edges": g.number_of_edges()}
    return {
        "users": [
            {"user_id": u, "nodes": g.number_of_nodes(), "edges": g.number_of_edges()}
            for u, g in graph.graphs.items()
        ]
    }


@app.post("/graph/upsert")
def graph_upsert(req: UpsertRequest):
    return graph.upsert(req.user_id, req)


@app.post("/graph/neighbors")
def graph_neighbors(req: QueryRequest):
    g = graph.get(req.user_id)
    q = _norm(req.query).lower()
    out = []
    for name, attrs in g.nodes(data=True):
        if not q or q in name.lower() or q in (attrs.get("description") or "").lower():
            out.append({"id": name, "name": name, "type": attrs.get("type", "other"), "description": attrs.get("description", "")})
    # expand one hop from matched nodes
    matched = {n["id"] for n in out}
    seen = set(matched)
    for n in list(out):
        for nb in g.neighbors(n["id"]):
            if nb not in seen:
                seen.add(nb)
                a = g.nodes[nb]
                out.append({"id": nb, "name": nb, "type": a.get("type", "other"), "description": a.get("description", "")})
    return {"user_id": req.user_id, "neighbors": out}


@app.post("/graph/relations")
def graph_relations(req: QueryRequest):
    g = graph.get(req.user_id)
    q = _norm(req.query).lower()
    out = []
    for (s, o, attrs) in g.edges(data=True):
        hay = f"{s} {o} {attrs.get('predicate', '')}".lower()
        if not q or q in hay:
            out.append({"subject": s, "predicate": attrs.get("predicate", "related_to"), "object": o})
    return {"user_id": req.user_id, "relations": out}


@app.post("/graph/remove_node")
def graph_remove_node(req: RemoveNodeRequest):
    for g in list(graph.graphs.values()):
        if g.has_node(req.node_id):
            g.remove_node(req.node_id)
    return {"ok": True}


@app.post("/graph/remove_user")
def graph_remove_user(req: RemoveUserRequest):
    return graph.remove_user(req.user_id)


if __name__ == "__main__":
    import uvicorn

    get_model()  # preload at startup so /health reports dim without a first request
    uvicorn.run(app, host="0.0.0.0", port=PORT, log_level="info")
