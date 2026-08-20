import { useState } from 'react';
import type { WidgetProps } from '@rjsf/utils';
import { useTranslation } from 'react-i18next';
import { ModelSelectorPopover } from '@vibe/ui/components/ModelSelectorPopover';
import type { ModelListModel } from '@vibe/ui/components/ModelList';
import { DropdownMenuTriggerButton } from '@vibe/ui/components/Dropdown';
import { useTheme, getResolvedTheme } from '@/shared/hooks/useTheme';
import { TextWidget } from './Widgets';

export interface ModelSelectFormContext {
  modelSelector?: {
    models: ModelListModel[];
    providers: { id: string; name: string }[];
  };
}

/// Provider-grouped, searchable model picker for the `model`/`default_model`
/// fields of a coding-agent-CLI config form — the same `ModelSelectorPopover`
/// used to pick a model when creating a workspace, so the two surfaces don't
/// diverge. Falls back to a plain text input when this CLI has no live
/// discovered models (e.g. not installed, or discovery unavailable) so a
/// model id can still be typed in by hand.
export const ModelSelectWidget = (props: WidgetProps) => {
  const { id, value, disabled, readonly, onChange, registry } = props;
  const { t } = useTranslation('common');
  const [isOpen, setIsOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [expandedProviderId, setExpandedProviderId] = useState('');
  const { theme } = useTheme();
  const resolvedTheme = getResolvedTheme(theme);

  const formContext = registry.formContext as
    | ModelSelectFormContext
    | undefined;
  const models = formContext?.modelSelector?.models ?? [];
  const providers = formContext?.modelSelector?.providers ?? [];

  if (models.length === 0) {
    return <TextWidget {...props} />;
  }

  const selectedModelId: string | null = value ?? null;
  const selectedModel = selectedModelId
    ? (models.find((m) => m.id === selectedModelId) ?? null)
    : null;

  return (
    <ModelSelectorPopover
      isOpen={isOpen}
      onOpenChange={setIsOpen}
      trigger={
        <DropdownMenuTriggerButton
          id={id}
          label={
            selectedModel?.name ?? selectedModelId ?? t('modelSelector.default')
          }
          className="w-full justify-between"
          disabled={disabled || readonly}
        />
      }
      config={{ models, providers }}
      selectedProviderId={selectedModel?.provider_id ?? null}
      selectedModelId={selectedModelId}
      selectedReasoningId={null}
      searchQuery={searchQuery}
      onSearchChange={setSearchQuery}
      onModelSelect={(modelId) => {
        onChange(modelId);
        setIsOpen(false);
      }}
      onReasoningSelect={() => {}}
      showDefaultOption
      onSelectDefault={() => {
        onChange(undefined);
        setIsOpen(false);
      }}
      expandedProviderId={expandedProviderId}
      onExpandedProviderIdChange={setExpandedProviderId}
      resolvedTheme={resolvedTheme}
    />
  );
};
