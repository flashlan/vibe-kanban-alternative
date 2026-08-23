# Capítulo 1 — Introdução: vibe coding e o projeto

## O que é vibe coding

"Vibe coding" é o nome que ficou para um jeito novo de programar: você descreve **o que quer** — em linguagem natural — e um agente de IA escreve, roda e corrige o código. Você codifica por intenção. O teclado continua seu, mas o seu trabalho muda: de digitar sintaxe para **dirigir contexto, verificar resultados e tomar decisões**.

A parte que ninguém conta no primeiro dia: a IA não falha por falta de inteligência, falha por falta de **contexto**. Um agente solto num repositório sem documentação de ambiente, sem comandos canônicos e sem fronteiras claras produz código que não compila, quebra convenções e mistura responsabilidades. A diferença entre "vibe coding que funciona" e "vibe coding que gera lixo" não está no modelo — está em como o projeto está estruturado para ser lido por uma máquina.

Este livro é sobre essa estruturação. E ele não é teórico: cada princípio é mostrado num projeto real.

## O estudo de caso: vibe-kanban-indie

O projeto que atravessa todos os capítulos é este próprio repositório: um kanban self-hosted para **um desenvolvedor solo dirigir uma equipe de agentes de IA**. Ele existe justamente para fazer vibe coding em escala — então cada decisão de arquitetura nele é, ao mesmo tempo, uma lição sobre como preparar um projeto para agentes.

O que ele faz, em um parágrafo: você cria cards num quadro kanban; cada card vira um **workspace** (uma worktree git isolada); um executor spawna o agente de coding da sua preferência (Claude Code, OpenCode, Codex, Gemini, Cursor, Amp, Copilot, Qwen, Droid — todos em `crates/executors/src/executors/`); o agente trabalha, reporta progresso por marcadores de texto no log, pede aprovações e avisa quando precisa de revisão humana — com direito a alarme sonoro e escalação para o Telegram.

## A stack em uma página

| Lado | Tecnologia | Papel |
| --- | --- | --- |
| Backend | Rust (edição 2024), 19 crates num workspace Cargo | Tudo que toca estado: HTTP/WebSocket (axum 0.8), banco (SQLx + SQLite), processos, git, filesystem |
| Frontend | TypeScript + React, Vite, pnpm workspaces, TanStack Router, Tailwind | Tudo que é apresentação e interação |
| Contrato | `ts-rs` gera `shared/types.ts` a partir dos structs Rust | Os dois lados falam a mesma língua de tipos |
| Orquestração | Servidor MCP próprio + pipelines em TOML | Os agentes dirigem o próprio fluxo de trabalho |

Números de contexto na data da escrita: 19 crates no workspace (`Cargo.toml` raiz), 93 migrations SQL (`crates/db/migrations/`), 12+ executores de agentes, 9 pipelines em `assets/pipelines/`.

## O que você vai aprender

Os quatro pilares do manual (os tópicos do plano original) viraram seis capítulos:

- **Cap. 2 — The Vibe Coding Setup:** como documentar o ambiente para que uma IA leia o projeto e entenda o contexto instantaneamente.
- **Cap. 3 e 4 — Spec-Driven Architecture:** como as fronteiras do sistema são desenhadas (o que o TypeScript faz no Node, onde o Rust entra) e como o contrato entre os lados é gerado, não combinado.
- **Cap. 5 e 6 — The Engineering Loop:** como documentar comandos e padrões de erro para que agentes rodem testes, leiam logs de compilação e se autocorrigirem — e como o projeto inteiro se torna orquestrável por marcadores de texto.
- **Cap. 7 — Ancoragem de Imagens:** como screenshots funcionam como validação visual para humano e IA.
- **Cap. 8 — Da escrita à Amazon KDP:** o caminho deste manuscrito até a loja.

Cada capítulo termina com um checklist prático para você aplicar no seu próprio projeto.

## Convenção deste livro

Sempre que um capítulo afirmar algo sobre o código, ele cita o caminho do arquivo. Exemplo: o alarme de revisão humana vive em `crates/services/src/services/review_request.rs`. Se um dia o capítulo e o código divergirem, o código está certo — e o capítulo precisa de um commit.
