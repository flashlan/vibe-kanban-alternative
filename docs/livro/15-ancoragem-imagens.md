# Capítulo 15 — Ancoragem de imagens

> **Princípio:** uma screenshot bem escolhida vale como assertion. Ela diz a um humano "parece certo" e a uma IA "compare o estado atual com este".

## Por que imagem ancorada

Texto descreve; imagem prova. Num app com UI rica, muitas regressões são visuais — um botão que sumiu, uma coluna do kanban que quebrou, um diálogo que não abre. Um agente que só lê texto pode achar que está tudo certo quando a tela está vazia. Imagens ancoradas fecham essa lacuna: são o "teste de snapshot" que um humano entende num relance e que uma IA pode comparar pixel ou semanticamente.

Este repositório já trata imagem como parte da documentação, não como decoração.

## Como a doc já faz

Os arquivos Mintlify em `docs/` envolvem toda imagem em `<Frame>`:

```mdx
<Frame>
  <img src="/images/workspaces-preview-no-script.png"
       alt="Preview panel showing prompt to set up a dev server script" />
</Frame>
```

O caso mais completo é `docs/browser-testing.mdx`: um passo-a-passo de 3 etapas (configurar dev server → iniciar → usar o preview browser) ilustrado por quatro screenshots em `/images/workspaces-preview-*.png` — prompt sem script, diálogo de script, botão "Start dev server", painel de log, browser anotado com 7 controles numerados (Back/Forward, Inspect, DevTools…). O texto e a imagem se ancoram mutuamente: cada controle numerado na imagem é explicado na lista logo abaixo.

O `docs/mobile-testing.md` segue o mesmo padrão para testes em dispositivo físico.

Regras que emergem (e que o `docs/AGENTS.md` reforça com frontmatter obrigatório, alt text descritivo e Frames):

- Toda imagem tem `alt` que descreve o que deve ser visto.
- Imagens de UI têm borda/nome que identifica o estado (ex.: `preview-no-script` vs `preview-dev-server-running`).
- O caminho é `/images/...` — relativo ao site de docs, versionado no repo.

## O plano de ancoragem para este app

O capítulo anterior mostrou que o app tem poucas telas, mas cada uma com muitos estados. O plano abaixo é o que o livro propõe ancorar — cada linha é uma screenshot com nome sugerido, rota/estado que deve ser capturado e o que a imagem valida.

### Quadro kanban

| Nome do arquivo | Rota/estado | O que valida |
| --- | --- | --- |
| `livro/board-empty.png` | Board vazio (projeto novo, sem cards) | Colunas, botão de criar card, estado vazio |
| `livro/board-with-cards.png` | Board com 6–8 cards em 3 colunas (Todo / In Progress / Done) | Drag-and-drop, badges de prioridade/tag, coluna arquivada |
| `livro/card-detail.png` | Card aberto (descrição + pipeline stages + checklist) | Render do pipeline ao vivo, `VK-PIPELINE-STAGE` refletido na UI |

### Workspace

| Nome | Rota/estado | Valida |
| --- | --- | --- |
| `livro/workspace-overview.png` | Workspace aberta, aba Overview | Branch `vk/xxxx`, status, repos vinculados |
| `livro/workspace-diff.png` | Aba Diff/Changes | Arquivos alterados e diff inline |
| `livro/workspace-terminal.png` | Aba Terminal | Terminal embutido funcional |
| `livro/workspace-preview.png` | Aba Preview (dev server rodando) | Preview browser carregado, toolbar 1–7 visível |
| `livro/workspace-conversation.png` | Aba Conversation | Transcript do agente com cache de conversa |

### Aprovações e revisão

| Nome | Estado | Valida |
| --- | --- | --- |
| `livro/approvals-inbox.png` | TUI ou painel de approvals com 1 tool-permission + 1 pergunta | Inbox renderizada, botões Approve/Deny/Answer |
| `livro/review-request.png` | Card em `review-manual` com banner `VK-REVIEW-REQUEST` | Banner de revisão + descrição da entrega |

### Conversa e criação de card

| Nome | Estado | Valida |
| --- | --- | --- |
| `livro/create-issue-dialog.png` | Diálogo de criar card (com anexo de imagem, botões de urgência/tags — `vk/8dfb`, `vk/5f5b`, `vk/160b`) | Campos do card, upload de imagem |
| `livro/cache-conversation.png` | Troca de workspaces com cache (feature `vk/f804`) | Conversa não re-streama ao voltar |

As imagens devem ser capturadas em resolução consistente (ex.: 1440×900), com dados de seed iguais (mesmo projeto/branch) para que a comparação seja estável. O diretório sugerido é `docs/images/livro/` — espelhando `docs/images/` já existente.

## Como a IA usa a âncora

Dois usos práticos:

1. **Validação visual pós-mudança.** Após alterar um componente em `packages/web-core/src/`, o agente roda o dev server, navega até a rota e compara a screenshot atual com a âncora. Diferença inesperada → corrige antes de commitar.

2. **Especificação por imagem.** Ao criar uma feature visual, o card pode anexar a imagem-âncora desejada (ex.: mock do diálogo de criar card). O agente tem, além da spec em texto, a imagem-alvo — e sabe quando terminou porque a tela coincide.

O `AGENTS.md` do design system (`packages/local-web/AGENTS.md`) já orienta styling; as imagens ancoradas são o complemento visual desse texto.

## Checklist do capítulo

- [ ] Cada feature visual nova tem screenshot ancorada em `docs/images/` com nome previsível.
- [ ] Toda imagem tem `alt` descritivo e está envolvida em `<Frame>` na doc.
- [ ] O plano de ancoragem cobre: board, workspace (5 abas), approvals, diálogos.
- [ ] Screenshots são capturadas em resolução/dados consistentes para comparação estável.
- [ ] O card de feature visual referencia a imagem-âncore na descrição.
