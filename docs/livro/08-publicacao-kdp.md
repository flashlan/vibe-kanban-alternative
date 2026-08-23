# Capítulo 8 — Da escrita à Amazon KDP

> **Princípio:** publicar é um pipeline como qualquer outro — com estágios, checklist e critério de pronto. A diferença é que o "deploy" é uma loja.

## Escrever aqui, publicar lá

Este livro nasceu como `docs/livro/*.md` dentro do próprio repositório que ele descreve. Isso não é acidente — é o loop do capítulo 5 levado ao extremo: o manuscrito é versionado, revisado em PR, verificado por `pnpm run check` e ancorado por imagens, exatamente como o código. Quando o conteúdo fica pronto, ele atravessa a fronteira para fora do repo e vira produto na Amazon. O checklist que governa essa travessia vive em `docs/livro-vibe-kanban-amazon-checklist.md`.

Este capítulo não repete o checklist linha a linha — ele explica **como decidir** em cada ponto onde o KDP te dá escolhas.

## Cinco decisões que importam

### 1. eBook, paperback ou os dois?

Comece por eBook Kindle. Custo marginal zero (Kindle Create é gratuito), publicação em horas, royalties de até 70% e distribuição global sem logística. Paperback é o segundo estágio: exige miolo em PDF com margens por trim size, capa em PDF frente+lombada+contracapa (template da calculadora de capa do KDP, bleed 0,125", 300 DPI, CMYK), e prova física. O checklist separa as duas trilhas — Fase 5 (eBook) e Fase 6 (paperback) — para que você possa lançar o eBook primeiro e iterar.

### 2. Preço e royalty

O KDP te dá duas opções por eBook (regras verificadas em ago/2026; revalide antes de publicar):

- **70%** entre US$ 2,99 e **US$ 12,99** (teto subiu de US$ 9,99 em jul/2026), com taxa de entrega de US$ 0,15/MB. Vendas para Brasil/Japão/México/Índia só pagam 70% se o livro estiver no KDP Select.
- **35%** entre US$ 0,99 e US$ 200 (mínimo sobe com o tamanho do arquivo), sem taxa de entrega.

Para um manual técnico com imagens, o tamanho do arquivo importa: um eBook com muitas screenshots em alta pode pagar taxa de entrega relevante na faixa de 70%. Simule nos dois cenários antes de decidir. O paperback paga 50% ou 60% menos custo de impressão, com corte em US$ 9,99.

### 3. KDP Select: sim ou não?

KDP Select dá 90 dias de exclusividade digital em troca de: Kindle Unlimited (pago por páginas lidas), promoções extras e — ponto que interessa aqui — **70% no Brasil**. Se o seu público principal está no Brasil, Select paga a conta. Se você precisa vender também em Apple Books/Kobo, não entre. A decisão é reversível a cada 90 dias.

### 4. Categorias e palavras-chave

Você tem **até 3 categorias** por formato (escolhidas no seletor do KDP; o esquema antigo de pedir 10 por e-mail não existe mais) e **7 campos de 50 caracteres** para palavras-chave. A lição do capítulo 4 vale aqui: a "spec" da descobribilidade é textual. Categorias dizem onde o livro aparece; palavras-chave dizem para quem. Escolha categorias onde um livro novo consegue rankear; use as palavras-chave para cobrir as buscas que o título não cobre. Cada eBook, paperback e hardcover tem seus próprios 3+7 slots.

### 5. Quando pedir a prova física

Sempre, antes de liberar o paperback. A prova custa impressão + frete e é a única forma de validar margens, lombada, cores (CMYK) e legibilidade em tamanho real — a versão digital do previewer mente sobre esses detalhes.

## O critério de pronto

O checklist termina com quatro caixas:

- eBook live na Amazon.
- Paperback live (se escolhido).
- Página de autor criada no Author Central.
- Metadados revisados na página do produto.

Tradução para linguagem de pipeline: `VK-PIPELINE-STAGE: done` só quando um leitor consegue comprar, abrir e recomendar. Antes disso, é rascunho — por mais que o `git log` diga "done".

## Checklist do capítulo

- [ ] Manuscrito em `docs/livro/` revisado e com imagens ancoradas (cap. 7).
- [ ] Capa do eBook em 1600×2560, legível em thumbnail.
- [ ] Metadados (título, descrição 4.000 chars, 3 categorias, 7×50 keywords) preenchidos.
- [ ] Preço simulado nos dois royalties; decisão de KDP Select tomada.
- [ ] Prova física do paperback aprovada (se houver paperback).
- [ ] Author Central criado e `VK-REVIEW-REQUEST` interno respondido: o livro está pronto para um leitor pagar por ele.
