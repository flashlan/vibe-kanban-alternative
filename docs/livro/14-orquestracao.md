# Capítulo 14 — Orquestração de agentes: MCP, pipelines e o alarme

> **Princípio:** quando o próprio agente pode dirigir o fluxo de trabalho (criar cards, reportar progresso, pedir revisão), a ferramenta de gestão deixa de ser passiva e vira parte do loop. Este capítulo mostra como este projeto faz isso com três peças: um servidor MCP, pipelines em TOML e marcadores de texto.

## Os executores: uma dúzia de agentes, uma interface

`crates/executors/src/executors/` tem um módulo por agente de coding suportado: `claude`, `codex`, `gemini`, `opencode`, `cursor`, `amp`, `copilot`, `droid`, `qwen`, `antigravity`, `acp` — e `qa_mock.rs`, um executor falso para testes (detalhe que importa: você testa orquestração sem gastar tokens). Ao redor deles, `approvals.rs` (o fluxo de permissões de ferramenta), `command.rs` e `env.rs` (como processos são montados), `stdout_dup.rs` (duplicar a saída para log + UI) e `mcp_config.rs` (injetar o MCP server no agente — veja abaixo).

## O servidor MCP: a API do quadro, falada por agentes

O binário `vibe-kanban-mcp` (`crates/mcp/`) expõe o quadro kanban como ferramentas MCP — o protocolo que Claude Code, OpenCode e cia. já falam. As tools vivem em `crates/mcp/src/task_server/tools/`, um arquivo por domínio:

| Arquivo | Tools |
| --- | --- |
| `issues.rs` | criar/ler/atualizar/listar cards, prioridades |
| `workspaces.rs` / `sessions.rs` | criar workspaces, iniciar sessões, mandar prompt follow-up (`run_session_prompt`) |
| `pipeline.rs` / `rules.rs` | `get_pipeline`, `report_pipeline_stage`, `get_rules` — os protocolos do AGENTS.md (cap. 2) |
| `approvals.rs` | `list_pending_approvals`, `respond_to_approval` — um agente pode destravar outro |
| `mem0.rs` | memória compartilhada do projeto (`memory_search`, `memory_save`, grafo, checagem de staleness) |
| `context.rs`, `projects.rs`, `repos.rs`, `tags.rs`... | metadados e organização |

O efeito: o card que você está lendo foi executado por um agente que chamou `get_pipeline`, reportou `VK-PIPELINE-STAGE` e commitou — tudo pelas tools acima. A ferramenta de gestão e o executor do trabalho são o mesmo sistema.

## Pipelines em TOML: o processo como configuração

O processo de trabalho não está hard-coded: vive em `assets/pipelines/*.toml` (`quick`, `basic`, `speckit`, `swarm-multi-agent`, `wikillm`, variantes `async-*`...). Anatomia de um estágio, do `quick.toml`:

```toml
[[stage]]
id = "review-manual"
label = "Manual review (alarm)"
default_enabled = false
prompt = "MANUAL REVIEW: stop here and hand the work to the operator..."
```

Cada estágio é um **fragmento de prompt** com um id, um rótulo e se vem ligado por padrão. O card carrega só um ponteiro para o pipeline; o conteúdo pesado vem do `get_pipeline`. Isso importa para contexto de IA: o prompt do estágio entra na janela do agente **só quando o card roda**, não em toda listagem de board.

## Marcadores de texto: a orquestração invisível

Dois marcadores sustentam o loop humano↔agente, ambos parseados do stream de log (`MsgStore`) por serviços dedicados:

- **`VK-PIPELINE-STAGE: N`** → `pipeline_stage.rs`: regex `(?i)VK-PIPELINE-STAGE:\s*(\d+)`, com guarda de fronteira (`has_valid_boundary`) para não casar com `FOOVK-PIPELINE-STAGE` nem com o placeholder literal `<n>`. O último marcador válido da linha vence; o estágio é persistido em `workspaces.current_pipeline_stage` — e o checklist de progresso do card se atualiza ao vivo na UI.
- **`VK-REVIEW-REQUEST: <msg>`** → `review_request.rs`: ao detectar, chama `NotificationService.notify("Manual Review Required", ...)` — o alarme sonoro que tira o operador do sofá. Idempotente por execução (`TRACKED_EXECUTIONS`) e best-effort: notificação falha nunca bloqueia trabalho.

A lição de design: o canal agente→sistema é **texto no log com gramática formal**, parseado da mesma forma nos dois modos de execução (headless, via stdout do processo filho; headed, via tail do transcript). Nenhum executor precisa saber da existência do marcador — quem sabe é o serviço que lê o stream.

## Supervisão: TUI, Telegram e o cachorro de guarda do orquestrador

Para o humano (ou outro agente) supervisionar sem vigiar, três peças (`automation/README.md`):

- **TUI** (`cargo run -p tui`): cockpit de terminal — lista workspaces/sessões, transcripts ao vivo e caixa de approvals. Teclas: `a` approvals, `n` nova task, `i` mensagem ao agente, `?` ajuda.
- **Telegram bridge** (`cargo run -p telegram-bridge`): daemon **send-only** — aprovações do backend viram mensagens no Telegram (com tópico por worktree, configurado em `~/.vibe-kanban/telegram.toml`). Nunca lê de volta: quem responde é o humano ou o PM agent via MCP.
- **OrchestratorCompactor** (`crates/services/src/services/orchestrator_compactor.rs`): um watchdog que evita a sessão do orquestrador estourar contexto em runs de dias. A cada 60s mede os tokens do transcript; se passar de 400k (ou 1h sem compactar com pelo menos 50k), digita `/compact` na sessão tmux — pelo caminho de teclas digitadas, porque slash commands não funcionam como texto colado. Cooldown de 10min entre envios; 3 falhas seguidas escalam para o Telegram.

## Checklist do capítulo

- [ ] Os agentes têm uma API de ferramentas para o sistema que os gerencia (MCP ou equivalente).
- [ ] O processo (pipeline) é configuração versionada, não código espalhado.
- [ ] Progresso e pedidos de humano são marcadores de texto com gramática, parser e testes.
- [ ] Existe um executor falso para testar orquestração sem custo.
- [ ] Supervisão tem caminho humano (TUI/UI) e caminho remoto (Telegram), com escalação.
