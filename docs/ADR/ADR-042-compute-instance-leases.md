# ADR-042: Cloud compute instances and expiring worktree leases

## Status

Accepted

## Date

2026-09-03

## Context

AuraPunk needs to place worktrees on machines with available CPU, memory and
disk without treating every workspace as a network node. A temporary Docker
container may host a worktree, but its lifetime and resource use must be
visible and chargeable. The Cloud is the control plane; the instance remains
the authority that creates and removes its local container.

## Decision

Register one stable `instance_id` per execution node. Tailcat addresses and
connection blobs are transport metadata and never the identity or the
authorization boundary. A user-scoped Cloud registry stores capacity and
heartbeats. A `lease_id` reserves a bounded amount of vCPU, RAM and disk for a
workspace/container and expires automatically by policy.

Workers report append-only usage samples. The initial transparent credit rate
is 10 credits per vCPU-hour, 4 credits per GiB of RAM-hour and 1 credit per GiB
of disk-day. The worker must remove the container and release the worktree when
the lease ends; creating a lease in the Cloud does not execute a container.

## Consequences

- Many worktrees can share one Tailcat node without consuming one IP each.
- The Cloud can reject a request that exceeds the advertised capacity.
- Usage can be audited from samples instead of inferred from wall-clock time.
- A worker heartbeat and Docker launcher are still required before remote
  container execution is production-ready.

