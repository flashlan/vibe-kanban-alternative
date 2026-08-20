import { useMemo, useEffect, useState, useCallback } from 'react';
import Form from '@rjsf/core';
import type { IChangeEvent } from '@rjsf/core';
import { RJSFValidationError } from '@rjsf/utils';
import validator from '@rjsf/validator-ajv8';
import { useTranslation } from 'react-i18next';
import { BaseCodingAgent } from 'shared/types';
import type { ModelListModel } from '@vibe/ui/components/ModelList';
import { settingsRjsfTheme } from './rjsf/theme';
import { SettingsSaveBar } from './SettingsComponents';
import { useModelSelectorConfig } from '@/shared/hooks/useExecutorDiscovery';
import { configApi } from '@/shared/lib/api';

interface ExecutorConfigFormProps {
  executor: BaseCodingAgent;
  value: unknown;
  onChange?: (formData: unknown) => void;
  onSave?: (formData: unknown) => Promise<void>;
  onDiscard?: () => void;
  disabled?: boolean;
  saving?: boolean;
  isDirty?: boolean;
}

import schemas from 'virtual:executor-schemas';

export function ExecutorConfigForm({
  executor,
  value,
  onChange,
  onSave,
  onDiscard,
  disabled = false,
  saving = false,
  isDirty = false,
}: ExecutorConfigFormProps) {
  const { t } = useTranslation('settings');
  const [formData, setFormData] = useState<unknown>(value || {});
  const [validationErrors, setValidationErrors] = useState<
    RJSFValidationError[]
  >([]);

  const { config: discoveredConfig, loadingModels: streamLoading } =
    useModelSelectorConfig(executor);
  const [httpModels, setHttpModels] = useState<ModelListModel[]>([]);
  const [httpLoading, setHttpLoading] = useState(false);

  useEffect(() => {
    if (!executor) return;
    let cancelled = false;
    setHttpLoading(true);
    configApi
      .getAgentModels(executor)
      .then((res) => {
        if (!cancelled && res && res.length > 0) {
          setHttpModels(
            res.map((m) => ({
              id: m.id,
              name: m.name || m.id,
              provider_id: m.provider ?? null,
              reasoning_options: [],
            }))
          );
        }
      })
      .catch(() => {})
      .finally(() => {
        if (!cancelled) setHttpLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [executor]);

  // The WebSocket discovery stream (`discoveredConfig`) is the primary
  // source — it carries `providers` for grouping. The HTTP poll
  // (`httpModels`) is a fallback with no provider list, so it always
  // renders as a flat (ungrouped) list in the picker.
  const modelSelector = useMemo(() => {
    if (discoveredConfig?.models && discoveredConfig.models.length > 0) {
      return {
        models: discoveredConfig.models,
        providers: discoveredConfig.providers,
      };
    }
    return { models: httpModels, providers: [] };
  }, [discoveredConfig, httpModels]);

  const loadingModels =
    streamLoading && httpLoading && modelSelector.models.length === 0;

  const baseSchema = useMemo(() => {
    return schemas[executor];
  }, [executor]);

  // Custom handler for env field updates
  const handleEnvChange = useCallback(
    (envData: Record<string, string> | undefined) => {
      const newFormData = {
        ...(formData as Record<string, unknown>),
        env: envData,
      };
      setFormData(newFormData);
      if (onChange) {
        onChange(newFormData);
      }
    },
    [formData, onChange]
  );

  const uiSchema = useMemo(
    () => ({
      env: {
        'ui:field': 'KeyValueField',
      },
      model: {
        'ui:widget': 'ModelSelectWidget',
      },
      default_model: {
        'ui:widget': 'ModelSelectWidget',
      },
    }),
    []
  );

  // Pass the env update handler and the discovered model list/providers via
  // formContext — ModelSelectWidget (registered in rjsf/theme.ts) reads
  // `modelSelector` from here to render the same provider-grouped, searchable
  // picker used when creating a workspace, instead of a flat native select.
  const formContext = useMemo(
    () => ({
      onEnvChange: handleEnvChange,
      modelSelector,
    }),
    [handleEnvChange, modelSelector]
  );

  useEffect(() => {
    setFormData(value || {});
    setValidationErrors([]);
  }, [value, executor]);

  const handleChange = (event: IChangeEvent<unknown>) => {
    const newFormData = event.formData;
    setFormData(newFormData);
    if (onChange) {
      onChange(newFormData);
    }
  };

  const handleSave = async () => {
    if (onSave) {
      await onSave(formData);
    }
  };

  const handleError = (errors: RJSFValidationError[]) => {
    setValidationErrors(errors);
  };

  if (!baseSchema) {
    return (
      <div className="bg-error/10 border border-error/50 rounded-sm p-4 text-error">
        {t('settings.agents.errors.schemaNotFound', { executor })}
      </div>
    );
  }

  const hasValidationErrors = validationErrors.length > 0;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between bg-panel/60 border border-border px-3 py-1.5 rounded-sm text-xs">
        <span className="text-low">Live CLI Discovery:</span>
        {loadingModels ? (
          <span className="text-yellow-500 animate-pulse font-mono flex items-center gap-1">
            <span className="inline-block size-1.5 rounded-full bg-yellow-500 animate-ping" />
            Querying {executor} models...
          </span>
        ) : modelSelector.models.length > 0 ? (
          <span className="text-green-500 font-medium font-mono">
            ● {modelSelector.models.length} models discovered from {executor}
          </span>
        ) : (
          <span className="text-low font-mono">
            No live models — type a model id below
          </span>
        )}
      </div>

      <Form
        schema={baseSchema}
        uiSchema={uiSchema}
        formData={formData}
        formContext={formContext}
        onChange={handleChange}
        onError={handleError}
        validator={validator}
        disabled={disabled}
        liveValidate
        showErrorList={false}
        widgets={settingsRjsfTheme.widgets}
        templates={settingsRjsfTheme.templates}
        fields={settingsRjsfTheme.fields}
      >
        {/* No submit button - SettingsSaveBar handles saving */}
        <></>
      </Form>

      {hasValidationErrors && (
        <div className="bg-error/10 border border-error/50 rounded-sm p-4 text-error">
          <ul className="list-disc list-inside space-y-1">
            {validationErrors.map((error, index) => (
              <li key={index}>
                {error.property}: {error.message}
              </li>
            ))}
          </ul>
        </div>
      )}

      {onSave && (
        <SettingsSaveBar
          show={isDirty}
          saving={saving}
          saveDisabled={hasValidationErrors}
          unsavedMessage={t('settings.agents.save.unsavedChanges')}
          onSave={handleSave}
          onDiscard={onDiscard}
        />
      )}
    </div>
  );
}
