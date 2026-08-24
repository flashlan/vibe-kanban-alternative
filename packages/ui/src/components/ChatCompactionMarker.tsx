import type { ReactNode } from 'react';
import {
  CaretDownIcon,
  CaretRightIcon,
  ScissorsIcon,
  DatabaseIcon,
} from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import { cn } from '../lib/cn';

export interface ChatCompactionMarkerRenderProps {
  content: string;
  workspaceId?: string;
  className?: string;
}

export interface ChatCompactionMarkerProps {
  content: string;
  previousTokens?: number | null;
  compactedTokens?: number | null;
  mem0Synced?: boolean;
  className?: string;
  workspaceId?: string;
  expanded?: boolean;
  onToggle?: () => void;
  renderMarkdown: (props: ChatCompactionMarkerRenderProps) => ReactNode;
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) {
    const m = n / 1_000_000;
    return m % 1 === 0 ? `${m}M` : `${m.toFixed(1)}M`;
  }
  if (n >= 1_000) return `${Math.round(n / 1_000)}K`;
  return n.toString();
}

/**
 * Collapsed-by-default compaction block, matching the style of ChatThinkingMessage.
 * Displays a clean inline row with caret, scissors icon, title and token reduction.
 * Expanding reveals the markdown summary with left-border styling.
 */
export function ChatCompactionMarker({
  content,
  previousTokens,
  compactedTokens,
  mem0Synced = true,
  className,
  workspaceId,
  expanded = false,
  onToggle,
  renderMarkdown,
}: ChatCompactionMarkerProps) {
  const { t } = useTranslation('common');

  const tokenText =
    previousTokens && compactedTokens
      ? `${formatTokens(previousTokens)} → ${formatTokens(compactedTokens)}`
      : previousTokens
        ? `${formatTokens(previousTokens)}`
        : null;

  return (
    <div className={cn('text-sm leading-none py-1', className)}>
      <button
        type="button"
        onClick={onToggle}
        className="flex h-5 items-center gap-1.5 self-start rounded-sm px-1 -mx-1 align-middle text-low transition-colors hover:bg-tertiary hover:text-normal cursor-pointer"
      >
        {expanded ? (
          <CaretDownIcon className="size-3" weight="bold" />
        ) : (
          <CaretRightIcon className="size-3" weight="bold" />
        )}
        <ScissorsIcon className="size-3 text-warning" weight="bold" />
        <span className="text-[10px] font-medium uppercase tracking-wide">
          {t('conversation.compaction.title', 'Compacted Context')}
        </span>
        {tokenText && (
          <span className="text-[10px] text-low/70 font-mono">
            ({tokenText})
          </span>
        )}
        {mem0Synced && (
          <span
            className="inline-flex items-center gap-0.5 text-[10px] text-accent font-normal"
            title={t(
              'conversation.compaction.mem0Tooltip',
              'Memory synced to Mem0. Model can recall details via memory_search.'
            )}
          >
            <DatabaseIcon className="size-2.5 text-accent" />
            <span>Mem0</span>
          </span>
        )}
      </button>

      {expanded && (
        <div className="mt-1.5 max-h-72 overflow-y-auto border-l-2 border-border/80 pl-3 text-sm leading-relaxed text-low">
          {renderMarkdown({
            content,
            workspaceId,
            className: 'text-sm space-y-1.5',
          })}
        </div>
      )}
    </div>
  );
}
