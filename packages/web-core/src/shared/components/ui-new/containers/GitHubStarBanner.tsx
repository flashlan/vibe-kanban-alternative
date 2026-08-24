import { useCallback, useState } from 'react';
import { GithubLogoIcon, StarIcon, XIcon } from '@phosphor-icons/react';
import { cn } from '@/shared/lib/utils';

const REPO_URL = 'https://github.com/flashlan/vibe-kanban-alternative';
const REPO_LABEL = 'flashlan/vibe-kanban-alternative';
const DISMISS_KEY = 'vibe.github-star-banner.dismissed';

function readDismissed(): boolean {
  try {
    return window.localStorage.getItem(DISMISS_KEY) === '1';
  } catch {
    return false;
  }
}

export function GitHubStarBanner({ className }: { className?: string }) {
  const [dismissed, setDismissed] = useState(readDismissed);

  const handleDismiss = useCallback(() => {
    setDismissed(true);
    try {
      window.localStorage.setItem(DISMISS_KEY, '1');
    } catch {
      // Storage unavailable (private mode) — session-only dismissal.
    }
  }, []);

  if (dismissed) return null;

  return (
    <div
      className={cn(
        'flex h-6 items-center justify-center gap-1.5 px-double',
        'bg-gradient-to-r from-brand/15 via-brand/5 to-transparent',
        className
      )}
    >
      <GithubLogoIcon className="h-3 w-3 shrink-0 text-low" />
      <span className="truncate text-base leading-none text-low">
        Gostou do projeto? Deixe uma{' '}
        <StarIcon
          weight="fill"
          className="inline-block h-3 w-3 -translate-y-px text-brand"
        />{' '}
        no GitHub:{' '}
        <a
          href={REPO_URL}
          target="_blank"
          rel="noreferrer noopener"
          className="font-medium text-normal underline decoration-dotted underline-offset-2 transition-colors hover:text-brand"
        >
          {REPO_LABEL}
        </a>
      </span>
      <button
        type="button"
        onClick={handleDismiss}
        aria-label="Dispensar banner"
        className="shrink-0 cursor-pointer rounded-sm p-half text-low transition-colors hover:text-normal focus:outline-none focus:ring-1 focus:ring-brand"
      >
        <XIcon className="h-3 w-3" weight="bold" />
      </button>
    </div>
  );
}
