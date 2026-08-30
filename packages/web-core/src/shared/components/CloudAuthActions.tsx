import { SignInIcon, UserPlusIcon } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import { SidebarBarButton } from '@vibe/ui/components/SidebarBarButton';
import { useCloudUrl } from '@/shared/hooks/useAppMode';

/**
 * Cloud authentication entry points. Authentication is completed by the
 * AuraPunk Cloud website so the desktop app never handles a provider password
 * or stores a browser session itself.
 */
export function CloudAuthActions() {
  const { t } = useTranslation('common');
  const cloudUrl = useCloudUrl();

  const openCloudAuth = () => {
    try {
      const url = new URL('/dashboard?source=desktop', cloudUrl);
      window.open(url.toString(), '_blank', 'noopener,noreferrer');
    } catch {
      // A malformed self-hosted URL should not break the rest of the sidebar.
      window.open(
        'https://aurapunk-cloud.datapoint.chatgpt.site/dashboard',
        '_blank',
        'noopener,noreferrer'
      );
    }
  };

  return (
    <>
      <SidebarBarButton
        label={t('sidebar.logIn')}
        icon={SignInIcon}
        onClick={openCloudAuth}
        title={t('sidebar.cloudAuthTitle')}
        aria-label={t('sidebar.logIn')}
        className="text-normal"
      />
      <SidebarBarButton
        label={t('sidebar.signUp')}
        icon={UserPlusIcon}
        onClick={openCloudAuth}
        title={t('sidebar.cloudAuthTitle')}
        aria-label={t('sidebar.signUp')}
        className="text-normal"
      />
    </>
  );
}
