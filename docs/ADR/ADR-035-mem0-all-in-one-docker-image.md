# ADR-035: mem0 All-in-One Docker Distribution

- **Status**: Accepted
- **Date**: 2026-08-25

## Context

The development-oriented `mem0-vk/docker-compose.yml` builds and runs four services: the Node API, Qdrant, Redis, and the Python embeddings/graph service. This separation is useful for development but makes first-time self-hosting require a repository clone and several images.

The project's primary audience is a single developer running everything locally. For that audience, installation simplicity is more important than independently scaling the memory components.

## Decision

Publish `datyapoint/vk-mem0` as an all-in-one Linux image containing:

- the mem0-vk Node API;
- Qdrant;
- Redis;
- the CPU sentence-transformers embeddings service;
- the NetworkX graph service; and
- Supervisor as the process manager.

Only port `8000` is public. Qdrant, Redis, and embeddings bind to or are addressed through the container's loopback interface. A single `/data` volume persists Qdrant vectors, the Redis append-only file, and GraphML files. The image healthcheck verifies all four internal services.

The existing multi-container Compose stack remains the development and advanced deployment option. Docker Hub releases use the application version for immutable tags and update `latest` only for stable versions.

## Consequences

- A user can start the complete memory stack with one `docker run` command.
- Backup and restoration require only the `/data` volume and provider configuration.
- The image is significantly larger because it includes the local embedding model and every runtime.
- Components cannot be scaled or upgraded independently inside the all-in-one image.
- Supervisor restarts failed child processes, while the container healthcheck exposes partial failures to Docker.
