# Capítulo 7 — Projeto prático: Criando um SaaS com Vibe Kanban

> **Objetivo:** construir um SaaS do zero usando só a interface do Vibe Kanban — cada seção é um card que você cria, despacha e revisa.

## O produto: AssinaFácil

**AssinaFácil** é um SaaS fictício de gestão de assinaturas: landing page, autenticação, página de planos, checkout (mock), área logada com lista de assinaturas, e webhooks. É propositalmente simples para caber num livro, mas com as peças que todo SaaS tem.

Stack sugerida (ajuste ao seu gosto — o fluxo no Vibe Kanban é o mesmo):

- Frontend: Vite + React + Tailwind (em `app-web/`)
- Backend: Node (ou Rust) em `api/` — aqui usamos um mock em memória para manter o livro focado na interface

## Preparação (1 card)

**Card:** `Setup do monorepo AssinaFácil`
- **Descrição:** "Criar monorepo pnpm com `app-web` (Vite + React + Tailwind) e `api` (Node). Configurar `pnpm run dev` (app-web na 5173, api na 3000), `pnpm run check` (tsc) e `pnpm run format` (prettier). O dev server do Vibe Kanban deve subir `app-web`."
- **Pipeline:** `quick` (é trivial e bem especificado).
- **Workspace:** crie a partir do card, selecione um repo novo em disco (**Create new repo on disk**), target `main`, descreva a tarefa, escolha seu agente e clique Create. Acompanhe `VK-PIPELINE-STAGE` no Logs; quando pedir `VK-REVIEW-REQUEST`, revise diffs em **Changes** e o app no **Preview**.

Ao final, configure no projeto:
- Setup script: `pnpm install`
- Dev server script: `pnpm --filter app-web dev`
- Cleanup script: (vazio por enquanto)

## Épico e sub-tarefas

Crie o épico **AssinaFácil — MVP** e, dentro dele, sub-issues:

1. **Landing page + design system** — hero, features, CTA para /planos.
2. **Auth (login/cadastro)** — formulários + estado mockado.
3. **Página de planos e checkout** — tabela de planos, botão de assinar, fluxo mock.
4. **Área logada — Minhas assinaturas** — lista, cancelar, recibo.
5. **Webhooks + entitlements** — endpoint `/webhooks` que marca assinatura como ativa.

Cada sub-issue vira um card com sua própria pipeline. Despache em paralelo quando não houver dependência (ex.: 1 e 2 podem rodar juntos — cada um no seu workspace/worktree).

## Passo a passo de cada sub-issue (repita o ciclo)

Para cada sub-issue, faça o ciclo completo — é o "usar o aplicativo para desenvolver" que o livro prometeu:

1. **Crie o card** com título com verbo e descrição específica (cap. 4). Inclua critério de pronto: "Preview mostra X; `pnpm run check` passa".
2. **Crie o workspace vinculado** (cap. 6) — o Vibe Kanban cria o worktree `vk/xxxx-nome` a partir de `main`.
3. **Acompanhe no board:** arraste o card para **In Progress**.
4. **Veja o agente trabalhar:** Conversation (chat), Logs (`VK-PIPELINE-STAGE: N`), Changes (diffs), Preview (app rodando).
5. **Revise:** quando surgir `VK-REVIEW-REQUEST`, abra Changes e Preview, comente inline o que precisa ajustar, envie follow-up no chat.
6. **Mova para In Review → Done** quando o critério de pronto bater. O pipeline `quick` pode fazer squash-merge sozinho (estágio `merge`); ou abra PR pela aba Git e faça merge manual (`docs/workspaces/git-operations.mdx`).

## Roteiro sugerido dos cards do SaaS

| Ordem | Card (título) | O que validar no Preview |
| --- | --- | --- |
| 1 | Setup do monorepo | `pnpm run dev` sobe; Preview mostra "Hello AssinaFácil" |
| 2 | Landing page | Hero + CTA funcionando; âncora em `docs/images/livro/saas-landing.png` |
| 3 | Auth — login/cadastro | Formulários com validação; sem backend real ainda |
| 4 | Planos e checkout | Tabela de 3 planos; clique em Assinar leva a /checkout mock |
| 5 | Área logada | Lista de assinaturas mockadas; cancelar muda estado |
| 6 | Webhooks | `POST /webhooks` muda entitlement; teste via terminal da workspace |

As imagens-âncora (cap. 14) para este projeto: `saas-landing.png`, `saas-planos.png`, `saas-checkout.png`, `saas-minhas-assinaturas.png` — capture cada uma quando o card for para Done e guarde em `docs/images/livro/`.

## Quando algo dá errado

- **Agente travou pedindo aprovação:** veja `Needs Attention` na sidebar ou no TUI (`cargo run -p tui`, tecla `a`). Aprove/nege, ou responda a pergunta — o Telegram bridge também escala se configurado (`automation/README.md`).
- **Conflito de porta ao subir dev server:** confira quem segura `:5173`/`:3000`/`:3001` com `lsof -nP -i :5173 -sTCP:LISTEN` e o `cwd` do processo (cap. 2).
- **Card não avança de coluna:** confira se o sort do board está em Manual; senão, troque status pelo painel do card.

## O que você tem no final

Um board com o histórico completo do produto — épico + 6 cards em Done, cada um com sua branch `vk/xxxx-*` e PR (ou merge local). Esse board **é** a documentação do SaaS: qualquer pessoa que abrir o projeto vê como ele foi construído, card a card.

## Checklist do capítulo

- [ ] Monorepo criado via workspace vinculada ao card de Setup, com dev server configurado.
- [ ] Épico + 5 sub-issues criados; ao menos 2 workspaces rodaram em paralelo.
- [ ] Cada card passou por Todo → In Progress → In Review → Done com preview validado.
- [ ] Screenshots-âncora do SaaS capturadas em `docs/images/livro/saas-*.png`.
- [ ] Merge/PR de cada workspace concluído; board final em Done.
