# Capítulo 2 — Noções de vibe coding: o vocabulário que você vai usar o livro inteiro

> **Objetivo:** aprender o jargão atual do desenvolvimento com IA — para que os capítulos práticos não precisem parar a cada palavra nova.

## 1. Vibe coding, em uma frase

Vibe coding é **codar por intenção**: você descreve o resultado que quer em linguagem natural e um agente escreve, roda e corrige o código. O termo foi popularizado por Andrej Karpathy em fev/2025 e pegou porque descreve bem a sensação — você "vibe" a ideia, a máquina materializa.

No Vibe Kanban, vibe coding não é "pedir e torcer": é um fluxo com artefatos — cards, pipelines, workspaces — que tornam a intenção auditável e repetível.

## 2. Spec Development (desenvolvimento guiado por especificação)

**Spec = o que o software deve fazer, antes de como faz.**

Em vez de mandar o agente "codar direto", você primeiro fixa a spec: requisitos, critérios de pronto, arquivos afetados, validações. Neste repositório:

- Um card bem escrito **é** a spec (`docs/issue-management.mdx:57` — a descrição vira o prompt do agente). A diferença entre "Fix bug" e "Timeout de login em 3G → retry com backoff" (cap. 05) é a diferença entre agente perdido e agente certeiro.
- Pipelines como `speckit` e `basic` (`assets/pipelines/*.toml`) formalizam o passo de spec: geram `SPEC.md` / `IMPLEMENTATION_PLAN.md` antes de tocar no código.
- O `AGENTS.md` raiz manda consultar `docs/ADR/` antes de propor alternativas — a spec arquitetural também é artefato.

Jargões que você vai ver:

| Termo | O que quer dizer aqui |
| --- | --- |
| **Spec-Driven Architecture** | A spec dita quem faz o quê: TypeScript no Node cuida de UI; Rust cuida de estado/processos/git. A fronteira é desenhada na spec, não no improviso (ver cap. 11). |
| **Spec Intake** | Transformar um briefing curto ("quero um SaaS de assinaturas") em uma tarefa pronta para desenvolvimento (`docs/cockpit/spec-intake.mdx`). |
| **Contrato gerado** | A spec da fronteira vive nos structs Rust com `#[derive(TS)]` e é gerada para TypeScript (`shared/types.ts`, `crates/server/src/bin/generate_types.rs` — cap. 12). |

> Regra prática: se você não consegue escrever a spec em 5 frases, o agente também não vai conseguir implementar.

## 3. Engineering Loop (o loop de engenharia e a autocorreção)

O **Engineering Loop** é o ciclo que permite ao agente **se corrigir sozinho**:

```
escrever → rodar checks → ler o erro no log → corrigir → repetir
```

Três ideias encadeadas:

- **CLI como interface do agente.** Comandos canônicos em `package.json` (`pnpm run check`, `pnpm run lint`, `cargo test --workspace`, `pnpm run generate-types` — ver `apendice-comandos.md`). O agente não adivinha comandos; ele lê `AGENTS.md` e roda os mesmos que você rodaria.
- **Erros que ensinam.** Guards como `scripts/check-migration-frozen.sh` e `scripts/check-legacy-frontend-paths.sh` falham com mensagem que aponta a correção; `cargo clippy -- -D warnings` (com `--features qa-mode`) transforma aviso em erro para não acumular dívida.
- **Logs como protocolo.** O agente reporta progresso escrevendo `VK-PIPELINE-STAGE: N` no log; o backend observa o `MsgStore` e persiste `workspaces.current_pipeline_stage` (`crates/services/src/services/pipeline_stage.rs`). Pedir revisão humana é escrever `VK-REVIEW-REQUEST: ...` (`review_request.rs`) — o log é, ao mesmo tempo, saída humana e API de máquina.

Quando o loop é curto e legível, 90% dos erros se resolvem sem você. Quando é longo ou opaco, o agente para e escala — e é aí que entra o próximo tópico.

## 4. Multi-agente (desenvolvimento com vários agentes em paralelo)

Um agente sozinho já ajuda. Vários, em paralelo, mudam a escala — mas só se o repositório isolar o trabalho de cada um.

Neste projeto:

- **Workspaces = worktrees git isolados.** Cada workspace é uma pasta em `.vibe-kanban-workspaces/` com seu próprio branch `vk/xxxx-nome` criado a partir do `target branch` (`docs/workspaces/creating-workspaces.mdx:12`). Seu repo original não é tocado.
- **Um card, um workspace — ou vários.** Você pode vincular vários workspaces ao mesmo card e rodar Claude, Codex e OpenCode em paralelo, cada um no seu worktree.
- **Pipelines multi-agente.** `swarm-multi-agent.toml` orquestra subagentes que trabalham em frentes diferentes do mesmo épico (ver cap. 06). O **Orchestrator** é o agente singleton que dirige o board inteiro (`docs/cockpit/orchestrator.mdx`, `crates/services/src/services/orchestrator_compactor.rs`).

Jargões que você vai ver:

| Termo | O que quer dizer aqui |
| --- | --- |
| **Swarm / crew** | Conjunto de agentes trabalhando em workspaces diferentes do mesmo projeto. |
| **Orchestration** | Alguém (humano ou agente-orquestrador) decide quem faz o quê e em que ordem — via `get_pipeline` / `report_pipeline_stage` (MCP). |
| **YOLO mode** | Rodar o agente sem pedir permissão a cada tool call (`docs/vibe-guide.mdx:52` — "Use YOLO mode" para async funcionar; sem isso você reinventou pair programming, só que mais lento). |
| **Needs Attention** | Estado da workspace na sidebar quando há aprovação pendente — o agente levantou a mão e está esperando você (`docs/workspaces/interface.mdx:70`). |

## 5. Jargões que aparecem toda hora (glossário rápido)

| Jargão | Tradução prática |
| --- | --- |
| **Context engineering** | Escolher o que o agente vê (arquivos, logs, regras). É o trabalho do `AGENTS.md` e do `get_rules`/`memory_search`. |
| **Prompt engineering** | Escrever a descrição do card de um jeito que o agente acerta (cap. 05: título com verbo + critério de pronto). |
| **Spec intake / intake** | Pegar um pedido vago e transformar em card bem especificado antes de codar. |
| **Approval** | Pausa do agente pedindo permissão — tool permission ("posso rodar `rm`?") ou pergunta ("qual prioridade?") — respondida via TUI, Telegram ou `respond_to_approval` (MCP). |
| **Setup / Cleanup / Dev scripts** | Comandos por projeto/repo que o Vibe Kanban roda ao criar/abrir/fechar workspace (Settings → Projects & Repositories). |
| **Worktree** | Cópia leve do repo com branch próprio — o isolamento que permite multi-agente sem conflito. |
| **Target vs Working branch** | Target = onde vai mergear (ex.: `main`, você define); Working = onde o agente trabalha (`vk/xxxx`, auto-criado). |
| **Preview proxy** | O Rust serve o dev server Node dentro do painel Preview (cap. 07). |
| **TUI / Telegram bridge** | Superfícies de controle para um dev solo: `vibe-tui` (`crates/tui`) no terminal e `vibe-telegram-bridge` (`crates/telegram-bridge`) no Telegram — ambas falam com a mesma API de approvals (`automation/README.md`). |

## 6. Como esses conceitos se encontram no dia a dia

Um fluxo típico de 2 minutos, com o vocabulário certo:

1. Você faz **spec intake**: escreve um card com título, descrição (spec) e critério de pronto.
2. Cria uma **workspace** (worktree + working branch) vinculada ao card, escolhe o **pipeline** (`quick` para trivial) e despacha o agente.
3. O agente entra no **Engineering Loop**: escreve → `pnpm run check` → lê erro → corrige, reportando `VK-PIPELINE-STAGE: N` a cada estágio.
4. Se precisar de você, levanta `VK-REVIEW-REQUEST` ou `Needs Attention` — você responde na interface, na **TUI** ou no **Telegram**.
5. Quando o critério de pronto bate, o card vai para **Done** e o pipeline faz **squash-merge** no target branch. Outro agente já pode estar rodando em paralelo em outro card — **multi-agente**.

Nos próximos capítulos você vai viver esse fluxo na prática, começando pela instalação.

## Checklist do capítulo

- [ ] Sei explicar vibe coding em uma frase para alguém fora da área.
- [ ] Sei a diferença entre spec, prompt e contrato gerado.
- [ ] Sei descrever o Engineering Loop e onde ele aparece nos comandos do repo.
- [ ] Sei por que worktrees permitem multi-agente sem conflito.
- [ ] Reconheço os jargões da tabela quando eles aparecerem nos caps. 03–08.
