# mem0-vk

Camada de memória compartilhada para agentes de código — MCP (Streamable-HTTP) + REST, Qdrant, grafos opcionais. Zero GPU: embeddings locais por sentence-transformers (CPU).

## Arquitetura

```
                          ┌──────────────────────────────────────┐
   ~10 CLI agents (vk)    │  Container 1: mem0-vk (Node 20)      │
                          │  ┌────────────────────────────────┐  │
 ┌──────────────┐  /mcp   │  │ MCP Streamable-HTTP  (Hono)    │  │
 │ Qwen Code    │────────▶│  ├────────────────────────────────┤  │
 │ Claude Code  │  /api/* │  │ REST  /api/memories …          │  │
 │ Codex        │────────▶│  ├────────────────────────────────┤  │
 │ opencode …   │         │  │ resolvers:                     │  │
 └──────────────┘         │  │  • LLM extração (1 ativo)      │  │
                          │  │  • embeddings (fallback 4)     │  │
                          │  └──────┬──────────────┬──────────┘  │
                          └─────────┼──────────────┼─────────────┘
                                   ▼              ▼
                          ┌──────────────┐  ┌─────────────────────────────┐
                          │ Qdrant       │  │ Container 2 (Python, opc.)  │
                          │ (vectors)    │  │ sentence-transformers (CPU) │
                          │ user_id/repo │  │ NetworkX graph              │
                          └──────────────┘  └─────────────────────────────┘
```

- **Container 1 (`mem0-vk`)** — o endpoint. Expõe MCP e REST sobre a MESMA memória Qdrant, particionada por `user_id` (slug do repo).
- **Qdrant** — store de vetores; um ponto por fato, com payload `{content, user_id, created_at, entities, relations}`.
- **Container 2 (Python, opcional)** — sentence-transformers em CPU servindo `/v1/embeddings` (formato OpenAI) + API de grafos NetworkX. Enquanto não existe, o sistema roda em modo vector-only com qualquer backend de embedding configurado.

## Como o isolamento por repo funciona

Cada repo tem um slug (ex.: `kiky-android`, `kiky-ocr`) definido no AGENTS.md. Os agentes passam esse slug como `user_id`. O server **nunca** injeta o `user_id` sozinho — quem chama sabe em que repo está rodando.

```
memory_store("Android usa Kotlin", "kiky-android")
memory_store("Python usa tipagem dinâmica", "kiky-ocr")
memory_search("tipagem", "kiky-ocr")  → só retorna de kiky-ocr
```

## Quick Start

### 1. Inicie o Docker (se o daemon estiver parado)

| Runtime | Comando | Aguarde |
|---|---|---|
| OrbStack (macOS) | `open -a OrbStack` | ~5 s |
| Docker Desktop (macOS) | `open -a Docker` | ~15–30 s |
| Linux (systemd) | `sudo systemctl start docker` | imediato |

Confira que o daemon está no ar:

```bash
docker info >/dev/null && echo "daemon ok"
```

### 2. Suba os containers

```bash
cd mem0-vk
export GROQ_API_KEY=gsk_...        # ou outro provider (ver tabela abaixo)
docker compose up -d --build
docker compose ps                 # qdrant e mem0-vk devem estar "Up"
```

Saúde:

```bash
curl localhost:8000/health
curl localhost:8000/               # índice de endpoints + status
```

### REST

```bash
# armazenar
curl -X POST localhost:8000/api/memories \
  -H 'content-type: application/json' \
  -d '{"content":"O build usa ./gradlew assembleDebug","user_id":"kiky-android"}'

# buscar
curl -X POST localhost:8000/api/search \
  -H 'content-type: application/json' \
  -d '{"query":"como fazer build","user_id":"kiky-android"}'

# listar tudo de um repo (resposta inclui prompt_block — ver "Injeção no prompt")
curl localhost:8000/api/memories/kiky-android

# atualizar
curl -X PATCH localhost:8000/api/memories/<memory_id> \
  -H 'content-type: application/json' \
  -d '{"content":"O build usa ./gradlew bundleRelease","user_id":"kiky-android"}'

# apagar um ponto (UUID)
curl -X DELETE localhost:8000/api/memories/<memory_id>

# apagar tudo de um repo (qualquer string que não seja UUID = user_id)
curl -X DELETE localhost:8000/api/memories/kiky-android
```

### MCP (Streamable-HTTP)

Configuração para hosts MCP com suporte HTTP (Qwen Code, Claude Code, Cursor, opencode…):

```json
{
  "mcpServers": {
    "mem0": {
      "type": "streamableHttp",
      "url": "http://localhost:8000/mcp"
    }
  }
}
```

Tools expostas: `memory_store`, `memory_search`, `memory_recall`, `memory_update`, `memory_forget`.

> **Nota:** o Codex só suporta MCP via stdio — para ele, use a API REST (mesma memória, mesmo Qdrant).

## Injeção no prompt (KV-cache friendly)

`memory_recall` (MCP) e `GET /api/memories/:user_id` (REST) retornam as memórias **já formatadas** num bloco pronto para injetar:

```
--- project memories (3) — inject after stable prefix to keep KV cache ---
- O build usa ./gradlew assembleDebug
- Qdrant roda na porta 6333
- Pipeline: ocr → re-image → re-document
```

Estrutura de prompt recomendada (mantém o prefixo estável e preserva o cache de KV do LLM):

```
{system_message}          ← prefixo estável
{conversation_history}    ← prefixo estável
{prompt_block}            ← memórias SEMPRE no fim, após o prefixo
```

Se as memórias forem injetadas no **início** do prompt, o prefixo muda a cada request e o cache de KV quebra. Injetando no **fim**, o prefixo fica idêntico entre turns e o cache é reaproveitado.

## Variáveis de ambiente

### Server

| Variável | Padrão | Descrição |
|---|---|---|
| `PORT` | `8000` | Porta HTTP |
| `HOST` | `0.0.0.0` | Bind address |
| `QDRANT_URL` | `http://qdrant:6333` | URL do Qdrant |
| `MEM0_COLLECTION` | `mem0-vk` | Nome da collection |
| `MEM0_DEFAULT_USER` | `default` | `user_id` quando o caller não envia |
| `EMBED_DIM` | `768` | **Deve casar** com o dim do backend de embedding vencedor |

### LLM de extração (1 ativo, via `MEM0_LLM_PROVIDER`)

| Provider | Variáveis | Modelo padrão |
|---|---|---|
| `groq` (padrão) | `GROQ_API_KEY`, `GROQ_MODEL` | `llama-3.3-70b-versatile` |
| `openrouter` | `MEM0_OPENROUTER_KEY`, `MEM0_OPENROUTER_MODEL` | `nvidia/nemotron-3-nano-30b-a3b:free` |
| `llama` | `MEM0_LLAMA_URL`, `MEM0_LLAMA_KEY`, `MEM0_LLAMA_MODEL` | — |

Se o LLM falhar, a extração faz fallback para o texto bruto (1 fato = texto inteiro).

### Embeddings (fallback em ordem; primeiro com sucesso vence)

| Ordem | Backend | Variáveis |
|---|---|---|
| 1 | local (sentence-transformers, CPU) | `EMBED_LOCAL_URL`, `EMBED_LOCAL_MODEL` |
| 2 | llama-server | `EMBED_LLAMA_URL`, `EMBED_LLAMA_MODEL`, `EMBED_LLAMA_KEY` |
| 3 | OpenAI | `OPENAI_API_KEY`, `EMBED_OPENAI_MODEL` |
| 4 | OpenRouter | `EMBED_OPENROUTER_KEY`, `EMBED_OPENROUTER_MODEL` |

Todos os backends falam formato OpenAI: `POST {base}/embeddings` com `{model, input: [text]}`.

**Dimensões comuns:**

| Modelo | Dim |
|---|---|
| `sentence-transformers/all-MiniLM-L6-v2` | 384 |
| `nomic-embed-text` | 768 |
| `text-embedding-3-small` | 1536 |

Se a dim do backend não casar com `EMBED_DIM`, ele é rejeitado e o fallback continua.

### Grafos (opcional)

| Variável | Descrição |
|---|---|
| `GRAPH_URL` | Base URL do container Python com a API de grafos. Vazio = vector-only. |

Endpoints esperados no container Python (todos com `user_id` no body para isolamento):

```
POST /graph/upsert        {user_id, entities:[{name,type,description}], relations:[{subject,predicate,object}]}
POST /graph/neighbors     {user_id, query} → [{name, description, score}]
POST /graph/relations     {user_id, query} → [{subject,predicate,object}]
POST /graph/remove_node   {user_id, name}
POST /graph/remove_user   {user_id}
```

## Comandos

```bash
docker compose up -d          # iniciar (dados persistem)
docker compose down           # parar
docker compose down -v        # parar + apagar Qdrant
docker compose logs -f        # logs
docker compose ps             # status
```

## Estrutura

```
mem0-vk/
├── src/
│   └── index.ts                    # endpoint: MCP + REST + resolvers + proxy de grafos
├── test/
│   ├── harness.ts                  # boot/teardown compartilhado (stub e embedding real)
│   ├── test.ts                     # smoke test HTTP (REST + MCP), embedding stub
│   ├── context-drift.test.ts       # perda/drift de contexto no handoff, embedding stub
│   ├── semantic-recall.test.ts     # recall semântico real (embedding real, sem custo)
│   └── extraction-quality.test.ts  # qualidade da extração LLM real (localhost:8000, groq/etc.)
├── embeddings/          # container Python sentence-transformers (CPU) + grafo NetworkX
├── Dockerfile           # Node 20 slim, EXPOSE 8000
├── docker-compose.yml   # qdrant + mem0-vk + embeddings
├── package.json
├── tsconfig.json
└── README.md
```

## Testes

```bash
npm run build

npm test              # smoke test HTTP+MCP (stub — sem custo, sem chaves)
npm run test:drift    # mecanismo de ranking/dedup/cap sob ruído sintético (stub)
npm run test:semantic # recall real (embeddings/ sidecar — grátis, CPU local)
npm run test:extraction # qualidade da extração de entidades/relações (LLM real — precisa de chave configurada)
```

`test`, `test:drift` e `test:semantic` são auto-contidos (sobem seu próprio processo isolado) e não precisam de chaves de API. `test:extraction` bate direto no container já rodando em `localhost:8000` — pula (não falha) se o container não estiver de pé ou sem provedor de extração configurado.

## Licença

MIT
