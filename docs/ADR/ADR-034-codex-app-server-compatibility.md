# ADR-034: Codex app-server compatibility policy

- **Status:** Accepted
- **Date:** 2026-08-25

## Context

The Codex executor prefers the locally installed Codex CLI while its Rust
protocol crates are pinned independently. Codex app-server 0.149 replaced the
legacy `permissionProfile` request field with a named `permissions` profile id.
It can also enable Code Mode, but this client does not implement dynamic tool
execution. These mismatches allowed sessions to initialize and then fail before
performing repository work.

## Decision

The executor detects `codex --version` before starting app-server and accepts
only the tested range from 0.124.0 inclusive through 0.150.0 exclusive.

Protocol serialization is selected by version:

- 0.124 through 0.148 preserve the legacy request shape.
- 0.149 rejects the legacy `permissionProfile` field and rejects a named
  `permissions` profile when `sandbox` is present. The executor therefore
  omits both permission fields and keeps its sandbox policy.

The executor explicitly disables `code_mode` and `code_mode_host` because its
app-server client does not implement `DynamicToolCall`. Codex therefore uses
its built-in shell execution path.

The minimum and current supported versions are fixtures in the executor test
suite. Versions outside the range fail before session startup with an
actionable compatibility message.

## Consequences

- Updating the local Codex CLI within the tested range does not require manual
  application changes.
- A new minor version outside the range requires an intentional schema review,
  adapter update when necessary, and test-range update.
- Code Mode remains disabled until the executor implements dynamic tool calls
  and can verify the required host capability.
