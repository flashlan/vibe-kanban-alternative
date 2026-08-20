import type { ReactNode } from 'react';
import {
  CaretDownIcon,
  CaretRightIcon,
  ChatDotsIcon,
} from '@phosphor-icons/react';
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
    <div className={cn('text-sm leading-none', className)}>
      <button
        type="button"
        onClick={onToggle}
        className="flex h-4 items-center gap-0.5 self-start rounded-sm px-0.5 -mx-0.5 align-middle text-low transition-colors hover:bg-tertiary hover:text-normal"
      >
        {expanded ? (
          <CaretDownIcon className="size-3" weight="bold" />
        ) : (
          <CaretRightIcon className="size-3" weight="bold" />
        )}
        <ChatDotsIcon className="size-3 text-low" />
        <span className="text-[10px] font-medium uppercase tracking-wide">
          Thinking
        </span>
      </button>
      {expanded && (
        <div className="mt-1 max-h-64 overflow-y-auto border-l-2 border-border pl-3 text-sm leading-relaxed text-low">
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
