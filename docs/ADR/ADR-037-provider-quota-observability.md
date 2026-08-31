# ADR-037: Provider Quota Observability

- **Status**: Accepted
- **Date**: 2026-08-28

## Context

The Usage dashboard already records local execution and token observations,
but local token counts are not the same as an account's provider quota. The
application must distinguish the two so that a user can see remaining plan
capacity and reset times without treating an estimate as billing data.

Provider CLIs do not expose equivalent machine-readable interfaces. Codex
provides account rate-limit windows through its app-server protocol. Claude
Code can emit rate-limit events, including window type and reset time. AGY and
OpenCode do not currently provide a stable, non-interactive quota response in
the local executors that can be safely polled by the application.

## Decision

Add a best-effort, process-local provider quota snapshot owned by the
`executors` crate and exposed through `GET /api/usage/summary` as
`provider_limits`.

The snapshot model supports:

- provider and plan name;
- multiple independent windows;
- percentage used and percentage remaining when supplied by the provider;
- monetary or credit limits when supplied by the provider;
- window duration and absolute reset timestamp; and
- provider status and credit balance when available.

The Codex executor requests `account/rateLimits/read` after app-server
initialisation and records its primary and secondary windows. The Claude
normaliser records `rate_limit_event` messages and merges separate windows,
such as `five_hour` and `seven_day`, into the same provider snapshot.

The Settings → Usage panel labels this data as **Provider limits**. It shows
used percentage, remaining percentage, reset time, duration, and credits when
available. When a provider does not expose a safe live value, the panel shows
“Usage unavailable” and does not infer quota from the local token ledger.

This feature is observational only. It does not block agent execution, does
not send provider slash commands, does not expose credentials, and does not
claim to reproduce provider billing or subscription accounting. Snapshots are
process-local and may be empty after a backend restart until a compatible
executor reports its status.

## Consequences

- Codex and Claude can provide live quota windows without scraping terminal
  output or requiring user interaction.
- Remaining capacity is calculated only from a provider-supplied percentage.
- Reset timestamps remain provider data and are not converted into an
  assumed five-hour or weekly schedule.
- AGY and OpenCode remain explicitly unsupported for live quota percentages
  until their executors expose a stable structured response.
- The existing durable token ledger remains useful for issue/model/agent
  benchmarks, but it must not be presented as remaining plan usage.
- Provider snapshot state is intentionally ephemeral; durable historical
  quota accounting would require a separate provider-specific persistence and
  reconciliation design.

## Verification

- `cargo check -p executors -p server`
- `cargo test -p executors --lib` (72 tests passed)
- Claude rate-limit parsing test covering percentage, five-hour duration, and
  reset timestamp
- Provider snapshot test covering merging independent windows
