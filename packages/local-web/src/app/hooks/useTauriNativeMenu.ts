import { useEffect, useRef } from 'react';
import { isTauriApp } from '@/shared/lib/platform';
import { Actions } from '@/shared/actions';
import { useActions } from '@/shared/hooks/useActions';

type NativeMenuAction = 'settings' | 'project-settings' | 'new-issue';

const ACTIONS: Record<
  NativeMenuAction,
  (typeof Actions)[keyof typeof Actions]
> = {
  settings: Actions.Settings,
  'project-settings': Actions.ProjectSettings,
  'new-issue': Actions.CreateIssue,
};

/** Dispatch actions selected from the native Tauri menu into the shared action system. */
export function useTauriNativeMenu() {
  const { executeAction } = useActions();
  const actionInFlight = useRef(false);
  const lastAction = useRef<{ value: NativeMenuAction; at: number } | null>(
    null
  );

  useEffect(() => {
    if (!isTauriApp()) return;

    let disposed = false;
    let unlisten: (() => void) | undefined;

    async function setup() {
      const { listen } = await import('@tauri-apps/api/event');
      const listener = await listen<string>('native-menu-action', (event) => {
        const value = event.payload as NativeMenuAction;
        const action = ACTIONS[value];
        if (!action) return;

        const now = Date.now();
        if (
          lastAction.current?.value === value &&
          now - lastAction.current.at < 500
        ) {
          return;
        }
        lastAction.current = { value, at: now };

        // A native menu click must result in one modal/action only. This also
        // protects against a late event from a listener being replaced during
        // React provider remounts.
        if (actionInFlight.current) return;
        actionInFlight.current = true;
        void executeAction(action).finally(() => {
          actionInFlight.current = false;
        });
      });

      if (disposed) {
        listener();
      } else {
        unlisten = listener;
      }
    }

    void setup();
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [executeAction]);
}
