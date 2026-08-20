import os
from fastapi import FastAPI
from mem0 import Memory
from dotenv import load_dotenv

load_dotenv()

app = FastAPI(title="mem0-vk")

config = {
    "version": "v1.1",
    "embedder": {
        "provider": "huggingface",
        "config": {"model": "sentence-transformers/all-MiniLM-L6-v2"},
    },
    "vector_store": {
        "provider": "qdrant",
        "config": {
            "host": os.getenv("QDRANT_URL", "http://localhost:6333"),
        },
    },
}

memory = Memory.from_config(config)


@app.get("/api/memories/{user_id}")
def get_memories(user_id: str):
    memories = memory.get_all(user_id=user_id)
    return {"count": len(memories.get("results", [])), "memories": memories.get("results", [])}


@app.post("/api/memories")
def add_memory(data: dict):
    result = memory.add(data["content"], user_id=data["user_id"])
    return {"ok": True, "results": result.get("results", [])}


@app.post("/api/search")
def search_memory(data: dict):
    results = memory.search(data["query"], user_id=data["user_id"])
    return {"vector": results.get("results", [])}


@app.post("/api/re-extract/{user_id}")
def re_extract(user_id: str):
    return {"ok": True, "message": f"Re-extraction triggered for {user_id}"}


@app.get("/api/usage/tokens")
def token_usage():
    return {"tokens": 0}


@app.get("/api/config")
def get_config():
    return {"config": config}


@app.post("/api/config")
def update_config(data: dict):
    return {"ok": True}
