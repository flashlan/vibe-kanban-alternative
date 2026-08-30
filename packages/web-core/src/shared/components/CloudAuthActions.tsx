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

  useEffect(() => {
    try {
      const saved = window.localStorage.getItem(CLOUD_ACCOUNT_STORAGE_KEY);
      if (saved) {
        const parsed = JSON.parse(saved) as CloudAccount;
        setAccount(parsed);
        void syncMem0Account(parsed.userId);
      }
    } catch {
      // Ignore malformed device-local account state.
    }
  }, [syncMem0Account]);

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
};

type DesktopAuthStatus =
  | { status: 'pending' }
  | { status: 'complete'; account: CloudAccount };

const CLOUD_ACCOUNT_STORAGE_KEY = 'aurapunk-cloud-account';
