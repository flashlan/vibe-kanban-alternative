# Agradecimentos

Este livro documenta o **Aurapunk IDE** — um kanban self-hosted para um desenvolvedor solo dirigir agentes de IA. Mas ele não nasceu do zero: apoia-se em dois projetos anteriores, e esta seção existe para creditá-los com clareza.

## A linhagem do software

```
Vibe Kanban (BloopAI)
   └─ Vibe Kanban Indie (dexloom)        ← fork-base deste repositório
        └─ Aurapunk IDE       ← o projeto que este livro documenta
```

- **Vibe Kanban — BloopAI** ([github.com/BloopAI/vibe-kanban](https://github.com/BloopAI/vibe-kanban)): o projeto **original**. A ideia fundamental — um quadro kanban onde cada card sobe um *workspace* isolado com um agente escrevendo código — e boa parte do modelo de execução de agentes e da UI/UX vieram daqui. Sem o lançamento aberto da BloopAI, nada disso existiria.

- **Vibe Kanban Indie — dexloom** ([github.com/dexloom/vibe-kanban-indie](https://github.com/dexloom/vibe-kanban-indie)): o **fork independente** em que este repositório se baseia. Ele pegou o original e o reformatou para um fluxo de *desenvolvedor único, self-hosted, sem nuvem e sem auth* — o modelo de branches `vk/xxxx`, o cockpit local (TUI), a orquestração de agentes e o foco em um só dev. É exatamente esse substrato que este livro descreve.

- **Aurapunk IDE** (este repositório): o fork presente. Adiciona o manual de uso da interface, o passeio prático do SaaS **AssinaFácil** e o pipeline de publicação (incluindo este próprio livro), preservando o espírito self-hosted para um dev solo.

## Outros créditos

- Às ferramentas de ecossistema de agentes que tornam o vibe coding prático: **Claude Code, OpenCode, Codex, Gemini, Cursor, Copilot** e o protocolo **MCP** — sem eles, "dirigir agentes" seria só teoria.
- Ao **Kindle Direct Publishing** e à comunidade de autores técnicos que mantêm viva a cultura de documentar ferramentas em português.
- A você, leitor, por dedicar tempo a aprender a **dirigir** agentes em vez de só pedir a eles.

> Este livro é um documento vivo. Se a linhagem acima mudar (novo fork, novo upstream), atualize esta seção — ela existe para que nenhum esforço anterior seja apagado da história.
