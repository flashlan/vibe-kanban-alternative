# Capítulo 9 — Da escrita à Amazon KDP

> **Princípio:** publicar é um pipeline como qualquer outro — com estágios, checklist e critério de pronto. A diferença é que o "deploy" é uma loja.

## Escrever aqui, publicar lá

Este livro nasceu como `docs/livro/*.md` dentro do próprio repositório que ele descreve — exatamente o fluxo que ele ensina nos caps. 02–08. Isso não é acidente: o manuscrito é versionado, revisado em PR, verificado por `pnpm run check` e ancorado por imagens (`docs/images/livro/`), como o código. Quando o conteúdo fica pronto, ele atravessa a fronteira para fora do repo e vira produto na Amazon. O checklist que governa essa travessia vive em `docs/livro-vibe-kanban-amazon-checklist.md` — este capítulo explica **como decidir** nos pontos onde o KDP te dá escolhas.

## O caminho do manuscrito ao produto

```
docs/livro/*.md (Markdown no repo)
  → Kindle Create / conversor → .kpf / .epub (eBook)
  → KDP (upload + metadados + preço) → Kindle Store
  → (opcional) PDF de miolo + PDF de capa → KDP Print → paperback
```

Para um livro com screenshots (como este, com 7 âncoras em `docs/images/livro/`), o tamanho do arquivo e a resolução das imagens importam — entram na decisão de preço/royalty abaixo.

## Cinco decisões que importam

### 1. eBook, paperback ou os dois?

Comece por **eBook Kindle**. Custo marginal zero (Kindle Create é gratuito), publicação em horas, royalties de até 70% e distribuição global sem logística. **Paperback** é o segundo estágio: exige miolo em PDF com margens por trim size, capa em PDF frente+lombada+contracapa (template da calculadora de capa do KDP, bleed 0,125", 300 DPI, CMYK), e prova física. O checklist separa as duas trilhas — Fase 5 (eBook) e Fase 6 (paperback) — para que você possa lançar o eBook primeiro e iterar.

Neste livro, o eBook é o MVP; o paperback entra quando as imagens estiverem em 300 DPI e o miolo validado na prova física.

### 2. Preço e royalty — simule antes de escolher

O KDP te dá duas opções por eBook (regras verificadas em ago/2026; **revalide antes de publicar** — mudam):

- **70%** entre US$ 2,99 e **US$ 12,99** (teto subiu de US$ 9,99 em jul/2026), com **taxa de entrega de US$ 0,15/MB** (tamanho do arquivo). Vendas para Brasil/Japão/México/Índia só pagam 70% se o livro estiver no **KDP Select**.
- **35%** entre US$ 0,99 e US$ 200 (mínimo sobe com o tamanho do arquivo), **sem** taxa de entrega.

Para um manual com 7 screenshots em alta + diagramas, o arquivo pode facilmente passar de 5–10 MB. Simule:

| Cenário | Arquivo | 70% (com entrega) | 35% (sem entrega) |
| --- | --- | --- | --- |
| eBook 6 MB, US$ 9,99 | 6 × 0,15 = US$ 0,90 de entrega | (9,99 − 0,90) × 70% ≈ US$ 6,36 | 9,99 × 35% = US$ 3,50 |
| Mesmo, US$ 12,99 | 6 × 0,15 = US$ 0,90 | (12,99 − 0,90) × 70% ≈ US$ 8,46 | 12,99 × 35% = US$ 4,55 |

O paperback paga **50% ou 60% menos custo de impressão**, com corte em US$ 9,99 (`kdp.amazon.com/earn`).

### 3. KDP Select: sim ou não?

KDP Select dá **90 dias de exclusividade digital** em troca de: Kindle Unlimited (pago por páginas lidas), promoções extras (Countdown, Free) e — ponto que interessa aqui — **70% no Brasil**. Se o seu público principal está no Brasil, Select paga a conta. Se você precisa vender também em Apple Books/Kobo, não entre. A decisão é reversível a cada 90 dias — trate como um estágio do pipeline que você pode reverter.

### 4. Categorias e palavras-chave — a spec da descobribilidade

Você tem **até 3 categorias** por formato (escolhidas no seletor do KDP; o esquema antigo de pedir 10 por e-mail não existe mais) e **7 campos de 50 caracteres** para palavras-chave. A lição do cap. 02 vale aqui: a "spec" da descobribilidade é textual.

- **Categorias** dizem **onde** o livro aparece (ex.: Computers / Software Development).
- **Palavras-chave** dizem **para quem** (ex.: "vibe coding", "claude code tutorial", "kanban para desenvolvedores").

Escolha categorias onde um livro novo consegue rankear (nicho > geral); use as palavras-chave para cobrir as buscas que o título não cobre. Cada eBook, paperback e hardcover tem seus próprios 3+7 slots — preencha todos.

### 5. Quando pedir a prova física

Sempre, antes de liberar o paperback. A prova custa impressão + frete e é a única forma de validar margens, lombada, cores (CMYK) e legibilidade em tamanho real — a versão digital do previewer mente sobre esses detalhes. É o `VK-REVIEW-REQUEST` do mundo físico: pare, revise, só então publique.

## O critério de pronto

O checklist termina com quatro caixas:

- eBook live na Amazon.
- Paperback live (se escolhido).
- Página de autor criada no Author Central.
- Metadados revisados na página do produto.

Tradução para linguagem de pipeline (cap. 06): `VK-PIPELINE-STAGE: done` só quando um leitor consegue comprar, abrir e recomendar. Antes disso, é rascunho — por mais que o `git log` diga "done".

## Checklist do capítulo

- [ ] Manuscrito em `docs/livro/` revisado e com imagens ancoradas (cap. 15) em 300 DPI para paperback.
- [ ] Capa do eBook em 1600×2560, legível em thumbnail (KDP Cover Creator ou designer).
- [ ] Metadados (título, descrição 4.000 chars, 3 categorias, 7×50 keywords) preenchidos.
- [ ] Preço simulado nos dois royalties (tabela acima) para o tamanho real do arquivo; decisão de KDP Select tomada.
- [ ] Prova física do paperback aprovada (se houver paperback).
- [ ] Author Central criado e `VK-REVIEW-REQUEST` interno respondido: o livro está pronto para um leitor pagar por ele.
