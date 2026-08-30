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

Cloud authentication actions in the desktop app open the AuraPunk Cloud
dashboard in the user's browser. The browser completes the hosted sign-in
flow and remains the owner of the provider session. The app only knows the
configured cloud URL and never receives credentials or browser cookies.

The URL is returned by `/api/app-mode` and can be overridden for self-hosted
deployments with `AURAPUNK_CLOUD_URL`. The default is the official AuraPunk
Cloud URL. The actions are exposed only when the app is launched with
`VIBE_KANBAN_MODE=cloud` (including the `--cloud` desktop launcher mode).

## Consequences

- Local mode remains independent of cloud availability and authentication.
- Login and sign-up actions use the same hosted account system as the site.
- Self-hosted installations can point the app at their own account service.
- The desktop app does not yet display the browser's authenticated account
  state inside its local UI; a future device authorization flow may add that
  without changing the credential boundary.
