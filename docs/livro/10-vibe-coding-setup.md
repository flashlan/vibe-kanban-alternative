# Capítulo 10 — The Vibe Coding Setup

> **Princípio:** o contexto é o código-fonte da IA. Antes de escrever uma linha de código, escreva os arquivos que dizem a uma máquina como o projeto funciona.

## O problema que este capítulo resolve

Um agente de coding chega ao seu repositório como um desenvolvedor novo no primeiro dia: sem saber onde nada mora, quais comandos rodam, o que nunca deve ser tocado. Um humano perguntaria; o agente **assume** — e assumir errado custa caro (editar `shared/types.ts` à mão, reintroduzir `crates/remote`, subir na porta errada). O Vibe Coding Setup é a documentação que transforma suposição em leitura. Neste fork, ele tem um nome e um lugar: `AGENTS.md` na raiz.

## Os arquivos de contexto — um canônico, o resto aponta

O ecossistema convergiu para arquivos de contexto na raiz, lidos automaticamente pelas ferramentas:

| Arquivo | Quem lê | Status neste repo |
| --- | --- | --- |
| `AGENTS.md` | Padrão aberto (agents.md): OpenCode, Codex, Cursor e qualquer ferramenta compatível | **Canônico** — escrito para "every agent that works here — Claude Code, OpenCode, Codex, Cursor" |
| `CLAUDE.md` | Claude Code | Ponte para `AGENTS.md` (`docs/CLAUDE.md` existe só como redirect) |
| `.clinerules` | Cline | Não usado — `AGENTS.md` cobre |
| `.cursorrules` / `.cursor/rules/` | Cursor | Não usado — `AGENTS.md` cobre |

Não precisa de todos. Precisa de **um canônico e ponteiros**. Manter dois arquivos com o mesmo conteúdo é pedir divergência; mantenha um e referencie. O `AGENTS.md` raiz deste repo tem ~120 linhas e cobre tudo que um agente precisa para o primeiro commit sem perguntar.

## Anatomia de um AGENTS.md que funciona (com linhas reais)

O `AGENTS.md` da raiz deste repositório é um bom espécime. Seção por seção, e o porquê de cada uma:

### 1. Identidade em uma frase

> "Aurapunk IDE — fork independente e self-hosted do Vibe Kanban, feito para um processo de desenvolvedor único (sem equipe, sem nuvem, sem auth)."

> Nota: o `AGENTS.md` raiz ainda carrega o nome histórico do fork-base; o produto é comercializado como **Aurapunk IDE** e deriva do **Vibe Kanban Indie** (dexloom), que por sua vez deriva do **Vibe Kanban** original (BloopAI) — ver Agradecimentos.

Essa linha sozinha impede que um agente "ajude" reintroduzindo auth ou cloud. Logo abaixo vem a seção explícita listando os crates deletados (`crates/remote`, `crates/relay-*`) com a ordem **"do not reintroduce"** — e o aviso que `shared/remote-types.ts` é contrato congelado, não lixo (ver cap. 12).

### 2. Estado vivo do trabalho — o Board Status

```md
## Board Status (agent-maintained checklist)

- [x] Done — Add image attachment to create issue dialog (`vk/5f5b-...`)
- [~] In Progress — Livro Vibe Kanban na Amazon (`vk/1f98-livre-vibo-kanba`)
```

Uma linha por card, com o branch (`vk/xxxx-slug`) para outro agente dar `git switch` direto. Contexto não é só estático — é o estado atual do trabalho em andamento. O agente que lê sabe o que já foi feito sem re-query no board.

### 3. Protocolos de interação — o arquivo vira contrato

Este repo vai além de documentar: define **protocolos MCP** que o agente deve executar:

- Buscar o pipeline do card (`get_pipeline`) antes de qualquer edição (o card só carrega `<!-- vk:pipeline:start -->`).
- Reportar estágio (`report_pipeline_stage` + linha `VK-PIPELINE-STAGE: N` no log).
- Buscar regras gerais (`get_rules`) no início e checar `post` antes de finalizar.

O arquivo de contexto vira contrato de comportamento — ver cap. 14 para a implementação em `crates/mcp`.

### 4. Mapa do território

"Project Structure & Module Organization": um parágrafo por diretório de primeiro nível (`crates/`, `packages/`, `shared/`, `assets/`…), incluindo o aviso **"shared/types.ts é gerado — não edite à mão"**. O agente que lê isso não perde tempo procurando nem edita o arquivo errado.

### 5. Comandos canônicos

Exatamente como rodar: `pnpm i`, `pnpm run dev`, `pnpm run check`, `cargo test --workspace`, `pnpm run generate-types`, `pnpm run format`. O cap. 13 destrincha o loop; aqui basta listar — o agente copia e cola.

### 6. Convenções e armadilhas

Estilo (rustfmt, Prettier 2 espaços/aspas simples/80 col), **portas fixas de dev (3001/3002/3003)**, "nunca commite secrets", "antes de completar: `pnpm run format`". Cada armadilha documentada é um erro que o agente não vai cometer.

### 7. Decisões arquiteturais

Aponta `docs/ADR/` como o lugar onde decisões vivem — e manda o agente consultar antes de propor alternativas. Ver cap. 11.

## Contexto em camadas: um AGENTS.md por escopo

O contexto certo no lugar certo. Este repo tem três camadas — e o AssinaFácil (cap. 08) deveria copiar:

```
AGENTS.md              ← vale para todo o repo
docs/AGENTS.md         ← só para quem edita documentação (Mintlify, frontmatter, <Frame>)
packages/local-web/AGENTS.md ← só para quem edita UI (Tailwind, design tokens)
```

Um agente editando um componente React não precisa das regras de escrita de docs; um agente editando docs não precisa das convenções de Tailwind. Contexto por diretório **evita diluir o que importa — e economiza tokens** em cada sessão (cada camada só é lida quando o agente toca naquela pasta).

> **Exercício:** crie `AGENTS.md` no seu SaaS com 4 seções — identidade (1 frase), mapa (1 linha por pasta), comandos (`dev`/`check`/`format`) e "o que é gerado / o que nunca fazer". Quando um agente criar `shared/types.ts` à mão e quebrar o CI, você saberá que faltou a linha "não edite à mão".

## Ambiente reproduzível — o que travar

Contexto também é ambiente. Os pontos que este projeto trava e que o seu deveria travar igual:

- **Versões de runtime:** `package.json` declara `engines: node >= 20, pnpm >= 8` e `packageManager: pnpm@10.13.1`; o workspace Cargo declara `edition = "2024"` e `version` compartilhada por todos os crates (`Cargo.toml` raiz). Sem isso, o agente instala com npm 9 e quebra o lockfile.
- **Portas fixas de dev:** frontend 3001, backend 3002, preview proxy 3003 — documentadas no `AGENTS.md` e exportadas por `pnpm run dev` como `FRONTEND_PORT`/`BACKEND_PORT`. Agente nenhum precisa adivinhar porta — e o erro `AddrInUse` fica previsível (cap. 13).
- **Segredos fora do repo:** `.env` para overrides locais (ignorado no `.gitignore`); config de Telegram em `~/.vibe-kanban/telegram.toml` com exemplo commitado em `automation/telegram.toml.example` — o exemplo ensina o formato sem vazar o valor. O mesmo vale para `STRIPE_SECRET_KEY` no AssinaFácil.

## Checklist do capítulo

- [ ] Existe um arquivo de contexto canônico na raiz (e ponteiros, não cópias, para ferramentas específicas).
- [ ] Ele abre com o que o projeto **é e o que ele não é** (inclui "o que foi removido e não deve voltar").
- [ ] Lista os comandos exatos de install/dev/check/test/format (copiar-colar funciona).
- [ ] Lista o que é gerado e não pode ser editado à mão (`shared/types.ts`, `routeTree.gen.ts`).
- [ ] Contexto específico de subárea vive em `AGENTS.md` do subdiretório (docs, web).
- [ ] Versões de runtime, gerenciador de pacotes e portas estão declarados.
- [ ] Segredos têm arquivo de exemplo commitado e arquivo real ignorado.
- [ ] Um agente novo consegue fazer o primeiro commit sem perguntar nada — teste com `vk/quick`.
