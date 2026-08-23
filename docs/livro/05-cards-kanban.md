# Capítulo 4 — Cards e Kanban — ciclo de vida na prática

> **Objetivo:** criar cards que viram prompts bons, mover com intenção e quebrar trabalho grande em sub-tarefas.

## O que cabe num card

Em `docs/issue-management.mdx:13`, um card tem:

| Campo | O que preencher |
| --- | --- |
| **Title** | O resultado esperado, específico ("Criar página de planos do SaaS", não "Fix bug") |
| **Description** | Contexto + requisitos + instruções — vira o prompt do agente |
| **Status** | Coluna onde o card está (Todo, In Progress, In Review, Done) |
| **Priority** | Urgent / High / Medium / Low |
| **Tags** | Etiquetas do projeto (crie inline no seletor) |
| **Simple ID** | Identificador curto tipo `ACME-123` (vem da chave do projeto, aparece no card e na URL) |

## 1. Criar um card

No board, clique no **+** da coluna desejada (o card já nasce nessa coluna) ou no botão **New Issue** da barra de filtros (`docs/issue-management.mdx:22`, screenshot `/images/issue-mgmt-create-button.png`).

Passos (`docs/issue-management.mdx:30`):

1. **Title** — claro e com verbo ("Adicionar checkout com Stripe").
2. **Status** — escolha a coluna inicial.
3. **Priority e tags** (opcional) — ex.: `High` + `billing`.
4. **Description** (opcional, mas decisivo) — use o editor rico (negrito, listas, código inline, `#` para heading, `[texto](url)`).
5. **Create** — o card aparece na coluna.

> **Dica do Vibe Guide (`docs/vibe-guide.mdx:22`):** cinco minutos de plano economizam dez de revisão. Um card bem escrito é o plano.

### Escrever descrições que o agente acerta

O agente recebe a descrição como prompt. Compare (`docs/issue-management.mdx:70`):

| Fraco | Forte |
| --- | --- |
| "Tá quebrado" | "Usuários em 3G veem timeout após 5s no login. Esperado: retry com backoff exponencial." |

Inclua: o que fazer, restrições, arquivos/áreas relevantes, e como vai validar (ex.: "rodar `pnpm run check` deve passar; screenshot do checkout em `/images/livro/saas-checkout.png` deve coincidir").

## 2. Editar e mover

- Clique no card para abrir o **painel direito**. **Título e descrição salvam automaticamente** após você parar de digitar; **status/prioridade/tags salvam imediatamente** (`docs/issue-management.mdx:96`).
- **Mover** = arrastar o card para outra coluna, ou trocar o Status no painel. Se o sort do board não estiver em **Manual**, o drag-to-reorder é desabilitado — troque para Manual no header do board.

## 3. Seções do card

No painel do card (`docs/issue-management.mdx:102`, `/images/issue-mgmt-link-workspace.png`):

- **Workspaces** — workspaces vinculados ao card (onde o agente trabalha). Use **+** para criar um workspace já vinculado, ou vincule um existente. Você pode vincular vários para rodar agentes em paralelo.
- **Sub-Issues** — quebre um épico em tarefas menores (`/images/issue-mgmt-sub-issues.png`). Cada sub-issue tem status próprio, link de volta ao pai, e pode ter sub-issues recursivamente. O board impede auto-parenting, ciclos e links cross-project.
- **Comments** — discussão da tarefa.

## 4. Quebrar trabalho grande

Para o SaaS do capítulo 7, crie um card pai "SaaS AssinaFácil — MVP" e sub-issues como:

- "Setup do monorepo (Vite + Tailwind)"
- "Auth (login/cadastro)"
- "Página de planos e checkout"
- "Webhooks Stripe + entitlements"

Cada sub-issue vira um workspace independente — você pode despachar 3 agentes em paralelo sem conflito (cada um no seu worktree/branch).

## 5. Ações, seleção múltipla e bulk

- **More (⋯)** no painel do card ou **command bar** (`Cmd/Ctrl + K` → Issue Actions): mudar status/prioridade, transformar em sub-issue, vincular workspace, duplicar, deletar.
- **Seleção múltipla**: `Cmd/Ctrl + Click` alterna, `Shift + Click` seleciona intervalo, `Cmd/Ctrl + A` seleciona visíveis. Com 2+ selecionados, surge a **bulk action bar** para mudar status/prioridade ou deletar em lote (cuidado: bulk delete é permanente).

## 6. Erros comuns

- **Card não aparece:** limpe filtros (**Clear All**) ou veja a **list view** — pode estar em status oculto.
- **Não consigo arrastar:** troque o sort para **Manual**.
- **Descrição não salvou:** aguarde um instante após parar de digitar (auto-save com debounce).

## Checklist do capítulo

- [ ] Criei um card com título com verbo, descrição específica e status correto.
- [ ] Movi um card arrastando e pelo painel — ambos funcionam.
- [ ] Criei um épico com 3 sub-issues e vinculei um workspace ao épico.
- [ ] Sei usar `Cmd/Ctrl + K` → Issue Actions e bulk actions.
