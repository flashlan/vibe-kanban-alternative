# Capítulo 2 — Noções de vibe coding: Engineering Loop, Spec Development e multi-agente

> **Objetivo:** aprender o jargão atual do desenvolvimento com IA — com exemplos reais do SaaS AssinaFácil — para que os capítulos práticos (03 em diante) não precisem parar a cada palavra nova.

## 1. Vibe coding, em uma frase — e por que não é "pedir e torcer"

Vibe coding é **codar por intenção**: você descreve o resultado que quer em linguagem natural e um agente escreve, roda e corrige o código. O termo foi popularizado por Andrej Karpathy em fev/2025 e pegou porque descreve bem a sensação — você "vibe" a ideia, a máquina materializa.

No Vibe Kanban, vibe coding não é prompt solto no chat. É um fluxo com artefatos — **cards (spec), pipelines (receita), workspaces (worktrees isolados)** — que tornam a intenção auditável, repetível e paralelizável. Sem artefatos, o agente alucina; com artefatos, ele entrega.

```
intenção (você) → spec (card) → pipeline (receita) → workspace (worktree) → loop (agente) → review (você)
```

Este capítulo apresenta os três pilares que sustentam esse fluxo.

## 2. Spec Development — a spec vem antes do código

**Spec = o que o software deve fazer, antes de como faz.** Se a spec está fraca, o código nasce torto e todo conserto depois é remendo.

### Onde a spec vive neste repositório

- **O card é a spec.** Em `docs/issue-management.mdx:57`, a descrição do card vira o prompt que o agente recebe ao criar a workspace. A diferença entre um card fraco e um forte é a diferença entre agente perdido e agente certeiro.
- **Pipelines formalizam a spec.** `speckit` e `basic` (`assets/pipelines/*.toml`) geram `SPEC.md` / `IMPLEMENTATION_PLAN.md` antes de tocar no código — o agente só codifica depois que a spec foi revisada.
- **ADRs guardam a spec arquitetural.** O `AGENTS.md` raiz manda consultar `docs/ADR/` antes de propor alternativas.

### Exemplo concreto: "Página de planos" do AssinaFácil

**Spec fraca** (o agente vai adivinhar — e errar):

> Fazer página de planos.

**Spec forte** (o agente acerta de primeira):

> **Título:** Criar página `/planos` com 3 planos (Free, Pro R$49, Enterprise) e CTA para checkout
>
> **Descrição:**
> - Tabela comparativa com 3 colunas, badges "Mais popular" no Pro, CTA "Assinar" em cada coluna.
> - Arquivos: `packages/web-core/src/pages/plans.tsx` (novo), `packages/web-core/src/features/billing/plans.ts` (dados dos planos).
> - Validação: Preview em 1440px e 375px mostra tabela sem quebra; `pnpm run check` passa; screenshot ancorada em `docs/images/livro/saas-planos.png` coincide.
> - Restrições: usar Tailwind já configurado (`packages/local-web/AGENTS.md`); não adicionar dependência nova; seguir padrão de `packages/web-core/src/features/`.

Perceba a estrutura: **o que** (tabela de 3 planos), **onde** (arquivos), **como validar** (check + screenshot), **restrições** (sem nova lib). O agente recebe isso como prompt e já sabe quando terminou — quando o Preview coincide com a âncora.

### Jargões de spec que você vai ver

| Termo | O que quer dizer aqui |
| --- | --- |
| **Spec-Driven Architecture** | A spec dita quem faz o quê: TypeScript no Node cuida de UI; Rust cuida de estado/processos/git. A fronteira `crates/` vs `packages/` vs `shared/` é desenhada na spec, não no improviso (ver cap. 11). |
| **Spec Intake** | Transformar um briefing vago ("quero um SaaS de assinaturas") em tarefa pronta para desenvolvimento (`docs/cockpit/spec-intake.mdx`). No livro, o intake do AssinaFácil vira o épico + 5 sub-issues do cap. 08. |
| **Contrato gerado** | A spec da fronteira vive nos structs Rust com `#[derive(TS)]` e é gerada para TypeScript (`shared/types.ts` via `crates/server/src/bin/generate_types.rs` — cap. 12). O código **é** a spec. |
| **Critério de pronto** | Frase que diz quando o card está Done — sempre com validação observável ("Preview mostra X; `pnpm run check` passa"). |

> Regra prática: se você não consegue escrever a spec em 5 frases + 1 critério de pronto, o agente também não vai conseguir implementar.

## 3. Engineering Loop — o loop que deixa o agente se corrigir sozinho

O **Engineering Loop** é o ciclo que permite ao agente **se corrigir sem você**:

```
escrever → rodar checks → ler o erro no log → corrigir → repetir
```

Quando o loop é curto e legível, 90% dos erros se resolvem sozinhos. Quando é longo ou opaco, o agente para e escala — e você é interrompido.

### Os três ingredientes do loop

**a) CLI como interface do agente.** Comandos canônicos em `package.json` que o agente lê em `AGENTS.md`:

```bash
pnpm run check        # tsc (local-web, web-core, ui) + cargo check + guards
pnpm run lint         # ESLint + cargo clippy -- -D warnings (com --features qa-mode)
pnpm run format       # cargo fmt + Prettier
cargo test --workspace
pnpm run generate-types  # regenera shared/types.ts (cap. 12)
```

O agente não adivinha comandos; ele roda os mesmos que você rodaria.

**b) Erros que ensinam.** Guards falham com mensagem que aponta a correção:

- `scripts/check-migration-frozen.sh` — impede editar migration publicada e diz por quê.
- `scripts/check-legacy-frontend-paths.sh` — impede importar de caminho antigo e aponta o novo.
- `cargo clippy -- -D warnings` — transforma aviso em erro; nada passa como "só um warning".

**c) Logs como protocolo.** O agente reporta progresso escrevendo no log; o backend observa o `MsgStore`:

- `VK-PIPELINE-STAGE: N` → persiste `workspaces.current_pipeline_stage` (`crates/services/src/services/pipeline_stage.rs`, regex `(?i)VK-PIPELINE-STAGE:\s*(\d+)`).
- `VK-REVIEW-REQUEST: ...` → dispara som + notificação (`crates/services/src/services/review_request.rs`, regex `(?i)VK-REVIEW-REQUEST:\s*(.+)`).

O log é, ao mesmo tempo, saída humana e API de máquina — e funciona igual para Claude, Codex, OpenCode, Gemini… porque todos escrevem no mesmo `MsgStore` (`crates/executors/src/stdout_dup.rs`).

### Walkthrough real do loop (com erro de verdade)

Imagine o card "Página de planos" do exemplo acima. O agente escreve `plans.tsx` e roda:

```bash
pnpm run check
# → error TS2322: Type 'string' is not assignable to type 'PlanTier' in plans.ts:14
```

O agente lê o erro, abre `plans.ts:14`, vê que usou `"pro"` em vez de `"Pro"` (o tipo `PlanTier` vem de `shared/types.ts`, gerado do Rust — cap. 12), corrige, e roda de novo:

```bash
pnpm run check   # → passa
pnpm run lint    # → passa (clippy -D warnings não perdoa)
```

Só então ele escreve `VK-PIPELINE-STAGE: 2` e segue. Você não fez nada — o loop fechou sozinho porque o erro era legível e o comando era canônico.

Se o erro fosse opaco ("falhou"), o agente pararia em `Needs Attention` e você teria que adivinhar. Por isso o livro insiste: **invista no loop antes de investir no prompt**.

## 4. Multi-agente — vários agentes em paralelo sem pisar no pé do outro

Um agente sozinho já ajuda. Vários, em paralelo, mudam a escala — mas só se o repositório isolar o trabalho de cada um.

### O isolamento: workspaces = worktrees

Cada workspace é uma pasta em `.vibe-kanban-workspaces/` com seu próprio branch `vk/xxxx-nome` criado a partir do `target branch` (`docs/workspaces/creating-workspaces.mdx:12`). Seu repo original não é tocado.

```
repo original (main)
  ├─ .vibe-kanban-workspaces/
  │   ├─ vk-a1b2-landing-page/   (workspace A, branch vk/a1b2-...)
  │   └─ vk-c3d4-auth/           (workspace B, branch vk/c3d4-...)
  └─ (intocado)
```

- **Um card, um workspace — ou vários.** Você pode vincular vários workspaces ao mesmo card e rodar Claude, Codex e OpenCode em paralelo, cada um no seu worktree.
- **Pipelines multi-agente.** `swarm-multi-agent.toml` orquestra subagentes em frentes diferentes do mesmo épico. O **Orchestrator** é o agente singleton que dirige o board inteiro (`docs/cockpit/orchestrator.mdx`, `crates/services/src/services/orchestrator_compactor.rs` — com watchdog de 400k tokens / 1h / 10m cooldown).

### Exemplo prático: AssinaFácil em paralelo

Épico **AssinaFácil — MVP** com sub-issues:

1. Landing page
2. Auth (login/cadastro)

Você despacha os dois ao mesmo tempo:

- Workspace A (`vk-a1b2-landing`) — agente 1 escreve `landing.tsx`.
- Workspace B (`vk-c3d4-auth`) — agente 2 escreve `auth.tsx`.

Cada um roda seu próprio `pnpm run check` no seu worktree, sem conflito. Quando ambos pedem `VK-REVIEW-REQUEST`, você revisa os diffs em **Changes** e o app no **Preview** de cada workspace, move os cards para Done, e o pipeline faz squash-merge no `main` — um de cada vez, sem merge hell.

### Jargões de multi-agente

| Termo | O que quer dizer aqui |
| --- | --- |
| **Swarm / crew** | Conjunto de agentes em workspaces diferentes do mesmo projeto. |
| **Orchestration** | Decidir quem faz o quê e em que ordem — via `get_pipeline` / `report_pipeline_stage` (MCP). |
| **YOLO mode** | Rodar sem pedir permissão a cada tool call (`docs/vibe-guide.mdx:52` — "Use YOLO mode" para async funcionar; sem isso você reinventou pair programming, só que mais lento). |
| **Needs Attention** | Estado da workspace na sidebar quando há approval pendente — o agente levantou a mão (`docs/workspaces/interface.mdx:70`). |

## 5. Glossário rápido — os jargões que aparecem toda hora

| Jargão | Tradução prática |
| --- | --- |
| **Context engineering** | Escolher o que o agente vê (arquivos, logs, regras). É o trabalho do `AGENTS.md` e do `get_rules`/`memory_search`. |
| **Prompt engineering** | Escrever a descrição do card de um jeito que o agente acerta (cap. 05: título com verbo + critério de pronto). |
| **Spec intake** | Pegar um pedido vago e transformar em card bem especificado antes de codar. |
| **Approval** | Pausa pedindo permissão — tool permission ("posso rodar `rm`?") ou pergunta ("qual prioridade?") — respondida via TUI, Telegram ou `respond_to_approval` (MCP). |
| **Setup / Cleanup / Dev scripts** | Comandos por projeto/repo que o Vibe Kanban roda ao criar/abrir/fechar workspace (Settings → Projects & Repositories). |
| **Worktree** | Cópia leve do repo com branch próprio — o isolamento que permite multi-agente sem conflito. |
| **Target vs Working branch** | Target = onde vai mergear (ex.: `main`, você define); Working = onde o agente trabalha (`vk/xxxx`, auto-criado). |
| **Preview proxy** | O Rust serve o dev server Node dentro do painel Preview (cap. 07). |
| **TUI / Telegram bridge** | Superfícies de controle para um dev solo: `vibe-tui` (`crates/tui`) no terminal e `vibe-telegram-bridge` (`crates/telegram-bridge`) no Telegram — ambas falam com a mesma API de approvals (`automation/README.md`). |
| **Squash-merge** | Pipeline `quick` junta os commits da workspace num só commit no target branch. |
| **ADR** | Architecture Decision Record em `docs/ADR/` — a spec arquitetural versionada. |

## 6. Como tudo se encontra em 2 minutos

Um fluxo típico, com o vocabulário certo:

1. Você faz **spec intake**: escreve um card "Página de planos" com a spec forte do §2 e critério de pronto.
2. Cria uma **workspace** (worktree + working branch `vk/xxxx-planos`) vinculada ao card, escolhe o **pipeline** `quick` e despacha o agente.
3. O agente entra no **Engineering Loop**: escreve `plans.tsx` → `pnpm run check` → lê `TS2322` → corrige → `VK-PIPELINE-STAGE: 2`.
4. Se precisar de você, levanta `VK-REVIEW-REQUEST` ou `Needs Attention` — você responde na interface, na **TUI** (`cargo run -p tui`, tecla `a`) ou no **Telegram**.
5. Quando o critério de pronto bate (Preview 1440px + 375px ok, check passa, screenshot `saas-planos.png` coincide), o card vai para **Done** e o pipeline faz **squash-merge** no `main`. Enquanto isso, outro agente já corre em paralelo no card "Auth" — **multi-agente**.

Nos próximos capítulos você vai viver esse fluxo na prática, começando pela instalação (cap. 03).

## Checklist do capítulo

- [ ] Sei explicar vibe coding, spec e engineering loop em uma frase cada.
- [ ] Sei transformar "Fazer página de planos" em spec forte com onde/validar/restrições.
- [ ] Sei descrever o loop `escrever → check → ler erro → corrigir` com um exemplo real (`TS2322`).
- [ ] Sei por que worktrees permitem multi-agente sem conflito (e o que é target vs working branch).
- [ ] Reconheço os 15 jargões da tabela quando eles aparecerem nos caps. 03–08.
- [ ] Consigo narrar o fluxo de 2 minutos (spec → workspace → loop → review → merge) sem consultar o livro.
