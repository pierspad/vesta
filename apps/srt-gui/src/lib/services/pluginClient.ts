import { invokeCommand } from "$lib/services/tauriClient";

export interface PluginSettingField {
  id: string;
  label: string;
  type: "text" | "password" | "checkbox" | "number";
  description?: string;
  defaultValue?: any;
  hint?: string | { text?: string; link?: { label: string; url: string } };
  helpText?: string;
  link?: { label: string; url: string };
  expertOnly?: boolean;
}

export interface PluginTab {
  title: string;
  icon?: string;
}

export interface PluginEntry {
  command: string[];
  os?: Record<string, string[]>;
}

export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  apiVersion: number;
  description?: string;
  longDescription?: string;
  readme?: string;
  author?: string;
  homepage?: string;
  repository?: string;
  entry: PluginEntry;
  permissions: string[];
  tab?: PluginTab;
  settings?: PluginSettingField[];
}

export interface ResolvedPluginSource {
  originalUrl: string;
  archiveUrl?: string;
  gitCloneUrl: string;
  gitRef: string;
}

export interface PluginInspectResult {
  manifest: PluginManifest;
  source: ResolvedPluginSource;
  readme?: string;
}

export interface InstalledPlugin {
  id: string;
  name: string;
  version: string;
  description?: string;
  author?: string;
  enabled: boolean;
  sourceUrl: string;
  sourceRef?: string;
  installedAt: string;
  updatedAt?: string;
  readme?: string;
  manifest: PluginManifest;
  settings: Record<string, any>;
}

export interface PluginEventPayload {
  screen?: string;
  widgetId: string;
  eventType: string;
  values: Record<string, any>;
  expertMode?: boolean;
}

export interface PluginPatch {
  values?: Record<string, any>;
  setProps?: Record<string, Record<string, any>>;
  visibility?: Record<string, boolean>;
  insert?: { parentId: string; index?: number; node: WidgetNode };
  remove?: { id: string };
  move?: { id: string; newParentId: string; newIndex?: number };
}

// Widget Schema definitions
export type WidgetType =
  | "column"
  | "row"
  | "text"
  | "heading"
  | "textInput"
  | "passwordInput"
  | "numberInput"
  | "select"
  | "checkbox"
  | "button"
  | "list"
  | "table"
  | "image"
  | "progress"
  | "fileOutput"
  | "divider"
  | "settings"
  | "link"
  | "externalLink"
  | "segmented"
  | "pathInput"
  | "languageSelect";

export interface BaseWidget {
  type: WidgetType;
  id?: string;
  hidden?: boolean;
  expertOnly?: boolean;
  width?: string | number;
  flex?: number;
}

export interface ColumnWidget extends BaseWidget {
  type: "column";
  gap?: number | string;
  children: WidgetNode[];
}

export interface RowWidget extends BaseWidget {
  type: "row";
  gap?: number | string;
  align?: "start" | "center" | "end" | "between";
  children: WidgetNode[];
}

export interface TextWidget extends BaseWidget {
  type: "text";
  text: string;
  variant?: "normal" | "muted" | "danger" | "success";
  size?: "sm" | "base" | "lg";
}

export interface HeadingWidget extends BaseWidget {
  type: "heading";
  text: string;
  level?: 1 | 2 | 3 | 4;
  icon?: string;
}

export interface TextInputWidget extends BaseWidget {
  type: "textInput";
  id: string;
  label?: string;
  placeholder?: string;
  defaultValue?: string;
  disabled?: boolean;
  icon?: string;
  hint?: string | { text?: string; link?: { label: string; url: string } };
  helpText?: string;
  link?: { label: string; url: string };
}

export interface PasswordInputWidget extends BaseWidget {
  type: "passwordInput";
  id: string;
  label?: string;
  placeholder?: string;
  defaultValue?: string;
  disabled?: boolean;
  icon?: string;
  hint?: string | { text?: string; link?: { label: string; url: string } };
  helpText?: string;
  link?: { label: string; url: string };
}

export interface NumberInputWidget extends BaseWidget {
  type: "numberInput";
  id: string;
  label?: string;
  min?: number;
  max?: number;
  step?: number;
  defaultValue?: number;
  disabled?: boolean;
  hint?: string | { text?: string; link?: { label: string; url: string } };
  helpText?: string;
  link?: { label: string; url: string };
}

export interface SelectWidget extends BaseWidget {
  type: "select";
  id: string;
  label?: string;
  options: Array<{ label: string; value: string }>;
  defaultValue?: string;
  disabled?: boolean;
  hint?: string | { text?: string; link?: { label: string; url: string } };
  helpText?: string;
  link?: { label: string; url: string };
}

export interface CheckboxWidget extends BaseWidget {
  type: "checkbox";
  id: string;
  label: string;
  defaultValue?: boolean;
  disabled?: boolean;
}

export interface ButtonWidget extends BaseWidget {
  type: "button";
  id: string;
  label: string;
  variant?: "primary" | "secondary" | "danger";
  disabled?: boolean;
  icon?: string;
}

export interface ListItemAction {
  id: string;
  label: string;
  variant?: "primary" | "secondary" | "danger";
}

export interface ListItem {
  id: string;
  title: string;
  subtitle?: string;
  badge?: string;
  image?: string;
  actions?: ListItemAction[];
}

export interface ListWidget extends BaseWidget {
  type: "list";
  items: ListItem[];
}

export interface TableWidget extends BaseWidget {
  type: "table";
  headers: string[];
  rows: Array<Array<string | number>>;
}

export interface ImageWidget extends BaseWidget {
  type: "image";
  src: string;
  alt?: string;
  width?: number;
  height?: number;
}

export interface ProgressWidget extends BaseWidget {
  type: "progress";
  value: number;
  max?: number;
  label?: string;
}

export interface FileOutputWidget extends BaseWidget {
  type: "fileOutput";
  path: string;
  label?: string;
}

export interface DividerWidget extends BaseWidget {
  type: "divider";
}

export interface SettingsWidget extends BaseWidget {
  type: "settings";
  fields: PluginSettingField[];
}

export interface LinkWidget extends BaseWidget {
  type: "link" | "externalLink";
  label: string;
  url: string;
  icon?: string;
}

export interface SegmentedOption {
  value: string;
  label: string;
  icon?: string;
  badge?: string;
}

export interface SegmentedWidget extends BaseWidget {
  type: "segmented";
  id: string;
  label?: string;
  options: SegmentedOption[];
  defaultValue?: string;
  disabled?: boolean;
}

export interface PathInputWidget extends BaseWidget {
  type: "pathInput";
  id: string;
  label?: string;
  placeholder?: string;
  defaultValue?: string;
  disabled?: boolean;
  pickerType?: "file" | "folder";
  filters?: Array<{ name: string; extensions: string[] }>;
  hint?: string | { text?: string; link?: { label: string; url: string } };
  helpText?: string;
  link?: { label: string; url: string };
}

export interface LanguageSelectWidget extends BaseWidget {
  type: "languageSelect";
  id: string;
  label?: string;
  placeholder?: string;
  defaultValue?: string;
  disabled?: boolean;
  hint?: string | { text?: string; link?: { label: string; url: string } };
  helpText?: string;
  link?: { label: string; url: string };
}

export type WidgetNode =
  | ColumnWidget
  | RowWidget
  | TextWidget
  | HeadingWidget
  | TextInputWidget
  | PasswordInputWidget
  | NumberInputWidget
  | SelectWidget
  | CheckboxWidget
  | ButtonWidget
  | ListWidget
  | TableWidget
  | ImageWidget
  | ProgressWidget
  | FileOutputWidget
  | DividerWidget
  | SettingsWidget
  | LinkWidget
  | SegmentedWidget
  | PathInputWidget
  | LanguageSelectWidget;

export async function pluginInspect(url: string, gitRef?: string): Promise<PluginInspectResult> {
  return invokeCommand<PluginInspectResult>("plugin_inspect", { url, gitRef });
}

export async function pluginInstall(url: string, gitRef?: string): Promise<InstalledPlugin> {
  return invokeCommand<InstalledPlugin>("plugin_install", { url, gitRef });
}

export async function pluginList(): Promise<InstalledPlugin[]> {
  return invokeCommand<InstalledPlugin[]>("plugin_list");
}

export async function pluginEnable(id: string): Promise<void> {
  return invokeCommand<void>("plugin_enable", { id });
}

export async function pluginDisable(id: string): Promise<void> {
  return invokeCommand<void>("plugin_disable", { id });
}

export async function pluginUninstall(id: string): Promise<void> {
  return invokeCommand<void>("plugin_uninstall", { id });
}

export async function pluginUpdate(id: string): Promise<InstalledPlugin> {
  return invokeCommand<InstalledPlugin>("plugin_update", { id });
}

export async function pluginGetSettings(id: string): Promise<Record<string, any>> {
  return invokeCommand<Record<string, any>>("plugin_get_settings", { id });
}

export async function pluginSaveSettings(id: string, settings: Record<string, any>): Promise<void> {
  return invokeCommand<void>("plugin_save_settings", { id, settings });
}

export async function pluginRender(
  pluginId: string,
  screen?: string,
  params?: any,
  locale?: string,
  expertMode?: boolean,
): Promise<{ root?: WidgetNode; [key: string]: any }> {
  return invokeCommand<{ root?: WidgetNode; [key: string]: any }>("plugin_render", {
    pluginId,
    screen,
    params,
    locale,
    expertMode,
  });
}

export async function pluginDispatchEvent(
  pluginId: string,
  event: PluginEventPayload,
  locale?: string,
  expertMode?: boolean,
): Promise<{ root?: WidgetNode; [key: string]: any }> {
  return invokeCommand<{ root?: WidgetNode; [key: string]: any }>("plugin_dispatch_event", {
    pluginId,
    event,
    locale,
    expertMode,
  });
}
