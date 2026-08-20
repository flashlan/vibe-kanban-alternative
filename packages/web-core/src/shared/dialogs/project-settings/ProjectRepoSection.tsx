import { useState, useEffect } from 'react';
import { Button } from '@vibe/ui/components/Button';
import { Label } from '@vibe/ui/components/Label';
import { Alert, AlertDescription } from '@vibe/ui/components/Alert';
import { useTranslation } from 'react-i18next';
import { repoApi } from '@/shared/lib/api';
import {
  getProjectRepoDefaults,
  saveProjectRepoDefaults,
} from '@/shared/hooks/useProjectRepoDefaults';
import type { Repo } from 'shared/types';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@vibe/ui/components/DropdownMenu';
import { FolderGit, ChevronsUpDown } from 'lucide-react';

export interface ProjectRepoSectionProps {
  projectId: string;
  /** Called after a successful save. Omit for a persistent panel (no dismiss). */
  onSaved?: () => void;
  /** Renders a Cancel button next to Save when provided. */
  onCancel?: () => void;
}

/**
 * The project's primary repository — pre-fills the repo picker when a new
 * workspace/card is created for this project (`PROJECT_REPO_DEFAULTS`
 * scratch). Still overridable per workspace at creation time; this only sets
 * the starting point.
 */
export function ProjectRepoSection({
  projectId,
  onSaved,
  onCancel,
}: ProjectRepoSectionProps) {
  const { t } = useTranslation('projects');
  const [repos, setRepos] = useState<Repo[]>([]);
  const [selectedRepoId, setSelectedRepoId] = useState<string>('');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [dropdownOpen, setDropdownOpen] = useState(false);

  useEffect(() => {
    let cancelled = false;

    (async () => {
      setLoading(true);
      try {
        const [allRepos, defaults] = await Promise.all([
          repoApi.list(),
          getProjectRepoDefaults(projectId),
        ]);
        if (cancelled) return;
        setRepos(allRepos);
        if (defaults && defaults.length > 0) {
          setSelectedRepoId(defaults[0].repo_id);
        }
        setLoading(false);
      } catch {
        if (!cancelled) {
          setRepos([]);
          setLoading(false);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [projectId]);

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    try {
      if (selectedRepoId) {
        await saveProjectRepoDefaults(projectId, [
          { repo_id: selectedRepoId, target_branch: '' },
        ]);
      }
      onSaved?.();
    } catch (err) {
      setError(
        err instanceof Error ? err.message : 'Failed to save project repo'
      );
    } finally {
      setSaving(false);
    }
  };

  const selectedRepo = repos.find((r) => r.id === selectedRepoId);

  return (
    <div className="space-y-4">
      {loading ? (
        <p className="text-sm text-low">Loading repositories...</p>
      ) : repos.length === 0 ? (
        <Alert>
          <AlertDescription>
            {t(
              'editProjectRepoDialog.noRepos',
              'No repositories registered. Add one in Settings > Repositories.'
            )}
          </AlertDescription>
        </Alert>
      ) : (
        <div className="space-y-2">
          <Label>{t('editProjectRepoDialog.repoLabel', 'Repository')}</Label>
          <DropdownMenu open={dropdownOpen} onOpenChange={setDropdownOpen}>
            <DropdownMenuTrigger asChild>
              <button
                type="button"
                className="flex items-center justify-between w-full px-3 py-2 rounded-sm border border-border bg-secondary text-sm text-normal"
              >
                <div className="flex items-center gap-2 min-w-0">
                  <FolderGit className="h-4 w-4 flex-shrink-0" />
                  <span className="truncate">
                    {selectedRepo
                      ? selectedRepo.display_name || selectedRepo.name
                      : t(
                          'editProjectRepoDialog.repoPlaceholder',
                          'Select a repository'
                        )}
                  </span>
                </div>
                <ChevronsUpDown className="h-4 w-4 flex-shrink-0 text-muted-foreground" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent className="w-[var(--radix-dropdown-menu-trigger-width)]">
              <DropdownMenuItem onSelect={() => setSelectedRepoId('')}>
                {t('editProjectRepoDialog.repoNone', 'No repository')}
              </DropdownMenuItem>
              {repos.map((repo) => (
                <DropdownMenuItem
                  key={repo.id}
                  onSelect={() => setSelectedRepoId(repo.id)}
                >
                  <div className="flex items-center gap-2">
                    <FolderGit className="h-3.5 w-3.5" />
                    <span className="truncate">
                      {repo.display_name || repo.name}
                    </span>
                  </div>
                </DropdownMenuItem>
              ))}
            </DropdownMenuContent>
          </DropdownMenu>
          <p className="text-xs text-low">
            {t(
              'editProjectRepoDialog.repoHint',
              'Pre-fills the repository picker when you create a new workspace for this project. You can still pick a different (or additional) repo at workspace-creation time.'
            )}
          </p>
        </div>
      )}

      {error && (
        <Alert variant="destructive">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      <div className="flex justify-end gap-2">
        {onCancel && (
          <Button variant="outline" onClick={onCancel} disabled={saving}>
            {t('common:buttons.cancel', 'Cancel')}
          </Button>
        )}
        <Button onClick={handleSave} disabled={saving || loading}>
          {saving
            ? t('common:buttons.saving', 'Saving...')
            : t('common:buttons.save', 'Save')}
        </Button>
      </div>
    </div>
  );
}
