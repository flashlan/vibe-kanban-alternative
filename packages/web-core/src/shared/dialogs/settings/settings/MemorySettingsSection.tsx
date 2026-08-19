import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { makeRequest } from '@/shared/lib/remoteApi';
import { handleApiResponse } from '@/shared/lib/api';

export interface Mem0ProviderCfg {
  url: string;
  model: string;
  has_key: boolean;
}

export interface Mem0Config {
  ok: boolean;
  provider: string;
  graph_enabled: boolean;
  graph_url: string;
  collection: string;
  providers: Record<string, Mem0ProviderCfg>;
}

const PROVIDER_ORDER = ['groq', 'openrouter', 'llama', 'openai'] as const;

async function fetchMem0Config(): Promise<Mem0Config> {
  const response = await makeRequest('/api/usage/mem0-config', {
    method: 'GET',
    cache: 'no-store',
  });
  return handleApiResponse<Mem0Config>(response);
}

async function updateMem0Config(body: {
  provider?: string;
  graph_enabled?: boolean;
  providers?: Record<string, { url?: string; model?: string; key?: string }>;
}): Promise<Mem0Config> {
  const response = await makeRequest('/api/usage/mem0-config', {
    method: 'POST',
    body: JSON.stringify(body),
    cache: 'no-store',
  });
  return handleApiResponse<Mem0Config>(response);
}

interface ProviderDraft {
  url: string;
  model: string;
  key: string;
}

export function MemorySettingsSection() {
  const { t } = useTranslation('settings');
  const [config, setConfig] = useState<Mem0Config | null>(null);
  const [provider, setProvider] = useState('groq');
  const [graphEnabled, setGraphEnabled] = useState(true);
  const [drafts, setDrafts] = useState<Record<string, ProviderDraft>>({});
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  const load = useCallback(async () => {
    try {
      setError(null);
      const cfg = await fetchMem0Config();
      setConfig(cfg);
      setProvider(cfg.provider);
      setGraphEnabled(cfg.graph_enabled);
      const draftsInit: Record<string, ProviderDraft> = {};
      for (const p of PROVIDER_ORDER) {
        const pc = cfg.providers[p] ?? { url: '', model: '', has_key: false };
        draftsInit[p] = { url: pc.url, model: pc.model, key: '' };
      }
      setDrafts(draftsInit);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load memory config');
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const setDraft = (p: string, field: keyof ProviderDraft, value: string) => {
    setDrafts((prev) => ({
      ...prev,
      [p]: { ...(prev[p] ?? { url: '', model: '', key: '' }), [field]: value },
    }));
  };

  const handleSave = async () => {
    setBusy(true);
    setSaved(false);
    setError(null);
    try {
      const providers: Record<
        string,
        { url: string; model: string; key?: string }
      > = {};
      for (const p of PROVIDER_ORDER) {
        const d = drafts[p] ?? { url: '', model: '', key: '' };
        const patch: { url: string; model: string; key?: string } = {
          url: d.url,
          model: d.model,
        };
        // Only send a key when the user typed a new one — the server keeps the
        // existing masked key otherwise.
        if (d.key) patch.key = d.key;
        providers[p] = patch;
      }
      const cfg = await updateMem0Config({
        provider,
        graph_enabled: graphEnabled,
        providers,
      });
      setConfig(cfg);
      setSaved(true);
      // Re-run the re-extract check: toggling the graph or a provider can make
      // previously-stored memories extractable.
      window.setTimeout(() => setSaved(false), 2500);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to save memory config');
    } finally {
      setBusy(false);
    }
  };

  const providerLabel = (p: string): string => {
    switch (p) {
      case 'groq':
        return 'Groq';
      case 'openrouter':
        return 'OpenRouter';
      case 'llama':
        return 'Local llama (OpenAI /v1)';
      case 'openai':
        return 'OpenAI-compatible (DeepSeek, NVIDIA…)';
      default:
        return p;
    }
  };

  return (
    <div className="flex flex-col gap-6 overflow-y-auto p-4">
      {error && (
        <div className="rounded-sm border border-error/30 bg-error/10 px-3 py-2 text-sm text-error">
          {error}
        </div>
      )}
      {!config && !error && (
        <div className="text-sm text-low">
          {t('settings.memory.loading', 'Loading memory config…')}
        </div>
      )}

      {config && (
        <>
          <div className="rounded-sm border border-border bg-panel p-3">
            <div className="flex items-center justify-between">
              <div>
                <div className="text-sm font-medium text-high">
                  {t('settings.memory.graph', 'Memory graph')}
                </div>
                <div className="text-xs text-low">
                  {config.graph_url
                    ? `mem0 · ${config.collection} · ${config.graph_url}`
                    : t(
                        'settings.memory.graphDisabled',
                        'Graph service not configured'
                      )}
                </div>
              </div>
              <button
                type="button"
                role="switch"
                aria-checked={graphEnabled}
                onClick={() => setGraphEnabled((v) => !v)}
                className={`relative h-6 w-11 rounded-full transition-colors ${
                  graphEnabled ? 'bg-brand' : 'bg-secondary'
                }`}
              >
                <span
                  className={`absolute top-0.5 h-5 w-5 rounded-full bg-white transition-all ${
                    graphEnabled ? 'left-[22px]' : 'left-0.5'
                  }`}
                />
              </button>
            </div>
            <div className="mt-2 text-xs text-low">
              {t(
                'settings.memory.graphHint',
                'When off, memories are stored and searched as vectors only; no graph is built or persisted.'
              )}
            </div>
          </div>

          <div className="rounded-sm border border-border bg-panel p-3">
            <label className="mb-1 block text-xs text-low">
              {t('settings.memory.provider', 'Extraction provider (primary)')}
            </label>
            <select
              value={provider}
              onChange={(e) => setProvider(e.target.value)}
              className="w-full rounded-sm border border-border bg-secondary px-2 py-1.5 text-sm text-high focus:outline-none focus:ring-1 focus:ring-brand"
            >
              {PROVIDER_ORDER.map((p) => (
                <option key={p} value={p}>
                  {providerLabel(p)}
                </option>
              ))}
            </select>
            <div className="mt-1 text-xs text-low">
              {t(
                'settings.memory.providerHint',
                'Configured providers are tried in order when the primary is rate-limited or fails.'
              )}
            </div>
          </div>

          <div className="rounded-sm border border-border bg-panel p-3">
            <div className="mb-2 text-sm font-medium text-high">
              {t('settings.memory.providers', 'Providers & API keys')}
            </div>
            <div className="flex flex-col gap-4">
              {PROVIDER_ORDER.map((p) => {
                const d = drafts[p] ?? { url: '', model: '', key: '' };
                const hasKey = config.providers[p]?.has_key ?? false;
                return (
                  <div key={p} className="rounded-sm bg-secondary/40 p-2">
                    <div className="mb-1 text-xs font-medium text-normal">
                      {providerLabel(p)}
                    </div>
                    <div className="grid grid-cols-1 gap-2 sm:grid-cols-3">
                      <input
                        type="text"
                        value={d.url}
                        onChange={(e) => setDraft(p, 'url', e.target.value)}
                        placeholder={t('settings.memory.baseUrl', 'Base URL')}
                        className="min-w-0 rounded-sm border border-border bg-panel px-2 py-1.5 text-xs text-high placeholder:text-low focus:outline-none focus:ring-1 focus:ring-brand"
                      />
                      <input
                        type="text"
                        value={d.model}
                        onChange={(e) => setDraft(p, 'model', e.target.value)}
                        placeholder={t('settings.memory.model', 'Model')}
                        className="min-w-0 rounded-sm border border-border bg-panel px-2 py-1.5 text-xs text-high placeholder:text-low focus:outline-none focus:ring-1 focus:ring-brand"
                      />
                      <div className="relative">
                        <input
                          type="password"
                          value={d.key}
                          onChange={(e) => setDraft(p, 'key', e.target.value)}
                          placeholder={
                            hasKey
                              ? '•••••••••• (saved)'
                              : t('settings.memory.apiKey', 'API key')
                          }
                          autoComplete="off"
                          className="min-w-0 w-full rounded-sm border border-border bg-panel px-2 py-1.5 pr-9 text-xs text-high placeholder:text-low focus:outline-none focus:ring-1 focus:ring-brand"
                        />
                        {hasKey && (
                          <span
                            title={t('settings.memory.keySaved', 'Key saved')}
                            className="absolute right-2 top-1/2 -translate-y-1/2 text-[10px] text-success"
                          >
                            ●
                          </span>
                        )}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
            <div className="mt-2 text-xs text-low">
              {t(
                'settings.memory.keysHint',
                'API keys are stored in the mem0 container and never shown again — only a "saved" indicator is returned. Leave a key field empty to keep the existing one.'
              )}
            </div>
          </div>

          <div className="flex items-center gap-3">
            <button
              type="button"
              onClick={() => void handleSave()}
              disabled={busy}
              className="rounded-sm bg-brand px-4 py-1.5 text-sm font-medium text-white hover:bg-brand/90 disabled:opacity-50"
            >
              {busy
                ? t('settings.memory.saving', 'Saving…')
                : t('settings.memory.save', 'Save memory config')}
            </button>
            {saved && (
              <span className="text-sm text-success">
                {t('settings.memory.saved', 'Saved')}
              </span>
            )}
          </div>
        </>
      )}
    </div>
  );
}
