# Chapter 9 — From writing to Amazon KDP

> **Principle:** publishing is a pipeline like any other — with stages, a checklist and a done criterion. The only difference is that the "deploy" is a store.

## Write here, publish there

This book was born as `docs/livro/*.md` inside the very repository it describes. That is no accident — it is the loop of ch. 05 taken to the extreme: the manuscript is versioned, reviewed in a PR, checked by `pnpm run check` and anchored by images, exactly like code. When the content is ready, it crosses the boundary out of the repo and becomes a product on Amazon. The checklist governing that crossing lives in `docs/livro-vibe-kanban-amazon-checklist.md`.

This chapter does not repeat the checklist line by line — it explains **how to decide** at each point where KDP gives you choices.

## Five decisions that matter

### 1. eBook, paperback or both?

Start with Kindle eBook. Zero marginal cost (Kindle Create is free), hours to publish, up to 70% royalties and global distribution without logistics. Paperback is stage two: it requires a PDF body with trim-size margins, a full-cover PDF (front+spine+back, KDP cover calculator template, bleed 0.125", 300 DPI, CMYK) and a physical proof. The checklist separates the two tracks — Phase 5 (eBook) and Phase 6 (paperback) — so you can launch the eBook first and iterate.

### 2. Price and royalty

KDP gives two options per eBook (rules verified Aug/2026; revalidate before publishing):

- **70%** between US$ 2.99 and **US$ 12.99** (ceiling raised from US$ 9.99 in Jul/2026), with a US$ 0.15/MB delivery fee. Sales to Brazil/Japan/Mexico/India pay 70% only if the book is in KDP Select.
- **35%** between US$ 0.99 and US$ 200 (minimum rises with file size), no delivery fee.

For a technical manual with images, file size matters: a heavy-screenshot eBook may pay a real delivery fee in the 70% band. Simulate both before deciding. Paperback pays 50% or 60% minus print cost, with a US$ 9.99 cut.

### 3. KDP Select: yes or no?

KDP Select gives 90 days of digital exclusivity in exchange for: Kindle Unlimited (paid per page read), extra promos and — the point here — **70% in Brazil**. If your main audience is in Brazil, Select pays for itself. If you must also sell on Apple Books/Kobo, don't enroll. The decision reverses every 90 days.

### 4. Categories and keywords

You get **up to 3 categories** per format (chosen in the KDP selector; the old "email for 10 more" scheme is gone) and **7 fields of 50 characters** for keywords. The lesson from ch. 04 holds: the "spec" of discoverability is textual. Categories say where the book appears; keywords say for whom. Pick categories where a new book can rank; use keywords to cover searches the title misses. Each eBook, paperback and hardcover has its own 3+7 slots.

### 5. When to order the physical proof

Always, before releasing the paperback. The proof costs print + shipping and is the only way to validate margins, spine, colors (CMYK) and legibility at real size — the digital previewer lies about those details.

## The done criterion

The checklist ends with four boxes:

- eBook live on Amazon.
- Paperback live (if chosen).
- Author page created in Author Central.
- Metadata reviewed on the product page.

Translated to pipeline language: `VK-PIPELINE-STAGE: done` only when a reader can buy, open and recommend it. Until then it's a draft — however much the `git log` says "done".

## Chapter checklist

- [ ] Manuscript in `docs/livro/` reviewed with anchored images (ch. 15).
- [ ] eBook cover at 1600×2560, readable as thumbnail.
- [ ] Metadata (title, 4000-char description, 3 categories, 7×50 keywords) filled.
- [ ] Price simulated in both royalties; KDP Select decision taken.
- [ ] Paperback physical proof approved (if any).
- [ ] Author Central created and internal `VK-REVIEW-REQUEST` answered: the book is ready for a paying reader.
