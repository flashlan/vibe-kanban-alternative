'use client';

import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from './KeyboardDialog';

/** Lifecycle metrics for a single card (see backend `IssueMetrics`). */
export interface IssueMetrics {
  issue_id: string;
  created_at: string;
  completed_at: string | null;
  total_seconds: number;
  cycles: number;
  rework_count: number;
  status_changes: number;
  current_status_name: string;
}

function formatDuration(totalSeconds: number): string {
  const s = Math.max(0, Math.floor(totalSeconds));
  const d = Math.floor(s / 86400);
  const h = Math.floor((s % 86400) / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (d > 0) return `${d}d ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

export interface CardInfoDialogProps {
  issueId: string;
  issueTitle?: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Async loader provided by the host so this component stays UI-only. */
  loadMetrics: (issueId: string) => Promise<IssueMetrics>;
}

export function CardInfoDialog({
  issueId,
  issueTitle,
  open,
  onOpenChange,
  loadMetrics,
}: CardInfoDialogProps) {
  const { t } = useTranslation('common');
  const [metrics, setMetrics] = useState<IssueMetrics | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!open || !issueId) return;
    let cancelled = false;
    setLoading(true);
    setError(null);
    setMetrics(null);
    loadMetrics(issueId)
      .then((m) => {
        if (!cancelled) setMetrics(m);
      })
      .catch((e) => {
        if (!cancelled)
          setError(e instanceof Error ? e.message : 'Failed to load card info');
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [open, issueId, loadMetrics]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>
            {t('kanban.cardInfo.title', 'Card info')}
            {issueTitle ? ` — ${issueTitle}` : ''}
          </DialogTitle>
        </DialogHeader>

        {loading && (
          <div className="p-base text-sm text-low">
            {t('common:loading', 'Loading…')}
          </div>
        )}
        {error && <div className="p-base text-sm text-error">{error}</div>}
        {metrics && (
          <div className="flex flex-col gap-1 p-base">
            <MetricRow
              label={t('kanban.cardInfo.totalTime', 'Total time')}
              value={formatDuration(metrics.total_seconds)}
            />
            <MetricRow
              label={t('kanban.cardInfo.cycles', 'Review cycles')}
              value={String(metrics.cycles)}
            />
            <MetricRow
              label={t('kanban.cardInfo.rework', 'Rework')}
              value={String(metrics.rework_count)}
            />
            <MetricRow
              label={t('kanban.cardInfo.statusChanges', 'Status changes')}
              value={String(metrics.status_changes)}
            />
            <MetricRow
              label={t('kanban.cardInfo.currentStatus', 'Current status')}
              value={metrics.current_status_name || '—'}
            />
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}

function MetricRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-2 border-b border-border pb-1 last:border-0">
      <span className="text-sm text-low">{label}</span>
      <span className="text-base font-medium text-high">{value}</span>
    </div>
  );
}
