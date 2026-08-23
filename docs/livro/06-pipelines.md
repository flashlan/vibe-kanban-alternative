# Capítulo 6 — Pipelines na prática

> **Objetivo:** entender o que é um pipeline, escolher o certo para cada card e acompanhar o progresso sem adivinhar.

## O que é um pipeline

Um pipeline é uma **receita em TOML** que diz ao agente o que fazer, em que ordem, e como reportar progresso. As receitas vivem em `assets/pipelines/*.toml` — `quick`, `basic`, `speckit`, `swarm-multi-agent`, `wikillm`, `async-claude-*`, `async-opencode-glm` (e `retired/`).

Quando você cria um card, a descrição carrega só um **ponteiro compacto** para o pipeline; o conteúdo pesado é resolvido via `get_pipeline` (MCP). O agente executa os `[[stage]]` em ordem, sem pular nem reordenar.

Anatomia de um `[[stage]]` (ex.: `assets/pipelines/quick.toml`):

```toml
[[stage]]
id = "implement"
label = "Implement directly"
default_enabled = true
prompt = "Implement this card directly from its description — do NOT write SPEC.md..."
```

`default_enabled` diz se o estágio roda por padrão naquele pipeline. Alguns estágios existem mas vêm desabilitados — você habilita quando precisa.

## O pipeline Quick — seu primeiro

O `quick.toml` é o pipeline de **cards triviais** (classificados como `trivial`). Estágios:

| Estágio | default | O que faz |
| --- | --- | --- |
| `memory` | on | Busca `get_rules` (guardrails do projeto) |
| `implement` | on | Implementa direto da descrição, verifica com `pnpm run check` + check manual |
| `review-manual` | off | Escreve `VK-REVIEW-REQUEST: ...` e **para** — dispara som/notificação para você revisar |
| `merge` | on | Squash-merge no branch base |

A tripwire do `implement` é importante: se o agente descobrir que o card não era trivial (precisa mexer em >3 arquivos, há decisão de design aberta), ele deve commitar o que fez e parar com `VK-ESCALATE: trivial->light — <motivo>`.

Outros pipelines acrescentam estágios como `code-review` (via Codex), `orchestrate`, abertura de PR, etc. Comece pelo Quick; evolua quando o card pedir.

## Como usar na interface

1. **Crie o card** (cap. 4) com descrição específica — ela é a "work order" do pipeline Quick.
2. **Crie um workspace vinculado** ao card e descreva a tarefa no chat — o agente vai buscar `get_pipeline` e seguir os estágios.
3. **Acompanhe ao vivo:** a cada estágio o agente escreve `VK-PIPELINE-STAGE: N` no log e chama `report_pipeline_stage`. O backend persiste em `workspaces.current_pipeline_stage` (ver `crates/services/src/services/pipeline_stage.rs` — regex `(?i)VK-PIPELINE-STAGE:\s*(\d+)` varrendo o `MsgStore`) e a UI mostra o progresso do card.
4. **Quando o agente precisa de você:** ele escreve `VK-REVIEW-REQUEST: <o que revisar>` (`crates/services/src/services/review_request.rs`, regex `(?i)VK-REVIEW-REQUEST:\s*(.+)`) — a UI toca alarme e você revisa antes dele seguir.

Se o card não tem pipeline, `stages` vem vazio e o agente segue sem ele — útil para tarefas exploratórias.

## Escolher o pipeline certo

| Situação | Pipeline sugerido |
| --- | --- |
| Correção/tarefa de 1–3 arquivos com descrição clara | `quick` |
| Feature média que merece spec/plan | `basic` ou `speckit` |
| Tarefa grande com subagentes em paralelo | `swarm-multi-agent` |
| Pesquisa/escrita | `wikillm` ou `async-*` |

Você troca o pipeline do card antes de despachar o workspace. Na dúvida, comece trivial e deixe a tripwire te dizer se precisa escalar.

## Checklist do capítulo

- [ ] Sei onde vivem as receitas (`assets/pipelines/*.toml`) e o que é `default_enabled`.
- [ ] Criei um card Quick, vinculei workspace e vi `VK-PIPELINE-STAGE` avançando no painel de Logs.
- [ ] Sei o que fazer quando vejo `VK-REVIEW-REQUEST` (revisar, responder, liberar).
- [ ] Sei quando escalar de Quick para um pipeline maior.
