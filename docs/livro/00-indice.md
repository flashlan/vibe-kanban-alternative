# Manual Moderno de Vibe Coding — Índice

**Subtítulo:** *Manual prático da interface do Vibe Kanban Indie — do `npx` ao SaaS em produção, com um projeto-guia.*

Este livro foi escrito dentro do próprio repositório que ele ensina a usar. Todo caminho de arquivo citado existe no código; quando uma regra externa muda (ex.: preços do KDP), o capítulo marca a data de verificação.

## Como usar este livro

- **Parte I (caps. 1–9):** manual de uso — primeiro o vocabulário do vibe coding (cap. 2), depois instale, navegue, mexa com cards, pipelines e git, e feche com o projeto prático **Criando um SaaS com Vibe Kanban**.
- **Parte II (caps. 10–15):** bastidores para quem quer customizar — arquitetura, tipos gerados, loop de engenharia, orquestração e ancoragem de imagens.
- **Apêndice:** referência rápida de comandos.
- O checklist de publicação na Amazon vive em `../livro-vibe-kanban-amazon-checklist.md`.

## Parte I — Manual de Uso + Projeto Prático

| # | Capítulo | Arquivo | Estado |
| --- | --- | --- | --- |
| 1 | Introdução: o que este manual resolve | `01-introducao.md` | Escrito |
| 2 | Noções de vibe coding: Engineering Loop, Spec Development, multi-agente e jargões | `02-nocoes-vibe-coding.md` | Escrito |
| 3 | Instalação e configuração | `03-instalacao-configuracao.md` | Escrito |
| 4 | Tour da interface | `04-tour-interface.md` | Escrito |
| 5 | Cards e Kanban — ciclo de vida na prática | `05-cards-kanban.md` | Escrito |
| 6 | Pipelines na prática | `06-pipelines.md` | Escrito |
| 7 | Git, workspaces e worktrees | `07-git-workspaces.md` | Escrito |
| 8 | Projeto prático: Criando um SaaS com Vibe Kanban | `08-projeto-saas.md` | Escrito |
| 9 | Da escrita à Amazon KDP | `09-publicacao-kdp.md` | Escrito |

## Parte II — Bastidores (para quem customiza o app)

| # | Capítulo | Arquivo | Estado |
| --- | --- | --- | --- |
| 10 | The Vibe Coding Setup (arquivos de contexto) | `10-vibe-coding-setup.md` | Escrito |
| 11 | Arquitetura spec-driven: fronteiras Node × Rust | `11-arquitetura-spec-driven.md` | Escrito |
| 12 | O contrato de tipos: ts-rs na prática | `12-contrato-de-tipos.md` | Escrito |
| 13 | The Engineering Loop: CLI e autocorreção | `13-engineering-loop.md` | Escrito |
| 14 | Orquestração de agentes: MCP, pipelines e o alarme | `14-orquestracao.md` | Escrito |
| 15 | Ancoragem de imagens | `15-ancoragem-imagens.md` | Escrito |
| A | Apêndice: referência de comandos | `apendice-comandos.md` | Escrito |

## Convenções

- Caminhos como `crates/server/src/routes/kanban.rs` são reais no repositório desta branch (`vk/1f98-livre-vibo-kanba`).
- Screenshots sugeridas usam `docs/images/livro/` e estão descritas no cap. 15.
