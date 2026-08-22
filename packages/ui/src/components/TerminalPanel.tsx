import { useCallback } from 'react';
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
  onClose?: () => void;
  title?: string;
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
  onClose,
  title,
  leading,
}: TerminalPanelProps) {
  const handleCloseTab = useCallback(
    (tabId: string) => {
      onCloseTab?.(tabId);
      // Closing the last remaining tab should also dismiss the whole panel,
      // so a single click closes the terminal instead of requiring a second
      // click on the panel's close button.
      if (tabs.length <= 1 && onClose) {
        onClose();
      }
    },
    [tabs.length, onCloseTab, onClose]
  );
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
              {tabs.map((tab) => {
                const isActive = tab.id === activeTab?.id;
                const displayTitle = tab.cwd
                  ? `${tab.title} — ${shortenCwd(tab.cwd)}`
                  : tab.title;
                const showClose =
                  onCloseTab &&
                  (tabs.length > 1 || (tabs.length === 1 && onClose));
                return (
                  <div
                    key={tab.id}
                    className={cn(
                      'group flex items-stretch border-r border-border shrink-0 h-full',
                      isActive ? 'bg-secondary' : 'bg-tertiary'
                    )}
                  >
                    <div
                      role="button"
                      tabIndex={0}
                      className={cn(
                        'flex items-center gap-1 pl-2 pr-1 py-0.5 text-xs cursor-pointer h-full',
                        isActive ? 'text-normal' : 'text-low hover:text-normal'
                      )}
                      onClick={() => onSelectTab?.(tab.id)}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') {
                          e.preventDefault();
                          onSelectTab?.(tab.id);
                        }
                      }}
                    >
                      <span className="truncate max-w-[140px]">
                        {displayTitle}
                      </span>
                    </div>
                    {showClose && (
                      <button
                        type="button"
                        title="Close terminal"
                        aria-label="Close terminal"
                        className="flex items-center justify-center w-6 px-1 text-low hover:text-normal hover:bg-secondary/70 shrink-0 h-full"
                        onClick={() => handleCloseTab(tab.id)}
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

          {/* Right side controls: [ Painel Terminal ] [ ⚡ TUI ] [ X ] */}
          <div className="flex items-stretch shrink-0 border-l border-border">
            {title && (
              <div className="flex items-center px-2.5 text-xs font-medium text-low select-none">
                {title}
              </div>
            )}
            {onNewTuiTab && (
              <button
                type="button"
                title="Open Cockpit TUI (vibe-tui)"
                aria-label="Open Cockpit TUI"
                className="flex items-center gap-1 px-2 text-xs font-medium text-amber-500 hover:text-amber-400 hover:bg-secondary border-l border-border h-full transition-colors"
                onClick={onNewTuiTab}
              >
                <span>⚡</span>
                <span>TUI</span>
              </button>
            )}
            {onClose && (
              <button
                type="button"
                title="Close panel"
                aria-label="Close panel"
                className="flex items-center justify-center px-2 text-low hover:text-normal hover:bg-secondary border-l border-border h-full transition-colors"
                onClick={onClose}
              >
                <XIcon className="size-icon-xs" weight="bold" />
              </button>
            )}
          </div>
        </div>
      )}
      <div className="flex-1 min-h-0 w-full">
        {activeTab && renderTab(activeTab.id, true)}
      </div>
    </div>
  );
}
