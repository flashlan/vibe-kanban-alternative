# Capítulo 13 — The Engineering Loop: CLI e autocorreção

> **Princípio:** um agente só se autocorrige se consegue rodar, falhar, ler o erro e repetir — sem pedir permissão a cada passo. O seu trabalho é tornar esse loop curto, legível e sem surpresas.

## O loop em uma frase

```
escrever → rodar testes/checks → ler o erro no log → corrigir → repetir
```

Quando o loop é rápido, o agente resolve sozinho 90% dos problemas. Quando é lento ou ilegível, ele para e pede ajuda — exatamente o que o sistema de aprovações e o alarme de revisão tentam evitar (cap. 6). Este capítulo é sobre tornar o loop tão bom que a escalação vira exceção.

## Os comandos canônicos (caso real)

No `package.json` raiz, os scripts são a spec do loop. Qualquer agente (Claude, OpenCode, Codex, Gemini…) que leia `AGENTS.md` aprende a mesma sequência:

```bash
pnpm i                                    # instala
pnpm run dev                              # sobe web (3001) + backend (3002) com portas fixas
pnpm run check                            # tsc nos 3 workspaces + cargo check + guards
pnpm run lint                             # ESLint nos packages + cargo clippy -- -D warnings
pnpm run format                           # cargo fmt + Prettier nos packages
cargo test --workspace                    # testes Rust
pnpm run generate-types                   # regenera shared/types.ts (cap. 4)
pnpm run prepare-db                       # SQLx offline
```

O `pnpm run check` é o guardião: `local-web:legacy-path-guard` (`scripts/check-legacy-frontend-paths.sh`), `check:db` (`scripts/check-migration-frozen.sh`), `local-web:check`, `web-core:check`, `ui:check`, `backend:check` (`cargo check --workspace`). Cada guard tem mensagem de erro ensinando o que fazer — não só "falhou".

Regra do `AGENTS.md`: antes de completar qualquer task, `pnpm run format`. Não é polidez — é a garantia de que `cargo fmt --all` e Prettier não geram diff fantasma no próximo commit.

## Erros que ensinam

Um bom loop não só falha — ele explica. Três padrões deste repositório:

1. **Guards com mensagem.** `check-migration-frozen.sh` não deixa você editar uma migration já publicada sem dizer por quê; `check-legacy-frontend-paths.sh` impede importar de caminhos antigos e aponta o novo. O agente que lê esse erro sabe exatamente o que corrigir.

2. **Warnings como erros.** `backend:lint` roda `cargo clippy --workspace --all-targets --features qa-mode -- -D warnings`. Em `qa-mode`, nada passa como aviso — tudo que o Clippy reclama quebra o CI. O agente não deixa dívida técnica "para depois".

3. **Logs por crate.** Em `crates/server/src/main.rs:33`, o `EnvFilter` é montado por crate (`server`, `services`, `db`, `executors`, `deployment`…) a partir de `RUST_LOG`. Com `DISABLE_WORKTREE_CLEANUP=1 RUST_LOG=debug cargo watch -w crates -x 'run --bin server'` (`backend:dev:watch`), o agente lê o log filtrado e sabe se o erro veio do banco, do executor ou do roteamento.

## Os logs como interface de máquina

O detalhe mais importante para automação: os mesmos logs que um humano lê são a interface que os trackers de pipeline leem. Em `crates/services/src/services/pipeline_stage.rs` e `review_request.rs`, um `Regex` varre o `MsgStore` (o log unificado de `stdout` de execuções headless e headed) procurando marcadores de texto:

- `VK-PIPELINE-STAGE: N` — em que estágio do pipeline o card está (`parse_pipeline_stage_marker`, com `has_valid_boundary` para lidar com `\n` escapado em transcripts).
- `VK-REVIEW-REQUEST: <mensagem>` — o agente pede revisão humana e dispara o alarme sonoro via `NotificationService`.

O agente não chama uma API para dizer "mudei de estágio"; ele escreve uma linha no log. O backend observa o log. Essa escolha mantém todos os executores (Claude, OpenCode, Codex…) iguais do ponto de vista do orquestrador — nenhum precisa de integração especial. O log é o protocolo.

## Dev ports fixas e o erro previsível

Frontend `3001`, backend `3002`, preview proxy `3003` — fixas, documentadas e exportadas por `pnpm run dev`. Quando um agente tenta subir o dev server dentro de um workspace e a porta já está ocupada por outra instância (por exemplo, a instância principal em `/Users/.../vibe-kanban-alternative` enquanto o worktree tenta subir a sua), o erro é `AddrInUse` em `crates/server/src/main.rs` — previsível, pesquisável e corrigível checando `lsof -nP -i :3002 -sTCP:LISTEN` e o `cwd` do processo que segura a porta. O capítulo 2 já ensinou a não matar a instância errada.

## Checklist do capítulo

- [ ] Cada comando do loop está em `package.json` com um nome canônico (`check`, `lint`, `format`, `dev`).
- [ ] `check` inclui guards que explicam o erro e apontam a correção.
- [ ] Lint trata warnings como erros (ao menos em CI/qa-mode).
- [ ] `format` é obrigatório antes de completar — e está documentado.
- [ ] Logs são filtráveis por crate/componente via variável de ambiente.
- [ ] Mensagens que dirigem automação (estágio, revisão) são linhas de log com regex estável — não chamadas de API por executor.
- [ ] Portas de dev são fixas e o erro de conflito tem diagnóstico documentado.
