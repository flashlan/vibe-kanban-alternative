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

  const { config: discoveredConfig, loadingModels } =
    useModelSelectorConfig(executor);

  const baseSchema = useMemo(() => {
    return schemas[executor];
  }, [executor]);

  const dynamicSchema = useMemo(() => {
    if (!baseSchema) return null;
    if (!discoveredConfig?.models || discoveredConfig.models.length === 0) {
      return baseSchema;
    }

    try {
      const cloned = JSON.parse(JSON.stringify(baseSchema));
      const modelList = discoveredConfig.models;
      const modelIds = modelList.map((m) => m.id);
      const modelNames = modelList.map((m) => m.name || m.id);

      if (cloned.properties?.model) {
        cloned.properties.model.enum = modelIds;
        cloned.properties.model.enumNames = modelNames;
      }
      if (cloned.properties?.default_model) {
        cloned.properties.default_model.enum = modelIds;
        cloned.properties.default_model.enumNames = modelNames;
      }
      return cloned;
    } catch {
      return baseSchema;
    }
  }, [baseSchema, discoveredConfig?.models]);

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

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between bg-panel/60 border border-border px-3 py-1.5 rounded-sm text-xs">
        <span className="text-low">Live CLI Discovery:</span>
        {loadingModels ? (
          <span className="text-yellow-500 animate-pulse font-mono flex items-center gap-1">
            <span className="inline-block size-1.5 rounded-full bg-yellow-500 animate-ping" />
            Querying {executor} models...
          </span>
        ) : discoveredConfig?.models?.length ? (
          <span className="text-green-500 font-medium font-mono">
            ● {discoveredConfig.models.length} models discovered from {executor}
          </span>
        ) : (
          <span className="text-low font-mono">
            Static default schemas active
          </span>
        )}
      </div>

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
