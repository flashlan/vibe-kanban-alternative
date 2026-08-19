import type { ReactNode } from 'react';
import { CaretDownIcon, CaretRightIcon, ChatDotsIcon } from '@phosphor-icons/react';
import { cn } from '../lib/cn';

export interface ChatThinkingMessageRenderProps {
  content: string;
  workspaceId?: string;
  className?: string;
}

interface ChatThinkingMessageProps {
  content: string;
  className?: string;
  workspaceId?: string;
  expanded?: boolean;
  onToggle?: () => void;
  renderMarkdown: (props: ChatThinkingMessageRenderProps) => ReactNode;
}

/**
 * Collapsed-by-default thinking block. Only the model's reasoning header shows
 * ("Thinking") plus a caret — expand on demand. Keeps the chat history clean
 * when the model emits long `<think>` blocks (qwen3, gpt-oss, …).
 */
export function ChatThinkingMessage({
  content,
  className,
  workspaceId,
  expanded = false,
  onToggle,
  renderMarkdown,
}: ChatThinkingMessageProps) {
  return (
    <div className={cn('flex flex-col text-sm', className)}>
      <button
        type="button"
        onClick={onToggle}
        className="flex items-center gap-1 self-start rounded-sm px-1 -mx-1 text-low transition-colors hover:bg-tertiary hover:text-normal"
      >
        {expanded ? (
          <CaretDownIcon className="size-icon-xs" weight="bold" />
        ) : (
          <CaretRightIcon className="size-icon-xs" weight="bold" />
        )}
        <ChatDotsIcon className="size-icon-base text-low" />
        <span className="text-xs font-medium uppercase tracking-wide">
          Thinking
        </span>
        <span className="text-xs text-low/70">· {content.length} chars</span>
      </button>
      {expanded && (
        <div className="mt-1 max-h-64 overflow-y-auto border-l-2 border-border pl-3 text-sm text-low">
          {renderMarkdown({
            content,
            workspaceId,
            className: 'text-sm',
          })}
        </div>
      )}
    </div>
  );
}
