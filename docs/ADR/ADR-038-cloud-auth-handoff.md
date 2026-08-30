# ADR-038: Cloud authentication handoff

## Status

Accepted

## Date

2026-08-30

## Context

The desktop app is local-first, while AuraPunk Cloud owns account identity,
tenant membership, billing, and provider authentication. The app must expose
account access in `--cloud` mode without handling passwords or duplicating the
cloud session in a local webview.

## Decision

Cloud authentication actions in the desktop app start a short-lived,
single-use device handoff. The app generates an unpredictable state value and
opens AuraPunk Cloud in the user's browser. The browser completes the hosted
ChatGPT sign-in flow; the cloud site associates the state with the
authenticated account and the app consumes the result through a CORS-enabled
status endpoint. Only the confirmed account identity is returned to the app.
The browser remains the owner of the provider session, and the app never
receives credentials, browser cookies, or provider tokens.

The URL is returned by `/api/app-mode` and can be overridden for self-hosted
deployments with `AURAPUNK_CLOUD_URL`. The default is the official AuraPunk
Cloud URL. The actions are exposed only when the app is launched with
`VIBE_KANBAN_MODE=cloud` (including the `--cloud` desktop launcher mode).

## Consequences

- Local mode remains independent of cloud availability and authentication.
- Login and sign-up actions use the same hosted account system as the site.
- Self-hosted installations can point the app at their own account service.
- The desktop app displays the confirmed account identity locally after the
  one-time handoff. The identity is device-local UI state, not a cloud session.
- Cloud-mode pilot builds may point `MEM0_URL` at the shared local AuraPunk
  Mem0 service. `AURAPUNK_CLOUD_MEM0_URL` overrides the pilot default
  (`http://192.168.254.107:8000`); local mode keeps its normal local Mem0
  endpoint.
- The pilot URL selection is transport configuration, not billing
  authorization. Before exposing the service beyond the trusted local
  network, add a scoped token exchange and enforce the active Personal/Pro or
  Enterprise subscription at the cloud gateway.
