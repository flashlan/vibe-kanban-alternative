import { useState, useEffect, useMemo } from 'react';
import { Button } from '@vibe/ui/components/Button';
import { Input } from '@vibe/ui/components/Input';
import { Label } from '@vibe/ui/components/Label';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@vibe/ui/components/KeyboardDialog';
import { Alert, AlertDescription } from '@vibe/ui/components/Alert';
import { create, useModal } from '@ebay/nice-modal-react';
import { useTranslation } from 'react-i18next';
import { defineModal } from '@/shared/lib/modals';
import { useShape } from '@/shared/integrations/electric/hooks';
import {
  PROJECTS_SHAPE,
  PROJECT_MUTATION,
  type Project,
} from 'shared/remote-types';
import { getRandomPresetColor, PRESET_COLORS } from '@/shared/lib/colors';
import { ColorPicker } from '@/shared/components/ui-new/containers/ColorPickerContainer';
import { repoApi } from '@/shared/lib/api';
import { saveProjectRepoDefaults } from '@/shared/hooks/useProjectRepoDefaults';
import type { Repo } from 'shared/types';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@vibe/ui/components/DropdownMenu';
import { FolderGit, ChevronsUpDown } from 'lucide-react';

export type CreateProjectDialogProps = {
  /** ADR-015: when set, the new project is created as a child board of the
   * given parent id (parent_id = parentId). Omit for a top-level project. */
  parentId?: string;
};

export type CreateProjectResult = {
  action: 'created' | 'canceled';
  project?: Project;
};

const CreateProjectDialogImpl = create<CreateProjectDialogProps>(
  ({ parentId }) => {
    const modal = useModal();
    const { t } = useTranslation('projects');
    const [name, setName] = useState('');
    const [color, setColor] = useState<string>(() => getRandomPresetColor());
    const [error, setError] = useState<string | null>(null);
    const [isCreating, setIsCreating] = useState(false);
    const [repos, setRepos] = useState<Repo[]>([]);
    const [selectedRepoId, setSelectedRepoId] = useState<string>('');
    const [reposLoading, setReposLoading] = useState(true);
    const [repoDropdownOpen, setRepoDropdownOpen] = useState(false);

    // ADR-018 — projects are tenant-less; subscribe PROJECTS_SHAPE with
    // empty params (single global cache key).
    const params = useMemo(() => ({}), []);

    const { insert, error: syncError } = useShape(PROJECTS_SHAPE, params, {
      mutation: PROJECT_MUTATION,
    });

    useEffect(() => {
      // Reset form when dialog opens
      if (modal.visible) {
        setName('');
        setColor(getRandomPresetColor());
        setError(null);
        setIsCreating(false);
        setSelectedRepoId('');
        setRepoDropdownOpen(true);
      }
    }, [modal.visible]);

    // Fetch available repos
    useEffect(() => {
      if (!modal.visible) return;
      let cancelled = false;
      setReposLoading(true);
      repoApi
        .list()
        .then((data) => {
          if (!cancelled) {
            setRepos(data);
            if (data.length === 1) {
              setSelectedRepoId(data[0].id);
            }
            setReposLoading(false);
          }
        })
        .catch(() => {
          if (!cancelled) {
            setRepos([]);
            setReposLoading(false);
          }
        });
      return () => {
        cancelled = true;
      };
    }, [modal.visible]);

    useEffect(() => {
      if (syncError) {
        setError(syncError.message || 'Failed to create project');
        setIsCreating(false);
      }
    }, [syncError]);

    const validateName = (value: string): string | null => {
      const trimmedValue = value.trim();
      if (!trimmedValue) return 'Project name is required';
      if (trimmedValue.length < 2)
        return 'Project name must be at least 2 characters';
      if (trimmedValue.length > 100)
        return 'Project name must be 100 characters or less';
      return null;
    };

    const handleCreate = async () => {
      const nameError = validateName(name);
      if (nameError) {
        setError(nameError);
        return;
      }

      setError(null);
      setIsCreating(true);

      try {
        const { data: project, persisted } = insert({
          name: name.trim(),
          color: color,
          ...(parentId ? { parent_id: parentId } : {}),
        });

        const persistedProject = await persisted;
        const finalProject = persistedProject ?? project;

        // Save repo defaults if a repo was selected
        if (selectedRepoId && finalProject?.id) {
          saveProjectRepoDefaults(finalProject.id, [
            { repo_id: selectedRepoId, target_branch: '' },
          ]).catch((err) =>
            console.warn('Failed to save project repo defaults:', err)
          );
        }

        modal.resolve({
          action: 'created',
          project: finalProject,
        } as CreateProjectResult);
        modal.hide();
      } catch (err) {
        setError(
          err instanceof Error ? err.message : 'Failed to create project'
        );
        setIsCreating(false);
      }
    };

    const handleCancel = () => {
      modal.resolve({ action: 'canceled' } as CreateProjectResult);
      modal.hide();
    };

    const handleOpenChange = (open: boolean) => {
      if (isCreating) return;

      if (!open) {
        handleCancel();
      }
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
      if (e.key === 'Enter' && name.trim() && !isCreating) {
        e.preventDefault();
        void handleCreate();
      }
    };

    return (
      <Dialog open={modal.visible} onOpenChange={handleOpenChange}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>
              {parentId
                ? t('createProjectDialog.titleChildBoard', 'Create child board')
                : t('createProjectDialog.title', 'Create Project')}
            </DialogTitle>
            <DialogDescription>
              {parentId
                ? t(
                    'createProjectDialog.descriptionChildBoard',
                    'Create a new child board under this project.'
                  )
                : t('createProjectDialog.description', 'Create a new project.')}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="project-name">
                {t('createProjectDialog.nameLabel', 'Project name')}
              </Label>
              <div className="flex items-center gap-2">
                <Input
                  id="project-name"
                  value={name}
                  onChange={(e) => {
                    setName(e.target.value);
                    setError(null);
                  }}
                  onKeyDown={handleKeyDown}
                  placeholder={t(
                    'createProjectDialog.namePlaceholder',
                    'Enter project name'
                  )}
                  maxLength={100}
                  autoFocus
                  disabled={isCreating}
                  className="flex-1"
                />
                <ColorPicker
                  value={color}
                  onChange={setColor}
                  colors={PRESET_COLORS}
                  disabled={isCreating}
                  align="start"
                  side="bottom"
                >
                  <button
                    type="button"
                    className="w-10 h-10 rounded border cursor-pointer shrink-0 disabled:opacity-50 disabled:cursor-not-allowed"
                    style={{ backgroundColor: `hsl(${color})` }}
                    disabled={isCreating}
                    aria-label={t(
                      'createProjectDialog.selectColor',
                      'Select project color'
                    )}
                  />
                </ColorPicker>
              </div>
            </div>

            {repos.length > 0 && (
              <div className="space-y-2">
                <Label>
                  {t('createProjectDialog.repoLabel', 'Repository (optional)')}
                </Label>
                <DropdownMenu
                  open={repoDropdownOpen}
                  onOpenChange={setRepoDropdownOpen}
                >
                  <DropdownMenuTrigger asChild>
                    <button
                      type="button"
                      className="flex items-center justify-between w-full px-3 py-2 rounded-sm border border-border bg-secondary text-sm text-normal disabled:opacity-50 disabled:cursor-not-allowed"
                      disabled={isCreating || reposLoading}
                    >
                      <div className="flex items-center gap-2 min-w-0">
                        <FolderGit className="h-4 w-4 flex-shrink-0" />
                        <span className="truncate">
                          {selectedRepoId
                            ? repos.find((r) => r.id === selectedRepoId)
                                ?.display_name ||
                              repos.find((r) => r.id === selectedRepoId)?.name
                            : t(
                                'createProjectDialog.repoPlaceholder',
                                'Select a repository'
                              )}
                        </span>
                      </div>
                      <ChevronsUpDown className="h-4 w-4 flex-shrink-0 text-muted-foreground" />
                    </button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent className="w-[var(--radix-dropdown-menu-trigger-width)]">
                    <DropdownMenuItem onSelect={() => setSelectedRepoId('')}>
                      {t('createProjectDialog.repoNone', 'No repository')}
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
              </div>
            )}

            {repos.length === 0 && !reposLoading && (
              <p className="text-xs text-low">
                {t(
                  'createProjectDialog.noRepos',
                  'No repositories registered. Add one in Settings > Repositories.'
                )}
              </p>
            )}

            {error && (
              <Alert variant="destructive">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={handleCancel}
              disabled={isCreating}
            >
              {t('common:buttons.cancel', 'Cancel')}
            </Button>
            <Button
              onClick={handleCreate}
              disabled={!name.trim() || isCreating}
            >
              {isCreating
                ? t('createProjectDialog.creating', 'Creating...')
                : t('createProjectDialog.createButton', 'Create Project')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    );
  }
);

export const CreateProjectDialog = defineModal<
  CreateProjectDialogProps,
  CreateProjectResult
>(CreateProjectDialogImpl);
