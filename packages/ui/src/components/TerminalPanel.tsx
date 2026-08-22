import type { ReactNode } from 'react';
import { PlusIcon, XIcon } from '@phosphor-icons/react';
import { cn } from '../lib/cn';

interface TerminalPanelTab {
  id: string;
  title: string;
  cwd?: string;
}

interface TerminalPanelProps {
  tabs: TerminalPanelTab[];
  activeTabId: string | null;
  renderTab: (tabId: string, isActive: boolean) => ReactNode;
  onSelectTab?: (tabId: string) => void;
  onCloseTab?: (tabId: string) => void;
  onNewTab?: () => void;
  onNewTuiTab?: () => void;
  /** Optional content rendered at the left end of the tab bar. */
  leading?: ReactNode;
}

function shortenCwd(cwd: string): string {
  if (!cwd) return '';
  const parts = cwd.split('/').filter(Boolean);
  if (parts.length <= 2) return '/' + parts.join('/');
  return '~/' + parts.slice(-2).join('/');
}

export function TerminalPanel({
  tabs,
  activeTabId,
  renderTab,
  onSelectTab,
  onCloseTab,
  onNewTab,
  onNewTuiTab,
  leading,
}: TerminalPanelProps) {
  // Render only the active terminal. Inactive ones stay alive in the provider
  // (their xterm instance and WebSocket persist), so switching back re-attaches
  // the existing element rather than spawning a new session.
  const activeTab = tabs.find((t) => t.id === activeTabId) ?? tabs[0] ?? null;

  return (
    <div className="flex flex-col h-full min-h-0 w-full">
      {tabs.length > 0 && (
        <div className="flex items-stretch shrink-0 border-b border-border bg-tertiary h-7">
          {leading && (
            <div className="flex items-center gap-1 pl-2 pr-2 py-0.5 border-r border-border shrink-0">
              {leading}
            </div>
          )}
          <div className="flex-1 min-w-0 overflow-x-auto">
            <div className="flex items-stretch gap-px h-full">
              {onNewTuiTab && (
                <button
                  type="button"
                  title="Open Cockpit TUI (vibe-tui)"
                  aria-label="Open Cockpit TUI"
                  className="flex items-center gap-1 px-2 text-xs font-medium text-amber-500 hover:text-amber-400 hover:bg-secondary border-r border-border shrink-0 h-full transition-colors"
                  onClick={onNewTuiTab}
                >
                  <span>⚡</span>
                  <span>TUI</span>
                </button>
              )}
              {tabs.map((tab) => {
                const isActive = tab.id === activeTab?.id;
                const displayTitle = tab.cwd
                  ? `${tab.title} — ${shortenCwd(tab.cwd)}`
                  : tab.title;
                return (
                  <div
                    key={tab.id}
                    role="button"
                    tabIndex={0}
                    className={cn(
                      'group flex items-center gap-1 pl-2 pr-1 py-0.5 text-xs cursor-pointer border-r border-border shrink-0 h-full',
                      isActive
                        ? 'bg-secondary text-normal'
                        : 'text-low hover:text-normal'
                    )}
                    onClick={() => onSelectTab?.(tab.id)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault();
                        onSelectTab?.(tab.id);
                      }
                    }}
                  >
                    <span className="truncate max-w-[140px]">{displayTitle}</span>
                    {onCloseTab && tabs.length > 1 && (
                      <button
                        type="button"
                        title="Close terminal"
                        aria-label="Close terminal"
                        className="opacity-50 hover:opacity-100 shrink-0"
                        onClick={(e) => {
                          e.stopPropagation();
                          onCloseTab(tab.id);
                        }}
                      >
                        <XIcon className="size-icon-xs" weight="bold" />
                      </button>
                    )}
                  </div>
                );
              })}
              {onNewTab && (
                <button
                  type="button"
                  title="New terminal"
                  aria-label="New terminal"
                  className="flex items-center px-1.5 text-low hover:text-normal shrink-0 h-full"
                  onClick={onNewTab}
                >
                  <PlusIcon className="size-icon-xs" weight="bold" />
                </button>
              )}
            </div>
          </div>
        </div>
      )}
      <div className="flex-1 min-h-0 w-full">
        {activeTab && renderTab(activeTab.id, true)}
      </div>
    </div>
  );
}
