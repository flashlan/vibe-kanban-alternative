# Capítulo 3 — Tour da interface

> **Objetivo:** saber onde cada coisa mora antes de criar o primeiro card.

## O app em um mapa

Tudo no Vibe Kanban acontece em dois lugares:

1. **Board do projeto** — onde você planeja (cards e colunas).
2. **Workspace view** — onde você executa (conversa com o agente + diffs + preview).

A **global sidebar** (barra lateral esquerda, presente em todas as telas) conecta os dois. Ela é descrita em `docs/workspaces/interface.mdx:54`.

## A global sidebar

```
Projetos
 └─ Meu SaaS (projeto raiz)
    ├─ Tasks          ← cards deste projeto
    └─ Workspaces     ← todos os workspaces deste projeto (Active / Running / Idle / Archived)
```

- **Projects** no topo, com **+** para criar projeto.
- Cada projeto tem **Tasks** (os cards) e, se for raiz, **Workspaces** agregados.
- Workspaces aparecem como folhas agrupadas em **Active / Running / Idle / Needs Attention / Archived**. Um ponto azul indica dev server rodando; um badge indica PR vinculado; um ícone de mão levantada indica `Needs Attention` (aprovação pendente).
- Para uma lista plana com busca/filtros, abra o **Workspaces dashboard** (`/workspaces`).

## O board (kanban)

Abra um projeto para cair no board (`docs/getting-started.mdx:44`):

1. **App bar** — navega entre projetos, Workspaces e Settings.
2. **Cards** — issues como cartões nas colunas (Todo, In Progress, In Review, Done — configuráveis por projeto).
3. **+ em cada coluna / botão New Issue** — cria card já na coluna certa.
4. **Painel direito** — detalhes do card selecionado (ou do rascunho em criação).

Screenshots de referência: `/images/onboarding-projects.png`, `/images/onboarding-workspaces-page.png`.

## A workspace view — os três painéis

Ao abrir um workspace (`docs/workspaces/interface.mdx:10`), a tela se divide em:

| Painel | Posição | Para que serve |
| --- | --- | --- |
| **Conversation** | Esquerda (principal) | Chat com o agente, troca de sessões, envio de follow-ups |
| **Context** | Direita (principal, alternável) | **Changes** (diffs) / **Logs** (stdout em tempo real) / **Preview** (browser embutido) |
| **Details Sidebar** | Borda direita | Git (repo/branch, ahead/behind), Terminal (xterm.js), Notes (auto-save por workspace) |

### Navbar da workspace

Na barra superior da workspace (`docs/workspaces/interface.mdx:20`):

- Esquerda: **Archive Workspace**.
- Centro-direita (controles de painel): Toggle Left Sidebar / Chat / Changes / Logs / Preview / Right Sidebar.
- Direita (utilidades): **Spawn Orchestrator**, **Command Bar** (`Cmd/Ctrl + K`), **Projects Guide**, **Settings**.

Uma dica salva tempo: o **Context Bar** — barra flutuante arrastável com atalhos para abrir no IDE, copiar caminho da workspace, ligar dev server e alternar Preview/Changes — descrita em `docs/workspaces/interface.mdx:239`.

### Conversation panel

- Histórico completo com o agente, suporte a rich text e aprovação de planos.
- **Session dropdown** na toolbar do chat: alterna entre sessões, cria **New Session** quando o contexto fica grande.
- Atalhos: `Cmd/Ctrl + Enter` envia; `Shift + Cmd/Ctrl + Enter` envia em modo alternativo; `Cmd/Ctrl + B/I/U` formata.

### Context panel — Changes / Logs / Preview

- **Changes** (`/images/workspaces-changes-panel.png`): árvore de arquivos modificados + diffs com syntax highlight + comentários inline para dar feedback ao agente.
- **Logs** (`/images/workspaces-logs-panel.png`): abas por processo, busca no log, stdout/stderr em tempo real. É aqui que você vê `VK-PIPELINE-STAGE: N` sendo reportado ao vivo quando o agente avança no pipeline.
- **Preview** (`/images/workspaces-preview-panel.png`): browser embutido que sobe via **Preview proxy** (Rust) + seu **Dev server script** (Node). Suporta múltiplas tabs, modos desktop/mobile e detecção automática da URL nos logs (ver `docs/browser-testing.mdx:34`).

### Details sidebar — Git / Terminal / Notes

- **Git** (`/images/workspaces-git-panel.png`): repo e branch atuais, target branch, contagem de mudanças não commitadas, commits à frente/atrás — e atalho para operações git (`docs/workspaces/git-operations.mdx`).
- **Terminal** (`/images/workspaces-terminal.png`): xterm.js direto no ambiente da workspace — rode `git`, `npm`, `pnpm`, `cargo` ali mesmo.
- **Notes** (`/images/workspaces-notes.png`): editor rich text por workspace, auto-save.

## Command bar

`Cmd/Ctrl + K` abre a command bar (`docs/workspaces/command-bar.mdx`): criar workspace, arquivar, duplicar, alternar painéis, ações de issue — tudo sem tirar a mão do teclado.

## Checklist do capítulo

- [ ] Sei abrir o board de um projeto e identificar app bar / colunas / cards / painel direito.
- [ ] Sei abrir uma workspace e alternar entre Conversation / Changes / Logs / Preview.
- [ ] Sei usar a global sidebar para navegar entre projetos e workspaces por estado.
- [ ] Sei abrir a command bar (`Cmd/Ctrl + K`) e o terminal embutido.
