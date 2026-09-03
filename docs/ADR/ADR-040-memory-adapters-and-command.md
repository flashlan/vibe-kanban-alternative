# ADR-040: Memory adapters and `/memory` configuration

## Status

Accepted

## Date

2026-09-03

## Context

The existing memory integration speaks the private `mem0-vk` REST contract. A
Mem0 API key alone is not a drop-in replacement: Mem0 Platform uses a separate
authenticated API, and Qdrant is a vector store rather than a memory/extraction
service. The operator also needs to disable memory calls without uninstalling
the MCP tools and switch between local, shared self-hosted, and managed memory.

## Decision

AuraPunk supports two explicit memory adapters:

- `mem0_vk`: the existing self-hosted REST service, with local/cloud endpoint
  switching, custom extraction-provider configuration, graph support, and
  optional Qdrant URL/API key/dimension settings;
- `mem0_platform`: the official Mem0 Platform API, using `Token <api-key>` and
  `/v3/memories/add/` plus `/v3/memories/search/`.

The `/memory` chat command opens Settings → Memory. That panel can enable or
disable memory endpoints, select the adapter, save the Mem0 key, and configure
Qdrant credentials for the self-hosted adapter. Platform-managed extraction,
embeddings, vector storage, and asynchronous processing are not overridden by
the Qdrant fields.

The server stores this operator configuration in
`~/.vibe-kanban/memory.toml` with mode `0600` on Unix. API responses expose only
`*_key_configured` flags; secret values are never returned to the browser.
Environment variables remain supported as explicit deployment overrides.

## Consequences

- Existing mem0-vk deployments keep their current endpoints and Bearer auth.
- Switching adapters changes the protocol and namespace; it does not migrate
  existing memories between stores.
- Graph traversal and the custom re-extraction endpoint gracefully report no
  result/unsupported when Mem0 Platform is selected.
- Mem0 Platform extraction is managed by Mem0; self-hosted extraction still
  requires a configured LLM provider and an embedding dimension matching the
  Qdrant collection (the project default is 384).
- Project memory can be migrated through a preview-first endpoint exposed in
  Settings → Memory. Migration is scoped to one repository slug, compares
  normalized content, skips duplicates, never deletes the source, and requires
  explicit confirmation before enqueueing writes. The destination always
  reprocesses imported content with its own extraction pipeline; vectors and
  provider-specific graph IDs are not portable.
