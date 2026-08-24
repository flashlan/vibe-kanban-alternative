# Capítulo 8 — Projeto prático: Criando um SaaS com Vibe Kanban

> **Objetivo:** construir um SaaS do zero usando só a interface — cada seção é um card que você cria, despacha e revisa. No final, o board é a documentação do produto.

## O produto: AssinaFácil

**AssinaFácil** é um SaaS fictício de gestão de assinaturas — propositalmente simples para caber num livro, mas com as peças que todo SaaS tem: landing, auth, planos, checkout, área logada e webhooks. Você vai construí-lo card a card, exatamente como usaria o Vibe Kanban no seu produto real.

Stack sugerida (ajuste ao seu gosto — o fluxo no Vibe Kanban é o mesmo):

- `app-web` — Vite + React + Tailwind
- `api` — Node (ou Rust) em `api/` — aqui um mock em memória para focar na interface

## Preparação — o card que destrava todo o resto

**Card:** `Setup do monorepo AssinaFácil`

- **Descrição (spec forte, cap. 02):**
  > Criar monorepo pnpm com `app-web` (Vite + React + Tailwind) e `api` (Node). Configurar `pnpm run dev` (app-web na 5173, api na 3000), `pnpm run check` (tsc) e `pnpm run format` (prettier). O dev server do Vibe Kanban deve subir `app-web` no Preview.
  > Arquivos: `package.json` (workspaces), `app-web/`, `api/`, `pnpm-workspace.yaml`.
  > Critério de pronto: `pnpm run dev` sobe; Preview mostra "Hello AssinaFácil"; `pnpm run check` passa.
- **Pipeline:** `quick` (trivial e bem especificado — cap. 06).
- **Workspace:** crie a partir do card (seção **Workspaces → Create** dentro do card, cap. 05), selecione **Create new repo on disk** (`~/code/assina-facil`), target `main`, escolha o agente e clique **Create**. Acompanhe `VK-PIPELINE-STAGE` em **Logs**; quando surgir `VK-REVIEW-REQUEST`, revise diffs em **Changes** e o app em **Preview**.

Ao finalizar, configure no projeto (cap. 03):

- Setup script: `pnpm install`
- Dev server script: `pnpm --filter app-web dev`
- `copy_files = [".env"]` se o SaaS precisar de env

## Épico e sub-tarefas — quebre antes de codar

Crie o épico **AssinaFácil — MVP** (card pai) e, dentro dele, sub-issues (cap. 05 §5):

1. **Landing page + design system** — hero, features, CTA para /planos.
2. **Auth (login/cadastro)** — formulários + estado mockado.
3. **Página de planos e checkout** — tabela de 3 planos (Free, Pro R$49, Enterprise), botão Assinar, fluxo mock.
4. **Área logada — Minhas assinaturas** — lista, cancelar, recibo.
5. **Webhooks + entitlements** — `POST /webhooks` que marca assinatura como ativa.

Cada sub-issue tem status próprio, link de volta ao pai e pode ter sub-issues recursivamente (`docs/issue-management.mdx:126`). Cada uma vira um workspace independente — despache em paralelo quando não houver dependência (ex.: 1 e 2 podem rodar juntos, cada um no seu `vk/xxxx-*`).

Exemplo de descrição forte para o card 3 (reaproveita o exemplo do cap. 02):

> **Título:** Criar página `/planos` com 3 planos e CTA para checkout
> Arquivos: `app-web/src/pages/plans.tsx`, `app-web/src/features/billing/plans.ts`
> Validação: Preview em 1440px e 375px sem quebra; `pnpm run check` passa; âncora `docs/images/livro/saas-planos.png` coincide.
> Restrição: usar Tailwind já configurado; não adicionar lib nova.

## O ciclo que você vai repetir (5 vezes)

Para cada sub-issue, faça o loop completo — é o "usar o aplicativo para desenvolver":

1. **Crie o card** com título com verbo e descrição específica (cap. 05) + critério de pronto observável.
2. **Crie a workspace vinculada** (cap. 07) — worktree `vk/xxxx-nome` a partir de `main`.
3. **Board:** o card vai para **In Progress** ao enviar a primeira mensagem na workspace.
4. **Acompanhe:** Conversation (chat), **Logs** (`VK-PIPELINE-STAGE: N`), **Changes** (diffs), **Preview** (app).
5. **Revise:** quando surgir `VK-REVIEW-REQUEST`, abra Changes e Preview; comente inline ou envie follow-up no chat ("o CTA deveria ser azul, não verde").
6. **Ajuste ou aprove:** se pediu ajuste, o card volta para **In Progress** (cap. 05 §3); se OK, mova para **In Review → Done**. O estágio `merge` do `quick` faz squash-merge sozinho; ou abra PR pela aba **Git** (`docs/workspaces/git-operations.mdx`).

## Roteiro dos cards e o que validar no Preview

| Ordem | Card | O que validar |
| --- | --- | --- |
| 1 | Setup do monorepo | `pnpm run dev` sobe; Preview mostra "Hello AssinaFácil" |
| 2 | Landing page | Hero + CTA funcionando; âncora `docs/images/livro/saas-landing.png` |
| 3 | Auth — login/cadastro | Formulários com validação; estado mockado |
| 4 | Planos e checkout | Tabela 3 planos; Assinar → /checkout mock |
| 5 | Área logada | Lista mockada; Cancelar muda estado |
| 6 | Webhooks | `POST /webhooks` muda entitlement; teste via Terminal da workspace |

Capture cada âncora quando o card for para Done e guarde em `docs/images/livro/saas-*.png` (arraste a imagem no chat da workspace — `crates/server/src/routes/attachments.rs:83` — ou salve direto; ver cap. 15).

## Quando algo dá errado (atalhos)

- **Agente travou em `Needs Attention`:** sidebar → badge de mão levantada, ou TUI `cargo run -p tui` tecla `a`, ou Telegram bridge (`automation/README.md`).
- **Card não muda de coluna:** sort do board em **Manual** (cap. 05 §3); senão troque Status no painel.
- **Porta ocupada:** `lsof -nP -i :5173 -sTCP:LISTEN` + `cwd` (cap. 03 §2).
- **Pipeline não avança:** confira `VK-PIPELINE-STAGE: N` em Logs — se parou, veja se caiu na tripwire `VK-ESCALATE: trivial->light` (card precisa de pipeline maior).

## O que você tem no final

Um board com o histórico completo — épico + 6 cards em **Done**, cada um com sua branch `vk/xxxx-*` e PR/merge. Esse board **é** a documentação do SaaS: qualquer pessoa que abrir o projeto vê como ele foi construído, card a card. É o mesmo board da âncora `ancora-board-principal.png` (cap. 04), agora com o seu produto dentro.

## Checklist do capítulo

- [ ] Monorepo criado via workspace do card de Setup, com dev server no Preview.
- [ ] Épico + 5 sub-issues criados; ao menos 2 workspaces rodaram em paralelo.
- [ ] Cada card fez Todo → In Progress → In Review → Done com Preview validado.
- [ ] Screenshots-âncora `docs/images/livro/saas-*.png` capturadas.
- [ ] Merge/PR de cada workspace concluído; board final em Done.
