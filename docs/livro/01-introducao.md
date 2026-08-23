# Capítulo 1 — Introdução: o que este manual resolve

## Para quem é este livro

Para um desenvolvedor que acabou de instalar o Vibe Kanban Indie e quer **usar a interface para desenvolver de verdade** — não para estudar a arquitetura do app. Ao final da Parte I você vai saber:

- instalar e configurar o app no seu `projects.toml`;
- navegar na interface (board, workspaces, painéis);
- criar e mover **cards** pelas colunas do kanban;
- entender o que são **pipelines** e como eles movem o seu card sozinho;
- usar **git sem medo** dentro do Vibe Kanban (workspaces, worktrees, branches, PRs);
- construir um projeto do zero — **um SaaS completo** — usando só a interface.

A Parte II fica para quando você quiser customizar o próprio Vibe Kanban. O foco agora é **usar o aplicativo para desenvolver**.

## O que é o Vibe Kanban Indie, em uma página

O Vibe Kanban Indie é um **kanban self-hosted para um desenvolvedor solo dirigir agentes de IA**. Cada cartão do quadro é uma tarefa ("consertar login", "criar página de planos do SaaS"). Cada tarefa vira um **workspace** — uma pasta isolada com seu próprio branch git — onde um agente (Claude Code, OpenCode, Codex, Gemini, Cursor, Copilot, etc.) escreve código por você. Você acompanha o progresso no board, revisa diffs e dá merge.

Os conceitos que você vai usar todo dia:

| Conceito | O que é, numa frase |
| --- | --- |
| **Issue / Card** | Unidade de trabalho. Título + descrição + status + prioridade + tags. A descrição vira o prompt do agente. |
| **Board / Colunas** | Quadro kanban por projeto. Cada coluna é um `project_status` (ex.: Todo → In Progress → Done). Você arrasta cards entre colunas. |
| **Workspace** | Ambiente isolado de uma tarefa: um git worktree + branch `vk/xxxx-nome` + sessão do agente. |
| **Pipeline** | Receita em TOML (`assets/pipelines/*.toml`) que diz ao agente o que fazer e em que ordem — e como reportar progresso (`VK-PIPELINE-STAGE: N`). |
| **Sessão** | Conversa com um agente dentro de um workspace. Um workspace pode ter várias sessões. |
| **Setup/Cleanup/Dev scripts** | Comandos por repositório/projeto que o Vibe Kanban roda automaticamente ao criar/abrir/fechar um workspace. |

## O projeto-guia deste livro

A partir do capítulo 7 você constrói um SaaS de verdade — **AssinaFácil**, um SaaS fictício de gestão de assinaturas — inteiramente pela interface do Vibe Kanban. Cada capítulo da Parte I deixa um card pronto para o próximo, de modo que no final você tem um board com o histórico completo do produto.

## Como ler

- Siga a Parte I em ordem na primeira leitura; cada capítulo termina com um **checklist** que você pode marcar no seu próprio board.
- Caminhos como `docs/getting-started.mdx` ou `crates/server/src/main.rs` existem de verdade nesta branch (`vk/1f98-livre-vibo-kanba`) — abra e confira.
- Screenshots citadas vivem em `/images/` (docs do site) e `docs/images/livro/` (âncoras do livro, cap. 14).
