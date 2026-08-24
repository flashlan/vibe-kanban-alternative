# Capítulo 6 — Pipelines na prática

> **Objetivo:** entender o que é um pipeline, escolher o certo para cada card e acompanhar o progresso sem adivinhar.

## O que é um pipeline

Um pipeline é uma **receita em TOML** que diz ao agente o que fazer, em que ordem, e como reportar progresso. As receitas vivem em `assets/pipelines/*.toml`. Este projeto tem 9:

| Arquivo | Quando usar |
| --- | --- |
| `quick.toml` | Card trivial (1–3 arquivos, spec completa) — implementa direto, sem spec/plan |
| `basic.toml` | Feature média — spec → plan → implement → review → merge |
| `speckit.toml` | Spec-Driven Development — `/speckit.*` em `specs/<branch>/` |
| `swarm-multi-agent.toml` | Épico grande — Antigravity planeja, Claude implementa, Codex revisa |
| `wikillm.toml` | Tarefa que depende de conhecimento prévio — recall antes, enrich depois |
| `async-claude-fable.toml` / `opus` / `sonnet` | Fan-out com subagentes Fable/Opus/Sonnet (spec e plan em subagente) |
| `async-opencode-glm.toml` | Mesmo fan-out, mas para executor OpenCode/GLM (sem subagentes Claude) |

Quando você cria um card, a descrição carrega só um **ponteiro compacto** (`<!-- vk:pipeline:start --> … <!-- vk:pipeline:end -->`); o conteúdo pesado é resolvido via `get_pipeline` (MCP, `crates/mcp/src/task_server/tools/pipeline.rs`). O agente executa os `[[stage]]` em ordem, sem pular nem reordenar — e a cada estágio chama `report_pipeline_stage` + escreve `VK-PIPELINE-STAGE: N` no log.

Anatomia de um `[[stage]]`:

```toml
[[stage]]
id = "implement"
label = "Implement directly"
default_enabled = true
prompt = "Implement this card directly from its description — do NOT write SPEC.md..."
```

`default_enabled` diz se o estágio roda por padrão. Alguns existem mas vêm desabilitados — habilite quando precisa.

## O pipeline Quick por dentro

O `quick.toml` é o seu primeiro pipeline (cards `trivial`):

```toml
name = "Quick"
# "Minimal flow for trivial cards: no spec, no plan, no subagent fan-out"

[[stage]] # orchestrate  — default_enabled = false (só se ligar auto-drive)
[[stage]] # memory       — true  — get_rules (guardrails do AGENTS.md)
[[stage]] # implement    — true  — implementa direto da descrição + verifica
[[stage]] # code-review  — false — review via Codex (opcional)
[[stage]] # review-manual — false — VK-REVIEW-REQUEST + STOP (alarme)
[[stage]] # merge         — true  — squash-merge no branch base
[[stage]] # pr            — false — abrir pull request
```

| Estágio | default | O que faz de verdade |
| --- | --- | --- |
| `memory` | on | Chama `get_rules` (o `pre` são guardrails, `post` é checklist de fechamento) |
| `implement` | on | Implementa direto da descrição, roda `pnpm run check` + check manual e corrige |
| `review-manual` | **off** | Escreve `VK-REVIEW-REQUEST: <o que revisar>` e **para** — `crates/services/src/services/review_request.rs:18` toca som/notificação |
| `merge` | on | Squash-merge autorizado (não espera aprovação externa) |

A **tripwire** do `implement` é o mecanismo de segurança: se o agente descobrir que o card não era trivial (precisa mexer em >3 arquivos, há decisão de design aberta, o root cause está em outro lugar), ele commita o WIP e para com a primeira linha exatamente `VK-ESCALATE: trivial->light — <motivo>` (ou `trivial->medium`). O orquestrador re-roteia para um pipeline maior. É melhor escalar do que empurrar.

Outros pipelines acrescentam estágios visíveis no TOML: `spec`/`plan` (basic, async-*), `plan-review-codex`, `speckit-constitution`/`specify`/`clarify`, `recall-knowledge` (wikillm), e no `swarm-multi-agent` os estágios têm `executor = "antigravity"` / `"claude"` / `"codex"` — cada estágio pode rodar em um agente diferente, com memória compartilhada via `mem0` (`memory_search`/`memory_save` a cada estágio).

## Como usar na interface — o ciclo completo

1. **Crie o card** (cap. 5) com descrição específica — no Quick, ela **é** a work order. Se a descrição já contém `### Outcome`, `### Scope` e `### Testing & acceptance criteria` (cada um no início da linha), pipelines maiores reaproveitam a spec e escrevem `SPEC.md` copiando-a verbatim (com `<!-- vk:pipeline:start/end -->` removido).
2. **Crie um workspace vinculado** ao card e descreva a tarefa no chat — o agente busca `get_pipeline` e segue os estágios.
3. **Acompanhe ao vivo em Logs:** a cada estágio o agente escreve `VK-PIPELINE-STAGE: N` e chama `report_pipeline_stage`. O backend persiste em `workspaces.current_pipeline_stage` (`crates/services/src/services/pipeline_stage.rs:28`, regex `(?i)VK-PIPELINE-STAGE:\s*(\d+)` com `has_valid_boundary` para lidar com `\n` escapado em transcripts) e a UI mostra o progresso.
4. **Quando precisa de você:** `VK-REVIEW-REQUEST: <o que revisar>` (`review_request.rs:18`, regex `(?i)VK-REVIEW-REQUEST:\s*(.+)`, guard idempotente por `execution_process_id`) — a UI toca alarme e o agente para até você liberar. Você revisa em **Changes** (diffs) e **Preview** (app rodando).
5. Se o card não tem pipeline, `stages` vem vazio e o agente segue sem ele — útil para spikes exploratórios.

### Ver na prática (exercício de 5 minutos)

- Crie um card "Adicionar badge de prioridade no card" com descrição de 3 linhas (onde: `packages/web-core/src/features/kanban/ui/`, critério: `pnpm run check` passa).
- Vincule um workspace, escolha pipeline `quick`, envie a primeira mensagem.
- Abra **Logs** e observe `VK-PIPELINE-STAGE: 1` → `2` aparecendo. Quando surgir `VK-REVIEW-REQUEST`, abra **Changes** e valide.

## Escolher o pipeline certo

| Situação | Pipeline |
| --- | --- |
| Correção/tarefa de 1–3 arquivos, spec clara | `quick` |
| Feature média que merece `SPEC.md` + `IMPLEMENTATION_PLAN.md` | `basic` ou `speckit` |
| Épico com frentes paralelas | `swarm-multi-agent` |
| Pesquisa/escrita com base de conhecimento | `wikillm` ou `async-*` |

Troque o pipeline do card **antes** de despachar o workspace. Na dúvida, comece em `quick` e deixe a tripwire `VK-ESCALATE` te dizer se precisa escalar — ela existe para isso.

## Checklist do capítulo

- [ ] Sei onde vivem as receitas (`assets/pipelines/*.toml`) e o que `default_enabled` controla.
- [ ] Sei o que cada estágio do `quick` faz (memory/implement/review-manual/merge) e onde fica a tripwire.
- [ ] Criei um card Quick, vinculei workspace e vi `VK-PIPELINE-STAGE` avançando em Logs.
- [ ] Sei o que fazer em `VK-REVIEW-REQUEST` e quando escalar para `basic`/`swarm`.
