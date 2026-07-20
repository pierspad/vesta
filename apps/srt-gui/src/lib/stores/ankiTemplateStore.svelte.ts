import { t } from "$lib/i18n";
import { snackbar } from "$lib/stores/snackbarStore.svelte";
import {
  NOTE_TYPES_UPDATED_EVENT,
  defaultCardTemplates,
  defaultFieldNames,
  limitNoteTypeFieldValue,
  loadCardTemplates,
  loadFieldNames,
  resetCardTemplates,
  saveCardTemplates,
  saveFieldNames,
  loadActiveNoteTypeId,
  saveActiveNoteTypeId,
  type FieldNamesConfig,
} from "$lib/types/noteTypes";
import * as vestaConfig from "$lib/config/vestaConfig";

const ANKI_FIELD_PRESETS_KEY = "vesta-anki-field-presets";
const ACTIVE_ANKI_FIELD_PRESET_KEY = "vesta-active-anki-field-preset";

export type AnkiFieldKey = keyof FieldNamesConfig;
export type AnkiFieldPreset = {
  id: string;
  name: string;
  noteTypeName: string;
  fields: FieldNamesConfig;
};

export type TemplateCodeTab = "front" | "back" | "css";

function loadStoredValue(key: string, fallback = ""): string {
  try {
    return vestaConfig.getItem(key) || fallback;
  } catch {
    return fallback;
  }
}

function sanitizeAnkiFieldPreset(raw: Partial<AnkiFieldPreset>): AnkiFieldPreset | null {
  if (!raw.id || !raw.name || !raw.fields) return null;
  const fields = raw.fields as Partial<FieldNamesConfig>;
  return {
    id: raw.id,
    name: limitNoteTypeFieldValue(raw.name),
    noteTypeName: limitNoteTypeFieldValue(raw.noteTypeName || defaultCardTemplates.noteTypeName),
    fields: {
      expression: limitNoteTypeFieldValue(fields.expression !== undefined && fields.expression !== "" ? fields.expression : defaultFieldNames.expression),
      meaning: limitNoteTypeFieldValue(fields.meaning !== undefined ? fields.meaning : defaultFieldNames.meaning),
      reading: limitNoteTypeFieldValue(fields.reading !== undefined ? fields.reading : defaultFieldNames.reading),
      audio: limitNoteTypeFieldValue(fields.audio !== undefined ? fields.audio : defaultFieldNames.audio),
      snapshot: limitNoteTypeFieldValue(fields.snapshot !== undefined ? fields.snapshot : defaultFieldNames.snapshot),
      video: limitNoteTypeFieldValue(fields.video !== undefined ? fields.video : defaultFieldNames.video),
      tags: limitNoteTypeFieldValue(fields.tags !== undefined ? fields.tags : defaultFieldNames.tags),
      sequenceMarker: limitNoteTypeFieldValue(fields.sequenceMarker !== undefined ? fields.sequenceMarker : defaultFieldNames.sequenceMarker),
      notes: limitNoteTypeFieldValue(fields.notes !== undefined ? fields.notes : defaultFieldNames.notes),
    },
  };
}

function loadAnkiFieldPresets(): AnkiFieldPreset[] {
  try {
    const raw = vestaConfig.getItem(ANKI_FIELD_PRESETS_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed
      .map((preset) => sanitizeAnkiFieldPreset(preset))
      .filter((preset): preset is AnkiFieldPreset => Boolean(preset));
  } catch {
    return [];
  }
}

const initTemplates = loadCardTemplates();
const initFieldNames = loadFieldNames();
const initAnkiFieldPresets = loadAnkiFieldPresets();
const initAnkiFieldPresetId = loadStoredValue(ACTIVE_ANKI_FIELD_PRESET_KEY, "default");
const initSelectedAnkiFieldPreset = initAnkiFieldPresets.find(
  (preset) => preset.id === initAnkiFieldPresetId,
);

/** Anki note-type field mapping + card-template (front/back/css) editor.
 * Self-contained within SettingsTab.svelte's "anki" section -- state and
 * save/load/preset logic that used to be scattered across ~400 lines of
 * script plus its own markup block. Nothing outside this feature reads any
 * of it (verified via grep before moving), unlike e.g. apiKeyEditorStore
 * which genuinely has two external trigger sites. */
class AnkiTemplateStore {
  activeTemplateCodeTab = $state<TemplateCodeTab>("front");

  templateFrontHtml = $state(initTemplates.frontHtml);
  templateBackHtml = $state(initTemplates.backHtml);
  templateCss = $state(initTemplates.css);
  noteTypeName = $state(initTemplates.noteTypeName);

  fieldExpression = $state(initFieldNames.expression);
  fieldMeaning = $state(initFieldNames.meaning);
  fieldReading = $state(initFieldNames.reading);
  fieldAudio = $state(initFieldNames.audio);
  fieldSnapshot = $state(initFieldNames.snapshot);
  fieldVideo = $state(initFieldNames.video);
  fieldTags = $state(initFieldNames.tags);
  fieldSequenceMarker = $state(initFieldNames.sequenceMarker);
  fieldNotes = $state(initFieldNames.notes);

  savedAnkiFieldPresets = $state<AnkiFieldPreset[]>(initAnkiFieldPresets);
  selectedAnkiFieldPresetId = $state(initSelectedAnkiFieldPreset?.id || "default");
  ankiFieldPresetName = $state(initSelectedAnkiFieldPreset?.name || "");
  activeNoteTypeId = $state(loadActiveNoteTypeId());

  allAnkiFieldPresets = $derived<AnkiFieldPreset[]>([
    {
      id: "default",
      name: "Default_Vesta",
      noteTypeName: defaultCardTemplates.noteTypeName,
      fields: defaultFieldNames,
    },
    ...this.savedAnkiFieldPresets,
  ]);

  refreshActiveNoteTypeId() {
    this.activeNoteTypeId = loadActiveNoteTypeId();
  }

  saveTemplates() {
    saveCardTemplates({
      frontHtml: this.templateFrontHtml,
      backHtml: this.templateBackHtml,
      css: this.templateCss,
      noteTypeName: this.noteTypeName,
    });
  }

  syncTemplateStateFromStorage() {
    const templates = loadCardTemplates();
    this.noteTypeName = templates.noteTypeName;
  }

  getCurrentFieldNames(): FieldNamesConfig {
    return {
      expression: this.fieldExpression,
      meaning: this.fieldMeaning,
      reading: this.fieldReading,
      audio: this.fieldAudio,
      snapshot: this.fieldSnapshot,
      video: this.fieldVideo,
      tags: this.fieldTags,
      sequenceMarker: this.fieldSequenceMarker,
      notes: this.fieldNotes,
    };
  }

  setCurrentFieldNames(fields: FieldNamesConfig) {
    this.fieldExpression = fields.expression;
    this.fieldMeaning = fields.meaning;
    this.fieldReading = fields.reading;
    this.fieldAudio = fields.audio;
    this.fieldSnapshot = fields.snapshot;
    this.fieldVideo = fields.video;
    this.fieldTags = fields.tags;
    this.fieldSequenceMarker = fields.sequenceMarker;
    this.fieldNotes = fields.notes;
  }

  getFieldValue(key: AnkiFieldKey): string {
    return this.getCurrentFieldNames()[key];
  }

  getFieldVariable(field: { key: AnkiFieldKey; variable: string }): string {
    const fieldName = this.getFieldValue(field.key).trim() || field.variable.slice(2, -2);
    return `{{${fieldName}}}`;
  }

  setFieldValue(key: AnkiFieldKey, value: string) {
    const fields = this.getCurrentFieldNames();
    fields[key] = value;
    this.setCurrentFieldNames(fields);
  }

  saveFields() {
    saveFieldNames(this.getCurrentFieldNames());
  }

  syncFieldStateFromStorage() {
    this.setCurrentFieldNames(loadFieldNames());
  }

  private persistAnkiFieldPresets() {
    vestaConfig.setItem(ANKI_FIELD_PRESETS_KEY, JSON.stringify(this.savedAnkiFieldPresets));
  }

  applyAnkiFieldPreset(presetId: string) {
    const preset = this.allAnkiFieldPresets.find((item) => item.id === presetId);
    if (!preset) return;
    this.selectedAnkiFieldPresetId = preset.id;
    this.ankiFieldPresetName = preset.id === "default" ? "" : preset.name;
    vestaConfig.setItem(ACTIVE_ANKI_FIELD_PRESET_KEY, preset.id);
    this.noteTypeName = preset.noteTypeName;
    this.setCurrentFieldNames(preset.fields);
    this.saveTemplates();
    this.saveFields();
    window.dispatchEvent(new CustomEvent(NOTE_TYPES_UPDATED_EVENT));
  }

  saveCurrentAnkiFieldPreset() {
    const existingPreset = this.savedAnkiFieldPresets.find((preset) => preset.id === this.selectedAnkiFieldPresetId);
    const fallbackName = existingPreset?.name || this.noteTypeName || "Template Anki";
    const presetName = limitNoteTypeFieldValue((this.ankiFieldPresetName || fallbackName).trim());
    const preset: AnkiFieldPreset = {
      id: existingPreset?.id || `custom-${Date.now().toString(36)}`,
      name: presetName,
      noteTypeName: limitNoteTypeFieldValue(this.noteTypeName || defaultCardTemplates.noteTypeName),
      fields: this.getCurrentFieldNames(),
    };

    if (existingPreset) {
      this.savedAnkiFieldPresets = this.savedAnkiFieldPresets.map((item) =>
        item.id === existingPreset.id ? preset : item,
      );
    } else {
      this.savedAnkiFieldPresets = [...this.savedAnkiFieldPresets, preset];
    }

    this.selectedAnkiFieldPresetId = preset.id;
    this.ankiFieldPresetName = preset.name;
    vestaConfig.setItem(ACTIVE_ANKI_FIELD_PRESET_KEY, preset.id);
    this.persistAnkiFieldPresets();
    window.dispatchEvent(new CustomEvent(NOTE_TYPES_UPDATED_EVENT));
    snackbar.show(t("settings.anki.presetSaved"));
  }

  deleteCurrentAnkiFieldPreset() {
    if (this.selectedAnkiFieldPresetId === "default") return;
    const deletedId = this.selectedAnkiFieldPresetId;
    this.savedAnkiFieldPresets = this.savedAnkiFieldPresets.filter((preset) => preset.id !== deletedId);
    this.persistAnkiFieldPresets();

    // Reset activeNoteTypeId to default if it was deleted
    const currentActiveId = loadActiveNoteTypeId();
    const formattedDeletedId = deletedId.startsWith("custom:") ? deletedId : `custom:${deletedId}`;
    const formattedActiveId = currentActiveId.startsWith("custom:") ? currentActiveId : `custom:${currentActiveId}`;
    if (formattedActiveId === formattedDeletedId) {
      saveActiveNoteTypeId("default");
      this.activeNoteTypeId = "default";
    }

    this.applyAnkiFieldPreset("default");
    window.dispatchEvent(new CustomEvent(NOTE_TYPES_UPDATED_EVENT));
    snackbar.show(t("settings.anki.presetDeleted"));
  }

  private resetAnkiFieldsToDefault() {
    this.selectedAnkiFieldPresetId = "default";
    this.ankiFieldPresetName = "";
    vestaConfig.setItem(ACTIVE_ANKI_FIELD_PRESET_KEY, "default");
    this.noteTypeName = defaultCardTemplates.noteTypeName;
    this.setCurrentFieldNames(defaultFieldNames);
    this.saveTemplates();
    this.saveFields();
    window.dispatchEvent(new CustomEvent(NOTE_TYPES_UPDATED_EVENT));
  }

  /** Full reset, triggered from SettingsTab.svelte's shared reset-confirm dialog. */
  resetAll() {
    const defaults = resetCardTemplates();
    this.templateFrontHtml = defaults.frontHtml;
    this.templateBackHtml = defaults.backHtml;
    this.templateCss = defaults.css;
    this.noteTypeName = defaults.noteTypeName;
    this.resetAnkiFieldsToDefault();
    snackbar.show(t("settings.anki.resetSuccess"), "info", 1300);
  }
}

export const ankiTemplateStore = new AnkiTemplateStore();
