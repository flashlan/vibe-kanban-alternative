# ADR-040: Public core and private AuraPunk Cloud boundary

## Status

Accepted

## Date

2026-08-30

## Context

The project has two complementary products:

- a public, local-first Vibe Kanban Alternative for one human operator and
  multiple coding agents;
- AuraPunk Cloud, a private hosted service that adds accounts, teams, billing,
  hosted memory, quotas, and commercial administration.

Maintaining two copies of the application core would cause semantic drift,
make security fixes difficult to promote, and risk publishing private SaaS
code or credentials by accident.

## Decision

The public repository is the source of truth for the local application core.
The private `aurapunk-ide` repository is the source of truth for the hosted
control plane and commercial services.

The repositories have separate Git histories and must remain physically
separate. The private repository consumes tagged public releases instead of
copying and independently modifying the core.

### Public repository responsibilities

- Kanban, issues, workspaces, worktrees, and local agent execution;
- Integration Guard, diff validation, and merge serialization;
- local Mem0/Qdrant support and local-first operation;
- TUI, MCP, Telegram, desktop builds, and self-hosting documentation;
- stable cloud connector interfaces without private service credentials.

### Private repository responsibilities

- AuraPunk website, authenticated dashboard, and account sessions;
- users, tenants, memberships, roles, plans, limits, and audit;
- Stripe billing, hosted Mem0/Qdrant, provisioning, and operations;
- cloud usage metering, team coordination, and commercial administration.

The cloud connector uses a versioned contract for authentication, license and
entitlement checks, usage reporting, memory backend selection, and preference
synchronization. Local mode remains functional when the cloud is unavailable.

### Release flow

1. The public repository publishes a tagged core release.
2. AuraPunk pins that release explicitly.
3. A private update change runs API, auth, memory, and tenant-isolation
   compatibility tests.
4. The release is promoted to hosted environments only after those tests pass.

Private features must not be implemented as unreviewable patches to the public
core. If a hosted capability is useful locally, it must first be designed as a
public, provider-neutral interface.

## Consequences

- Local users receive a complete product without an AuraPunk account.
- Cloud business logic, credentials, and tenant data stay private.
- Core bug fixes are promoted once and consumed by the SaaS through releases.
- The private repository has an explicit upgrade step rather than accidental
  synchronization.
- The two Mem0 deployments remain separate services and require an explicit
  migration if memory is ever moved between them.

