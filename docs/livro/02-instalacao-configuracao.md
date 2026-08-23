# Capítulo 2 — Instalação e configuração

> **Objetivo:** sair do zero para um board com seu primeiro projeto, sem conta na nuvem.

## 1. Requisitos

| Requisito | Versão | Onde está declarado |
| --- | --- | --- |
| Node | ≥ 20 | `package.json` → `engines` |
| pnpm | ≥ 8 (recomendado 10.13.1) | `package.json` → `packageManager` |
| Rust / Cargo | edição 2024 | `Cargo.toml` → `[workspace.package] edition` |
| Git | recente | — |

No macOS/Linux, `pnpm` e `cargo` via `rustup` resolvem tudo.

## 2. Rodar o app

O Indie é **100% local** — sem login, sem cloud. A forma mais rápida:

```bash
npx vibe-kanban-indie
# ou, dentro do repositório clonado:
pnpm i
pnpm run dev
# → Frontend :3001  Backend :3002  Preview proxy :3003
```

As três portas são fixas (documentadas em `AGENTS.md` e exportadas por `pnpm run dev` como `FRONTEND_PORT`/`BACKEND_PORT`/`PREVIEW_PROXY_PORT`). Se uma porta já estiver ocupada, veja quem segura com `lsof -nP -i :3002 -sTCP:LISTEN` e confira o `cwd` do processo — pode ser outra instância do próprio Vibe Kanban em outro diretório.

O backend escreve `db.v2.sqlite` em `asset_dir()` (criado em `crates/server/src/main.rs:44`); na primeira execução ele copia `db.sqlite` → `db.v2.sqlite` se precisar.

## 3. Primeiras preferências

Na primeira vez o app pede (`docs/getting-started.mdx:19`):

- agente de coding preferido (Claude Code, OpenCode, Codex…),
- IDE,
- notificações (som do alarme de `VK-REVIEW-REQUEST`).

Altere depois em **Settings** (engrenagem no topo direito da interface). O onboarding mostra screenshots em `/images/onboarding-*.png`.

## 4. Declarar seus projetos

O Indie não tem "criar conta" — ele lê um arquivo `projects.toml` que você declara. Veja o formato em `docs/cockpit/local-projects.mdx`. Conceitualmente:

- **Projeto** = agrupamento de repositórios que formam um produto (ex.: `assinatra-facil` com repos `app-web` e `api`).
- **Repositório** = um repo git em disco. Você pode adicionar repos recentes, navegar no disco ou criar um repo novo direto pela UI de criação de workspace (`docs/workspaces/creating-workspaces.mdx:62`).
- **Scripts por projeto/repo** — em **Settings → Projects & Repositories** você configura:
  - **Setup script** (ex.: `pnpm install`) — roda ao criar o workspace;
  - **Dev server script** (ex.: `pnpm dev`) — o que o botão Play / painel Preview vai rodar;
  - **Cleanup script** — o que roda ao arquivar.

Esses scripts são o que permitem que o agente trabalhe offline sem que você prepare o ambiente à mão (ver `docs/workspaces/creating-workspaces.mdx:28` e `docs/browser-testing.mdx:8`).

## 5. Conferir que está tudo ok

1. Abra `http://localhost:3001` — o board deve listar seus projetos.
2. Crie um projeto e entre nele — deve aparecer um board vazio com colunas (Todo / In Progress / Done) e o botão **New Issue**.
3. Se o board não abrir, confira os logs do backend (`RUST_LOG=debug` em `crates/server/src/main.rs:33` filtra por crate: `server`, `services`, `db`, `executors`).

## Checklist do capítulo

- [ ] `npx vibe-kanban-indie` (ou `pnpm run dev`) abre em `http://localhost:3001` sem erro de porta.
- [ ] Preferências de agente/IDE/notificações definidas.
- [ ] `projects.toml` com ao menos um projeto e um repositório que você consegue abrir no board.
- [ ] Setup/dev/cleanup scripts configurados para o repositório do seu SaaS (cap. 7 vai usá-los).
