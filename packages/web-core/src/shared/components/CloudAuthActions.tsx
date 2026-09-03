import {
  SignInIcon,
  SignOutIcon,
  UserCircleIcon,
  UserPlusIcon,
} from '@phosphor-icons/react';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { SidebarBarButton } from '@vibe/ui/components/SidebarBarButton';
import {
  DEFAULT_AURAPUNK_CLOUD_URL,
  useCloudUrl,
} from '@/shared/hooks/useAppMode';
import { openLocalApiWebSocket } from '@/shared/lib/localApiTransport';
import { makeRequest } from '@/shared/lib/remoteApi';

/**
 * Cloud authentication entry points. Authentication is completed by the
 * AuraPunk Cloud website so the desktop app never handles a provider password
 * or stores a browser session itself.
 */
export function CloudAuthActions() {
  const { t } = useTranslation('common');
  const cloudUrl = useCloudUrl();
  const [account, setAccount] = useState<CloudAccount | null>(null);
  const [pending, setPending] = useState(false);

  const syncMem0Account = useCallback(async (accountId: string | null) => {
    try {
      await makeRequest('/api/usage/mem0-account', {
        method: 'PUT',
        body: JSON.stringify({ account_id: accountId }),
      });
    } catch {
      // The desktop app remains usable when its optional local backend is not
      // available; hosted Mem0 will simply reject requests without identity.
    }
  }, []);

  const syncCloudContext = useCallback(
    async (nextAccount: CloudAccount) => {
      if (!nextAccount.accessToken) return;

      try {
        const localResponse = await fetch('/api/mobile/context', {
          headers: { Accept: 'application/json' },
          cache: 'no-store',
        });
        if (!localResponse.ok) return;
        const localBody = (await localResponse.json()) as {
          success?: boolean;
          data?: { records?: CloudSyncRecord[] };
        };
        const records = localBody.data?.records ?? [];
        for (let index = 0; index < records.length; index += 100) {
          const batch = records.slice(index, index + 100);
          await fetch(`${cloudUrl.replace(/\/$/, '')}/api/sync`, {
            method: 'POST',
            headers: {
              Authorization: `Bearer ${nextAccount.accessToken}`,
              'Content-Type': 'application/json',
            },
            body: JSON.stringify({
              source: 'desktop',
              operations: batch.map((record) => ({
                entityType: record.entity_type,
                entityId: record.entity_id,
                operation: record.operation,
                payload: record.payload,
              })),
            }),
          });
        }
      } catch {
        // Cloud sync is deliberately best-effort. The local database remains
        // authoritative while the network or Cloud service is unavailable.
      }
    },
    [cloudUrl]
  );

  const syncCloudCommands = useCallback(
    async (nextAccount: CloudAccount, signal?: AbortSignal) => {
      if (!nextAccount.accessToken) return;

      const cursorKey = `${CLOUD_COMMAND_CURSOR_PREFIX}:${nextAccount.userId}`;
      const cursor = Number(window.localStorage.getItem(cursorKey) ?? '0');
      try {
        const response = await fetch(
          `${cloudUrl.replace(/\/$/, '')}/api/sync?after_revision=${Math.max(0, cursor)}&limit=200&wait_ms=25000`,
          {
            headers: { Authorization: `Bearer ${nextAccount.accessToken}` },
            cache: 'no-store',
            signal,
          }
        );
        if (!response.ok) {
          await new Promise((resolve) => window.setTimeout(resolve, 2500));
          return;
        }
        const body = (await response.json()) as { events?: CloudSyncEvent[] };
        let nextCursor = cursor;
        for (const event of body.events ?? []) {
          nextCursor = Math.max(nextCursor, event.revision ?? nextCursor);
          if (event.operation !== 'upsert') continue;
          if (event.entityType === 'chat_command') {
            const payload = event.payload as MobileChatCommand;
            if (!payload.workspace_id || !payload.prompt?.trim()) continue;
            const localResponse = await fetch('/api/mobile/chat', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify(payload),
              signal,
            });
            if (!localResponse.ok) {
              await new Promise((resolve) => window.setTimeout(resolve, 2500));
              return;
            }
          } else if (event.entityType === 'workspace_request') {
            const payload = event.payload as MobileWorkspaceRequest;
            if (!payload.issue_id) continue;
            const localResponse = await fetch('/api/mobile/workspace', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify(payload),
              signal,
            });
            if (!localResponse.ok) {
              await new Promise((resolve) => window.setTimeout(resolve, 2500));
              return;
            }
          } else if (event.entityType === 'issue') {
            const payload = event.payload as { status_id?: string };
            if (!payload.status_id || !event.entityId) continue;
            const localResponse = await fetch(`/api/issues/${event.entityId}`, {
              method: 'PATCH',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({ status_id: payload.status_id }),
              signal,
            });
            if (!localResponse.ok) {
              await new Promise((resolve) => window.setTimeout(resolve, 2500));
              return;
            }
          }
        }
        window.localStorage.setItem(cursorKey, String(nextCursor));
      } catch {
        if (signal?.aborted) return;
        await new Promise((resolve) => window.setTimeout(resolve, 1000));
      }
    },
    [cloudUrl]
  );

  const watchCloudCommands = useCallback(
    async (nextAccount: CloudAccount, signal: AbortSignal) => {
      while (!signal.aborted) {
        await syncCloudCommands(nextAccount, signal);
      }
    },
    [syncCloudCommands]
  );

  useEffect(() => {
    try {
      const saved = window.localStorage.getItem(CLOUD_ACCOUNT_STORAGE_KEY);
      if (saved) {
        const parsed = JSON.parse(saved) as CloudAccount;
        if (parsed.accessToken) {
          setAccount(parsed);
          void syncMem0Account(parsed.userId);
          void syncCloudContext(parsed);
        }
      }
    } catch {
      // Ignore malformed device-local account state.
    }
  }, [syncCloudCommands, syncCloudContext, syncMem0Account]);

  useEffect(() => {
    if (!account?.accessToken) return;
    const controller = new AbortController();
    void watchCloudCommands(account, controller.signal);
    let socket: WebSocket | null = null;
    let reconnectTimer: number | null = null;
    let cancelled = false;
    let contextSync: Promise<void> | null = null;
    let contextSyncQueued = false;

    const pushContext = () => {
      if (cancelled) return;
      if (contextSync) {
        contextSyncQueued = true;
        return;
      }

      contextSync = syncCloudContext(account).finally(() => {
        contextSync = null;
        if (contextSyncQueued) {
          contextSyncQueued = false;
          pushContext();
        }
      });
    };

    const connectToLocalEvents = async () => {
      if (cancelled) return;
      try {
        socket = await openLocalApiWebSocket('/api/workspaces/streams/ws');
        if (cancelled) {
          socket.close();
          return;
        }
        socket.onmessage = pushContext;
        socket.onerror = () => socket?.close();
        socket.onclose = () => {
          socket = null;
          if (!cancelled) {
            reconnectTimer = window.setTimeout(() => {
              void connectToLocalEvents();
            }, 3000);
          }
        };
      } catch {
        if (!cancelled) {
          reconnectTimer = window.setTimeout(() => {
            void connectToLocalEvents();
          }, 3000);
        }
      }
    };

    // Initial snapshot, then only local event notifications. The 3s delay is
    // reconnect backoff, not a UI polling interval.
    pushContext();
    void connectToLocalEvents();

    return () => {
      cancelled = true;
      controller.abort();
      if (reconnectTimer !== null) window.clearTimeout(reconnectTimer);
      socket?.close();
    };
  }, [account, syncCloudContext, watchCloudCommands]);

  const openExternal = useCallback(async (url: string) => {
    if ('__TAURI_INTERNALS__' in window) {
      await invoke('open_external_url', { url });
      return;
    }
    window.open(url, '_blank', 'noopener,noreferrer');
  }, []);

  const openCloudAuth = useCallback(async () => {
    const state = crypto.randomUUID().replaceAll('-', '');
    try {
      const url = new URL('/desktop-auth', cloudUrl);
      url.searchParams.set('state', state);
      setPending(true);
      await openExternal(url.toString());

      const deadline = Date.now() + 2 * 60 * 1000;
      while (Date.now() < deadline) {
        await new Promise((resolve) => window.setTimeout(resolve, 1500));
        const response = await fetch(
          `${url.origin}/api/desktop-auth/status?state=${state}`,
          { cache: 'no-store' }
        );
        if (!response.ok) continue;
        const result = (await response.json()) as DesktopAuthStatus;
        if (result.status !== 'complete') continue;

        setAccount(result.account);
        void syncMem0Account(result.account.userId);
        void syncCloudContext(result.account);
        window.localStorage.setItem(
          CLOUD_ACCOUNT_STORAGE_KEY,
          JSON.stringify(result.account)
        );
        break;
      }
    } catch (error) {
      console.warn('Could not start AuraPunk Cloud sign-in', error);
      // A malformed self-hosted URL or an unavailable external opener should
      // not break the local application.
      try {
        await openExternal(`${DEFAULT_AURAPUNK_CLOUD_URL}/dashboard`);
      } catch {
        // Keep the local app usable when no external browser is available.
      }
    } finally {
      setPending(false);
    }
  }, [cloudUrl, openExternal, syncMem0Account]);

  const openDashboard = useCallback(() => {
    void openExternal(`${cloudUrl.replace(/\/$/, '')}/dashboard`);
  }, [cloudUrl, openExternal]);

  const signOut = useCallback(() => {
    window.localStorage.removeItem(CLOUD_ACCOUNT_STORAGE_KEY);
    setAccount(null);
    void syncMem0Account(null);
  }, [syncMem0Account]);

  return (
    <>
      {account && (
        <SidebarBarButton
          label="Account"
          icon={UserCircleIcon}
          onClick={openDashboard}
          title={account.email}
          aria-label={`Signed in as ${account.email}`}
          className="text-normal"
        />
      )}
      {account ? (
        <SidebarBarButton
          label="Logout"
          icon={SignOutIcon}
          onClick={signOut}
          title="Sign out of this app"
          aria-label="Logout"
          className="text-normal"
        />
      ) : (
        <>
          <SidebarBarButton
            label={t('sidebar.logIn')}
            icon={SignInIcon}
            onClick={() => void openCloudAuth()}
            title={t('sidebar.cloudAuthTitle')}
            aria-label={t('sidebar.logIn')}
            className="text-normal"
            disabled={pending}
          />
          <SidebarBarButton
            label={t('sidebar.signUp')}
            icon={UserPlusIcon}
            onClick={() => void openCloudAuth()}
            title={t('sidebar.cloudAuthTitle')}
            aria-label={t('sidebar.signUp')}
            className="text-normal"
            disabled={pending}
          />
        </>
      )}
    </>
  );
}

type CloudAccount = {
  userId: string;
  displayName: string;
  email: string;
  accessToken: string;
  deviceId: string;
  scopes: string[];
  expiresAt: number;
};

type DesktopAuthStatus =
  | { status: 'pending' }
  | { status: 'complete'; account: CloudAccount };

type CloudSyncRecord = {
  entity_type:
    | 'project'
    | 'status'
    | 'workspace'
    | 'workspace_request'
    | 'issue_workspace'
    | 'chat'
    | 'chat_command'
    | 'issue'
    | 'job';
  entity_id: string;
  operation: 'upsert' | 'delete';
  payload: unknown;
};

type CloudSyncEvent = {
  entityType?: string;
  entityId?: string;
  operation?: string;
  payload?: unknown;
  revision?: number;
};

type MobileChatCommand = {
  workspace_id: string;
  prompt: string;
  executor?: string;
};

type MobileWorkspaceRequest = {
  issue_id: string;
  executor?: string;
};

const CLOUD_ACCOUNT_STORAGE_KEY = 'aurapunk-cloud-account';
const CLOUD_COMMAND_CURSOR_PREFIX = 'aurapunk-cloud-command-cursor';
