# Livro "Vibe Kanban" na Amazon — Investimento e Checklist de Publicação

Guia operacional para publicar o livro sobre o Vibe Kanban na Amazon via **KDP (Kindle Direct Publishing)**: eBook Kindle e, opcionalmente, capa comum (paperback). O manuscrito vive em `docs/livro/` (índice em `00-indice.md`). Dados de regras do KDP verificados em ago/2026 nas fontes oficiais listadas ao final — o KDP muda regras com frequência, revalide antes de publicar.

## 1. Tópicos do Manual Moderno (conteúdo do livro)

1. **The Vibe Coding Setup** — como documentar o ambiente Node/Rust para que uma IA (ou você, codando por intenção) consiga ler o projeto e entender o contexto instantaneamente: estruturação de `.clinerules`, `.cursorrules` ou arquivos de contexto do repositório (neste projeto, o `AGENTS.md` cumpre esse papel).
2. **Spec-Driven Architecture** — como desenhar a seção que explica as fronteiras do sistema: como a "Spec" dita o que o JavaScript/TypeScript faz no Node e onde o Rust entra para garantir performance e segurança de tipos.
3. **The Engineering Loop (CLI & Autocorreção)** — como documentar os comandos de terminal (`npm`, `cargo`) e os padrões de erro para que os agentes rodem testes, leiam os logs de compilação do Rust e se autocorrigirem sem intervenção humana a cada erro.
4. **Ancoragem de Imagens** — planejar os prints de tela exatos que servem de validação visual, para o desenvolvedor (ou a IA) confirmar que o comportamento do app está correto.

## 2. Investimento estimado

O KDP **não cobra taxa para publicar** — a Amazon fica com uma fatia de cada venda (royalties na seção 3). O investimento real vai para produção e divulgação. Faixas abaixo são estimativas de mercado a confirmar na hora da contratação:

| Item | Obrigatório? | Custo estimado |
| --- | --- | --- |
| Manuscrito (escrita autoral, baseada no projeto) | Sim | R$ 0 (tempo próprio) |
| Revisão / copyediting freelance | Recomendado | Variável — negociar por lauda/palavra |
| Capa do eBook | Sim | R$ 0 (Canva / KDP Cover Creator) ou designer freelance |
| Formatação do eBook | Sim | R$ 0 (Kindle Create, gratuito) |
| ISBN | Sim | R$ 0 (ISBN gratuito do KDP); ISBN próprio é opcional |
| Capa do paperback (PDF frente+lombada+contracapa) | Só se paperback | R$ 0 (template KDP + DIY) ou designer |
| Cópias de prova do paperback | Só se paperback | Custo de impressão + frete por unidade |
| Amazon Ads (pós-lançamento) | Opcional | Orçamento diário flexível, definido por você |

Cenário mínimo viável: **R$ 0** (tudo DIY). Cenário recomendado: reservar verba para capa profissional e um orçamento inicial pequeno de Ads.

## 3. Royalties e preço (regras vigentes, ago/2026)

- **eBook — opção 70%:** preço entre **US$ 2,99 e US$ 12,99** (o teto subiu de US$ 9,99 para US$ 12,99 em 07/jul/2026). Há taxa de entrega por tamanho de arquivo (US$ 0,15/MB na Amazon.com). Vendas para clientes no **Brasil, Japão, México e Índia só pagam 70% se o livro estiver no KDP Select**.
- **eBook — opção 35%:** qualquer preço de US$ 0,99 a US$ 200 (mínimo sobe com o tamanho do arquivo: US$ 1,99 a partir de 3 MB; US$ 2,99 a partir de 10 MB). Sem taxa de entrega.
- **Paperback:** royalty de 50% ou 60% (na Amazon.com, o corte é US$ 9,99) **menos o custo de impressão**.
- **Regra dos 20%:** para a opção de 70%, o preço do eBook deve estar pelo menos 20% abaixo do preço da edição física.

## 4. Checklist de publicação

### Fase 0 — Conta e dados fiscais

- [ ] Criar/acessar conta em [kdp.amazon.com](https://kdp.amazon.com)
- [ ] Completar dados fiscais (CPF e entrevista fiscal W-8BEN para autores fora dos EUA)
- [ ] Cadastrar conta bancária para receber royalties

### Fase 1 — Manuscrito

- [ ] Manuscrito em `docs/livro/` revisado (índice em `00-indice.md`, caps. 1–15 + apêndice)
- [ ] Texto final revisado (estrutura: introdução, capítulos, sobre o autor)
- [ ] Página de copyright incluída
- [ ] Formatação no **Kindle Create** (gratuito) com sumário navegável
- [ ] Capa interna embutida no arquivo do livro

### Fase 2 — Capa do eBook

- [ ] Formato **JPEG ou TIFF**, perfil de cor **RGB**
- [ ] Dimensões ideais **1600 × 2560 px** (proporção 1,6:1); mínimo 1000 × 625 px; arquivo < 50 MB
- [ ] Título legível em thumbnail pequena (~100 × 160 px)

### Fase 3 — Metadados

- [ ] Título e subtítulo com palavras-chave reais de busca (o subtítulo é o principal campo visível para SEO)
- [ ] Nome do autor (e colaboradores, se houver)
- [ ] Descrição de até **4.000 caracteres**, escrita como texto de venda
- [ ] **Até 3 categorias** por formato, escolhidas no seletor do KDP (o esquema antigo de pedir categorias extras por e-mail não existe mais)
- [ ] **7 campos de palavra-chave**, 50 caracteres cada — frases que o leitor digita, sem repetir o título

### Fase 4 — Direitos e preço

- [ ] Confirmar que você detém os direitos de todo o conteúdo (texto e imagens)
- [ ] Selecionar territórios de venda
- [ ] Escolher opção de royalty 70% vs 35% conforme seção 3
- [ ] Decidir sobre **KDP Select** (exclusividade digital de 90 dias; habilita Kindle Unlimited e o royalty de 70% no Brasil)
- [ ] Definir preço por marketplace

### Fase 5 — Publicação do eBook

- [ ] Upload do arquivo e revisão no **Kindle Previewer**
- [ ] Publicar — a análise da Amazon leva em geral **até 72 horas**

### Fase 6 — Paperback (opcional)

- [ ] Miolo em PDF com margens adequadas ao formato (trim size) escolhido
- [ ] Capa completa (frente + lombada + contracapa) em **PDF pronto para impressão**, usando o template da calculadora de capa do KDP (lombada varia com o número de páginas; bleed de 0,125"; 300 DPI; CMYK)
- [ ] ISBN gratuito do KDP (código de barras gerado automaticamente)
- [ ] Pedir **cópia de prova física** e conferir antes de liberar a venda

### Fase 7 — Pós-lançamento

- [ ] Criar página no **Author Central**
- [ ] Pedir reviews aos primeiros leitores
- [ ] Configurar campanha de **Amazon Ads** (se houver verba)
- [ ] Monitorar relatórios de vendas no painel KDP e ajustar preço/metadados

## 6. Rascunho de metadados (pré-preenchido — copiar no KDP)

Valores sugeridos para a Fase 3. Ajuste o autor e confira as categorias no seletor do KDP (o esquema de pedir categorias extras por e-mail não existe mais).

**Título:**
```
Manual Moderno de Vibe Coding
```

**Subtítulo (campo de subtítulo do KDP — forte para SEO):**
```
Uso prático do Vibe Kanban Indie: do npx ao SaaS em produção
```

**Autor:**
```
[seu nome / pseudônimo]
```

**Descrição (texto de venda, PT-BR — 1.200/4.000 chars usados):**
```
Você instalou o Vibe Kanban Indie e quer usá-lo de verdade — não só estudar a arquitetura? Este Manual Moderno de Vibe Coding é o guia prático, escrito por quem usa a ferramenta para desenvolver.

Em 15 capítulos (+ apêndice), você vai do `npx` à publicação de um SaaS completo (o projeto-guia AssinaFácil), aprendendo o vocabulário do vibe coding (Engineering Loop, Spec-Driven, multi-agente), a interface do app (board, workspaces, pipelines, worktrees) e como a máquina dirige o fluxo sozinha (MCP, marcadores de log, alarme de revisão).

Para quem: desenvolvedores solo que querem dirigir agentes de IA (Claude Code, OpenCode, Codex, Gemini) com cards, pipelines e git sem medo.

O que você vai aprender:
• Instalar e configurar o app no seu `projects.toml`
• Criar cards com spec forte e critério de pronto
• Usar pipelines que movem o card sozinho (VK-PIPELINE-STAGE)
• Fazer git em worktrees isolados, em paralelo, sem conflito
• Construir um SaaS de assinaturas inteiro pela interface
• Boas práticas: context engineering, autocompact, memória semântica e de grafo de agentes

Tudo ancorado em screenshots reais e exemplos do código. Escreva menos, dirija mais.
```

**Categorias (até 3 — escolha nichos onde um livro novo rankeia):**
```
1. Computadores e Internet > Desenvolvimento de Software > Geral
2. Computadores e Internet > Programação > Geral
3. Computadores e Internet > Inteligência Artificial (se disponível) — ou Negócios > Empreendedorismo
```

**Palavras-chave (7 campos × 50 chars — frases que o leitor digita, sem repetir o título):**
```
1. vibe coding
2. claude code tutorial
3. kanban para desenvolvedores
4. agentes de ia programacao
5. desenvolvimento com ia
6. vibe kanban indie
7. criar saas do zero
```

> Dica: o subtítulo e as keywords são onde mora a descoberta. Revalide a 7ª palavra-chave e as categorias na hora do cadastro — o KDP muda o catálogo com frequência.

## 7. Definition of done

- [ ] eBook disponível na Amazon
- [ ] Paperback disponível (se escolhido)
- [ ] Página de autor criada no Author Central
- [ ] Metadados revisados na página do produto (título, descrição, categorias)

## Fontes oficiais KDP

- [eBook List Price Requirements](https://kdp.amazon.com/help/topic/G200634560)
- [eBook Royalties](https://kdp.amazon.com/help/topic/G200644210)
- [Critérios da imagem de capa do eBook](https://kdp.amazon.com/help/topic/G200645690)
- [Visão geral de royalties KDP](https://kdp.amazon.com/earn)
