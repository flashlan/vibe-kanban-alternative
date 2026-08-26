# Acknowledgments

This book documents **Aurapunk IDE** — a self-hosted kanban for a solo developer to drive AI coding agents. It does not start from scratch: it stands on two prior projects, and this section credits them clearly.

## The software lineage

```
Vibe Kanban (BloopAI)
   └─ Vibe Kanban Indie (dexloom)        ← base fork of this repo
        └─ Aurapunk IDE       ← the project documented here
```

- **Vibe Kanban — BloopAI** ([github.com/BloopAI/vibe-kanban](https://github.com/BloopAI/vibe-kanban)): the **original** project. The core idea — a kanban board where each card spins up an isolated agent workspace — and much of the agent-execution model and UI/UX originated here.
- **Vibe Kanban Indie — dexloom** ([github.com/dexloom/vibe-kanban-indie](https://github.com/dexloom/vibe-kanban-indie)): the **independent fork** this repository is based on. It reshaped the original for a solo-dev, self-hosted, no-cloud, no-auth workflow — the `vk/xxxx` branch model, the local cockpit (TUI), the agent orchestration — the exact substrate this book describes.
- **Aurapunk IDE** (this repo): the present fork. It adds the interface manual, the AssinaFacil SaaS walkthrough and the publishing pipeline, keeping the solo-dev, self-hosted spirit.

## Further credits

- The agent-ecosystem tooling that makes vibe coding practical: Claude Code, OpenCode, Codex, Gemini, Cursor, Copilot and the MCP protocol.
- The KDP / technical-author community that keeps documenting tools in Portuguese alive.
- You, reader, for learning to *drive* agents instead of just prompting them.
