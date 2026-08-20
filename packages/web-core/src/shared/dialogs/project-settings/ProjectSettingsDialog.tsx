import { useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@vibe/ui/components/KeyboardDialog';
import { create, useModal } from '@ebay/nice-modal-react';
import { useTranslation } from 'react-i18next';
import { FolderGit } from 'lucide-react';
import { defineModal } from '@/shared/lib/modals';
import { cn } from '@/shared/lib/utils';
import { ProjectRepoSection } from './ProjectRepoSection';

export type ProjectSettingsDialogProps = {
  projectId: string;
};

// A single section for now (repository). Structured as a list so more
// project-scoped sections (tags, danger zone, ...) can be added here without
// reworking the shell — see the "Edit project settings" menu item this
// backs, which previously opened the unrelated global Settings dialog.
type ProjectSettingsSectionId = 'repository';

const SECTIONS: {
  id: ProjectSettingsSectionId;
  labelKey: string;
  labelFallback: string;
  icon: typeof FolderGit;
}[] = [
  {
    id: 'repository',
    labelKey: 'projectSettingsDialog.sections.repository',
    labelFallback: 'Repository',
    icon: FolderGit,
  },
];

const ProjectSettingsDialogImpl = create<ProjectSettingsDialogProps>(
  ({ projectId }) => {
    const modal = useModal();
    const { t } = useTranslation('projects');
    const [activeSection, setActiveSection] =
      useState<ProjectSettingsSectionId>('repository');

    const handleClose = () => {
      modal.resolve();
      modal.hide();
    };

    return (
      <Dialog
        open={modal.visible}
        onOpenChange={(open) => !open && handleClose()}
      >
        <DialogContent className="sm:max-w-2xl p-0 overflow-hidden">
          <div className="flex h-[420px]">
            <nav className="w-48 shrink-0 border-r border-border bg-secondary/40 p-2 flex flex-col gap-1">
              <DialogHeader className="px-2 pt-2 pb-3">
                <DialogTitle className="text-base">
                  {t('projectSettingsDialog.title', 'Project Settings')}
                </DialogTitle>
              </DialogHeader>
              {SECTIONS.map((section) => {
                const Icon = section.icon;
                const isActive = activeSection === section.id;
                return (
                  <button
                    key={section.id}
                    type="button"
                    onClick={() => setActiveSection(section.id)}
                    className={cn(
                      'flex items-center gap-2 text-left px-3 py-2 rounded-sm text-sm transition-colors',
                      isActive
                        ? 'bg-brand/10 text-brand font-medium'
                        : 'text-normal hover:bg-primary/10'
                    )}
                  >
                    <Icon className="h-4 w-4 shrink-0" />
                    <span className="truncate">
                      {t(section.labelKey, section.labelFallback)}
                    </span>
                  </button>
                );
              })}
            </nav>
            <div className="flex-1 overflow-y-auto p-4">
              {activeSection === 'repository' && (
                <ProjectRepoSection projectId={projectId} />
              )}
            </div>
          </div>
        </DialogContent>
      </Dialog>
    );
  }
);

export const ProjectSettingsDialog = defineModal<
  ProjectSettingsDialogProps,
  void
>(ProjectSettingsDialogImpl);
