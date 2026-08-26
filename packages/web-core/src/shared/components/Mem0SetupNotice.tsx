import { useEffect, useState } from 'react';
import { ArrowSquareOutIcon, WarningIcon, XIcon } from '@phosphor-icons/react';
import { Alert, AlertDescription, AlertTitle } from '@vibe/ui/components/Alert';
import { handleApiResponse, makeRequest } from '@/shared/lib/api';

const DOCKER_HUB_URL = 'https://hub.docker.com/r/datyapoint/vk-mem0';
const README_MEMORY_URL =
  'https://github.com/flashlan/vibe-kanban-alternative#project-memory-mem0';
const DISMISSED_STORAGE_KEY = 'vibe-mem0-setup-notice-dismissed';

type Mem0StatusResponse = {
  level: 'green' | 'yellow' | 'orange' | 'red';
};

function wasDismissed(): boolean {
  try {
    return sessionStorage.getItem(DISMISSED_STORAGE_KEY) === 'true';
  } catch {
    return false;
  }
}

function dismissForSession(): void {
  try {
    sessionStorage.setItem(DISMISSED_STORAGE_KEY, 'true');
  } catch {
    // The notice can still be dismissed for the current render.
  }
}

/**
 * Non-blocking first-run guidance for installations without the optional
 * mem0-vk Docker service. The core app and agents continue to work; only
 * semantic memory search/save are unavailable.
 */
export function Mem0SetupNotice() {
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    if (wasDismissed()) return;

    let cancelled = false;
    void (async () => {
      try {
        const response = await makeRequest('/api/usage/mem0-status');
        const status = await handleApiResponse<Mem0StatusResponse>(response);
        if (!cancelled && status?.level === 'red') setVisible(true);
      } catch {
        // A failed health check must never prevent the app from starting.
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  if (!visible) return null;

  return (
    <Alert
      variant="default"
      className="shrink-0 rounded-none border-x-0 border-t-0 border-warning/40 bg-warning/10 py-3"
    >
      <WarningIcon className="size-4 text-warning" weight="fill" />
      <AlertTitle className="flex items-center gap-2 pr-8 text-normal">
        Project memory is unavailable
      </AlertTitle>
      <AlertDescription className="pr-8 text-low">
        Vibe Kanban continues to work, but agents cannot search or save Mem0
        project memories until the optional Docker service is running.{' '}
        <a
          href={DOCKER_HUB_URL}
          target="_blank"
          rel="noreferrer"
          className="inline-flex items-center gap-1 text-normal underline underline-offset-2 hover:text-high"
        >
          Install from Docker Hub
          <ArrowSquareOutIcon className="size-3" weight="bold" />
        </a>{' '}
        <span>or</span>{' '}
        <a
          href={README_MEMORY_URL}
          target="_blank"
          rel="noreferrer"
          className="inline-flex items-center gap-1 text-normal underline underline-offset-2 hover:text-high"
        >
          read the setup guide
          <ArrowSquareOutIcon className="size-3" weight="bold" />
        </a>
        .
      </AlertDescription>
      <button
        type="button"
        onClick={() => {
          dismissForSession();
          setVisible(false);
        }}
        aria-label="Dismiss Mem0 setup notice"
        className="absolute right-3 top-3 rounded-sm p-1 text-low hover:bg-warning/20 hover:text-normal"
      >
        <XIcon className="size-4" weight="bold" />
      </button>
    </Alert>
  );
}
