#!/usr/bin/env python3
"""Consolida docs/livro/*.md em um único manuscrito para exportação KDP.

Ordem de leitura = numeração dos arquivos (00-indice ... 15 ... apendice).
Imagens /images/livro/* viram ../images/livro/* (relativas a docs/livro/manuscript.md),
para empacotar junto quando converter (pandoc/kindle-create).
"""
from pathlib import Path

LIVRO = Path("docs/livro")
OUT = LIVRO / "manuscript.md"

ORDER = [
    "00-indice.md",
    "01-introducao.md",
    "02-nocoes-vibe-coding.md",
    "03-instalacao-configuracao.md",
    "04-tour-interface.md",
    "05-cards-kanban.md",
    "06-pipelines.md",
    "07-git-workspaces.md",
    "08-projeto-saas.md",
    "09-publicacao-kdp.md",
    "10-vibe-coding-setup.md",
    "11-arquitetura-spec-driven.md",
    "12-contrato-de-tipos.md",
    "13-engineering-loop.md",
    "14-orquestracao.md",
    "15-ancoragem-imagens.md",
    "apendice-comandos.md",
]

TITLE = """# Manual Moderno de Vibe Coding

### Uso prático do Vibe Kanban Indie — do `npx` ao SaaS em produção

**Subtítulo:** *Manual prático da interface do Vibe Kanban Indie, com um projeto-guia SaaS (AssinaFácil).*

> Manuscrito gerado a partir de `docs/livro/*.md` (branch `vk/1f98-livre-vibo-kanba`).
> Regras externas (preços KDP) verificadas em ago/2026 — revalide antes de publicar.

---

"""

parts = [TITLE]
for name in ORDER:
    p = LIVRO / name
    text = p.read_text(encoding="utf-8")
    # imagens do livro: /images/livro/* -> ../images/livro/*
    text = text.replace("](/images/livro/", "](../images/livro/")
    parts.append(text.rstrip() + "\n\n---\n\n")

OUT.write_text("".join(parts), encoding="utf-8")
print(f"escrito {OUT} ({OUT.stat().st_size} bytes, {len(ORDER)} arquivos)")
