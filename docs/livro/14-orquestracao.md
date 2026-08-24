# Capítulo 14 — Orquestração de agentes: MCP, pipelines e o alarme

> **Princípio:** quando o próprio agente pode dirigir o fluxo de trabalho (criar cards, reportar progresso, pedir revisão), a ferramenta de gestão deixa de ser passiva e vira parte do loop. Este capítulo mostra como este projeto faz isso com três peças: um servidor MCP, pipelines em TOML e marcadores de texto no log.

## Os executores: uma dúzia de agentes, uma interface

`crates/executors/src/executors/` tem um módulo por agente de coding suportado — `claude`, `codex`, `gemini`, `opencode`, `cursor`, `amp`, `copilot`, `droid`, `qwen`, `antigravity`, `acp` — e `qa_mock.rs`, um **executor falso** para testes. Detalhe que importa: você testa orquestração sem gastar tokens, sem API key e sem flakiness de LLM.

Ao redor deles:

| Módulo | Papel |
| --- | --- |
| `approvals.rs` | Fluxo de permissões de ferramenta (o agente pede, o humano aprova/nega) |
| `command.rs` / `env.rs` | Como processos são montados (env, args, workdir) |
| `stdout_dup.rs` | Duplica saída para log + UI ao vivo |
| `mcp_config.rs` | Injeta o MCP server no agente (para que ele enxergue o board como tools) |
| `executors/mod.rs` | Registry: qual executor usar por workspace |

A lição para o seu SaaS: se você orquestra múltiplos agentes (ou múltiplas IAs para billing/suporte), isole o adaptador. O orquestrador não deve saber se é Claude ou Codex — só que existe `spawn(prompt) → stream de log`.

## O servidor MCP: a API do quadro, falada por agentes

O binário `vibe-kanban-mcp` (`crates/mcp/`) expõe o quadro kanban como **ferramentas MCP** — o protocolo que Claude Code, OpenCode e cia. já falam nativamente. As tools vivem em `crates/mcp/src/task_server/tools/`, um arquivo por domínio:

| Arquivo | Tools | Quando o agente usa |
| --- | --- | --- |
| `issues.rs` | `create_issue`, `get_issue`, `update_issue`, `list_issues`, `list_issue_priorities` | Criar sub-cards, mover de coluna, ler descrição |
| `workspaces.rs` / `sessions.rs` | `create_workspace`, `start_workspace`, `list_workspaces`, `run_session_prompt` | Abrir worktree, mandar follow-up |
| `pipeline.rs` / `rules.rs` | `get_pipeline`, `report_pipeline_stage`, `get_rules`, `get_orchestrator_prompt` | **Protocolos do `AGENTS.md`** (cap. 10) |
| `approvals.rs` | `list_pending_approvals`, `respond_to_approval` | Um agente (ou humano via TUI) destrava outro |
| `mem0.rs` | `memory_search`, `memory_save`, `memory_graph_traverse`, `memory_check_staleness` | Memória compartilhada do projeto |
| `context.rs`, `projects.rs`, `repos.rs`, `tags.rs`… | `get_context`, `list_projects`, `list_repos`, `list_tags` | Metadados e organização |

O efeito prático: **o card que você está lendo foi executado por um agente que chamou `get_pipeline`, reportou `VK-PIPELINE-STAGE` e commitou — tudo pelas tools acima**. A ferramenta de gestão e o executor do trabalho são o mesmo sistema. Não há "integração" — há um único binário que serve UI para humanos e tools para IAs.

> **Exercício:** abra `crates/mcp/src/task_server/tools/pipeline.rs` e leia `get_pipeline`. Note como o card carrega só `<!-- vk:pipeline:start -->` e o conteúdo pesado vem do TOML — isso economiza contexto do agente (cap. 06).

## Pipelines em TOML: o processo como configuração versionada

O processo de trabalho não está hard-coded no Rust: vive em `assets/pipelines/*.toml` — `quick`, `basic`, `speckit`, `swarm-multi-agent`, `wikillm` e variantes `async-*`. Anatomia de um estágio, do `quick.toml` (o do livro):

```toml
[[stage]]
id = "review-manual"
label = "Manual review (alarm)"
default_enabled = false
prompt = "MANUAL REVIEW: stop here and hand the work to the operator. Run `git log --oneline -5`, describe what was done, emit VK-REVIEW-REQUEST and STOP — do not merge."
```

Cada estágio é um **fragmento de prompt** com `id`, `label`, `default_enabled` e `prompt`. O card carrega só um ponteiro (`pipeline = "quick"`); o conteúdo pesado vem do `get_pipeline` quando o agente inicia. Isso importa para contexto de IA: o prompt do estágio entra na janela do agente **só quando o card roda**, não em toda listagem de board.

O ciclo de um card com pipeline (cap. 06 aprofundado):

```
get_pipeline("quick") → 3 stages (implement, verify, review-manual)
  → agente executa stage 1 → escreve VK-PIPELINE-STAGE: 1 no log → report_pipeline_stage(1)
  → stage 2 → VK-PIPELINE-STAGE: 2
  → stage 3 (se enabled) → VK-REVIEW-REQUEST + STOP
```

## Marcadores de texto: a orquestração invisível (o log é o protocolo)

Dois marcadores sustentam o loop humano↔agente, ambos parseados do **stream de log (`MsgStore`)** por serviços dedicados — a ideia do cap. 13 levada ao extremo:

### `VK-PIPELINE-STAGE: N` → `pipeline_stage.rs`

Regex: `(?i)VK-PIPELINE-STAGE:\s*(\d+)`, com guarda de fronteira `has_valid_boundary` para não casar com `FOOVK-PIPELINE-STAGE` nem com o placeholder literal `<n>` da doc. O **último marcador válido da linha vence**; o estágio é persistido em `workspaces.current_pipeline_stage` — e o checklist de progresso do card se atualiza ao vivo na UI (cap. 05, painel direito). Funciona igual em modo **headless** (stdout do processo filho) e **headed** (tail do transcript).

### `VK-REVIEW-REQUEST: <msg>` → `review_request.rs`

Ao detectar, chama `NotificationService.notify("Manual Review Required", ...)` — o **alarme sonoro** que tira o operador do sofá. Idempotente por execução (`TRACKED_EXECUTIONS`) e best-effort: notificação falha nunca bloqueia trabalho.

### Por que texto no log, não API?

Nenhum executor precisa saber da existência do marcador — quem sabe é o serviço que lê o stream. O agente (Claude, Codex, OpenCode…) só escreve texto; o backend observa. Isso mantém **todos os executores iguais** do ponto de vista do orquestrador e torna o protocolo **testável com `qa_mock`**: basta o mock escrever `VK-PIPELINE-STAGE: 1` no stdout.

## Supervisão: TUI, Telegram e o cachorro de guarda do orquestrador

Para o humano (ou outro agente) supervisionar sem vigiar, três peças (`automation/README.md`):

| Peça | Comando | O que faz | Quando usar |
| --- | --- | --- | --- |
| **TUI** | `cargo run -p tui` | Cockpit de terminal — lista workspaces/sessões, transcripts ao vivo, caixa de approvals | Você está no terminal e quer ver tudo sem abrir o browser |
| **Telegram bridge** | `cargo run -p telegram-bridge` | Daemon **send-only** — approvals do backend viram mensagens no Telegram (tópico por worktree, `~/.vibe-kanban/telegram.toml`) | Você está longe do desk e precisa aprovar |
| **OrchestratorCompactor** | `crates/services/src/services/orchestrator_compactor.rs` | Watchdog que evita a sessão do orquestrador estourar contexto em runs de dias | Automático — você não chama, ele vigia |

Teclas da TUI: `a` approvals, `n` nova task, `i` mensagem ao agente, `?` ajuda.

O **OrchestratorCompactor** merece destaque: a cada 60s mede os tokens do transcript; se passar de **400k** (ou 1h sem compactar com pelo menos 50k), digita `/compact` na sessão tmux — pelo caminho de **teclas digitadas**, porque slash commands não funcionam como texto colado. Cooldown de 10min entre envios; 3 falhas seguidas escalam para o Telegram. É o "garbage collector" de contexto do orquestrador.

> **Para o AssinaFácil:** você não precisa de TUI/Telegram no dia 1. Mas precisa do padrão: supervisionar é ler o log e aprovar — não ficar olhando a tela. O Telegram bridge é só um `tail -f` com notificação.

## Checklist do capítulo

- [ ] Os agentes têm uma API de ferramentas para o sistema que os gerencia (MCP ou equivalente) — e um executor falso para testes.
- [ ] O processo (pipeline) é configuração versionada (`*.toml`), não código espalhado por `if`.
- [ ] Progresso e pedidos de humano são marcadores de texto com gramática, parser e testes (não chamadas de API por executor).
- [ ] O log é o protocolo — funciona igual em headless e headed.
- [ ] Supervisão tem caminho humano (TUI/UI) e caminho remoto (Telegram), com escalação automática.
- [ ] O watchdog de contexto evita estouro silencioso em runs longos.
