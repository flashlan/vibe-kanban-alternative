# Capítulo 5 — Cards e Kanban — ciclo de vida na prática

> **Objetivo:** criar cards que viram prompts bons, mover com intenção e entender como os cards mudam de coluna sozinhos.

## O que cabe num card

Em `docs/issue-management.mdx:13`, um card tem:

| Campo | O que preencher |
| --- | --- |
| **Title** | O resultado esperado, específico ("Criar página de planos do SaaS", não "Fix bug") |
| **Description** | Contexto + requisitos + instruções — vira o prompt do agente |
| **Status** | Coluna onde o card está (Todo → Próximos passos / In Progress → Em andamento / In Review → Em revisão / Done → Concluído) |
| **Priority** | Urgent / High / Medium / Low |
| **Tags** | Etiquetas do projeto (crie inline no seletor) |
| **Simple ID** | Identificador curto tipo `AF-1` (vem de `project.key` em `projects.toml`) |

No código: `crates/db/src/models/issue.rs` e `crates/db/src/models/project_status.rs`. No frontend: `packages/web-core/src/shared/dialogs/kanban/CreateIssueDialog.tsx`.

## 1. Criar um card — a janela na prática

No board, clique no **+** da coluna desejada (o card já nasce nessa coluna) ou no botão **New Issue** da barra de filtros (`docs/issue-management.mdx:22`). A janela que abre é esta — com suas três partes:

**Topo — título, status, prioridade e tags:**

![Topo da janela de criar card — Title, Status, Priority e Tags](/images/livro/ancora-criar-card-topo.png)

- **Title** — claro e com verbo ("Adicionar checkout com Stripe"). É o que aparece no card no board.
- **Status** — coluna inicial (ex.: `Próximos passos`). Equivale a `project_status` no DB.
- **Priority** e **Tags** — `High` + `billing`, por exemplo. Tags são criadas inline no seletor (`docs/issue-management.mdx:91`).

**Base — descrição e botão Save:**

![Base da janela de criar card — Description e Save](/images/livro/ancora-criar-card-base.png)

- **Description** — use o editor rico (negrito, listas, código inline, `#` para heading, `[texto](url)`). É opcional, mas decisivo: **a descrição vira o prompt que o agente recebe** ao criar a workspace. Um card sem descrição gera um agente sem direção.
- Clique em **Save** para criar. Se marcar **Create draft workspace immediately**, o app já cria a workspace vinculada (atalho útil para ir direto ao trabalho).

> **Dica do Vibe Guide (`docs/vibe-guide.mdx:22`):** cinco minutos de plano economizam dez de revisão. Um card bem escrito é o plano.

### Escrever descrições que o agente acerta

Compare (`docs/issue-management.mdx:70`):

| Fraco | Forte |
| --- | --- |
| "Tá quebrado" | "Usuários em 3G veem timeout após 5s no login. Esperado: retry com backoff exponencial. Validar com `pnpm run check`." |

Inclua: o que fazer, restrições, arquivos/áreas relevantes, e como vai validar (ex.: "rodar `pnpm run check` deve passar; screenshot do checkout em `/images/livro/saas-checkout.png` deve coincidir").

## 2. Da criação à workspace — o fluxo básico

**Na mesma janela, crie a workspace:**

![Seção Workspaces dentro do card — botão Create](/images/livro/ancora-criar-card-workspace.png)

Após salvar o card, na mesma janela aparece a seção **Workspaces**. Clique em **Create** — o app cria um workspace vinculado ao card (worktree `vk/xxxx-nome` a partir do `target branch`, ver cap. 07), abre a **workspace view** principal e o agente já começa a trabalhar. A partir daí é só enviar mensagens no chat.

O workspace criado aparece na **global sidebar** à direita (ou esquerda, conforme layout) — é o mesmo que você vê no print do board principal (`ancora-board-principal.png`).

## 3. Como os cards se movem (manual e automático)

Os cards se movem de duas formas:

### Manual — você arrasta

Clique no card e arraste para outra coluna, ou troque o **Status** no painel do card. **Título e descrição salvam automaticamente** após parar de digitar; **status/prioridade/tags salvam imediatamente** (`docs/issue-management.mdx:96`). Se o sort do board não estiver em **Manual**, o drag-to-reorder é desabilitado — troque para Manual no header.

### Automático — o pipeline move por você

Quando você cria o card de um jeito que o agente consegue trabalhar (descrição clara) e envia a primeira mensagem na workspace, o agente inicia e o card vai para **In Progress** (Em andamento). Ao finalizar o trabalho, vai para **In Review** (Em revisão) — é onde você revisa diffs e Preview.

Fluxo completo:

```
Você cria o card          → Próximos passos (Todo)
Envia mensagem na workspace → Em andamento (In Progress)  ← agente trabalhando
Agente finaliza            → Em revisão (In Review)       ← você revisa
Você aprova (ou move manual) → Concluído (Done)
Precisa de ajuste? Instrui o agente → volta para Em andamento
```

Se em **Settings** o "mover cards automaticamente" estiver ligado, essas transições acontecem sozinhas; se não, você move manualmente — ambos funcionam. O livro recomenda deixar o auto-move ligado para o fluxo básico e mover manualmente quando quiser controlar o ritmo.

## 4. Barra da workspace — Tasks, conversa, presets e anexos

Dentro da workspace, a barra inferior concentra tudo que você usa enquanto o agente trabalha:

![Barra da workspace — Tasks, lista de mensagens, modelo, presets, permissões e anexos](/images/livro/ancora-workspace-chat-bar.png)

De cima para baixo, na mesma barra:

| Elemento | O que é | Código / doc |
| --- | --- | --- |
| **Detalhes de Tasks** | Tarefas cumpridas pelo modelo na barra superior | `crates/services/src/services/pipeline_stage.rs` (`VK-PIPELINE-STAGE: N`) |
| **Lista de mensagens** | Histórico da conversa com o agente | `crates/mcp` + `crates/executors` |
| **Open workspace** | Botão que abre a workspace em tela cheia | `docs/workspaces/interface.mdx:82` |
| **Ícone do modelo** | Qual agente está rodando (ex.: Claude Code) | `crates/executors/src/executors/` |
| **Contexto usado** | Tokens/contexto consumido na sessão | `crates/services/src/services/orchestrator_compactor.rs` |
| **Session** | Sessão atual (troque ou crie New Session) | `docs/workspaces/interface.mdx:119` |
| **Presets** | Atalhos de configuração do agente | `~/.vibe-kanban/projects.toml` / Settings |
| **Modelo** | Seletor de modelo (ex.: Sonnet, Opus) | `crates/executors/src/executors/` |
| **Permissões** | Nível de permissão do agente (YOLO / ask) | `crates/executors/src/approvals.rs` (`docs/vibe-guide.mdx:52`) |
| **Preset de agent** | Perfil do agente (varia por executor) | `crates/mcp/src/task_server/tools/` |
| **Anexos** | Arraste e solte imagens direto no chat — **a melhor forma de enviar referência visual** | `crates/server/src/routes/attachments.rs:83` (`POST /api/attachments/upload`, multipart `image`, 20 MB, `image/png|jpeg|gif|webp`) |
| **Preview Changes** | Alterna entre Preview e Changes | `docs/workspaces/interface.mdx:143` |
| **Quero mensagens** | Campo de envio de follow-ups | `docs/workspaces/interface.mdx:101` |

> **Dica de ouro para imagens:** arrastar e soltar a imagem direto no chat é a melhor alternativa — o app faz upload para `/api/attachments/upload`, cria `AttachmentResponse` (`attachments.rs:46`) e o agente recebe a imagem como contexto visual. Use para mandar mock, screenshot do erro ou referência de layout do SaaS (cap. 08).

## 5. Seções do card (após criado)

No painel do card já criado (`docs/issue-management.mdx:102`, `/images/issue-mgmt-link-workspace.png`):

- **Workspaces** — workspaces vinculados (onde o agente trabalha). Crie com **+** ou vincule existente; vários em paralelo são possíveis.
- **Sub-Issues** — quebre um épico em tarefas menores (`/images/issue-mgmt-sub-issues.png`). Cada sub-issue tem status próprio e pode ter sub-issues recursivamente.
- **Comments** — discussão da tarefa.

## 6. Quebrar trabalho grande (para o SaaS do cap. 08)

Crie um card pai "SaaS AssinaFácil — MVP" e sub-issues:

- "Setup do monorepo (Vite + Tailwind)"
- "Auth (login/cadastro)"
- "Página de planos e checkout"
- "Webhooks Stripe + entitlements"

Cada sub-issue vira um workspace independente — despache 3 agentes em paralelo sem conflito (cada um no seu worktree/branch `vk/xxxx-*`).

## 7. Ações, seleção múltipla e bulk

- **More (⋯)** no painel do card ou **command bar** (`Cmd/Ctrl + K` → Issue Actions): mudar status/prioridade, transformar em sub-issue, vincular workspace, duplicar, deletar.
- **Seleção múltipla**: `Cmd/Ctrl + Click` alterna, `Shift + Click` intervalo, `Cmd/Ctrl + A` todos. Com 2+ selecionados, surge a **bulk action bar** para mudar status/prioridade ou deletar em lote (bulk delete é permanente).

## Checklist do capítulo

- [ ] Criei um card pela janela (preenchi Title, Status, Priority/Tags, Description e cliquei Save).
- [ ] Criei uma workspace a partir do card (seção Workspaces → Create) e vi o agente iniciar.
- [ ] Entendi o fluxo Todo → In Progress (mensagem) → In Review (agente finaliza) → Done (eu aprovo) e quando o agente volta para In Progress.
- [ ] Sei usar a barra da workspace: anexar imagem por drag-drop, trocar modelo/preset, ver contexto e trocar sessão.
- [ ] Criei um épico com 3 sub-issues e vinculei workspaces.
