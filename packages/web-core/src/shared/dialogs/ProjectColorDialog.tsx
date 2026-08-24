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
import { DARK_THEME_PRESET_COLORS } from '@/shared/lib/colors';

export interface ProjectColorDialogProps {
  projectName: string;
  currentColor: string;
  onSave: (color: string) => Promise<void>;
}

export type ProjectColorDialogResult = {
  action: 'saved' | 'canceled';
  color?: string;
};

const ProjectColorDialogImpl = create<ProjectColorDialogProps>(
  ({ projectName, currentColor, onSave }) => {
    const modal = useModal();
    const { t } = useTranslation('common');
    const [color, setColor] = useState<string>(currentColor);
    const [error, setError] = useState<string | null>(null);
    const [isSubmitting, setIsSubmitting] = useState(false);

    const handleSave = async () => {
      if (color === currentColor) {
        modal.resolve({
          action: 'canceled',
        } satisfies ProjectColorDialogResult);
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
        } satisfies ProjectColorDialogResult);
        modal.hide();
      } catch (err) {
        setError(
          err instanceof Error ? err.message : 'Failed to save project color'
        );
      } finally {
        setIsSubmitting(false);
      }
    };

    const handleCancel = () => {
      modal.resolve({
        action: 'canceled',
      } satisfies ProjectColorDialogResult);
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
              {t('projectColor.title', 'Project Color')}
            </DialogTitle>
            <DialogDescription>
              {t(
                'projectColor.description',
                'Pick a new color for “{{name}}” — it tints the project row and its whole subtree in the sidebar tree.',
                { name: projectName }
              )}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4">
            <InlineColorPicker
              value={color}
              onChange={(c) => {
                setColor(c);
                setError(null);
              }}
              colors={DARK_THEME_PRESET_COLORS}
              disabled={isSubmitting}
            />
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
              {t('projectColor.save', 'Save')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    );
  }
);

export const ProjectColorDialog = defineModal<
  ProjectColorDialogProps,
  ProjectColorDialogResult
>(ProjectColorDialogImpl);
