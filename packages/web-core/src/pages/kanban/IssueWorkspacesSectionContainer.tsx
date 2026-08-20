import { useMemo, useCallback, useState, useRef, useEffect } from 'react';
import { useParams } from '@tanstack/react-router';
import { useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { LinkIcon, PlusIcon, XIcon } from '@phosphor-icons/react';
import { useProjectContext } from '@/shared/hooks/useProjectContext';
import { useWorkspacesContext } from '@/shared/hooks/useWorkspacesContext';
import { useWorkspaceContext } from '@/shared/hooks/useWorkspaceContext';
import { useAppNavigation } from '@/shared/hooks/useAppNavigation';
import { CreateModeProvider } from '@/features/create-mode/model/CreateModeProvider';
import { CreateChatBoxContainer } from '@/shared/components/CreateChatBoxContainer';
import { workspacesApi } from '@/shared/lib/api';
import { getWorkspaceDefaults } from '@/shared/lib/workspaceDefaults';
import {
  buildLinkedIssueCreateState,
  buildLocalWorkspaceIdSet,
  buildWorkspaceCreateInitialState,
  buildWorkspaceCreatePrompt,
} from '@/shared/lib/workspaceCreateState';
import { ConfirmDialog } from '@vibe/ui/components/ConfirmDialog';
import { DeleteWorkspaceDialog } from '@vibe/ui/components/DeleteWorkspaceDialog';
import { PROJECT_WORKSPACES_SHAPE } from 'shared/remote-types';
import { refreshShapeSource } from '@/shared/lib/electric/collections';
import type { WorkspaceWithStats } from '@vibe/ui/components/IssueWorkspaceCard';
import { IssueWorkspacesSection } from '@vibe/ui/components/IssueWorkspacesSection';
import type { SectionAction } from '@vibe/ui/components/CollapsibleSectionHeader';
import type { CreateModeInitialState } from '@/shared/types/createMode';

interface IssueWorkspacesSectionContainerProps {
  issueId: string;
  /** Overrides the route projectId (for contexts without route params, e.g. the Create Issue modal). */
  projectId?: string;
  /** Called right after the inline composer creates a workspace, just
   *  before navigating to its session view — lets the enclosing modal (e.g.
   *  the Create Issue dialog) close itself first so it doesn't stay open on
   *  top of the destination. The new workspace's id is passed through so the
   *  caller can fold it into its own resolution instead of separately
   *  navigating to a plain issue view and racing this component's own
   *  navigation below. */
  onNavigateAway?: (createdWorkspaceId?: string) => void;
}

/**
 * Container component for the workspaces section.
 * Fetches workspace data from ProjectContext and transforms it for display.
 */
export function IssueWorkspacesSectionContainer({
  issueId,
  projectId: projectIdProp,
  onNavigateAway,
}: IssueWorkspacesSectionContainerProps) {
  const { t } = useTranslation('common');
  const { projectId: routeProjectId } = useParams({ strict: false });
  const projectId = projectIdProp ?? routeProjectId;
  const queryClient = useQueryClient();
  const appNavigation = useAppNavigation();
  const { workspaces } = useWorkspacesContext();

  // Inline "create workspace right here" state: the full composer
  // (CreateModeProvider + CreateChatBoxContainer — the exact same one used
  // everywhere else a workspace gets created) mounted directly in this
  // section instead of navigating to a separate screen. `composeDraftId` is
  // regenerated each time the composer is (re)opened so a fresh, unrelated
  // scratch draft is used.
  const [composeOpen, setComposeOpen] = useState(false);
  const [composeDraftId, setComposeDraftId] = useState<string | null>(null);
  const [composeInitialState, setComposeInitialState] =
    useState<CreateModeInitialState | null>(null);
  const composeScrollRef = useRef<HTMLDivElement>(null);

  // The composer isn't internally scrollable (it just grows with content —
  // a long pre-filled prompt pushes the model/Create row below the fold of
  // our bounded-height wrapper), and it loads its content (prompt, model
  // discovery) asynchronously, so its final height isn't known on mount.
  // Keep pinning to the bottom as it grows so the model/Create row lands
  // already visible instead of requiring a manual scroll first — once the
  // user scrolls up themselves, stop fighting them.
  useEffect(() => {
    if (!composeOpen) return;
    const node = composeScrollRef.current;
    if (!node) return;

    let pinnedToBottom = true;
    const scrollToBottom = () => {
      node.scrollTop = node.scrollHeight;
    };
    scrollToBottom();

    const handleScroll = () => {
      const distanceFromBottom =
        node.scrollHeight - node.scrollTop - node.clientHeight;
      pinnedToBottom = distanceFromBottom < 16;
    };
    node.addEventListener('scroll', handleScroll);

    // `node` itself has a fixed height (it's the scroll viewport), so
    // observing its own box never fires as content grows inside it — and
    // the growing content isn't necessarily a stable direct child (React
    // swaps whole subtrees, e.g. a loading placeholder for the real form).
    // A subtree MutationObserver catches that regardless of which node it
    // happens to.
    const mutationObserver = new MutationObserver(() => {
      if (pinnedToBottom) scrollToBottom();
    });
    mutationObserver.observe(node, {
      childList: true,
      subtree: true,
      characterData: true,
    });

    return () => {
      node.removeEventListener('scroll', handleScroll);
      mutationObserver.disconnect();
    };
  }, [composeOpen, composeDraftId]);

  const {
    pullRequests,
    getIssue,
    getWorkspacesForIssue,
    issues,
    isLoading: projectLoading,
  } = useProjectContext();
  const { activeWorkspaces, archivedWorkspaces } = useWorkspaceContext();

  const localWorkspacesById = useMemo(() => {
    const map = new Map<string, (typeof activeWorkspaces)[number]>();

    for (const workspace of activeWorkspaces) {
      map.set(workspace.id, workspace);
    }

    for (const workspace of archivedWorkspaces) {
      map.set(workspace.id, workspace);
    }

    return map;
  }, [activeWorkspaces, archivedWorkspaces]);

  // Get workspaces for the issue, with PR info
  const workspacesWithStats: WorkspaceWithStats[] = useMemo(() => {
    const rawWorkspaces = getWorkspacesForIssue(issueId);

    return rawWorkspaces.map((workspace) => {
      const localWorkspace = workspace.local_workspace_id
        ? localWorkspacesById.get(workspace.local_workspace_id)
        : undefined;

      // Find all linked PRs for this workspace
      const linkedPrs = pullRequests
        .filter((pr) => pr.workspace_id === workspace.id)
        .map((pr) => ({
          number: pr.number,
          url: pr.url,
          status: pr.status as 'open' | 'merged' | 'closed',
        }));

      // Get owner
      const owner = null;

      return {
        id: workspace.id,
        localWorkspaceId: workspace.local_workspace_id,
        name: workspace.name,
        archived: workspace.archived,
        filesChanged: workspace.files_changed ?? 0,
        linesAdded: workspace.lines_added ?? 0,
        linesRemoved: workspace.lines_removed ?? 0,
        prs: linkedPrs,
        owner,
        updatedAt: workspace.updated_at,
        isRunning: localWorkspace?.isRunning,
        hasPendingApproval: localWorkspace?.hasPendingApproval,
        hasRunningDevServer: localWorkspace?.hasRunningDevServer,
        hasUnseenActivity: localWorkspace?.hasUnseenActivity,
        latestProcessCompletedAt: localWorkspace?.latestProcessCompletedAt,
        latestProcessStatus: localWorkspace?.latestProcessStatus,
      };
    });
  }, [issueId, getWorkspacesForIssue, pullRequests, localWorkspacesById]);

  const isLoading = projectLoading;
  const shouldAnimateCreateButton = useMemo(() => {
    if (issues.length !== 1) {
      return false;
    }

    return issues.every(
      (issue) => getWorkspacesForIssue(issue.id).length === 0
    );
  }, [issues, getWorkspacesForIssue]);

  // Handle clicking '+' (or the draft card's own "Create"): mounts the full
  // compose UI (CreateModeProvider + CreateChatBoxContainer) right here,
  // seeded with the issue's title/description as the prompt and the
  // resolved repo/branch defaults (falling back to each repo's configured
  // default branch). No navigation, no separate screen — works the same
  // whether this section is inside the Create Issue modal or the normal
  // issue side panel.
  const handleAddWorkspace = useCallback(async () => {
    if (!projectId) {
      return;
    }

    const issue = getIssue(issueId);
    const initialPrompt = buildWorkspaceCreatePrompt(
      issue?.title ?? null,
      issue?.description ?? null
    );
    const localWorkspaceIds = buildLocalWorkspaceIdSet(
      activeWorkspaces,
      archivedWorkspaces
    );

    const defaults = await getWorkspaceDefaults(
      workspaces,
      localWorkspaceIds,
      projectId
    );

    setComposeInitialState(
      buildWorkspaceCreateInitialState({
        prompt: initialPrompt,
        defaults,
        linkedIssue: buildLinkedIssueCreateState(issue, projectId),
      })
    );
    setComposeDraftId(crypto.randomUUID());
    setComposeOpen(true);
  }, [
    projectId,
    getIssue,
    issueId,
    activeWorkspaces,
    archivedWorkspaces,
    workspaces,
  ]);

  const handleComposeCancel = useCallback(() => {
    setComposeOpen(false);
  }, []);

  // The embedded composer created (and started) the workspace itself — jump
  // straight to its session view (the whole point of creating it) instead
  // of leaving the user back on the board to go find and open it
  // themselves. Close the enclosing modal first (if any) so it doesn't
  // stay stacked on top of the destination.
  const handleComposeWorkspaceCreated = useCallback(
    (workspaceId: string) => {
      setComposeOpen(false);
      if (onNavigateAway) {
        // The enclosing modal (e.g. Create Issue) owns final navigation once
        // it resolves with this workspace's id — calling it here too would
        // just push a second, redundant history entry to the same route.
        onNavigateAway(workspaceId);
        return;
      }
      if (projectId) {
        appNavigation.goToProjectIssueWorkspace(
          projectId,
          issueId,
          workspaceId
        );
      }
    },
    [projectId, issueId, appNavigation, onNavigateAway]
  );

  // Handle clicking link action to link an existing workspace
  const handleLinkWorkspace = useCallback(async () => {
    if (!projectId) {
      return;
    }

    const { WorkspaceSelectionDialog } = await import(
      '@/shared/dialogs/command-bar/WorkspaceSelectionDialog'
    );
    await WorkspaceSelectionDialog.show({ projectId, issueId });
  }, [projectId, issueId]);

  // The workspace↔issue relink only surfaces through the workspaces shape,
  // which the local build polls on a slow interval. Force a refresh so the
  // section (and the kanban card) picks up the link immediately instead of
  // on the next poll.
  const refreshWorkspaceLinks = useCallback(() => {
    if (projectId) {
      refreshShapeSource(PROJECT_WORKSPACES_SHAPE, { project_id: projectId });
    }
  }, [projectId]);

  // Handle "Send issue here": dispatch the issue to an existing workspace's
  // session as a follow-up (context retained), then refresh the workspace state.
  const handleRunIssue = useCallback(
    async (localWorkspaceId: string) => {
      try {
        await workspacesApi.dispatchIssueToWorkspace(issueId, localWorkspaceId);
        // The workspace↔issue relink only surfaces through the workspaces
        // shape, which the local build polls on a slow interval. Force a
        // refresh so the section (and the kanban card) picks up the link
        // immediately instead of on the next poll.
        refreshWorkspaceLinks();
        // Full invalidation: a dispatch fans out to many caches (issue's
        // workspaces, workspace session, execution processes, branch status),
        // each keyed differently; a partial scope would leave stale UI.
        await queryClient.invalidateQueries();
      } catch (error) {
        ConfirmDialog.show({
          title: t('common:error'),
          message:
            error instanceof Error
              ? error.message
              : t('workspaces.dispatchError'),
          confirmText: t('common:ok'),
          showCancelButton: false,
        });
      }
    },
    [issueId, queryClient, t, refreshWorkspaceLinks]
  );

  // Handle clicking a workspace card to open it
  const handleWorkspaceClick = useCallback(
    (localWorkspaceId: string | null) => {
      if (projectId && localWorkspaceId) {
        appNavigation.goToProjectIssueWorkspace(
          projectId,
          issueId,
          localWorkspaceId
        );
      }
    },
    [projectId, issueId, appNavigation]
  );

  // Handle unlinking a workspace from the issue
  const handleUnlinkWorkspace = useCallback(
    async (localWorkspaceId: string) => {
      const result = await ConfirmDialog.show({
        title: t('workspaces.unlinkFromIssue'),
        message: t('workspaces.unlinkConfirmMessage'),
        confirmText: t('workspaces.unlink'),
        variant: 'destructive',
      });

      if (result === 'confirmed') {
        try {
          await workspacesApi.unlinkFromIssue(localWorkspaceId);
          refreshWorkspaceLinks();
        } catch (error) {
          ConfirmDialog.show({
            title: t('common:error'),
            message:
              error instanceof Error
                ? error.message
                : t('workspaces.unlinkError'),
            confirmText: t('common:ok'),
            showCancelButton: false,
          });
        }
      }
    },
    [t, refreshWorkspaceLinks]
  );

  // Handle deleting a workspace (unlinks first, then deletes local)
  const handleDeleteWorkspace = useCallback(
    async (localWorkspaceId: string) => {
      const localWorkspace = localWorkspacesById.get(localWorkspaceId);
      if (!localWorkspace) {
        ConfirmDialog.show({
          title: t('common:error'),
          message: t('workspaces.deleteError'),
          confirmText: t('common:ok'),
          showCancelButton: false,
        });
        return;
      }

      const result = await DeleteWorkspaceDialog.show({
        branchName: localWorkspace.branch,
        hasOpenPR:
          workspacesWithStats
            .find(
              (workspace) => workspace.localWorkspaceId === localWorkspaceId
            )
            ?.prs.some((pr) => pr.status === 'open') ?? false,
        isLinkedToIssue: true,
        linkedIssueSimpleId: getIssue(issueId)?.simple_id,
      });

      if (result.action !== 'confirmed') {
        return;
      }

      try {
        await workspacesApi.delete(localWorkspaceId, result.deleteBranches);
        refreshWorkspaceLinks();
        // Unlink from remote after successful deletion
        if (result.unlinkFromIssue) {
          await workspacesApi.unlinkFromIssue(localWorkspaceId);
        }
      } catch (error) {
        ConfirmDialog.show({
          title: t('common:error'),
          message:
            error instanceof Error
              ? error.message
              : t('workspaces.deleteError'),
          confirmText: t('common:ok'),
          showCancelButton: false,
        });
      }
    },
    [
      localWorkspacesById,
      workspacesWithStats,
      t,
      issueId,
      getIssue,
      refreshWorkspaceLinks,
    ]
  );

  // Actions for the section header
  const actions: SectionAction[] = useMemo(
    () => [
      {
        icon: PlusIcon,
        onClick: handleAddWorkspace,
      },
      {
        icon: LinkIcon,
        onClick: handleLinkWorkspace,
      },
    ],
    [handleAddWorkspace, handleLinkWorkspace]
  );

  const composeContent =
    composeOpen && composeDraftId ? (
      <div className="relative flex max-h-[340px] min-h-[260px] flex-col rounded-sm border border-dashed border-border bg-primary overflow-hidden">
        <button
          type="button"
          onClick={handleComposeCancel}
          aria-label={t('buttons.cancel')}
          className="absolute right-2 top-2 z-10 p-1 rounded-sm text-low hover:text-high hover:bg-panel transition-colors"
        >
          <XIcon className="size-icon-sm" weight="bold" />
        </button>
        <div ref={composeScrollRef} className="flex-1 min-h-0 overflow-y-auto">
          <CreateModeProvider
            key={composeDraftId}
            draftId={composeDraftId}
            initialState={composeInitialState}
          >
            <CreateChatBoxContainer
              compact
              onWorkspaceCreated={handleComposeWorkspaceCreated}
            />
          </CreateModeProvider>
        </div>
      </div>
    ) : undefined;

  return (
    <IssueWorkspacesSection
      workspaces={workspacesWithStats}
      isLoading={isLoading}
      actions={actions}
      onWorkspaceClick={handleWorkspaceClick}
      onRunIssue={handleRunIssue}
      onCreateWorkspace={handleAddWorkspace}
      onUnlinkWorkspace={handleUnlinkWorkspace}
      onDeleteWorkspace={handleDeleteWorkspace}
      shouldAnimateCreateButton={shouldAnimateCreateButton}
      quickCreateContent={composeContent}
    />
  );
}
