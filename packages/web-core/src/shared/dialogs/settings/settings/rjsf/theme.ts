import { RegistryFieldsType, RegistryWidgetsType } from '@rjsf/utils';
import {
  TextWidget,
  SelectWidget,
  CheckboxWidget,
  TextareaWidget,
} from './Widgets.tsx';
import { ModelSelectWidget } from './ModelSelectWidget.tsx';
import {
  FieldTemplate,
  ObjectFieldTemplate,
  ArrayFieldTemplate,
  ArrayFieldItemTemplate,
  FormTemplate,
} from './Templates.tsx';
import { KeyValueField } from './Fields.tsx';

export const settingsWidgets: RegistryWidgetsType = {
  TextWidget,
  SelectWidget,
  CheckboxWidget,
  TextareaWidget,
  textarea: TextareaWidget,
  ModelSelectWidget,
};

export const settingsTemplates = {
  ArrayFieldTemplate,
  ArrayFieldItemTemplate,
  FieldTemplate,
  ObjectFieldTemplate,
  FormTemplate,
};

export const settingsFields: RegistryFieldsType = {
  KeyValueField,
};

export const settingsRjsfTheme = {
  widgets: settingsWidgets,
  templates: settingsTemplates,
  fields: settingsFields,
};
