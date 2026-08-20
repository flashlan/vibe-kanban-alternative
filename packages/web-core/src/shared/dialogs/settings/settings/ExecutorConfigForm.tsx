import { useMemo, useEffect, useState, useCallback } from 'react';
import Form from '@rjsf/core';
import type { IChangeEvent } from '@rjsf/core';
import { RJSFValidationError } from '@rjsf/utils';
import validator from '@rjsf/validator-ajv8';
import { useTranslation } from 'react-i18next';
import { BaseCodingAgent } from 'shared/types';
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
  const [httpModels, setHttpModels] = useState<
    Array<{ id: string; name: string; provider?: string }>
  >([]);
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
              provider: m.provider,
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

  const allDiscoveredModels = useMemo(() => {
    if (discoveredConfig?.models && discoveredConfig.models.length > 0) {
      return discoveredConfig.models.map((m) => ({
        id: m.id,
        name: m.name || m.id,
        provider: m.provider_id,
      }));
    }
    return httpModels;
  }, [discoveredConfig?.models, httpModels]);

  const loadingModels =
    streamLoading && httpLoading && allDiscoveredModels.length === 0;

  const baseSchema = useMemo(() => {
    return schemas[executor];
  }, [executor]);

  const dynamicSchema = useMemo(() => {
    if (!baseSchema) return null;
    if (allDiscoveredModels.length === 0) {
      return baseSchema;
    }

    try {
      const cloned = JSON.parse(JSON.stringify(baseSchema));
      const modelIds = allDiscoveredModels.map((m) => m.id);
      const modelNames = allDiscoveredModels.map((m) => m.name || m.id);

      if (cloned.properties?.model) {
        cloned.properties.model.enum = [null, ...modelIds];
        cloned.properties.model.enumNames = [
          'Default (omit / latest)',
          ...modelNames,
        ];
      }
      if (cloned.properties?.default_model) {
        cloned.properties.default_model.enum = [null, ...modelIds];
        cloned.properties.default_model.enumNames = [
          'Default (omit / latest)',
          ...modelNames,
        ];
      }
      return cloned;
    } catch {
      return baseSchema;
    }
  }, [baseSchema, allDiscoveredModels]);

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
    }),
    []
  );

  // Pass the env update handler via formContext
  const formContext = useMemo(
    () => ({
      onEnvChange: handleEnvChange,
    }),
    [handleEnvChange]
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

  if (!dynamicSchema) {
    return (
      <div className="bg-error/10 border border-error/50 rounded-sm p-4 text-error">
        {t('settings.agents.errors.schemaNotFound', { executor })}
      </div>
    );
  }

  const hasValidationErrors = validationErrors.length > 0;
  const currentModel = (formData as Record<string, unknown>)?.model;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between bg-panel/60 border border-border px-3 py-1.5 rounded-sm text-xs">
        <span className="text-low">Live CLI Discovery:</span>
        {loadingModels ? (
          <span className="text-yellow-500 animate-pulse font-mono flex items-center gap-1">
            <span className="inline-block size-1.5 rounded-full bg-yellow-500 animate-ping" />
            Querying {executor} models...
          </span>
        ) : allDiscoveredModels.length > 0 ? (
          <span className="text-green-500 font-medium font-mono">
            ● {allDiscoveredModels.length} models discovered from {executor}
          </span>
        ) : (
          <span className="text-low font-mono">Using default CLI config</span>
        )}
      </div>

      {allDiscoveredModels.length > 0 ? (
        <div className="bg-panel/40 border border-border/60 p-3 rounded-sm space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold text-high uppercase tracking-wide">
              Discovered Models ({allDiscoveredModels.length})
            </span>
            <span className="text-[11px] text-low">
              Click to select model for this profile
            </span>
          </div>
          <div className="flex flex-wrap gap-1.5">
            <button
              type="button"
              onClick={() => {
                const updated = {
                  ...(formData as Record<string, unknown>),
                  model: null,
                };
                setFormData(updated);
                onChange?.(updated);
              }}
              className={`px-2 py-1 rounded-sm text-xs border transition-colors ${
                !currentModel
                  ? 'bg-brand/20 border-brand text-brand font-medium'
                  : 'bg-secondary/60 border-border text-low hover:text-high'
              }`}
            >
              Default (Latest)
            </button>
            {allDiscoveredModels.map((m) => {
              const isSelected = currentModel === m.id;
              return (
                <button
                  key={m.id}
                  type="button"
                  onClick={() => {
                    const updated = {
                      ...(formData as Record<string, unknown>),
                      model: m.id,
                    };
                    setFormData(updated);
                    onChange?.(updated);
                  }}
                  className={`px-2 py-1 rounded-sm text-xs border transition-colors ${
                    isSelected
                      ? 'bg-brand/20 border-brand text-brand font-medium'
                      : 'bg-secondary/60 border-border text-low hover:text-high'
                  }`}
                >
                  {m.name || m.id}
                </button>
              );
            })}
          </div>
        </div>
      ) : null}

      <Form
        schema={dynamicSchema}
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
