# Capítulo 13 — The Engineering Loop: CLI e autocorreção

> **Princípio:** um agente só se autocorrige se consegue rodar, falhar, ler o erro e repetir — sem pedir permissão a cada passo. O seu trabalho é tornar esse loop curto, legível e sem surpresas.

## O loop em uma frase (e em diagrama)

```
escrever → rodar testes/checks → ler o erro no log → corrigir → repetir
   ▲                                                        │
   └────────────────────────────────────────────────────────┘
         quanto mais curta a volta, menos humano necessário
```

Quando o loop é rápido, o agente resolve sozinho 90% dos problemas. Quando é lento ou ilegível — erro sem mensagem acionável, log poluído, porta aleatória — ele para e pede ajuda. É exatamente o que o sistema de aprovações (`crates/executors/src/approvals.rs`) e o alarme de revisão (`review_request.rs`) tentam evitar (caps. 06 e 14). Este capítulo é sobre tornar o loop tão bom que a escalação vira exceção.

> **No AssinaFácil (cap. 08):** cada card do SaaS termina com `pnpm run check` verde + Preview validado. Se o `check` explica o erro, o agente corrige sozinho; se só diz "failed", você tem que intervir. A diferença está toda neste capítulo.

## Os comandos canônicos — a spec do loop

No `package.json` raiz, os scripts são a spec do loop. Qualquer agente (Claude, OpenCode, Codex, Gemini…) que leia `AGENTS.md` aprende a **mesma sequência** — não há "jeito do Claude" vs "jeito do Codex":

```bash
pnpm i                                    # instala (pnpm 10.13.1, node >= 20)
pnpm run dev                              # sobe web :3001 + backend :3002 + proxy :3003 (portas fixas)
pnpm run check                            # guardião: legacy-guard + check:db + tsc×3 + cargo check
pnpm run lint                             # ESLint (local-web, ui) + cargo clippy -D warnings --features qa-mode
pnpm run format                           # cargo fmt --all + Prettier (web-core, local-web) — obrigatório antes de completar
cargo test --workspace                    # testes Rust de todos os crates
pnpm run generate-types                   # regenera shared/types.ts (cap. 12)
pnpm run generate-types:check             # só verifica (CI) — falha se desatualizado
pnpm run prepare-db                       # SQLx .sqlx offline para builds sem banco
```

O `pnpm run check` é o guardião e roda 6 verificações em sequência:

| Verificação | O que faz | Mensagem quando falha |
| --- | --- | --- |
| `local-web:legacy-path-guard` | `scripts/check-legacy-frontend-paths.sh` — impede importar de caminhos legados | Aponta o novo caminho |
| `check:db` | `scripts/check-migration-frozen.sh` — impede editar migration já publicada | Explica por que está congelada |
| `local-web:check` / `web-core:check` / `ui:check` | `tsc --noEmit` em cada workspace | Erro de tipos com arquivo:linha |
| `backend:check` | `cargo check --workspace` | Erro do compilador Rust |

Cada guard tem **mensagem que ensina a correção** — não só "falhou". É o requisito número 1 de um bom loop: o erro é a spec da correção.

**Regra do `AGENTS.md`:** antes de completar qualquer task, `pnpm run format`. Não é polidez — é a garantia de que `cargo fmt --all` e Prettier não geram diff fantasma no próximo commit e no próximo `VK-PIPELINE-STAGE`.

## Três padrões que fazem o loop ensinar

### 1. Guards com mensagem acionável

`check-migration-frozen.sh` não deixa você editar uma migration já publicada **sem dizer por quê** (migrations são append-only); `check-legacy-frontend-paths.sh` impede importar de `packages/web-core/src/old/*` e aponta o novo `shared/`. O agente que lê esse erro sabe exatamente o que corrigir — sem precisar perguntar ao humano.

### 2. Warnings como erros (qa-mode)

`backend:lint` roda:

```bash
cargo clippy --workspace --all-targets --features qa-mode -- -D warnings
```

Em `qa-mode`, nada passa como aviso — tudo que o Clippy reclama quebra o CI. O agente não deixa dívida técnica "para depois" porque o `check` não deixa. Para o seu SaaS, copie: `eslint --max-warnings 0` tem o mesmo efeito no Node.

### 3. Logs por crate — filtráveis

Em `crates/server/src/main.rs:33`, o `EnvFilter` é montado por crate (`server`, `services`, `db`, `executors`, `deployment`…) a partir de `RUST_LOG`:

```bash
DISABLE_WORKTREE_CLEANUP=1 RUST_LOG=debug cargo watch -w crates -x 'run --bin server'
# só o que interessa:
RUST_LOG=services=debug,executors=info cargo run -p server
```

Com `backend:dev:watch` (`cargo watch -w crates -x 'run --bin server'`), o agente lê o log filtrado e sabe se o erro veio do banco, do executor ou do roteamento — sem grep manual em 10k linhas.

## Os logs como interface de máquina (ponte com o cap. 14)

O detalhe mais importante para automação: **os mesmos logs que um humano lê são a interface que os trackers de pipeline leem**. Em `crates/services/src/services/pipeline_stage.rs` e `review_request.rs`, um `Regex` varre o `MsgStore` — o log unificado de `stdout` de execuções headless e headed (`crates/utils/src/msg_store.rs`) — procurando marcadores de texto:

- `VK-PIPELINE-STAGE: N` — em que estágio do pipeline o card está (`parse_pipeline_stage_marker`, com `has_valid_boundary` para lidar com `\n` escapado em transcripts).
- `VK-REVIEW-REQUEST: <mensagem>` — o agente pede revisão humana e dispara o alarme sonoro via `NotificationService` (cap. 14).

O agente **não chama uma API** para dizer "mudei de estágio"; ele **escreve uma linha no log**. O backend observa o log. Essa escolha mantém todos os executores (Claude, Codex, OpenCode, Gemini…) iguais do ponto de vista do orquestrador — nenhum precisa de integração especial. **O log é o protocolo.** Essa ideia é tão central que o cap. 14 inteiro é sobre ela.

## Dev ports fixas e o erro previsível — `AddrInUse`

Frontend `3001`, backend `3002`, preview proxy `3003` — **fixas**, documentadas no `AGENTS.md` e exportadas por `pnpm run dev` como `FRONTEND_PORT`/`BACKEND_PORT`/`PREVIEW_PROXY_PORT`. Quando um agente tenta subir o dev server dentro de um workspace e a porta já está ocupada por outra instância, o erro é:

```
Error: Address already in use (os error 48) — crates/server/src/main.rs
```

Previsível, pesquisável e corrigível — e o diagnóstico está no cap. 02 §5:

```bash
lsof -nP -i :3002 -sTCP:LISTEN          # quem segura a porta?
ps -o pid,cwd,command -p <PID>          # de qual repo/worktree?
# se for a instância principal em ~/vibe-kanban-alternative, não mate — use outra porta ou pare o worktree
```

No livro, esse erro apareceu de verdade quando o worktree `vk/1f98` tentou `restart.sh` com a instância principal rodando em `vibe-kanban-alternative` (PIDs 50138/50146 desde 00:13 23/08). O `lsof` mostrou `cwd` diferente — diagnóstico em 10s, sem matar o processo errado.

> **Exercício:** quebre de propósito — adicione `unused_variable` num crate, rode `pnpm run check` e leia o erro do `cargo check`. Agora rode `pnpm run format` e `pnpm run check` de novo. Esse é o loop que o agente faz sozinho 20 vezes por card.

## Checklist do capítulo

- [ ] Cada comando do loop está em `package.json` com nome canônico (`check`, `lint`, `format`, `dev`) — copiar-colar funciona.
- [ ] `check` inclui guards que explicam o erro e apontam a correção (mensagem acionável, não só "failed").
- [ ] Lint trata warnings como erros (ao menos em CI/`qa-mode` / `--max-warnings 0`).
- [ ] `format` é obrigatório antes de completar — e está no `AGENTS.md` como regra.
- [ ] Logs são filtráveis por crate/componente via `RUST_LOG` / variável de ambiente.
- [ ] Mensagens que dirigem automação (estágio, revisão) são linhas de log com regex estável — não chamadas de API por executor (cap. 14).
- [ ] Portas de dev são fixas, exportadas e o erro `AddrInUse` tem diagnóstico documentado (`lsof` + `cwd`).
- [ ] Um agente consegue rodar `check` → ler erro → corrigir → repetir sem pedir ajuda.
