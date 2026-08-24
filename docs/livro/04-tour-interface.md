# Capítulo 4 — Tour da interface

> **Objetivo:** saber onde cada coisa mora antes de criar o primeiro card — e reconhecer os 3 lugares onde todo o trabalho acontece.

## O app em um mapa

Tudo no Vibe Kanban acontece em dois lugares:

1. **Board do projeto** — onde você **planeja** (cards e colunas).
2. **Workspace view** — onde você **executa** (conversa com o agente + diffs + preview).

A **global sidebar** (barra lateral, presente em todas as telas) conecta os dois. Ela é descrita em `docs/workspaces/interface.mdx:54` e é o seu GPS: de qualquer tela, você volta para qualquer projeto ou workspace em 2 cliques.

```
┌─ Global sidebar ──────────────────────┐  ┌─ Área principal ──────────────────┐
│ Projetos                             │  │ Board  OU  Workspace view         │
│  └─ Meu SaaS (projeto raiz)          │  │                                   │
│     ├─ Tasks          ← cards        │  │  ← o que muda quando você navega │
│     └─ Workspaces     ← workspaces   │  │                                   │
│        ├─ Active / Running / Idle    │  │                                   │
└──────────────────────────────────────┘  └───────────────────────────────────┘
```

## A global sidebar — leia em 10 segundos

```
Projetos
 └─ Meu SaaS (projeto raiz)
    ├─ Tasks          ← cards deste projeto
    └─ Workspaces     ← todos os workspaces deste projeto (Active / Running / Idle / Archived)
```

- **Projects** no topo, com **+** para criar projeto.
- Cada projeto tem **Tasks** (os cards) e, se for raiz, **Workspaces** agregados.
- Workspaces aparecem como folhas agrupadas em **Active / Running / Idle / Needs Attention / Archived**. Um ponto azul indica dev server rodando; um badge indica PR vinculado; um ícone de mão levantada (`Needs Attention`) indica que o agente pediu aprovação — é o equivalente visual do `VK-REVIEW-REQUEST` do cap. 02 §3.
- Para uma lista plana com busca/filtros, abra o **Workspaces dashboard** (`/workspaces`).

Atalho que salva tempo: `Cmd/Ctrl + K` → digite o nome do projeto ou workspace — a command bar (`docs/workspaces/command-bar.mdx`) te leva direto, sem scroll.

## O board (kanban) — onde você planeja

Abra um projeto para cair no board (`docs/getting-started.mdx:44`). O board tem 4 zonas:

1. **App bar** (topo) — navega entre projetos, Workspaces e **Settings** (engrenagem). É de onde você importa `projects.toml` e troca agente/IDE.
2. **Colunas** — cada coluna é um `project_status` (ex.: Todo → Próximos passos / In Progress → Em andamento / In Review → Em revisão / Done → Concluído). Configuráveis por projeto via `projects.toml → statuses` (`docs/cockpit/local-projects.mdx`) ou na UI.
3. **Cards** — issues como cartões. Cada card mostra título, prioridade, tags e Simple ID (`AF-1`). O header de cada coluna tem um **+** que cria card já na coluna certa — e o botão **New Issue** na barra de filtros faz o mesmo.
4. **Painel direito** — detalhes do card selecionado (ou do rascunho em criação). É onde vivem **Workspaces**, **Sub-Issues** e **Comments** do card (ver cap. 05).

**Âncora do livro — board principal:**

![Board principal — projeto Novo aplicativo SaaS com colunas Próximos passos / Em andamento / Em revisão / Concluído](/images/livro/ancora-board-principal.png)

*O board do livro (projeto "Novo aplicativo SaaS"): 4 colunas em PT-BR — Próximos passos, Em andamento, Em revisão, Concluído (são `project_status` configuráveis via `projects.toml` → `statuses`, `docs/cockpit/local-projects.mdx`). Cada coluna mostra a contagem; o painel direito abre o card selecionado. Screenshots de referência do site: `/images/onboarding-projects.png`.*

> **Exercício de 30 segundos:** conte as colunas na âncora acima. São 4 — as mesmas que você declarou em `projects.toml` no cap. 03. Mude `statuses` para 3 ou 5 e recarregue — o board reflete na hora. É assim que você sente que o board é só uma view do `project_status` no SQLite (`crates/db/src/models/project_status.rs`).

**Âncora do livro — workspace aberta:**

![Workspace aberta — Conversation à esquerda, Context (Changes/Logs/Preview) ao centro, Details (Git/Terminal) à direita](/images/livro/ancora-workspace-aberta.png)

*Workspace do AssinaFácil aberta: à esquerda a Conversation com o agente; ao centro o Context alternando Changes/Logs/Preview; à direita o Details com Git/Terminal/Notes — exatamente os três painéis de `docs/workspaces/interface.mdx:10`.*

## A workspace view — os três painéis (onde você executa)

Ao abrir um workspace (`docs/workspaces/interface.mdx:10`), a tela se divide em:

| Painel | Posição | Para que serve | Quando você usa |
| --- | --- | --- | --- |
| **Conversation** | Esquerda (principal) | Chat com o agente, troca de sessões, envio de follow-ups | 80% do tempo — é onde você pede e corrige |
| **Context** | Direita (principal, alternável) | **Changes** (diffs) / **Logs** (stdout) / **Preview** (browser) | Para revisar o que o agente fez e ver o app rodando |
| **Details Sidebar** | Borda direita | Git (repo/branch, ahead/behind), Terminal (xterm.js), Notes | Para git, comandos e anotações rápidas |

Você não precisa decorar — a **navbar da workspace** (`docs/workspaces/interface.mdx:20`) tem botões para ligar/desligar cada painel:

- Esquerda: **Archive Workspace**.
- Centro-direita (controles de painel): Toggle Left Sidebar / Chat / Changes / Logs / Preview / Right Sidebar.
- Direita (utilidades): **Spawn Orchestrator**, **Command Bar** (`Cmd/Ctrl + K`), **Projects Guide**, **Settings**.

Uma dica que economiza cliques: o **Context Bar** — barra flutuante arrastável com atalhos para abrir no IDE, copiar caminho da workspace (`Copy Path`), ligar dev server e alternar Preview/Changes — descrita em `docs/workspaces/interface.mdx:239`. Arraste para onde for confortável; ela persiste por workspace.

### Conversation panel — seu canal com o agente

- Histórico completo com o agente, suporte a rich text e aprovação de planos.
- **Session dropdown** na toolbar do chat: alterna entre sessões, cria **New Session** quando o contexto fica grande (o agente avisa; ver cap. 14 sobre o watchdog de 400k tokens).
- Atalhos: `Cmd/Ctrl + Enter` envia; `Shift + Cmd/Ctrl + Enter` envia em modo alternativo; `Cmd/Ctrl + B/I/U` formata.
- **Anexos:** arraste e solte imagem direto no chat — o app faz upload para `POST /api/attachments/upload` (`crates/server/src/routes/attachments.rs:83`, 20 MB, `image/png|jpeg|gif|webp`) e o agente recebe como contexto visual. É a melhor forma de mandar mock ou screenshot do erro (ver cap. 05 §4 detalhado).

### Context panel — Changes / Logs / Preview (alterna com um clique)

- **Changes** (`/images/workspaces-changes-panel.png`): árvore de arquivos modificados + diffs com syntax highlight + **comentários inline** para dar feedback ao agente ("este diff deveria mexer em `plans.ts`, não em `landing.tsx`").
- **Logs** (`/images/workspaces-logs-panel.png`): abas por processo, busca no log, stdout/stderr em tempo real. É aqui que você vê `VK-PIPELINE-STAGE: N` sendo reportado ao vivo quando o agente avança no pipeline (cap. 06) — e `VK-REVIEW-REQUEST` quando ele precisa de você.
- **Preview** (`/images/workspaces-preview-panel.png`): browser embutido que sobe via **Preview proxy** (Rust, `crates/preview-proxy`) + seu **Dev server script** (Node, `projects.toml` ou Settings). Suporta múltiplas tabs, modos desktop/mobile e detecção automática da URL nos logs (`docs/browser-testing.mdx:34`). Para o SaaS do cap. 08, é aqui que você vê `http://localhost:5173` rodando.

> **Quando usar cada um:** Changes para revisar código, Logs para entender por que o `check` quebrou, Preview para validar visual. O agente escreve nos três — você lê nos três.

### Details sidebar — Git / Terminal / Notes (sempre à mão)

- **Git** (`/images/workspaces-git-panel.png`): repo e branch atuais (`vk/xxxx-*`), target branch, contagem de mudanças não commitadas, commits ahead/behind — e atalho para **Create PR / Merge / Rebase** (`docs/workspaces/git-operations.mdx`, ver cap. 07).
- **Terminal** (`/images/workspaces-terminal.png`): xterm.js direto no ambiente da workspace — rode `git status`, `pnpm run check`, `cargo test` ali mesmo. Persiste entre trocas de painel.
- **Notes** (`/images/workspaces-notes.png`): editor rich text por workspace (auto-save). Use para anotar decisões do card — o próximo agente que abrir a workspace lê.

## Command bar e atalhos que importam

`Cmd/Ctrl + K` abre a command bar (`docs/workspaces/command-bar.mdx`) — crie workspace, arquive, duplique, alterne painéis, execute ações de issue, tudo sem mouse. Os 3 atalhos que você vai usar todo dia:

| Atalho | Onde | Faz |
| --- | --- | --- |
| `Cmd/Ctrl + K` | Global | Command bar |
| `Cmd/Ctrl + Enter` | Chat | Enviar mensagem |
| Arrastar imagem → chat | Chat | Anexar como contexto visual |

## Checklist do capítulo

- [ ] Sei apontar no board: app bar, colunas, cards, painel direito — e criar card pelo `+` da coluna ou New Issue.
- [ ] Sei apontar na workspace: Conversation, Context (Changes/Logs/Preview) e Details (Git/Terminal/Notes).
- [ ] Sei usar a global sidebar para navegar entre projetos e workspaces por estado (Active/Running/Needs Attention).
- [ ] Sei alternar Context entre Changes/Logs/Preview e abrir o terminal embutido.
- [ ] Sei abrir a command bar (`Cmd/Ctrl + K`) e enviar mensagem no chat.
