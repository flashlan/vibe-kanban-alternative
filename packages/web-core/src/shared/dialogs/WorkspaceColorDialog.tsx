import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@vibe/ui/components/KeyboardDialog';
import { Button } from '@vibe/ui/components/Button';
import { InlineColorPicker } from '@vibe/ui/components/ColorPicker';
import { create, useModal } from '@ebay/nice-modal-react';
import { defineModal } from '@/shared/lib/modals';
import { cn } from '@/shared/lib/utils';

/**
 * Workspace color palette — tuned for the dark theme (all swatches have
 * high luminance so they read as row tints/text on dark backgrounds, and
 * stay distinguishable on light). HSL triples matching the project-tint
 * format (`hsl(H S% L% / alpha)`).
 */
export const WORKSPACE_PRESET_COLORS = [
  '8 90% 72%', // Coral
  '25 100% 70%', // Apricot
  '45 100% 66%', // Amber
  '90 65% 62%', // Pistachio
  '152 62% 60%', // Mint
  '174 62% 56%', // Teal
  '197 88% 64%', // Sky
  '222 85% 70%', // Periwinkle
  '262 88% 72%', // Lavender
  '292 78% 68%', // Orchid
  '328 88% 70%', // Rose
  '348 85% 66%', // Raspberry
] as const;

export interface WorkspaceColorDialogProps {
  workspaceName: string;
  currentColor: string | null;
  onSave: (color: string | null) => Promise<void>;
}

export type WorkspaceColorDialogResult = {
  action: 'saved' | 'canceled';
  color?: string | null;
};

const WorkspaceColorDialogImpl = create<WorkspaceColorDialogProps>(
  ({ workspaceName, currentColor, onSave }) => {
    const modal = useModal();
    const { t } = useTranslation('common');
    const [color, setColor] = useState<string | null>(currentColor);
    const [error, setError] = useState<string | null>(null);
    const [isSubmitting, setIsSubmitting] = useState(false);

    const handleSave = async () => {
      if (color === currentColor) {
        modal.resolve({
          action: 'canceled',
        } satisfies WorkspaceColorDialogResult);
        modal.hide();
        return;
      }
      setIsSubmitting(true);
      setError(null);
      try {
        await onSave(color);
        modal.resolve({
          action: 'saved',
          color,
        } satisfies WorkspaceColorDialogResult);
        modal.hide();
      } catch (err) {
        setError(
          err instanceof Error ? err.message : 'Failed to save workspace color'
        );
      } finally {
        setIsSubmitting(false);
      }
    };

    const handleCancel = () => {
      modal.resolve({
        action: 'canceled',
      } satisfies WorkspaceColorDialogResult);
      modal.hide();
    };

    const handleOpenChange = (open: boolean) => {
      if (!open) handleCancel();
    };

    return (
      <Dialog open={modal.visible} onOpenChange={handleOpenChange}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>
              {t('workspaces.color.title', 'Workspace color')}
            </DialogTitle>
            <DialogDescription>
              {t(
                'workspaces.color.description',
                'Pick a color for “{{name}}” in the sidebar tree. Choose Default to fall back to the project color.',
                { name: workspaceName }
              )}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4">
            <InlineColorPicker
              value={color ?? ''}
              onChange={(c) => {
                setColor(c);
                setError(null);
              }}
              colors={WORKSPACE_PRESET_COLORS}
              disabled={isSubmitting}
            />
            <button
              type="button"
              onClick={() => setColor(null)}
              disabled={isSubmitting}
              className={cn(
                'flex items-center gap-2 rounded-sm border px-2 py-1.5 text-sm transition-colors cursor-pointer',
                color === null
                  ? 'border-brand text-high bg-brand/10'
                  : 'border-border text-normal hover:bg-secondary'
              )}
            >
              <span
                aria-hidden
                className="size-4 rounded-full border border-border bg-tertiary"
              />
              {t('workspaces.color.default', 'Default')}
            </button>
            {error && <p className="text-sm text-destructive">{error}</p>}
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={handleCancel}
              disabled={isSubmitting}
            >
              {t('buttons.cancel')}
            </Button>
            <Button onClick={() => void handleSave()} disabled={isSubmitting}>
              {t('workspaces.color.save', 'Save')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    );
  }
);

export const WorkspaceColorDialog = defineModal<
  WorkspaceColorDialogProps,
  WorkspaceColorDialogResult
>(WorkspaceColorDialogImpl);
