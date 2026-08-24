# Capítulo 7 — Git, workspaces e worktrees

> **Objetivo:** usar git dentro do Vibe Kanban sem medo — cada workspace é um branch isolado que você pode revisar, testar e mergear com confiança.

## O que acontece quando você clica em Create

`docs/workspaces/creating-workspaces.mdx:12` resume em 4 passos o que o botão faz:

1. **Git worktree** — cria um diretório separado com seu próprio branch, isolado do repo original. Seu código original não é tocado.
2. **Working branch** — branch auto-gerado a partir do **target branch** (ex.: `main` → `vk/a1b2-criar-pagina-de-planos`). É aqui que o agente commita.
3. **Sessão do agente** — o agente escolhido é inicializado e já recebe sua tarefa (a descrição do card).
4. **Setup scripts** — se o projeto/repo tiver `setup_script` (ex.: `pnpm install`), ele roda automaticamente.

No disco: worktrees ficam em `.vibe-kanban-workspaces/` por padrão (configurável em Settings → General → Workspace Directory). Cada workspace ganha sua pasta — por isso você pode rodar 3 agentes em paralelo sem conflito (cada um no seu `vk/xxxx-*`).

No código: `crates/worktree-manager`, `crates/workspace-manager`, `crates/git` e `crates/git-host` cuidam da criação, listagem e teardown; `crates/db/src/models/workspace.rs` e `workspace_repo.rs` persistem o vínculo.

## Criar um workspace, passo a passo (na interface)

`docs/workspaces/creating-workspaces.mdx:38`, com screenshots em `/images/workspaces-*.png`:

1. **Abra o Create View** — `Cmd/Ctrl + K` → New Workspace, ou Dashboard de Workspaces, ou o **+** na seção **Workspaces** de um card (já vincula o workspace ao card — o atalho mais usado).
2. **Selecione o Project** no dropdown da direita.
3. **Adicione Repositórios** — clique nos recentes, ou **Browse repos on disk**, ou **Create new repo on disk** (inicializa um git novo). Você pode adicionar **vários repos** num mesmo workspace — cada um mantém git independente (útil para o SaaS com `app-web` + `api`).
4. **Defina o Target Branch** por repo — onde seu trabalho vai mergear (ex.: `main`). Clique no dropdown ao lado do repo para trocar.
5. **Descreva a tarefa** no chat embaixo — seja específico (cap. 5: título com verbo + critério de pronto).
6. **Escolha o Agent** e variante (modelo, permissões).
7. **Create** — o agente começa imediatamente; o card vai para **In Progress**.

> **Target vs Working branch** (`docs/workspaces/creating-workspaces.mdx:80`):
> - **Target** = onde vai mergear (ex.: `main`). Você define.
> - **Working** = onde o agente trabalha (ex.: `vk/a1b2-...`). Auto-criado a partir do target. Só afeta o target quando você cria e mergeia um PR.

## Dentro da workspace — onde o git mora

Já visto no tour (cap. 4), aqui com foco em git:

- **Details Sidebar → Git** (`/images/workspaces-git-panel.png`): repo/branch atuais, target branch, contagem de mudanças não commitadas, commits ahead/behind — e atalho para **Create PR / Merge / Rebase** (`docs/workspaces/git-operations.mdx`).
- **Terminal** (`/images/workspaces-terminal.png`): xterm.js no ambiente da workspace — rode `git status`, `git log --oneline -5`, `pnpm run check`, `cargo test` ali mesmo. O terminal persiste entre trocas de painel.
- **Context → Changes** (`/images/workspaces-changes-panel.png`): árvore de arquivos + diffs inline com syntax highlight + comentários inline para dar feedback ao agente ("este diff deveria mexer em `plans.ts`, não em `landing.tsx`").
- **Context → Preview** (`/images/workspaces-preview-panel.png`): browser embutido. Configure o **Dev server script** do projeto (ex.: `pnpm --filter app-web dev` em `projects.toml` ou Settings) e ligue com o botão **Play** na context bar. O `crates/preview-proxy` serve o dev Node dentro do painel.

### O ciclo de revisão com git

Para cada card do SaaS (cap. 8), repita:

```
Changes (diffs) → Preview (app rodando) → Terminal (pnpm run check / cargo test)
      ↓ comentário inline         ↓ clique no Preview
   agente corrige            volta para In Progress
      ↓ OK
   In Review → Done → Merge/PR
```

O painel **Logs** mostra `VK-PIPELINE-STAGE: N` avançando — é o pipeline (cap. 6) reportando estágio no `MsgStore` (`crates/services/src/services/pipeline_stage.rs:28`).

## Duplicar, arquivar e pinar

- **Duplicar:** `Cmd/Ctrl + K` → Workspace Actions → **Duplicate Workspace** (mesma config de repos/branches, conversa nova — útil para tentar outra abordagem sem perder a anterior).
- **Arquivar:** botão **Archive** na navbar ou `Cmd/Ctrl + K` → Workspace Actions → **Archive**. Arquivadas vão para **View Archive** no fim da sidebar; o `cleanup_script` do repo roda aqui. **View Archive** também é onde fica o **Delete** permanente.
- **Pinar:** **Pin** mantém workspaces ativas no topo da lista — use para o épico do SaaS enquanto as sub-tarefas correm.

## Troubleshooting rápido

| Sintoma | Causa comum | O que fazer |
| --- | --- | --- |
| Repo não aparece na lista | Pasta não é git / não está num projeto | Browse repos on disk; confirme `.git` |
| Falha ao criar workspace | Mudanças não commitadas no repo original / conflito de nome de branch | Commit/stash no repo original; troque target branch |
| Agente não inicia | Agente não instalado / API key / rede | Rode o CLI do agente no terminal da workspace; confira Settings → Agents |
| Setup script falha | Erro no script / dependência | Teste o script no terminal; veja o painel Logs |
| `AddrInUse` ao subir dev server | Porta `:5173`/`:3000`/`:3001` já ocupada | `lsof -nP -i :5173 -sTCP:LISTEN` + `cwd` do PID (cap. 03) |
| Card não sai de In Review | Pipeline em `review-manual` esperando você | Verifique `VK-REVIEW-REQUEST` em Logs; aprove ou instrua o agente |

O `docs/workspaces/creating-workspaces.mdx:201` lista esses casos em `<Accordion>` — vale marcar como favorito.

## Checklist do capítulo

- [ ] Criei um workspace vinculado a um card e identifiquei o working branch `vk/xxxx-*` em Details → Git.
- [ ] Sei a diferença entre target e working branch e onde cada um aparece na UI.
- [ ] Rodei `git status` e `pnpm run check` dentro do terminal da workspace.
- [ ] Configurei dev server script e abri o Preview com o app do SaaS rodando.
- [ ] Dupliquei e arquivei uma workspace e a encontrei em View Archive.
