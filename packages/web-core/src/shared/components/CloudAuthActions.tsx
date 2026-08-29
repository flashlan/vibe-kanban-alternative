import { SignInIcon, UserPlusIcon } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import { SidebarBarButton } from '@vibe/ui/components/SidebarBarButton';

/**
 * Cloud authentication entry points. The cloud account service is not part
 * of this local-first build yet, so the controls stay visible but disabled
 * when `--cloud` is selected instead of implying that authentication works.
 */
export function CloudAuthActions() {
  const { t } = useTranslation('common');

  return (
    <>
      <SidebarBarButton
        label={t('sidebar.logIn')}
        icon={SignInIcon}
        disabled
        title={t('sidebar.cloudAuthComingSoon')}
        aria-label={t('sidebar.logIn')}
        className="cursor-not-allowed opacity-70 hover:bg-transparent hover:text-normal"
      />
      <SidebarBarButton
        label={t('sidebar.signUp')}
        icon={UserPlusIcon}
        disabled
        title={t('sidebar.cloudAuthComingSoon')}
        aria-label={t('sidebar.signUp')}
        className="cursor-not-allowed opacity-70 hover:bg-transparent hover:text-normal"
      />
    </>
  );
}
