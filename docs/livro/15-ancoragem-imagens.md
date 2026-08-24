# Capítulo 15 — Ancoragem de imagens

> **Princípio:** uma screenshot bem escolhida vale como assertion. Ela diz a um humano "parece certo" e a uma IA "compare o estado atual com este" — e no KDP, ela é literalmente o produto.

## Por que imagem ancorada — texto descreve, imagem prova

Num app com UI rica, muitas regressões são visuais — um botão que sumiu, uma coluna do kanban que quebrou, um diálogo que não abre. Um agente que só lê texto pode achar que está tudo certo quando a tela está vazia. Imagens ancoradas fecham essa lacuna: são o **"teste de snapshot" que um humano entende num relance** e que uma IA pode comparar pixel ou semanticamente.

Este repositório já trata imagem como parte da documentação, não como decoração. E este livro trata imagem como parte do **manuscrito** — cada cap. tem âncoras em `docs/images/livro/` que provam que a tela existe como descrita.

## Como a doc já faz — o padrão Mintlify

Os arquivos Mintlify em `docs/` envolvem toda imagem em `<Frame>` com `alt` descritivo:

```mdx
<Frame>
  <img src="/images/workspaces-preview-no-script.png"
       alt="Preview panel showing prompt to set up a dev server script" />
</Frame>
```

O caso mais completo é `docs/browser-testing.mdx`: um passo-a-passo de 3 etapas (configurar dev server → iniciar → usar o preview browser) ilustrado por **quatro screenshots** em `/images/workspaces-preview-*.png` — prompt sem script, diálogo de script, botão "Start dev server", painel de log, browser anotado com 7 controles numerados (Back/Forward, Inspect, DevTools…). O texto e a imagem se ancoram mutuamente: cada controle numerado na imagem é explicado na lista logo abaixo. O `docs/mobile-testing.md` segue o mesmo padrão para testes em dispositivo físico.

Regras que emergem (e que o `docs/AGENTS.md` reforça com frontmatter obrigatório, alt text descritivo e Frames):

- Toda imagem tem `alt` que descreve **o que deve ser visto** (não "screenshot").
- Imagens de UI têm nome que identifica o estado (`preview-no-script` vs `preview-dev-server-running`).
- O caminho é `/images/...` — relativo ao site de docs, versionado no repo; no livro, `docs/images/livro/`.
- Resolução consistente (1440×900 no livro) para comparação estável.

## O que já está ancorado neste livro — 12 imagens

O livro tem hoje **12 âncoras versionadas** em `docs/images/livro/`:

| Grupo | Arquivos | Cap. | O que prova |
| --- | --- | --- | --- |
| **App real** | `ancora-board-principal.png` (989 KB, 3156×1894) | 04 | Board com 4 colunas PT-BR |
|  | `ancora-workspace-aberta.png` (775 KB, 3118×1888) | 04 | 3 painéis (Conversation/Context/Details) |
|  | `ancora-settings.png` (254 KB) | 03 | Projects Guide / IDE / agente |
|  | `ancora-criar-card-*.png` (3 arquivos) | 05 | Diálogo criar card (topo/base) + Workspaces→Create |
|  | `ancora-workspace-chat-bar.png` (353 KB) | 05 | Barra do chat destrinchada |
| **AssinaFácil (prévias)** | `saas-landing.png` (53 KB, 1440×900) | 08 | Hero + MRR + features |
|  | `saas-planos.png` (44 KB, 1440×900) | 08 | 3 planos, Pro em destaque |
|  | `saas-checkout.png` (41 KB, 1440×900) | 08 | Formulário + resumo |
|  | `saas-minhas-assinaturas.png` (37 KB, 1440×900) | 08 | Tabela logada com ações |
|  | `saas-landing-mobile.png` (23 KB, 390×780) | 08 | Landing responsiva |

As 7 primeiras são **screenshots reais** do app rodando (capturadas via `Cmd+Shift+4` no macOS, salvas direto em `docs/images/livro/`). As 5 do AssinaFácil são **prévias geradas em PIL** (`python3` + `Pillow`, sem browser) — placeholder até os cards do cap. 08 irem para Done e serem substituídas por screenshots reais do Preview (1440×900). O gerador vive no histórico do commit `5371b672` e é reproduzível.

> **Por que PIL e não screenshot real ainda?** Porque o SaaS ainda não existe como código — as prévias permitem escrever o cap. 08 antes de codar, e dão ao agente um **alvo visual** ("parecido com isto") quando o card rodar. É spec por imagem (ver abaixo).

## O plano de ancoragem completo — o que falta capturar

O capítulo original propunha 12 nomes; com as 12 já feitas, o que falta é **substituir as prévias por reais** e cobrir os estados que ainda não têm âncora:

### Quadro kanban (falta 1)

| Nome | Rota/estado | Valida | Status |
| --- | --- | --- | --- |
| `livro/board-empty.png` | Board vazio (projeto novo) | Colunas, botão criar card, empty state | A fazer |
| `livro/board-with-cards.png` | Board com 6–8 cards em 3 colunas | Drag-and-drop, badges, coluna arquivada | Coberto por `ancora-board-principal.png` |
| `livro/card-detail.png` | Card aberto (pipeline checklist ao vivo) | `VK-PIPELINE-STAGE` refletido na UI | Coberto por `ancora-criar-card-*.png` |

### Workspace (falta 3)

| Nome | Rota/estado | Valida | Status |
| --- | --- | --- | --- |
| `livro/workspace-diff.png` | Aba Changes (diff inline) | Arquivos alterados + comentário inline | A fazer |
| `livro/workspace-terminal.png` | Aba Terminal (xterm.js) | `pnpm run check` rodando | A fazer |
| `livro/workspace-preview.png` | Aba Preview (dev server rodando, toolbar 1–7) | Preview browser + controles numerados | Parcial (`saas-*.png` são prévias) |
| `livro/workspace-conversation.png` | Aba Conversation (transcript) | Histórico + session dropdown | Coberto por `ancora-workspace-aberta.png` |

### Aprovações e criação (falta 2)

| Nome | Estado | Valida | Status |
| --- | --- | --- | --- |
| `livro/approvals-inbox.png` | TUI ou painel com 1 tool-permission + 1 pergunta | Inbox, Approve/Deny/Answer | A fazer |
| `livro/review-request.png` | Card em `review-manual` com banner `VK-REVIEW-REQUEST` | Banner + descrição da entrega | A fazer |

Capturar em **resolução consistente (1440×900)**, com dados de seed iguais (mesmo projeto `Novo aplicativo SaaS`, mesmo branch `vk/1f98-…`) para comparação estável. Para KDP paperback, exportar em **300 DPI** (cap. 09).

## Como capturar — 3 caminhos, do mais fiel ao mais rápido

### 1. Screenshot real do Preview (mais fiel — use para substituir `saas-*.png`)

1. Rode `pnpm run dev` → abra workspace do AssinaFácil → aba **Preview** (`http://localhost:5173`).
2. No macOS: `Cmd+Shift+4` → arraste → arquivo em `~/Desktop` → mova para `docs/images/livro/saas-landing.png`.
3. Valide: abra a imagem e o cap. 08 lado a lado — o hero, o MRR e os 3 features batem?

### 2. Upload via chat da workspace (vira `Attachment`, visível para o agente)

Arraste a imagem direto no **chat da workspace** — o app faz `POST /api/attachments/upload` (`crates/server/src/routes/attachments.rs:83`, 20 MB, `image/png|jpeg|gif|webp|bmp`) e o agente recebe como contexto visual. É a melhor forma de mandar mock ou screenshot de erro para o agente comparar (cap. 05 §4). O arquivo vai para `GET /api/attachments/{id}/file` com `Cache-Control: immutable, 1y`.

### 3. Geração PIL (mais rápido — use para prévias antes do código existir)

```python
from PIL import Image, ImageDraw, ImageFont
im = Image.new("RGB", (1440, 900), "#f8fafc")
# ... desenhe hero, cards, tabela (ver commit 5371b672 para o script completo)
im.save("docs/images/livro/saas-landing.png")
```

Reproduzível, sem browser, sem `pnpm run dev`. Ideal para escrever o cap. antes de codar.

## Como a IA usa a âncora — dois usos práticos

### 1. Validação visual pós-mudança

Após alterar um componente em `packages/web-core/src/`, o agente roda o dev server, navega até a rota e **compara a screenshot atual com a âncora**. Diferença inesperada (botão sumiu, coluna quebrou) → corrige antes de commitar. É o `pnpm run check` do visual.

### 2. Especificação por imagem

Ao criar uma feature visual, o card pode **anexar a imagem-âncora desejada** (ex.: mock do diálogo de criar card, `saas-planos.png` com Pro em destaque). O agente tem, além da spec em texto, a imagem-alvo — e sabe quando terminou porque **a tela coincide**. O `AGENTS.md` do design system (`packages/local-web/AGENTS.md`) já orienta styling; as imagens são o complemento visual desse texto.

> **Exercício:** abra `saas-planos.png` e `saas-checkout.png` lado a lado. Note como o CTA "Assinar Pro →" no card Pro leva ao checkout com "Plano Pro · R$ 49/mês" no header — a âncora valida o fluxo, não só a tela isolada.

## Checklist do capítulo

- [ ] Cada feature visual nova tem screenshot ancorada em `docs/images/livro/` com nome previsível (`saas-*.png`, `ancora-*.png`).
- [ ] Toda imagem tem `alt` descritivo e, na doc Mintlify, está em `<Frame>`.
- [ ] O plano de ancoragem cobre: board, workspace (5 abas), approvals, diálogos — e está versionado.
- [ ] Screenshots são capturadas em resolução/dados consistentes (1440×900, seed fixo) para comparação estável.
- [ ] Prévias PIL são substituídas por screenshots reais quando o card vai para Done (cap. 08).
- [ ] Para KDP paperback, imagens exportadas em 300 DPI, CMYK, com bleed 0,125" (cap. 09).
- [ ] O card de feature visual referencia a imagem-âncora na descrição — texto + imagem, não só texto.
