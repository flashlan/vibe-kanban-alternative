# Capítulo 10 — The Vibe Coding Setup

> **Princípio:** o contexto é o código-fonte da IA. Antes de escrever uma linha de código, escreva os arquivos que dizem a uma máquina como o projeto funciona.

## O problema que este capítulo resolve

Um agente de coding chega ao seu repositório como um desenvolvedor novo chega no primeiro dia: sem saber onde nada mora, quais comandos rodam, o que nunca deve ser tocado. Um humano perguntaria; o agente **assume** — e assumir errado custa caro. O Vibe Coding Setup é a documentação que transforma suposição em leitura.

## Os arquivos de contexto

O ecossistema convergiu para arquivos de contexto na raiz do repositório, lidos automaticamente pelas ferramentas:

| Arquivo | Quem lê |
| --- | --- |
| `AGENTS.md` | Padrão aberto (agents.md): OpenCode, Codex, Cursor e qualquer ferramenta compatível |
| `CLAUDE.md` | Claude Code |
| `.clinerules` | Cline |
| `.cursorrules` | Cursor (legado; hoje `.cursor/rules/`) |

Não precisa de todos. Precisa de **um canônico e ponteiros**. Neste repositório: `AGENTS.md` na raiz é o canônico (escrito para "every agent that works here — Claude Code, OpenCode, Codex, Cursor"), e `docs/CLAUDE.md` existe como ponte. Manter dois arquivos com o mesmo conteúdo é pedir divergência; mantenha um e referencie.

## Anatomia de um AGENTS.md que funciona

O `AGENTS.md` da raiz deste repositório é um bom espécime. Seção por seção, e o porquê de cada uma:

1. **Identidade em uma frase.** "O que é este projeto, para quem, e o que ele NÃO é." Aqui: fork indie, self-hosted, single-developer, sem cloud/auth. Essa linha sozinha impede que um agente "ajude" reintroduzindo código de cloud — e existe uma seção explícita listando os crates deletados (`crates/remote`, `crates/relay-*`) com a ordem "do not reintroduce".

2. **Estado vivo do trabalho.** A seção "Board Status" é um checklist mantido pelos próprios agentes: uma linha por card, com o branch (`vk/xxxx-slug`) para outro agente dar `git switch` direto. Contexto não é só estático — é o estado atual do trabalho em andamento.

3. **Protocolos de interação.** Este repo vai além de documentar: define **protocolos** que o agente deve executar via MCP — buscar o pipeline do card (`get_pipeline`), reportar estágio (`report_pipeline_stage` + linha `VK-PIPELINE-STAGE: N`), buscar regras gerais (`get_rules`). O arquivo de contexto vira contrato de comportamento.

4. **Mapa do território.** "Project Structure & Module Organization": um parágrafo por diretório de primeiro nível (`crates/`, `packages/`, `shared/`, `assets/`...), incluindo o aviso "shared/types.ts é gerado — não edite à mão". O agente que lê isso não perde tempo procurando nem edita o arquivo errado.

5. **Comandos canônicos.** Exatamente como rodar: `pnpm i`, `pnpm run dev`, `pnpm run check`, `cargo test --workspace`, `pnpm run generate-types`, `pnpm run format`. Capítulo 5 inteiro sobre isso.

6. **Convenções e armadilhas.** Estilo (rustfmt, Prettier 2 espaços/aspas simples/80 col), portas fixas de dev (3001/3002/3003), "nunca commite secrets", "antes de completar: `pnpm run format`".

7. **Decisões arquiteturais.** Aponta `docs/ADR/` como o lugar onde decisões vivem — e manda o agente consultar antes de propor alternativas.

## Contexto em camadas: um AGENTS.md por escopo

O contexto certo no lugar certo. Este repo tem três camadas:

- `AGENTS.md` (raiz) — vale para todo o repo.
- `docs/AGENTS.md` — regras de escrita Mintlify que só se aplicam a quem edita documentação.
- `packages/local-web/AGENTS.md` — design system e styling do app web.

Um agente editando um componente React não precisa das regras de escrita de docs; um agente editando docs não precisa das convenções de Tailwind. Contexto por diretório evita diluir o que importa — e economiza tokens em cada sessão.

## Ambiente reproduzível

Contexto também é ambiente. Os pontos que este projeto trava:

- **Versões de runtime:** `package.json` declara `engines: node >= 20, pnpm >= 8` e `packageManager: pnpm@10.13.1`; o workspace Cargo declara `edition = "2024"` e `version` compartilhada por todos os crates.
- **Portas fixas de dev:** frontend 3001, backend 3002, preview proxy 3003 — documentadas no AGENTS.md e usadas nos scripts (`pnpm run dev` exporta `FRONTEND_PORT=3001` etc.). Agente nenhum precisa adivinhar porta.
- **Segredos fora do repo:** `.env` para overrides locais; config de Telegram em `~/.vibe-kanban/telegram.toml` com exemplo commitado em `automation/telegram.toml.example` — o exemplo ensina o formato sem vazar o valor.

## Checklist do capítulo

- [ ] Existe um arquivo de contexto canônico na raiz (e ponteiros, não cópias, para ferramentas específicas).
- [ ] Ele abre com o que o projeto é **e o que ele não é**.
- [ ] Lista os comandos exatos de install/dev/check/test/format.
- [ ] Lista o que é gerado e não pode ser editado à mão.
- [ ] Lista o que foi removido e não deve voltar.
- [ ] Contexto específico de subárea vive em AGENTS.md do subdiretório.
- [ ] Versões de runtime, gerenciador de pacotes e portas estão declarados.
- [ ] Segredos têm arquivo de exemplo commitado e arquivo real ignorado.
