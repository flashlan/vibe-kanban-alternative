import { useCallback, useEffect, useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@vibe/ui/components/KeyboardDialog';
import { Button } from '@vibe/ui/components/Button';
import { ArchiveIcon, ArrowCounterClockwiseIcon, TrashIcon } from '@phosphor-icons/react';
import { create, useModal } from '@ebay/nice-modal-react';
import { useTranslation } from 'react-i18next';
import { defineModal } from '@/shared/lib/modals';
import { ConfirmDialog } from '@vibe/ui/components/ConfirmDialog';
import {
  deleteIssue,
  listArchivedIssues,
  restoreIssue,
} from '@/shared/lib/remoteApi';
import { refreshShapeSource } from '@/shared/lib/electric/collections';
import { PROJECT_ISSUES_SHAPE } from 'shared/remote-types';
import type { Issue } from 'shared/remote-types';

export interface ArchivedIssuesDialogProps {
  projectId: string;
}

const ArchivedIssuesDialogImpl = create<ArchivedIssuesDialogProps>(
  ({ projectId }: ArchivedIssuesDialogProps) => {
  const modal = useModal();
  const { t } = useTranslation('common');
  const [issues, setIssues] = useState<Issue[]>([]);
  const [loading, setLoading] = useState(true);
  const [busyId, setBusyId] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    const list = await listArchivedIssues(projectId);
    setIssues(list as Issue[]);
    setLoading(false);
  }, [projectId]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const handleRestore = useCallback(
    async (issue: Issue) => {
      setBusyId(issue.id);
      try {
        await restoreIssue(issue.id);
        refreshShapeSource(PROJECT_ISSUES_SHAPE, { project_id: projectId });
        await refresh();
      } finally {
        setBusyId(null);
      }
    },
    [projectId, refresh]
  );

  const handleDelete = useCallback(
    async (issue: Issue) => {
      const result = await ConfirmDialog.show({
        title: 'Delete Permanently',
        message: `Permanently delete "${issue.title || issue.simple_id}"? This cannot be undone.`,
        confirmText: 'Delete',
        cancelText: 'Cancel',
        variant: 'destructive',
      });
      if (result !== 'confirmed') return;
      setBusyId(issue.id);
      try {
        await deleteIssue(issue.id, { cleanupWorkspaces: true });
        await refresh();
      } finally {
        setBusyId(null);
      }
    },
    [refresh]
  );

  return (
    <Dialog open={modal.visible} onOpenChange={() => modal.hide()}>
      <DialogContent className="sm:max-w-[560px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <ArchiveIcon className="size-icon-sm" weight="bold" />
            {t('archive.dialogTitle', 'Archived Issues')}
          </DialogTitle>
          <DialogDescription className="text-left pt-2">
            {t(
              'archive.dialogDescription',
              'Issues you archived are listed here. Restore them to the board or delete them permanently.'
            )}
          </DialogDescription>
        </DialogHeader>

        <div className="max-h-[50vh] overflow-y-auto">
          {loading ? (
            <p className="text-sm text-low py-4">
              {t('archive.loading', 'Loading…')}
            </p>
          ) : issues.length === 0 ? (
            <p className="text-sm text-low py-4">
              {t('archive.empty', 'No archived issues.')}
            </p>
          ) : (
            <ul className="flex flex-col gap-1">
              {issues.map((issue) => (
                <li
                  key={issue.id}
                  className="flex items-center justify-between gap-2 rounded-sm border p-2"
                >
                  <div className="min-w-0">
                    <div className="truncate text-sm font-medium">
                      {issue.title || issue.simple_id}
                    </div>
                    <div className="text-xs text-low">{issue.simple_id}</div>
                  </div>
                  <div className="flex shrink-0 items-center gap-1">
                    <Button
                      variant="outline"
                      size="sm"
                      disabled={busyId === issue.id}
                      onClick={() => void handleRestore(issue)}
                    >
                      <ArrowCounterClockwiseIcon className="size-icon-xs" weight="bold" />
                      {t('archive.restore', 'Restore')}
                    </Button>
                    <Button
                      variant="destructive"
                      size="sm"
                      disabled={busyId === issue.id}
                      onClick={() => void handleDelete(issue)}
                    >
                      <TrashIcon className="size-icon-xs" weight="bold" />
                      {t('archive.deletePermanent', 'Delete')}
                    </Button>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => modal.hide()}>
            {t('common:buttons.close', 'Close')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
});

export const ArchivedIssuesDialog = defineModal<
  ArchivedIssuesDialogProps,
  void
>(ArchivedIssuesDialogImpl);
