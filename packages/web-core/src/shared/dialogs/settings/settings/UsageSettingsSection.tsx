import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { makeRequest } from '@/shared/lib/remoteApi';
import { handleApiResponse } from '@/shared/lib/api';

export interface DailyAgentActivity {
  day: string;
  agent: string;
  executions: number;
  seconds: number;
}

export interface DailyIssueActivity {
  day: string;
  created: number;
  completed: number;
}

export interface ProjectProgress {
  project_id: string;
  name: string;
  total: number;
  done: number;
  open: number;
}

export interface Mem0TokenProvider {
  provider: string;
  model: string;
  prompt: number;
  completion: number;
}

export interface Mem0TokenDay {
  day: string;
  prompt: number;
  completion: number;
  total: number;
  providers: Mem0TokenProvider[];
}

export interface Mem0TokenUsage {
  days: Mem0TokenDay[];
  providers: Mem0TokenProvider[];
  total: number;
}

export interface Mem0RelevanceDay {
  day: string;
  calls: number;
  weak_calls: number;
  avg_top_score: number | null;
}

export interface Mem0RelevanceSummary {
  days: Mem0RelevanceDay[];
  total_calls: number;
  total_weak_calls: number;
}

export interface UsageSummary {
  activity: DailyAgentActivity[];
  issues: DailyIssueActivity[];
  projects: ProjectProgress[];
  total_executions: number;
  total_seconds: number;
  mem0_tokens: Mem0TokenUsage;
  mem0_relevance: Mem0RelevanceSummary;
}

export interface ReExtractResponse {
  ok: boolean;
  scanned: number;
  updated: number;
  entities: number;
  relations: number;
}

export async function fetchUsageSummary(): Promise<UsageSummary> {
  const response = await makeRequest('/api/usage/summary', {
    method: 'GET',
    cache: 'no-store',
  });
  return handleApiResponse<UsageSummary>(response);
}

export async function triggerReExtract(
  userId: string
): Promise<ReExtractResponse> {
  const response = await makeRequest(
    `/api/usage/re-extract?user_id=${encodeURIComponent(userId)}`,
    { method: 'POST', cache: 'no-store' }
  );
  return handleApiResponse<ReExtractResponse>(response);
}

function formatDuration(totalSeconds: number): string {
  const h = Math.floor(totalSeconds / 3600);
  const m = Math.floor((totalSeconds % 3600) / 60);
  if (h >= 1) return `${h}h ${m}m`;
  return `${m}m`;
}

/** GitHub-style activity level 0..4 based on how active a day was. */
function activityLevel(count: number, max: number): number {
  if (count === 0 || max === 0) return 0;
  const ratio = count / max;
  if (ratio <= 0.25) return 1;
  if (ratio <= 0.5) return 2;
  if (ratio <= 0.75) return 3;
  return 4;
}

const ACTIVITY_COLORS = [
  'bg-panel',
  'bg-brand/25',
  'bg-brand/50',
  'bg-brand/75',
  'bg-brand',
];

/** Stable color per extraction provider (llama / openrouter / groq / other). */
function providerColor(provider: string): string {
  const p = provider.toLowerCase();
  if (p.includes('groq')) return 'bg-red-500/80';
  if (p.includes('openrouter')) return 'bg-sky-500/80';
  if (p.includes('llama') || p.includes('ollama')) return 'bg-emerald-500/80';
  if (p.includes('openai')) return 'bg-amber-500/80';
  return 'bg-purple-500/80';
}

export function UsageSettingsSection() {
  const { t } = useTranslation('settings');
  const [summary, setSummary] = useState<UsageSummary | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [reExtractUser, setReExtractUser] = useState('');
  const [reExtractBusy, setReExtractBusy] = useState(false);
  const [reExtractResult, setReExtractResult] =
    useState<ReExtractResponse | null>(null);

  const load = useCallback(async () => {
    try {
      setError(null);
      setSummary(await fetchUsageSummary());
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load usage data');
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const handleReExtract = async () => {
    const userId = reExtractUser.trim() || 'default';
    setReExtractBusy(true);
    setReExtractResult(null);
    try {
      setError(null);
      const res = await triggerReExtract(userId);
      setReExtractResult(res);
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Re-extraction failed');
    } finally {
      setReExtractBusy(false);
    }
  };

  // Build last-30-days grid. `activity` maps "YYYY-MM-DD" -> executions.
  const activityByDay = new Map<string, number>();
  const byDayAgent = new Map<string, Map<string, number>>();
  let maxDaily = 0;
  for (const row of summary?.activity ?? []) {
    const execs = activityByDay.get(row.day) ?? 0;
    activityByDay.set(row.day, execs + row.executions);
    maxDaily = Math.max(maxDaily, execs + row.executions);
    const byAgent = byDayAgent.get(row.day) ?? new Map<string, number>();
    byAgent.set(row.agent, (byAgent.get(row.agent) ?? 0) + row.executions);
    byDayAgent.set(row.day, byAgent);
  }

  const days: { date: Date; label: string; level: number }[] = [];
  for (let i = 29; i >= 0; i--) {
    const d = new Date();
    d.setDate(d.getDate() - i);
    const label = d.toISOString().slice(0, 10);
    days.push({
      date: d,
      label,
      level: activityLevel(activityByDay.get(label) ?? 0, maxDaily),
    });
  }

  // Agent totals across the window.
  const agentTotals = new Map<string, number>();
  for (const row of summary?.activity ?? []) {
    agentTotals.set(
      row.agent,
      (agentTotals.get(row.agent) ?? 0) + row.executions
    );
  }
  const agents = [...agentTotals.entries()].sort((a, b) => b[1] - a[1]);

  const issueCreated = new Map<string, number>();
  const issueCompleted = new Map<string, number>();
  let maxIssues = 0;
  for (const row of summary?.issues ?? []) {
    issueCreated.set(row.day, row.created);
    issueCompleted.set(row.day, row.completed);
    maxIssues = Math.max(maxIssues, row.created, row.completed);
  }

  return (
    <div className="flex flex-col gap-6 overflow-y-auto p-4">
      {error && (
        <div className="rounded-sm bg-error/10 border border-error/30 px-3 py-2 text-sm text-error">
          {error}
        </div>
      )}

      {/* Totals */}
      <div className="grid grid-cols-3 gap-3">
        <div className="rounded-sm border border-border bg-panel p-3">
          <div className="text-2xl font-semibold text-high">
            {summary?.total_executions ?? '—'}
          </div>
          <div className="text-xs text-low">
            {t('settings.usage.executions', 'Executions (30d)')}
          </div>
        </div>
        <div className="rounded-sm border border-border bg-panel p-3">
          <div className="text-2xl font-semibold text-high">
            {summary ? formatDuration(summary.total_seconds) : '—'}
          </div>
          <div className="text-xs text-low">
            {t('settings.usage.time', 'Agent time (30d)')}
          </div>
        </div>
        <div className="rounded-sm border border-border bg-panel p-3">
          <div className="text-2xl font-semibold text-high">
            {summary
              ? `${summary.projects.reduce((n, p) => n + p.open, 0)}`
              : '—'}
          </div>
          <div className="text-xs text-low">
            {t('settings.usage.openIssues', 'Open issues')}
          </div>
        </div>
      </div>

      {/* GitHub-style activity squares */}
      <section>
        <h3 className="mb-2 text-sm font-medium text-high">
          {t('settings.usage.activity', 'Activity (last 30 days)')}
        </h3>
        <div className="rounded-sm border border-border bg-panel p-3">
          <div className="grid grid-cols-[repeat(30,minmax(0,1fr))] gap-1">
            {days.map((d) => (
              <div
                key={d.label}
                title={`${d.label}: ${activityByDay.get(d.label) ?? 0} executions`}
                className={`aspect-square w-full rounded-[3px] ${ACTIVITY_COLORS[d.level]}`}
              />
            ))}
          </div>
          <div className="mt-2 flex items-center gap-1 text-[10px] text-low">
            <span>{t('settings.usage.less', 'Less')}</span>
            {ACTIVITY_COLORS.map((c) => (
              <span key={c} className={`h-2.5 w-2.5 rounded-[3px] ${c}`} />
            ))}
            <span>{t('settings.usage.more', 'More')}</span>
          </div>
        </div>
      </section>

      {/* Bars per day by agent */}
      <section>
        <h3 className="mb-2 text-sm font-medium text-high">
          {t('settings.usage.byAgent', 'Executions per day by agent')}
        </h3>
        <div className="rounded-sm border border-border bg-panel p-3">
          {agents.length === 0 ? (
            <div className="text-sm text-low">
              {t('settings.usage.empty', 'No activity in the last 30 days.')}
            </div>
          ) : (
            <div className="flex flex-col gap-3">
              {agents.map(([agent, total]) => (
                <div key={agent} className="flex items-center gap-3">
                  <span className="w-28 shrink-0 truncate text-xs text-normal">
                    {agent}
                  </span>
                  <div className="flex h-14 flex-1 items-end gap-px">
                    {days.map((d) => {
                      const n = byDayAgent.get(d.label)?.get(agent) ?? 0;
                      const h =
                        maxDaily > 0
                          ? Math.max(2, Math.round((n / maxDaily) * 56))
                          : 2;
                      return (
                        <div
                          key={d.label}
                          title={`${d.label}: ${n}`}
                          className="w-full rounded-[2px] bg-brand/40"
                          style={{ height: n > 0 ? h : 2 }}
                        />
                      );
                    })}
                  </div>
                  <span className="w-12 shrink-0 text-right text-xs text-low">
                    {total}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>
      </section>

      {/* Issue progress per project */}
      <section>
        <h3 className="mb-2 text-sm font-medium text-high">
          {t('settings.usage.projects', 'Project & issue progress')}
        </h3>
        <div className="rounded-sm border border-border bg-panel p-3">
          {(summary?.projects ?? []).length === 0 ? (
            <div className="text-sm text-low">
              {t('settings.usage.noProjects', 'No projects yet.')}
            </div>
          ) : (
            <div className="flex flex-col gap-3">
              {(summary?.projects ?? []).map((p) => {
                const pct =
                  p.total > 0 ? Math.round((p.done / p.total) * 100) : 0;
                return (
                  <div key={p.project_id}>
                    <div className="mb-1 flex items-center justify-between text-xs">
                      <span className="truncate text-normal">{p.name}</span>
                      <span className="text-low">
                        {p.done}/{p.total} · {pct}%
                      </span>
                    </div>
                    <div className="h-2 overflow-hidden rounded-full bg-secondary">
                      <div
                        className="h-full rounded-full bg-success"
                        style={{ width: `${pct}%` }}
                      />
                    </div>
                    <div className="mt-1 flex gap-3 text-[10px] text-low">
                      <span>
                        {t('settings.usage.open', 'Open')}: {p.open}
                      </span>
                      <span>
                        {t('settings.usage.done', 'Done')}: {p.done}
                      </span>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </section>

      {/* mem0 extraction tokens — segmented bars per provider */}
      <section>
        <h3 className="mb-2 text-sm font-medium text-high">
          {t('settings.usage.mem0Tokens', 'mem0 extraction tokens')}
        </h3>
        <div className="rounded-sm border border-border bg-panel p-3">
          <div className="mb-2 flex items-center justify-between">
            <span className="text-2xl font-semibold text-high">
              {(summary?.mem0_tokens.total ?? 0).toLocaleString()}
            </span>
            <span className="text-xs text-low">
              {t('settings.usage.totalTokens', 'total tokens')}
            </span>
          </div>
          {(summary?.mem0_tokens.days ?? []).length === 0 ? (
            <div className="text-sm text-low">
              {t(
                'settings.usage.noTokenData',
                'No extraction tokens recorded yet (mem0 may be offline or the extraction LLM unused).'
              )}
            </div>
          ) : (
            <>
              <div className="flex flex-col gap-3">
                {(summary?.mem0_tokens.days ?? []).map((d) => {
                  return (
                    <div key={d.day} className="flex items-center gap-3">
                      <span className="w-24 shrink-0 text-xs text-low">
                        {d.day}
                      </span>
                      <div className="flex h-5 flex-1 overflow-hidden rounded-sm bg-secondary">
                        {d.providers.map((p) => {
                          const share =
                            d.total > 0
                              ? Math.max(
                                  1,
                                  Math.round(
                                    ((p.prompt + p.completion) / d.total) * 100
                                  )
                                )
                              : 0;
                          return (
                            <div
                              key={`${p.provider}|${p.model}`}
                              title={`${p.provider} · ${p.model}: ${(p.prompt + p.completion).toLocaleString()} tok`}
                              className={providerColor(p.provider)}
                              style={{ width: `${share}%` }}
                            />
                          );
                        })}
                      </div>
                      <span className="w-16 shrink-0 text-right text-xs text-low">
                        {d.total.toLocaleString()}
                      </span>
                    </div>
                  );
                })}
              </div>
              <div className="mt-3 flex flex-wrap gap-x-4 gap-y-1 border-t border-border pt-2">
                {(summary?.mem0_tokens.providers ?? []).map((p) => (
                  <span
                    key={`${p.provider}|${p.model}`}
                    className="flex items-center gap-1.5 text-xs text-low"
                  >
                    <span
                      className={`h-2.5 w-2.5 rounded-sm ${providerColor(p.provider)}`}
                    />
                    {p.provider} · {p.model} ·{' '}
                    {(p.prompt + p.completion).toLocaleString()}
                  </span>
                ))}
              </div>
            </>
          )}
        </div>
      </section>

      {/* mem0 recall relevance — memory_search top-score per day, since last
          server restart (in-memory ledger; see ADR-030). */}
      <section>
        <h3 className="mb-2 text-sm font-medium text-high">
          {t('settings.usage.mem0Relevance', 'mem0 recall relevance')}
        </h3>
        <div className="rounded-sm border border-border bg-panel p-3">
          <div className="mb-2 flex items-center justify-between">
            <span className="text-2xl font-semibold text-high">
              {(summary?.mem0_relevance.total_calls ?? 0).toLocaleString()}
            </span>
            <span className="text-xs text-low">
              {t('settings.usage.recallCalls', 'memory_search calls')}
              {(summary?.mem0_relevance.total_weak_calls ?? 0) > 0 && (
                <>
                  {' · '}
                  <span className="text-warning">
                    {(
                      summary?.mem0_relevance.total_weak_calls ?? 0
                    ).toLocaleString()}{' '}
                    {t('settings.usage.weak', 'weak')}
                  </span>
                </>
              )}
            </span>
          </div>
          {(summary?.mem0_relevance.days ?? []).length === 0 ? (
            <div className="text-sm text-low">
              {t(
                'settings.usage.noRelevanceData',
                'No recall calls recorded yet this server run (in-memory only — resets on restart).'
              )}
            </div>
          ) : (
            <div className="flex flex-col gap-3">
              {(summary?.mem0_relevance.days ?? []).map((d) => {
                const score = d.avg_top_score;
                const widthPct =
                  score != null ? Math.max(4, Math.round(score * 100)) : 0;
                const barColor =
                  score == null
                    ? 'bg-secondary'
                    : score < 0.3
                      ? 'bg-red-500/80'
                      : score < 0.6
                        ? 'bg-amber-500/80'
                        : 'bg-emerald-500/80';
                return (
                  <div key={d.day} className="flex items-center gap-3">
                    <span className="w-24 shrink-0 text-xs text-low">
                      {d.day}
                    </span>
                    <div className="h-5 flex-1 overflow-hidden rounded-sm bg-secondary">
                      <div
                        title={
                          score != null
                            ? `avg top score ${score.toFixed(3)} · ${d.calls} call(s), ${d.weak_calls} weak`
                            : `no scored hits · ${d.calls} call(s), ${d.weak_calls} weak`
                        }
                        className={`h-full ${barColor}`}
                        style={{ width: `${widthPct}%` }}
                      />
                    </div>
                    <span className="w-20 shrink-0 text-right text-xs text-low">
                      {score != null ? score.toFixed(2) : '—'}
                    </span>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </section>

      {/* Re-extract graph entities (for memories saved before an LLM was configured) */}
      <section>
        <h3 className="mb-2 text-sm font-medium text-high">
          {t('settings.usage.reExtract', 'Re-extract graph entities')}
        </h3>
        <div className="rounded-sm border border-border bg-panel p-3">
          <p className="mb-2 text-xs text-low">
            {t(
              'settings.usage.reExtractHint',
              'Run graph extraction for memories stored before an extraction LLM was configured. Enter the repository slug (memories user_id), then trigger.'
            )}
          </p>
          <div className="flex gap-2">
            <input
              type="text"
              value={reExtractUser}
              onChange={(e) => setReExtractUser(e.target.value)}
              placeholder="repo-slug (e.g. vibe-kanban-alternative)"
              className="min-w-0 flex-1 rounded-sm border border-border bg-secondary px-2 py-1.5 text-sm text-high placeholder:text-low focus:outline-none focus:ring-1 focus:ring-brand"
            />
            <button
              type="button"
              onClick={() => void handleReExtract()}
              disabled={reExtractBusy}
              className="shrink-0 rounded-sm bg-panel px-3 py-1.5 text-sm text-normal ring-1 ring-border hover:text-high disabled:opacity-50"
            >
              {reExtractBusy
                ? t('settings.usage.reExtracting', 'Extracting…')
                : t('settings.usage.reExtractBtn', 'Re-extract')}
            </button>
          </div>
          {reExtractResult && (
            <div className="mt-2 text-xs text-low">
              {t('settings.usage.reExtractResult', 'Scanned', {
                count: reExtractResult.scanned,
              })}{' '}
              · {reExtractResult.entities}{' '}
              {t('settings.usage.reExtractEntities', 'entities')} ·{' '}
              {reExtractResult.relations}{' '}
              {t('settings.usage.reExtractRelations', 'relations')} extracted
            </div>
          )}
        </div>
      </section>

      <div className="flex justify-end">
        <button
          type="button"
          onClick={() => void load()}
          className="rounded-sm bg-panel px-3 py-1.5 text-sm text-normal ring-1 ring-border hover:text-high"
        >
          {t('settings.usage.refresh', 'Refresh')}
        </button>
      </div>
    </div>
  );
}
