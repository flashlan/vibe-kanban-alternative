import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { cn } from '../lib/cn';
import { Tooltip } from './Tooltip';

export interface ContextUsageInfo {
  total_tokens: number;
  model_context_window: number;
  input_tokens?: number | null;
  output_tokens?: number | null;
  cache_read_tokens?: number | null;
  cache_creation_tokens?: number | null;
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

function formatTokens(n: number) {
  if (n >= 1_000_000) {
    const m = n / 1_000_000;
    return m % 1 === 0 ? `${m}M` : `${m.toFixed(1)}M`;
  }
  if (n >= 1_000) return `${Math.round(n / 1_000)}K`;
  return n.toString();
}

export interface ContextUsageGaugeProps {
  tokenUsageInfo?: ContextUsageInfo | null;
  className?: string;
}

export function ContextUsageGauge({
  tokenUsageInfo,
  className,
}: ContextUsageGaugeProps) {
  const { t } = useTranslation('common');
  const { percentage, formattedUsed, formattedTotal, status, cacheInfo } =
    useMemo(() => {
      if (!tokenUsageInfo || tokenUsageInfo.model_context_window === 0) {
        return {
          percentage: 0,
          formattedUsed: '0',
          formattedTotal: '0',
          status: 'empty' as const,
          cacheInfo: null,
        };
      }

      let effectiveTotalTokens = tokenUsageInfo.total_tokens;
      let effectiveCacheRead = tokenUsageInfo.cache_read_tokens ?? 0;
      let effectiveInputTokens = tokenUsageInfo.input_tokens ?? 0;
      const effectiveContextWindow = tokenUsageInfo.model_context_window;

      // Guard against historical logs containing cumulative session token counts
      // rather than single-turn active context.
      if (
        effectiveContextWindow > 0 &&
        effectiveTotalTokens > effectiveContextWindow
      ) {
        effectiveTotalTokens = Math.min(
          effectiveTotalTokens,
          effectiveContextWindow
        );
        effectiveCacheRead = Math.min(
          effectiveCacheRead,
          effectiveContextWindow
        );
        effectiveInputTokens = Math.min(
          effectiveInputTokens,
          effectiveContextWindow
        );
      }

      const pct = Math.min(
        100,
        (effectiveTotalTokens / effectiveContextWindow) * 100
      );

      let statusValue: 'low' | 'medium' | 'high' | 'critical' | 'empty';
      if (pct < 50) statusValue = 'low';
      else if (pct < 75) statusValue = 'medium';
      else if (pct < 90) statusValue = 'high';
      else statusValue = 'critical';

      const cacheCreation = tokenUsageInfo.cache_creation_tokens ?? 0;
      const totalInput = effectiveInputTokens + effectiveCacheRead + cacheCreation;

      let cacheHitPct: number | null = null;
      if (totalInput > 0 && effectiveCacheRead > 0) {
        cacheHitPct = Math.round((effectiveCacheRead / totalInput) * 100);
      }

      return {
        percentage: pct,
        formattedUsed: formatTokens(effectiveTotalTokens),
        formattedTotal: formatTokens(effectiveContextWindow),
        status: statusValue,
        cacheInfo:
          cacheHitPct !== null
            ? {
                hitPct: cacheHitPct,
                cached: formatTokens(effectiveCacheRead),
              }
            : null,
      };
    }, [tokenUsageInfo]);

  const progress = clamp(percentage / 100, 0, 1);

  const tooltip = useMemo(() => {
    if (status === 'empty') return t('contextUsage.emptyTooltip');
    const baseText = t('contextUsage.tooltip', {
      percentage: Math.round(percentage),
      used: formattedUsed,
      total: formattedTotal,
    });
    if (cacheInfo) {
      return `${baseText} · Cache Hit: ${cacheInfo.hitPct}% (${cacheInfo.cached} cached)`;
    }
    return baseText;
  }, [status, percentage, formattedUsed, formattedTotal, cacheInfo, t]);

  const progressColor =
    status === 'empty'
      ? 'text-low/40'
      : status === 'critical'
        ? 'text-error'
        : status === 'high'
          ? 'text-brand-secondary'
          : status === 'medium'
            ? 'text-normal'
            : 'text-low';

  const radius = 8;
  const strokeWidth = 2;
  const circumference = 2 * Math.PI * radius;
  const dashOffset = circumference * (1 - progress);

  return (
    <Tooltip content={tooltip} side="bottom">
      <div
        className={cn(
          'flex items-center justify-center rounded-sm p-half',
          'hover:bg-panel transition-colors cursor-help',
          className
        )}
        aria-label={
          status === 'empty'
            ? t('contextUsage.label')
            : t('contextUsage.ariaLabel', {
                percentage: Math.round(percentage),
              })
        }
        role="img"
      >
        <svg
          viewBox="0 0 20 20"
          className="size-icon-base -rotate-90"
          aria-hidden="true"
        >
          <circle
            cx="10"
            cy="10"
            r={radius}
            fill="none"
            stroke="currentColor"
            strokeWidth={strokeWidth}
            className="text-border/60"
          />
          <circle
            cx="10"
            cy="10"
            r={radius}
            fill="none"
            stroke="currentColor"
            strokeWidth={strokeWidth}
            strokeLinecap="round"
            strokeDasharray={`${circumference} ${circumference}`}
            strokeDashoffset={dashOffset}
            className={cn(
              progressColor,
              'transition-all duration-500 ease-out'
            )}
          />
        </svg>
      </div>
    </Tooltip>
  );
}
