import { useTranslation } from 'react-i18next';
import { ProjectProvider } from '@/shared/providers/remote/ProjectProvider';
import { IssueWorkspacesSectionContainer } from '@/pages/kanban/IssueWorkspacesSectionContainer';
import { IssueRelationshipsSectionContainer } from '@/pages/kanban/IssueRelationshipsSectionContainer';
import { IssueSubIssuesSectionContainer } from '@/pages/kanban/IssueSubIssuesSectionContainer';
import { CollapsibleSectionHeader } from '@vibe/ui/components/CollapsibleSectionHeader';

interface CreateIssueLinkedSectionsProps {
  projectId: string;
  issueId: string | null;
  /** Called right before navigating away (e.g. once the Workspaces
   *  section's inline composer creates a workspace and jumps to its
   *  session view) so the enclosing modal can close itself first —
   *  otherwise it stays open on top of the destination. When a workspace
   *  was just created, its id is passed so the caller can fold it into
   *  its own resolution instead of racing the composer's navigation. */
  onNavigateAway?: (createdWorkspaceId?: string) => void;
}

function Placeholder({
  title,
  persistKey,
  hint,
}: {
  title: string;
  persistKey: string;
  hint: string;
}) {
  return (
    <CollapsibleSectionHeader
      title={title}
      persistKey={persistKey}
      defaultExpanded
    >
      <div className="p-base flex flex-col gap-half border-t">
        <p className="text-low py-half text-sm">{hint}</p>
      </div>
    </CollapsibleSectionHeader>
  );
}

/**
 * Workspaces + Relationships + Sub-issues for the Create Issue dialog.
 * Before the issue is saved these render as placeholders; after save they
 * render the EXACT same containers the right-side edit pane uses (wrapped in
 * a ProjectProvider, since the dialog lives above the route's provider), so
 * the two surfaces are behaviourally identical.
 */
export function CreateIssueLinkedSections({
  projectId,
  issueId,
  onNavigateAway,
}: CreateIssueLinkedSectionsProps) {
  const { t } = useTranslation('common');

  if (!issueId) {
    return (
      <div className="flex flex-col gap-base">
        <Placeholder
          title={t('kanban.workspaces', 'Workspaces')}
          persistKey="create-issue-workspaces"
          hint={t(
            'createIssueDialog.saveToAddWorkspaces',
            'Save the issue to add workspaces.'
          )}
        />
        <Placeholder
          title={t('kanban.relationships', 'Relationships')}
          persistKey="create-issue-relationships"
          hint={t(
            'createIssueDialog.saveToAddRelationships',
            'Save the issue to add relationships.'
          )}
        />
        <Placeholder
          title={t('kanban.subIssues', 'Sub-issues')}
          persistKey="create-issue-sub-issues"
          hint={t(
            'createIssueDialog.saveToAddSubIssues',
            'Save the issue to add sub-issues.'
          )}
        />
      </div>
    );
  }

  return (
    <ProjectProvider projectId={projectId}>
      <div className="flex flex-col gap-base">
        <IssueWorkspacesSectionContainer
          projectId={projectId}
          issueId={issueId}
          onNavigateAway={onNavigateAway}
        />
        <IssueRelationshipsSectionContainer
          projectId={projectId}
          issueId={issueId}
        />
        <IssueSubIssuesSectionContainer
          projectId={projectId}
          issueId={issueId}
        />
      </div>
    </ProjectProvider>
  );
}
