import { type Pipeline, type PipelineStep } from 'shared/types';

/**
 * Delimiters bounding the generated `## Pipeline` block inside a card
 * description. They let us append/replace the block idempotently without
 * clobbering the operator's own prose.
 */
export const PIPELINE_START = '<!-- vk:pipeline:start -->';
export const PIPELINE_END = '<!-- vk:pipeline:end -->';

/**
 * LEGACY instruction line that used to lead the full stage list embedded
 * directly in the description. No longer emitted by `composePipelineBlock`
 * (see `PIPELINE_POINTER_INSTRUCTION`) — kept only so `extractManualLines`
 * still recognises it as a generated line (not manual text) when recomposing
 * a pre-migration card that still has the old full block in its description.
 */
const ORDER_INSTRUCTION =
  'Execute these stages in the order listed. Do not add, skip, or reorder stages. ' +
  'As you begin each numbered stage below, call the `report_pipeline_stage` MCP tool with ' +
  "that stage's number, AND output a line exactly `VK-PIPELINE-STAGE: N` (N = the number of " +
  'the stage you are starting) so pipeline progress can be tracked.';

/**
 * Compact pointer emitted in place of the full stage list. The heavy content
 * (the ordered stage list, one report reminder per stage) now lives behind
 * the `get_pipeline` MCP tool — this line only needs to be salient enough,
 * in the card's own description (the highest-priority channel: the active
 * turn's user message), to make the agent call that tool before starting
 * work. See `docs/ADR` follow-up for the full design rationale.
 */
const PIPELINE_POINTER_INSTRUCTION =
  'This card has pipeline stages defined via `get_pipeline` — call that MCP tool ' +
  'BEFORE any code edits, execute the returned stages in order (do not add, skip, ' +
  "or reorder), and report each one via `report_pipeline_stage` as instructed in the tool's " +
  'response.';

/** Matches an executor-pin line in any pinned form (any agent name). */
const EXECUTOR_LINE_RE =
  /^- Run this card with the \*\*.+\*\* execution agent:/;
/** Matches a generated numbered stage line, capturing its remainder. */
const NUMBERED_LINE_RE = /^\d+\.\s+(.*)$/;
/** Matches the `## Pipeline` / `## Pipeline: <name(s)>` heading. */
const HEADING_RE = /^## Pipeline\b/;

/**
 * Compose the directive line that pins the card to a specific execution agent.
 * The orchestrator reads this from the description and passes the named agent as
 * the `executor` when it starts the workspace. Returns an empty string when no
 * agent is selected (the orchestrator then uses its default). `executor` is a
 * `BaseCodingAgent` key (e.g. `CODEX`, `CLAUDE_CODE`).
 */
export function composeExecutorLine(
  executor: string | null | undefined
): string {
  const trimmed = (executor ?? '').trim();
  if (!trimmed) return '';
  return `- Run this card with the **${trimmed}** execution agent: pass \`executor: "${trimmed}"\` when starting the workspace.`;
}

/** Normalize the `pipelines` argument to an array, dropping nulls. */
function normalizePipelines(
  pipelines: readonly Pipeline[] | Pipeline | null | undefined
): Pipeline[] {
  if (!pipelines) return [];
  const arr = Array.isArray(pipelines) ? pipelines : [pipelines];
  return arr.filter((p): p is Pipeline => p != null);
}

/**
 * Merge the FULL stage sequences of the given pipelines into one canonical
 * order via a stable topological sort (Kahn's algorithm), so a stage shared
 * by two pipelines appears once and the relative order declared by every
 * pipeline is respected. Tiebreak = first-seen index across pipelines in
 * selection order, which makes the result deterministic.
 */
export function canonicalStageOrder(
  pipelines: readonly Pipeline[]
): PipelineStep[] {
  const byId = new Map<string, PipelineStep>();
  const firstSeen = new Map<string, number>();
  let seenIdx = 0;
  for (const p of pipelines) {
    for (const s of p.stages) {
      if (!byId.has(s.id)) byId.set(s.id, s);
      if (!firstSeen.has(s.id)) firstSeen.set(s.id, seenIdx++);
    }
  }

  const indegree = new Map<string, number>();
  const adjacency = new Map<string, Set<string>>();
  for (const id of byId.keys()) {
    indegree.set(id, 0);
    adjacency.set(id, new Set());
  }
  for (const p of pipelines) {
    for (let i = 0; i < p.stages.length - 1; i++) {
      const from = p.stages[i].id;
      const to = p.stages[i + 1].id;
      const adj = adjacency.get(from);
      if (adj && !adj.has(to)) {
        adj.add(to);
        indegree.set(to, (indegree.get(to) ?? 0) + 1);
      }
    }
  }

  const remaining = new Set(byId.keys());
  let ready: string[] = [...remaining].filter(
    (id) => (indegree.get(id) ?? 0) === 0
  );
  const ordered: string[] = [];

  const takeSmallestFirstSeen = (ids: string[]): string => {
    let best = ids[0];
    let bestSeen = firstSeen.get(best) ?? Number.POSITIVE_INFINITY;
    for (const id of ids) {
      const seen = firstSeen.get(id) ?? Number.POSITIVE_INFINITY;
      if (seen < bestSeen) {
        best = id;
        bestSeen = seen;
      }
    }
    return best;
  };

  while (ready.length > 0) {
    const next = takeSmallestFirstSeen(ready);
    ready = ready.filter((id) => id !== next);
    ordered.push(next);
    remaining.delete(next);
    for (const succ of adjacency.get(next) ?? []) {
      const deg = (indegree.get(succ) ?? 0) - 1;
      indegree.set(succ, deg);
      if (deg === 0) ready.push(succ);
    }
  }

  // Safe fallback: if a cycle ever left nodes unprocessed (shouldn't happen
  // with well-formed pipeline files), append the remainder in first-seen
  // order rather than silently dropping stages.
  if (remaining.size > 0) {
    [...remaining]
      .sort((a, b) => (firstSeen.get(a) ?? 0) - (firstSeen.get(b) ?? 0))
      .forEach((id) => ordered.push(id));
  }

  return ordered.map((id) => byId.get(id)!);
}

/**
 * The canonical stage order for `pipelines`, filtered to the enabled union.
 * Shared by `composePipelineBlock` and `PipelineSection` so both use the
 * exact same ordering logic.
 */
export function orderedEnabledStages(
  pipelines: readonly Pipeline[],
  enabledIds: ReadonlySet<string> | readonly string[]
): PipelineStep[] {
  const enabled = enabledIds instanceof Set ? enabledIds : new Set(enabledIds);
  return canonicalStageOrder(pipelines).filter((s) => enabled.has(s.id));
}

/**
 * Extract the operator's manual/extra lines from a previously composed block:
 * everything that isn't a recognised generated line (heading, order
 * instruction, executor-pin line in any pinned form, or a numbered stage line
 * whose remainder exactly matches a fragment in `knownStageFragments`).
 *
 * `knownStageFragments` should be the FULL catalog of every available
 * pipeline's stage fragments (not just the currently selected/enabled ones),
 * so that deselecting a stage or an entire pipeline still recognises its old
 * generated line and drops it, instead of stranding it as "manual".
 */
export function extractManualLines(
  block: string,
  knownStageFragments: ReadonlySet<string>
): string[] {
  const start = block.indexOf(PIPELINE_START);
  const endDelim = block.indexOf(PIPELINE_END);
  const inner =
    start === -1
      ? block
      : block.slice(
          start + PIPELINE_START.length,
          endDelim === -1 ? undefined : endDelim
        );

  const manual: string[] = [];
  for (const rawLine of inner.split('\n')) {
    const line = rawLine.replace(/\s+$/, '');
    if (line.trim().length === 0) continue;
    if (HEADING_RE.test(line)) continue;
    if (line === ORDER_INSTRUCTION) continue;
    if (line === PIPELINE_POINTER_INSTRUCTION) continue;
    if (EXECUTOR_LINE_RE.test(line)) continue;
    const numbered = line.match(NUMBERED_LINE_RE);
    if (numbered && knownStageFragments.has(numbered[1])) continue;
    manual.push(line);
  }
  return manual;
}

/**
 * Compose the delimited `## Pipeline` markdown block for the chosen
 * pipeline(s). Enabled stages across all selected pipelines are still merged
 * into a single **canonical, deduped order** (see `canonicalStageOrder`) —
 * that computation is what decides *whether* there are any stages at all —
 * but the block no longer embeds the resulting stage list as text. Instead
 * it emits a compact pointer (`PIPELINE_POINTER_INSTRUCTION`) telling the
 * execution agent to call the `get_pipeline` MCP tool, which resolves the
 * same stage list server-side (`GET /api/workspaces/{id}/pipeline/resolve`,
 * reading `extension_metadata`, not this description text). This keeps the
 * card description small and the heavy, repeated instruction text out of
 * every model call, while the pointer itself stays inline in the
 * highest-priority channel (the active turn's own message) so the agent
 * reliably sees it. An optional pinned execution agent's directive line
 * still leads the block's content, since the orchestrator parses it
 * directly from the description (not via MCP).
 *
 * When `pipelines` is `null`/empty (the "None" option) stages are ignored
 * entirely: the block contains only the executor-pin line (if an agent is
 * pinned) and/or the operator's custom text. Returns an empty string when
 * there is nothing to emit, so callers can treat "no pipeline" as falsy.
 *
 * `options.previousBlock`, when given, is scanned for manual lines (any line
 * that isn't a recognised generated line) which are preserved verbatim after
 * the generated content — this makes recompose non-destructive, including
 * for a pre-migration card whose `previousBlock` still has the OLD full
 * stage list embedded (its numbered lines and `ORDER_INSTRUCTION` are still
 * recognised and dropped, not preserved as manual, by `extractManualLines`).
 * Pass `options.knownStageFragments` with the full catalog of every
 * available pipeline's stage fragments so deselected stages/pipelines are
 * correctly recognised and dropped rather than preserved as manual text.
 */
export function composePipelineBlock(
  pipelines: readonly Pipeline[] | Pipeline | null,
  enabledIds: ReadonlySet<string> | readonly string[],
  customText: string,
  executor?: string | null,
  options?: {
    previousBlock?: string | null;
    knownStageFragments?: ReadonlySet<string>;
  }
): string {
  const list = normalizePipelines(pipelines);
  const enabled = enabledIds instanceof Set ? enabledIds : new Set(enabledIds);
  const executorLine = composeExecutorLine(executor);
  const trimmedCustom = customText.trim();

  const stages = orderedEnabledStages(list, enabled);

  const knownFragments =
    options?.knownStageFragments ??
    new Set(list.flatMap((p) => p.stages.map((s) => s.prompt_fragment)));

  const manualLines = options?.previousBlock
    ? extractManualLines(options.previousBlock, knownFragments)
    : [];
  // Avoid duplicating a manual line that's also present verbatim in customText.
  const customLineSet = new Set(
    trimmedCustom.length > 0
      ? trimmedCustom.split('\n').map((l) => l.trim())
      : []
  );
  const extraManualLines = manualLines.filter(
    (l) => !customLineSet.has(l.trim())
  );

  if (
    stages.length === 0 &&
    !executorLine &&
    trimmedCustom.length === 0 &&
    extraManualLines.length === 0
  ) {
    return '';
  }

  const heading =
    list.length === 0
      ? '## Pipeline'
      : `## Pipeline: ${list.map((p) => p.name).join(' + ')}`;
  const lines: string[] = [heading, ''];

  if (stages.length > 0) {
    lines.push(PIPELINE_POINTER_INSTRUCTION);
    if (executorLine) lines.push('');
  }
  // The execution-agent directive leads so the orchestrator sees it first.
  if (executorLine) {
    lines.push(executorLine);
  }

  const trailing = [...extraManualLines];
  if (trimmedCustom.length > 0) trailing.push(trimmedCustom);
  if (trailing.length > 0) {
    if (stages.length > 0 || executorLine) lines.push('');
    lines.push(trailing.join('\n'));
  }

  return `${PIPELINE_START}\n${lines.join('\n')}\n${PIPELINE_END}`;
}

/**
 * Strip any previously-appended pipeline block from a description, returning the
 * remaining prose with trailing whitespace trimmed.
 */
export function stripPipelineBlock(description: string): string {
  const start = description.indexOf(PIPELINE_START);
  if (start === -1) return description;
  const endIdx = description.indexOf(PIPELINE_END, start);
  const after =
    endIdx === -1 ? '' : description.slice(endIdx + PIPELINE_END.length);
  return (description.slice(0, start) + after).replace(/\s+$/, '');
}

/**
 * Extract the delimited `## Pipeline` block (including delimiters) from a
 * card description, for seeding the edit-mode `PipelineSection`. Returns an
 * empty string when the description has no pipeline block.
 */
export function extractPipelineBlock(
  description: string | null | undefined
): string {
  if (!description) return '';
  const start = description.indexOf(PIPELINE_START);
  if (start === -1) return '';
  const endIdx = description.indexOf(PIPELINE_END, start);
  if (endIdx === -1) return description.slice(start);
  return description.slice(start, endIdx + PIPELINE_END.length);
}

/**
 * Append (or replace) the pipeline block at the end of a description. Idempotent:
 * any existing delimited block is removed first, so re-appending never stacks
 * duplicates. Passing an empty block strips the existing one.
 */
export function appendPipelineToDescription(
  description: string | null,
  block: string
): string {
  const base = stripPipelineBlock(description ?? '');
  if (!block) return base;
  return base.length > 0 ? `${base}\n\n${block}` : block;
}

/** One numbered stage as parsed from a card's `## Pipeline` block. */
export interface PipelineStage {
  /** 1-based position in the parsed list (re-sequenced; not the source card's stage id). */
  index: number;
  /** Short display label derived from the stage's prompt fragment. */
  label: string;
}

const MAX_STAGE_LABEL_LEN = 60;
const NUMBERED_LIST_ITEM = /^\s*\d+\.\s+(.*)$/;

/** Shorten a stage's prompt fragment to a compact display label. */
function shortenStageLabel(text: string): string {
  const trimmed = text.trim();
  const sentenceEnd = trimmed.indexOf('. ');
  const cut = sentenceEnd === -1 ? trimmed : trimmed.slice(0, sentenceEnd);
  if (cut.length <= MAX_STAGE_LABEL_LEN) return cut;
  return `${cut.slice(0, MAX_STAGE_LABEL_LEN).trimEnd()}…`;
}

/**
 * Slice out the text of the generated `## Pipeline` block from a description,
 * for `parsePipelineStages` to walk. Prefers the `PIPELINE_START`/`PIPELINE_END`
 * delimiters; falls back to the `## Pipeline` heading through end-of-text when
 * delimiters are absent (e.g. an older card, or one whose block was hand-edited
 * past the delimiters).
 */
function extractPipelineBlockText(description: string): string {
  const start = description.indexOf(PIPELINE_START);
  if (start !== -1) {
    const end = description.indexOf(PIPELINE_END, start);
    return end === -1
      ? description.slice(start + PIPELINE_START.length)
      : description.slice(start + PIPELINE_START.length, end);
  }
  const headingIdx = description.indexOf('## Pipeline');
  return headingIdx === -1 ? '' : description.slice(headingIdx);
}

/**
 * Parse the numbered stage list (M = length, plus a short label per stage)
 * out of a card's `## Pipeline` block, for rendering live "stage N of M"
 * progress. Always re-derived from the live description so it stays
 * consistent with whatever the operator most recently edited.
 *
 * Only counts the *contiguous* run of numbered list items: once the list has
 * started, the first line that isn't `N. ...` (blank line or prose) ends it.
 * This keeps numbered text the operator appends after a blank line (their own
 * notes, say) from being miscounted as pipeline stages. Returns `[]` when
 * there is no pipeline block, or the block has no numbered list.
 */
export function parsePipelineStages(
  description: string | null | undefined
): PipelineStage[] {
  const block = extractPipelineBlockText(description ?? '');
  if (!block) return [];

  const stages: PipelineStage[] = [];
  for (const line of block.split('\n')) {
    const match = NUMBERED_LIST_ITEM.exec(line);
    if (match) {
      stages.push({
        index: stages.length + 1,
        label: shortenStageLabel(match[1]),
      });
      continue;
    }
    if (stages.length > 0) break;
  }

  return stages;
}
