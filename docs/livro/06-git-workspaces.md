# Capítulo 6 — Git, workspaces e worktrees

> **Objetivo:** usar git dentro do Vibe Kanban sem ter medo de perder trabalho — cada workspace é um branch isolado.

## O que acontece quando você cria um workspace

`docs/workspaces/creating-workspaces.mdx:12` explica em 4 passos o que o botão **Create** faz por trás:

1. **Git worktree** — cria um diretório separado com seu próprio branch, isolado do seu repo original. Seu código original não é tocado.
2. **Working branch** — branch auto-gerado a partir do **target branch** (ex.: `main` → `vk/a1b2-criar-pagina-de-planos`). É aqui que o agente commita.
3. **Sessão do agente** — o agente escolhido é inicializado e já recebe sua descrição de tarefa.
4. **Setup scripts** — se o projeto/repo tiver setup script (ex.: `pnpm install`), ele roda automaticamente.

Worktrees ficam em `.vibe-kanban-workspaces/` por padrão (configurável em Settings → General → Workspace Directory). Cada workspace ganha sua pasta.

## Criar um workspace, passo a passo

Na UI (`docs/workspaces/creating-workspaces.mdx:38`):

1. **Abra o Create View** — `Cmd/Ctrl + K` → New Workspace, ou Dashboard de Workspaces, ou o **+** na seção Workspaces de um card (já vincula).
2. **Selecione o Project** no dropdown da direita.
3. **Adicione Repositórios** — clique nos recentes, ou **Browse repos on disk**, ou **Create new repo on disk** (inicializa um repo git novo). Você pode adicionar **vários repos** num mesmo workspace — cada um mantém git independente.
4. **Defina o Target Branch** por repo (onde seu trabalho vai ser mergeado — ex.: `main`). Clique no dropdown ao lado do repo para trocar.
5. **Descreva a tarefa** no chat embaixo — seja específico (cap. 4).
6. **Escolha o Agent** e variante.
7. **Create** — o agente começa imediatamente.

> **Target vs Working branch** (`docs/workspaces/creating-workspaces.mdx:80`):
> - **Target** = onde vai mergear (ex.: `main`). Você define.
> - **Working** = onde o agente trabalha (ex.: `vk/a1b2-...`). Auto-criado a partir do target. Só afeta o target quando você abre e mergeia um PR.

## Dentro da workspace — o que usar

Já visto no tour (cap. 3), mas aqui com foco em git:

- **Details Sidebar → Git** (`/images/workspaces-git-panel.png`): repo/branch atuais, target branch, mudanças não commitadas, commits ahead/behind — e atalho para operações git (`docs/workspaces/git-operations.mdx`: criar PR, merge, rebase).
- **Terminal** (`/images/workspaces-terminal.png`): xterm.js no ambiente da workspace — rode `git status`, `git log --oneline -5`, `pnpm run check`, `cargo test` ali mesmo.
- **Context → Changes** (`/images/workspaces-changes-panel.png`): árvore de arquivos + diffs inline — revise e comente para o agente corrigir.
- **Context → Preview** (`/images/workspaces-preview-panel.png`): browser embutido. Configure o **Dev server script** do projeto (ex.: `pnpm dev`) e ligue com o botão Play no context bar.

## Duplicar e arquivar

- **Duplicar:** `Cmd/Ctrl + K` → Workspace Actions → Duplicate Workspace (mesma config de repos/branches, conversa nova).
- **Arquivar:** botão Archive na navbar ou `Cmd/Ctrl + K` → Workspace Actions → Archive. Arquivadas vão para **View Archive** no fim da sidebar; use **Pin** para manter ativas importantes no topo.

## Troubleshooting rápido

| Sintoma | Causa comum | O que fazer |
| --- | --- | --- |
| Repo não aparece na lista | Pasta não é git / não está num projeto | Browse repos on disk; confirme `.git` |
| Falha ao criar workspace | Mudanças não commitadas no repo original / conflito de nome de branch | Commit/stash no repo original; troque target branch |
| Agente não inicia | Agente não instalado / API key / rede | Rode o CLI do agente no terminal; confira Settings → Agents |
| Setup script falha | Erro no script / dependência | Teste o script no terminal; veja o painel Logs |

## Checklist do capítulo

- [ ] Criei um workspace vinculado a um card e identifiquei o working branch gerado.
- [ ] Sei a diferença entre target e working branch e onde cada um aparece.
- [ ] Rodei `git status` e `pnpm run check` dentro do terminal da workspace.
- [ ] Configurei dev server script e abri o Preview.
- [ ] Arquivei uma workspace e a encontrei em View Archive.
