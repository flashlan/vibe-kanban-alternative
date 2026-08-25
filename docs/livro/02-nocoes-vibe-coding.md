# Capítulo 2 — Noções de vibe coding: Engineering Loop, Spec Development e multi-agente

> **Objetivo:** aprender o jargão atual do desenvolvimento com IA — com exemplos reais do SaaS AssinaFácil — para que os capítulos práticos (03 em diante) não precisem parar a cada palavra nova.

## 1. Vibe coding, em uma frase — e por que não é "pedir e torcer"

Vibe coding é **codar por intenção**: você descreve o resultado que quer em linguagem natural e um agente escreve, roda e corrige o código. O termo foi popularizado por Andrej Karpathy em fev/2025 e pegou porque descreve bem a sensação — você "vibe" a ideia, a máquina materializa.

### O que vibe coding **não** é

Para usar a ferramenta bem, vale desarmar três expectativas erradas:

- **Não é "no-code".** Você continua sendo o dono da arquitetura, da revisão e do merge. O agente é um desenvolvedor júnior que trabalha na velocidade dos tokens — rápido, mas que precisa de um sênior (você) para aprovar e direcionar.
- **Não é "pedir e torcer".** Um prompt solto no chat sem contexto gera código que *parece* certo e quebra em runtime. Vibe coding de verdade é um fluxo com **artefatos** que tornam a intenção auditável.
- **Não é mágica sem custo.** O agente tem memória finita (a *context window*) e atenção seletiva. Se você não gerir o que ele vê e o que ele lembra, ele alucina, esquece ou repete trabalho. Daí as práticas deste capítulo (e a seção 6: contexto, memória, autocompact).

### O que vibe coding **é**, no Vibe Kanban

No Vibe Kanban, vibe coding não é prompt solto no chat. É um fluxo com artefatos — **cards (spec), pipelines (receita), workspaces (worktrees isolados)** — que tornam a intenção auditável, repetível e paralelizável. Sem artefatos, o agente alucina; com artefatos, ele entrega.

```
intenção (você) → spec (card) → pipeline (receita) → workspace (worktree) → loop (agente) → review (você)
```

### Níveis de maturidade do vibe coding

| Nível | Como você trabalha | Onde este livro mora |
| --- | --- | --- |
| 1 — Chat solto | Você pede e copia o resultado | — (ponto de partida, frágil) |
| 2 — Spec-driven | Cada tarefa vira card com spec forte + critério de pronto (caps. 2–5) | Parte I |
| 3 — Orquestrado | Vários agentes em paralelo, pipelines, memória e autocompact cuidando do estado (caps. 6, 10–14) | Parte II |

Este livro te leva do nível 1 ao 3. O cap. 2 todo é o vocabulário; a seção 6 (abaixo) é o que separa um usuário que "pede" de um que "dirige".

Este capítulo apresenta os três pilares que sustentam esse fluxo e, em seguida, as boas práticas operacionais.

## 2. Spec Development — a spec vem antes do código

**Spec = o que o software deve fazer, antes de como faz.** Se a spec está fraca, o código nasce torto e todo conserto depois é remendo.

### Onde a spec vive neste repositório

- **O card é a spec.** Em `docs/issue-management.mdx:57`, a descrição do card vira o prompt que o agente recebe ao criar a workspace. A diferença entre um card fraco e um forte é a diferença entre agente perdido e agente certeiro.
- **Pipelines formalizam a spec.** `speckit` e `basic` (`assets/pipelines/*.toml`) geram `SPEC.md` / `IMPLEMENTATION_PLAN.md` antes de tocar no código — o agente só codifica depois que a spec foi revisada.
- **ADRs guardam a spec arquitetural.** O `AGENTS.md` raiz manda consultar `docs/ADR/` antes de propor alternativas.

### Exemplo concreto: "Página de planos" do AssinaFácil

**Spec fraca** (o agente vai adivinhar — e errar):

> Fazer página de planos.

**Spec forte** (o agente acerta de primeira):

> **Título:** Criar página `/planos` com 3 planos (Free, Pro R$49, Enterprise) e CTA para checkout
>
> **Descrição:**
> - Tabela comparativa com 3 colunas, badges "Mais popular" no Pro, CTA "Assinar" em cada coluna.
> - Arquivos: `packages/web-core/src/pages/plans.tsx` (novo), `packages/web-core/src/features/billing/plans.ts` (dados dos planos).
> - Validação: Preview em 1440px e 375px mostra tabela sem quebra; `pnpm run check` passa; screenshot ancorada em `docs/images/livro/saas-planos.png` coincide.
> - Restrições: usar Tailwind já configurado (`packages/local-web/AGENTS.md`); não adicionar dependência nova; seguir padrão de `packages/web-core/src/features/`.

Perceba a estrutura: **o que** (tabela de 3 planos), **onde** (arquivos), **como validar** (check + screenshot), **restrições** (sem nova lib). O agente recebe isso como prompt e já sabe quando terminou — quando o Preview coincide com a âncora.

### Jargões de spec que você vai ver

| Termo | O que quer dizer aqui |
| --- | --- |
| **Spec-Driven Architecture** | A spec dita quem faz o quê: TypeScript no Node cuida de UI; Rust cuida de estado/processos/git. A fronteira `crates/` vs `packages/` vs `shared/` é desenhada na spec, não no improviso (ver cap. 11). |
| **Spec Intake** | Transformar um briefing vago ("quero um SaaS de assinaturas") em tarefa pronta para desenvolvimento (`docs/cockpit/spec-intake.mdx`). No livro, o intake do AssinaFácil vira o épico + 5 sub-issues do cap. 08. |
| **Contrato gerado** | A spec da fronteira vive nos structs Rust com `#[derive(TS)]` e é gerada para TypeScript (`shared/types.ts` via `crates/server/src/bin/generate_types.rs` — cap. 12). O código **é** a spec. |
| **Critério de pronto** | Frase que diz quando o card está Done — sempre com validação observável ("Preview mostra X; `pnpm run check` passa"). |

> Regra prática: se você não consegue escrever a spec em 5 frases + 1 critério de pronto, o agente também não vai conseguir implementar.

## 3. Engineering Loop — o loop que deixa o agente se corrigir sozinho

O **Engineering Loop** é o ciclo que permite ao agente **se corrigir sem você**:

```
escrever → rodar checks → ler o erro no log → corrigir → repetir
```

Quando o loop é curto e legível, 90% dos erros se resolvem sozinhos. Quando é longo ou opaco, o agente para e escala — e você é interrompido.

### Os três ingredientes do loop

**a) CLI como interface do agente.** Comandos canônicos em `package.json` que o agente lê em `AGENTS.md`:

```bash
pnpm run check        # tsc (local-web, web-core, ui) + cargo check + guards
pnpm run lint         # ESLint + cargo clippy -- -D warnings (com --features qa-mode)
pnpm run format       # cargo fmt + Prettier
cargo test --workspace
pnpm run generate-types  # regenera shared/types.ts (cap. 12)
```

O agente não adivinha comandos; ele roda os mesmos que você rodaria.

**b) Erros que ensinam.** Guards falham com mensagem que aponta a correção:

- `scripts/check-migration-frozen.sh` — impede editar migration publicada e diz por quê.
- `scripts/check-legacy-frontend-paths.sh` — impede importar de caminho antigo e aponta o novo.
- `cargo clippy -- -D warnings` — transforma aviso em erro; nada passa como "só um warning".

**c) Logs como protocolo.** O agente reporta progresso escrevendo no log; o backend observa o `MsgStore`:

- `VK-PIPELINE-STAGE: N` → persiste `workspaces.current_pipeline_stage` (`crates/services/src/services/pipeline_stage.rs`, regex `(?i)VK-PIPELINE-STAGE:\s*(\d+)`).
- `VK-REVIEW-REQUEST: ...` → dispara som + notificação (`crates/services/src/services/review_request.rs`, regex `(?i)VK-REVIEW-REQUEST:\s*(.+)`).

O log é, ao mesmo tempo, saída humana e API de máquina — e funciona igual para Claude, Codex, OpenCode, Gemini… porque todos escrevem no mesmo `MsgStore` (`crates/executors/src/stdout_dup.rs`).

### Walkthrough real do loop (com erro de verdade)

Imagine o card "Página de planos" do exemplo acima. O agente escreve `plans.tsx` e roda:

```bash
pnpm run check
# → error TS2322: Type 'string' is not assignable to type 'PlanTier' in plans.ts:14
```

O agente lê o erro, abre `plans.ts:14`, vê que usou `"pro"` em vez de `"Pro"` (o tipo `PlanTier` vem de `shared/types.ts`, gerado do Rust — cap. 12), corrige, e roda de novo:

```bash
pnpm run check   # → passa
pnpm run lint    # → passa (clippy -D warnings não perdoa)
```

Só então ele escreve `VK-PIPELINE-STAGE: 2` e segue. Você não fez nada — o loop fechou sozinho porque o erro era legível e o comando era canônico.

Se o erro fosse opaco ("falhou"), o agente pararia em `Needs Attention` e você teria que adivinhar. Por isso o livro insiste: **invista no loop antes de investir no prompt**.

## 4. Multi-agente — vários agentes em paralelo sem pisar no pé do outro

Um agente sozinho já ajuda. Vários, em paralelo, mudam a escala — mas só se o repositório isolar o trabalho de cada um.

### O isolamento: workspaces = worktrees

Cada workspace é uma pasta em `.vibe-kanban-workspaces/` com seu próprio branch `vk/xxxx-nome` criado a partir do `target branch` (`docs/workspaces/creating-workspaces.mdx:12`). Seu repo original não é tocado.

```
repo original (main)
  ├─ .vibe-kanban-workspaces/
  │   ├─ vk-a1b2-landing-page/   (workspace A, branch vk/a1b2-...)
  │   └─ vk-c3d4-auth/           (workspace B, branch vk/c3d4-...)
  └─ (intocado)
```

- **Um card, um workspace — ou vários.** Você pode vincular vários workspaces ao mesmo card e rodar Claude, Codex e OpenCode em paralelo, cada um no seu worktree.
- **Pipelines multi-agente.** `swarm-multi-agent.toml` orquestra subagentes em frentes diferentes do mesmo épico. O **Orchestrator** é o agente singleton que dirige o board inteiro (`docs/cockpit/orchestrator.mdx`, `crates/services/src/services/orchestrator_compactor.rs` — com watchdog de 400k tokens / 1h / 10m cooldown).

### Exemplo prático: AssinaFácil em paralelo

Épico **AssinaFácil — MVP** com sub-issues:

1. Landing page
2. Auth (login/cadastro)

Você despacha os dois ao mesmo tempo:

- Workspace A (`vk-a1b2-landing`) — agente 1 escreve `landing.tsx`.
- Workspace B (`vk-c3d4-auth`) — agente 2 escreve `auth.tsx`.

Cada um roda seu próprio `pnpm run check` no seu worktree, sem conflito. Quando ambos pedem `VK-REVIEW-REQUEST`, você revisa os diffs em **Changes** e o app no **Preview** de cada workspace, move os cards para Done, e o pipeline faz squash-merge no `main` — um de cada vez, sem merge hell.

### Jargões de multi-agente

| Termo | O que quer dizer aqui |
| --- | --- |
| **Swarm / crew** | Conjunto de agentes em workspaces diferentes do mesmo projeto. |
| **Orchestration** | Decidir quem faz o quê e em que ordem — via `get_pipeline` / `report_pipeline_stage` (MCP). |
| **YOLO mode** | Rodar sem pedir permissão a cada tool call (`docs/vibe-guide.mdx:52` — "Use YOLO mode" para async funcionar; sem isso você reinventou pair programming, só que mais lento). |
| **Needs Attention** | Estado da workspace na sidebar quando há approval pendente — o agente levantou a mão (`docs/workspaces/interface.mdx:70`). |

## 5. Glossário rápido — os jargões que aparecem toda hora

| Jargão | Tradução prática |
| --- | --- |
| **Context engineering** | Escolher o que o agente vê (arquivos, logs, regras). É o trabalho do `AGENTS.md` e do `get_rules`/`memory_search`. |
| **Prompt engineering** | Escrever a descrição do card de um jeito que o agente acerta (cap. 05: título com verbo + critério de pronto). |
| **Spec intake** | Pegar um pedido vago e transformar em card bem especificado antes de codar. |
| **Approval** | Pausa pedindo permissão — tool permission ("posso rodar `rm`?") ou pergunta ("qual prioridade?") — respondida via TUI, Telegram ou `respond_to_approval` (MCP). |
| **Setup / Cleanup / Dev scripts** | Comandos por projeto/repo que o Vibe Kanban roda ao criar/abrir/fechar workspace (Settings → Projects & Repositories). |
| **Worktree** | Cópia leve do repo com branch próprio — o isolamento que permite multi-agente sem conflito. |
| **Target vs Working branch** | Target = onde vai mergear (ex.: `main`, você define); Working = onde o agente trabalha (`vk/xxxx`, auto-criado). |
| **Preview proxy** | O Rust serve o dev server Node dentro do painel Preview (cap. 07). |
| **TUI / Telegram bridge** | Superfícies de controle para um dev solo: `vibe-tui` (`crates/tui`) no terminal e `vibe-telegram-bridge` (`crates/telegram-bridge`) no Telegram — ambas falam com a mesma API de approvals (`automation/README.md`). |
| **Squash-merge** | Pipeline `quick` junta os commits da workspace num só commit no target branch. |
| **ADR** | Architecture Decision Record em `docs/ADR/` — a spec arquitetural versionada. |

## 6. Boas práticas — contexto, memória e autocompact

Vibe coding de nível 3 não é escrever prompts melhores; é **gerir o estado cognitivo do agente**. Três alavancas: o que ele *vê* (contexto), o que ele *lembra* entre sessões (memória) e como ele *sobrevive* a runs longos (autocompact). Dominar as três é o que separa "pedir" de "dirigir".

### 6.1 Context engineering — o contexto é o código-fonte da IA

**Context engineering** é a disciplina de escolher o que o agente enxerga a cada turno. O agente só é tão bom quanto o que está na janela dele; tudo fora dela, para ele, não existe.

No Vibe Kanban, o contexto é montado por camadas (cap. 10):

- `AGENTS.md` raiz (identidade, mapa, comandos) + `docs/AGENTS.md` (docs) + `packages/local-web/AGENTS.md` (UI) — cada camada só carrega quando o agente toca naquela pasta, economizando tokens.
- `get_rules` (MCP) traz as regras gerais no início de cada card; `get_pipeline` traz **só o estágio atual**, não o pipeline inteiro.
- Cards carregam apenas um **ponteiro** de pipeline (`<!-- vk:pipeline:start -->`), não o prompt pesado — o conteúdo entra na janela só quando o card roda.

**Manipulação de contexto** (o que você, humano, faz):

- **Injete contexto com intenção.** Anexe uma screenshot ou um trecho de log no chat (`POST /api/attachments/upload`, cap. 05 §4) em vez de descrever "o erro" por extenso — a imagem vale como mil palavras de contexto.
- **Reduza ruído.** Use subagentes (multi-agente, §4) para que o trabalho lateral não suje a janela principal; não enfie o board inteiro no prompt.
- **Podc contexto morto.** Quando o transcript fica grande, force `VK-PIPELINE-STAGE` a avançar e deixe o **OrchestratorCompactor** (§6.3) cortar o que não importa.

> Regra de ouro: trate a *context window* como orçamento finito. Cada linha que você coloca no card é uma linha a menos para o agente raciocinar.

### 6.2 Autocompact — como o agente não "esquece" em runs longos

**Autocompact** é a compactação automática do transcript quando a janela de contexto está prestes a estourar. Sem isso, em runs de horas o agente "perde o início" — esquece a spec e começa a contradizer o que fez.

No Vibe Kanban, o watchdog é o **OrchestratorCompactor** (`crates/services/src/services/orchestrator_compactor.rs`):

- Mede os tokens do transcript **a cada 60s**.
- Se passar de **400k tokens** (ou 1h sem compactar, com pelo menos 50k), ele digita `/compact` na sessão tmux.
- Digita **pelo caminho de teclas** (`tmux send-keys`), não como texto colado — porque slash commands não funcionam como texto colado numa sessão interativa.
- **Cooldown de 10min** entre envios; **3 falhas seguidas** escalam para o Telegram (`crates/telegram-bridge`).

Para você: não precisa vigiar o tamanho do contexto — o watchdog cuida. Para o seu SaaS (cap. 08): aplique o mesmo princípio — corte contexto morto antes de estourar, não depois.

> **Compact manual vs autocompact:** `/compact` é o comando que o agente (ou o watchdog) dispara; "autocompact" é esse disparo acontecendo sozinho. Em agentes sem watchdog, você mesmo manda `/compact` quando o transcript cresce.

### 6.3 Memória de agentes — mem0 (semântica + grafo)

**Memória de agente** é o que permite que o agente *lembre entre sessões* fatos verificados, em vez de reapprender tudo a cada card. No Vibe Kanban isso é o **mem0**, exposto como tools MCP (`crates/mcp/src/task_server/tools/mem0.rs`):

| Tool | O que faz | Quando usar |
| --- | --- | --- |
| `memory_search` | Busca por **similaridade semântica** (não palavra-chave) | "Como o pipeline de stage funciona?" antes de tocar código |
| `memory_save` | Persiste um fato **verificado e durável** | Depois de confirmar uma decisão de arquitetura |
| `memory_graph_traverse` | Atravessa arestas de dependência a partir de uma entidade | "O que consome o marcador `VK-PIPELINE-STAGE`?" |
| `memory_check_staleness` | Checa se uma entidade salva ainda existe no código (diff `commit_sha` → HEAD) | Antes de confiar em uma memória antiga |

**Memória semântica** (`memory_search`): responde por *proximidade de significado*. Você pergunta "qual o fluxo do card até produção?" e ela ranqueia os trechos mais relevantes — mesmo que nenhum contenha a palavra exata "fluxo". Dica do `AGENTS.md`: se os resultados não cobrem o que você precisa, **re-pesquise com query mais afiada** em vez de pedir mais hits — iterar vence buscar amplo.

**Memória de grafo** (`memory_graph_traverse`): segue a *estrutura* real do código. De um nó inicial (`start`, ex.: `pipeline_stage.rs`), você vai `out` (o que depende dele), `in` (do que ele depende) ou `both`, até `hops` passos (máx. 3). É como maps de "quem usa isso?" — útil quando a semântica não bate mas a dependência é clara.

**O que salvar (e o que não salvar):**

- ✅ Salve factos **verificados** e duráveis (decisões de ADR, contratos, onde mora cada crate). O `memory_save` é *best-effort*: retorna `stored=false` se o mem0 estiver indisponível — não é erro, apenas persista depois.
- ❌ Não salve especulação, segredos, ou fatos efêmeros (o `target branch` de hoje pode mudar amanhã).
- ❌ Não salve o que está no `AGENTS.md` — duplicar contexto é desperdício.

**Staleness:** `memory_check_staleness` olha o `commit_sha` capturado quando a memória foi salva e diffa o repo até HEAD. Se o texto da entidade some do código removido, ela está **stale**. `checked=false` significa "não consegui verificar" — trate como *desconhecido*, nunca como "confirmado fresco".

### 6.4 Outros termos que você verá

| Termo | Tradução prática |
| --- | --- |
| **Context window** | Tamanho máximo (em tokens) do que o agente enxerga de uma vez. É o seu orçamento (§6.1). |
| **Context engineering** | Escolher o que entra nessa janela — o trabalho do `AGENTS.md`, `get_rules` e da poda. |
| **Autocompact** | Compactação automática do transcript quando a janela estoura (§6.2). |
| **Compact (manual)** | Comando `/compact` que corta o transcript mantendo o essencial. |
| **Semantic memory** | Memória por similaridade de significado (`memory_search`). |
| **Graph memory** | Memória por arestas de dependência (`memory_graph_traverse`). |
| **Embeddings** | Como o mem0 transforma texto em vetor para comparar significado (detalhe interno; você só usa a busca). |
| **Retrieval / RAG** | Recuperar memória ou docs relevantes para injetar no contexto. |
| **Staleness** | Se uma memória ainda bate com o código atual (`memory_check_staleness`). |
| **Scratch / Notes** | Rascunho efêmero por workspace (painel Notes, cap. 04) — não confunda com memória persistente. |

### 6.5 Regras de ouro das boas práticas

- Trate a *context window* como orçamento: injete o essencial, corte o ruído.
- Compacte cedo — autocompact ou `/compact` — em runs longos, antes de estourar.
- Salve na memória **só fatos verificados e duráveis**; deixe especulação e segredos de fora.
- Use **grafo** para descobrir vizinhança (quem usa o quê) e **semântica** para recordar *como* algo funciona.
- **Verifique staleness** antes de confiar em memória antiga — `checked=false` não é "fresco".
- Memória e contexto não substituem o `AGENTS.md`: a fonte canônica fica no arquivo, não na memória volátil.

## 7. Como tudo se encontra em 2 minutos

Um fluxo típico, com o vocabulário certo:

1. Você faz **spec intake**: escreve um card "Página de planos" com a spec forte do §2 e critério de pronto.
2. Cria uma **workspace** (worktree + working branch `vk/xxxx-planos`) vinculada ao card, escolhe o **pipeline** `quick` e despacha o agente.
3. O agente entra no **Engineering Loop**: escreve `plans.tsx` → `pnpm run check` → lê `TS2322` → corrige → `VK-PIPELINE-STAGE: 2`.
4. Se precisar de você, levanta `VK-REVIEW-REQUEST` ou `Needs Attention` — você responde na interface, na **TUI** (`cargo run -p tui`, tecla `a`) ou no **Telegram**.
5. Quando o critério de pronto bate (Preview 1440px + 375px ok, check passa, screenshot `saas-planos.png` coincide), o card vai para **Done** e o pipeline faz **squash-merge** no `main`. Enquanto isso, outro agente já corre em paralelo no card "Auth" — **multi-agente**.

Nos próximos capítulos você vai viver esse fluxo na prática, começando pela instalação (cap. 03).

## Checklist do capítulo

- [ ] Sei explicar vibe coding, spec e engineering loop em uma frase cada.
- [ ] Sei transformar "Fazer página de planos" em spec forte com onde/validar/restrições.
- [ ] Sei descrever o loop `escrever → check → ler erro → corrigir` com um exemplo real (`TS2322`).
- [ ] Sei por que worktrees permitem multi-agente sem conflito (e o que é target vs working branch).
- [ ] Reconheço os jargões das tabelas (spec, multi-agente, glossário e boas práticas) quando eles aparecerem nos caps. 03–08.
- [ ] Sei o que é context engineering e como o Vibe Kanban injeta/poda contexto (camadas de AGENTS.md, pipeline pointer, anexos).
- [ ] Sei o que é autocompact e qual watchdog cuida disso (OrchestratorCompactor, 400k tokens, /compact via teclas).
- [ ] Sei a diferença entre memória semântica (memory_search) e de grafo (memory_graph_traverse), e o que salvar vs não salvar.
- [ ] Sei usar memory_check_staleness antes de confiar em memória antiga.
- [ ] Consigo narrar o fluxo de 2 minutos (spec → workspace → loop → review → merge) sem consultar o livro.
