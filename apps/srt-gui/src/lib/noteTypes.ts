import * as vestaConfig from "./vestaConfig";

// Note-type system: field names, note-type definitions (predefined +
// custom), and card templates. Kept together (not split further) because
// NoteTypeDef.fields is a FieldNamesConfig, sanitizeCustomNoteType/
// loadCustomNoteTypes depend on sanitizeFieldNamesConfig/defaultFieldNames,
// and CardTemplateConfig.noteTypeName ties card templates to the same
// concept — genuinely one cohesive module, unlike the adjacent-but-
// unrelated sections elsewhere in the original models.ts.

export interface CardTemplateConfig {
  frontHtml: string;
  backHtml: string;
  css: string;
  noteTypeName: string;
}

export interface FieldNamesConfig {
  expression: string;
  meaning: string;
  reading: string;
  audio: string;
  snapshot: string;
  video: string;
  tags: string;
  sequenceMarker: string;
  notes: string;
}

const CARD_TEMPLATE_KEY = "vesta-card-templates";
const FIELD_NAMES_KEY = "vesta-field-names";

export const NOTE_TYPE_FIELD_SOFT_LIMIT = 25;
export const CARD_TEMPLATES_UPDATED_EVENT = "vesta:card-templates-updated";
export const FIELD_NAMES_UPDATED_EVENT = "vesta:field-names-updated";

export function limitNoteTypeFieldValue(value: string): string {
  return value.slice(0, NOTE_TYPE_FIELD_SOFT_LIMIT);
}

function sanitizeFieldNamesConfig(config: FieldNamesConfig): FieldNamesConfig {
  return {
    expression: "Expression",
    meaning: limitNoteTypeFieldValue(config.meaning),
    reading: limitNoteTypeFieldValue(config.reading),
    audio: limitNoteTypeFieldValue(config.audio),
    snapshot: limitNoteTypeFieldValue(config.snapshot),
    video: limitNoteTypeFieldValue(config.video),
    tags: limitNoteTypeFieldValue(config.tags),
    sequenceMarker: "SequenceMarker",
    notes: limitNoteTypeFieldValue(config.notes),
  };
}

function sanitizeCardTemplateConfig(config: CardTemplateConfig): CardTemplateConfig {
  return {
    ...config,
    noteTypeName: limitNoteTypeFieldValue(config.noteTypeName),
  };
}

function dispatchWindowEvent(eventName: string) {
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent(eventName));
  }
}

export const defaultFieldNames: FieldNamesConfig = {
  expression: "Expression",
  meaning: "Meaning",
  reading: "Reading",
  audio: "Audio",
  snapshot: "Snapshot",
  video: "Video",
  tags: "Tags",
  sequenceMarker: "SequenceMarker",
  notes: "Notes",
};

export function loadFieldNames(): FieldNamesConfig {
  try {
    const raw = vestaConfig.getItem(FIELD_NAMES_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return sanitizeFieldNamesConfig({
        expression: parsed.expression !== undefined && parsed.expression !== "" ? parsed.expression : defaultFieldNames.expression,
        meaning: parsed.meaning !== undefined ? parsed.meaning : defaultFieldNames.meaning,
        reading: parsed.reading !== undefined ? parsed.reading : defaultFieldNames.reading,
        audio: parsed.audio !== undefined ? parsed.audio : defaultFieldNames.audio,
        snapshot: parsed.snapshot !== undefined ? parsed.snapshot : defaultFieldNames.snapshot,
        video: parsed.video !== undefined ? parsed.video : defaultFieldNames.video,
        tags: parsed.tags !== undefined ? parsed.tags : defaultFieldNames.tags,
        sequenceMarker: parsed.sequenceMarker !== undefined ? parsed.sequenceMarker : defaultFieldNames.sequenceMarker,
        notes: parsed.notes !== undefined ? parsed.notes : defaultFieldNames.notes,
      });
    }
  } catch { /* ignore */ }
  return sanitizeFieldNamesConfig({ ...defaultFieldNames });
}

export function saveFieldNames(config: FieldNamesConfig): void {
  const sanitized = sanitizeFieldNamesConfig(config);
  vestaConfig.setItem(FIELD_NAMES_KEY, JSON.stringify(sanitized));
  dispatchWindowEvent(FIELD_NAMES_UPDATED_EVENT);
}

export function resetFieldNames(): FieldNamesConfig {
  vestaConfig.removeItem(FIELD_NAMES_KEY);
  dispatchWindowEvent(FIELD_NAMES_UPDATED_EVENT);
  return sanitizeFieldNamesConfig({ ...defaultFieldNames });
}

// ─── Note Types ──────────────────────────────────────────────────────────────
//
// A "note type" is the unit the user selects when generating flashcards. It is a
// name + the nine canonical fields + which of them are active. There are two
// kinds:
//
//   • predefined — one per study language (e.g. "English_Vesta"). Locked: always
//     all nine fields, with the canonical names. Generated on the fly from the
//     languages list, never stored.
//   • custom — created by the user (typically by forking a predefined one),
//     stored in localStorage. May switch fields off to get a smaller schema.
//
// The Rust exporter keys the Anki model id off the note type *name* and emits
// exactly the active fields, so as long as a given name always carries the same
// active set (predefined are locked; a custom saves its own fixed set) Anki keeps
// merging re-imports into one note type.

export type FieldKey = keyof FieldNamesConfig;

/** Canonical field order — MUST match the schema order in the Rust exporter. */
export const NOTE_TYPE_FIELD_ORDER: FieldKey[] = [
  "expression",
  "meaning",
  "audio",
  "snapshot",
  "video",
  "tags",
  "sequenceMarker",
  "reading",
  "notes",
];

export type FieldToggles = Record<FieldKey, boolean>;

export const allFieldsIncluded: FieldToggles = {
  expression: true,
  meaning: true,
  reading: true,
  audio: true,
  snapshot: true,
  video: true,
  tags: true,
  sequenceMarker: true,
  notes: true,
};

export interface NoteTypeDef {
  /** "predef:<langCode>" for predefined, "custom:<id>" for custom. */
  id: string;
  /** Anki note type name. */
  name: string;
  predefined: boolean;
  /** Study language code this note type maps to (drives subtitle matching). */
  language: string;
  fields: FieldNamesConfig;
  included: FieldToggles;
}

/** Shape sent to the Rust backend as `output_fields`. */
export interface OutputFieldsPayload {
  include_subs1: boolean;
  include_subs2: boolean;
  include_audio: boolean;
  include_snapshot: boolean;
  include_video: boolean;
  include_tag: boolean;
  include_sequence: boolean;
  include_reading: boolean;
  include_notes: boolean;
}

const CUSTOM_NOTE_TYPES_KEY = "vesta-custom-note-types";
const LEGACY_ANKI_FIELD_PRESETS_KEY = "vesta-anki-field-presets";
export const NOTE_TYPES_UPDATED_EVENT = "vesta:note-types-updated";

function sanitizeToggles(raw: Partial<FieldToggles> | undefined): FieldToggles {
  const r = raw || {};
  // Missing keys default to ON, so legacy data (no `included`) becomes all-on.
  const at = (v: unknown) => v !== false;
  return {
    expression: at(r.expression),
    meaning: at(r.meaning),
    reading: at(r.reading),
    audio: at(r.audio),
    snapshot: at(r.snapshot),
    video: at(r.video),
    tags: at(r.tags),
    sequenceMarker: at(r.sequenceMarker),
    notes: at(r.notes),
  };
}

function sanitizeCustomNoteType(raw: any): NoteTypeDef | null {
  if (!raw || !raw.id || !raw.name) return null;
  const id = String(raw.id).startsWith("custom:") ? String(raw.id) : `custom:${raw.id}`;
  return {
    id,
    name: limitNoteTypeFieldValue(String(raw.name)),
    predefined: false,
    language: typeof raw.language === "string" ? raw.language : "",
    fields: sanitizeFieldNamesConfig({ ...defaultFieldNames, ...(raw.fields || {}) }),
    included: sanitizeToggles(raw.included),
  };
}

/** Custom note types saved by the user, loaded directly from the presets store. */
export function loadCustomNoteTypes(): NoteTypeDef[] {
  try {
    const raw = vestaConfig.getItem("vesta-anki-field-presets");
    if (raw) {
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed)) {
        return parsed
          .map((p: any) => {
            if (!p || !p.id) return null;
            const fields = sanitizeFieldNamesConfig({ ...defaultFieldNames, ...(p.fields || {}) });
            const included: FieldToggles = {
              expression: fields.expression.trim() !== "",
              meaning: fields.meaning.trim() !== "",
              reading: fields.reading.trim() !== "",
              audio: fields.audio.trim() !== "",
              snapshot: fields.snapshot.trim() !== "",
              video: fields.video.trim() !== "",
              tags: fields.tags.trim() !== "",
              sequenceMarker: fields.sequenceMarker.trim() !== "",
              notes: fields.notes.trim() !== "",
            };
            return {
              id: p.id.startsWith("custom:") ? p.id : `custom:${p.id}`,
              name: p.name || p.noteTypeName || "Unnamed Template",
              predefined: false,
              language: "",
              fields,
              included,
            };
          })
          .filter((nt): nt is NoteTypeDef => Boolean(nt));
      }
    }
  } catch { /* ignore */ }
  return [];
}

export function saveCustomNoteTypes(list: NoteTypeDef[]): void {
  // Keeping this function signature for backwards compatibility/types,
  // but presets are saved via SettingsTab's persistAnkiFieldPresets.
  dispatchWindowEvent(NOTE_TYPES_UPDATED_EVENT);
}

/** The locked, predefined note type. */
export function predefinedNoteTypeForLanguage(code: string): NoteTypeDef {
  return {
    id: "default",
    name: "Default_Vesta",
    predefined: true,
    language: "",
    fields: { ...defaultFieldNames },
    included: { ...allFieldsIncluded },
  };
}

/** Predefined note types (just returning the default template now). */
export function predefinedNoteTypes(): NoteTypeDef[] {
  return [predefinedNoteTypeForLanguage("")];
}

/** All selectable note types: predefined first, then custom (A→Z). */
export function listNoteTypes(): NoteTypeDef[] {
  const byName = (a: NoteTypeDef, b: NoteTypeDef) => a.name.localeCompare(b.name);
  const custom = loadCustomNoteTypes().slice().sort(byName);
  const defaultNT = predefinedNoteTypeForLanguage("");
  return [defaultNT, ...custom];
}

export function findNoteTypeById(id: string): NoteTypeDef | null {
  if (id === "default" || id.startsWith("predef:")) {
    return predefinedNoteTypeForLanguage("");
  }
  return loadCustomNoteTypes().find((nt) => nt.id === id) ?? null;
}

const ACTIVE_NOTE_TYPE_ID_KEY = "vesta-active-note-type-id";
export const ACTIVE_NOTE_TYPE_CHANGED_EVENT = "vesta:active-note-type-changed";

export function loadActiveNoteTypeId(): string {
  try {
    const saved = vestaConfig.getItem(ACTIVE_NOTE_TYPE_ID_KEY);
    if (saved) return saved;
  } catch { /* ignore */ }
  return "default";
}

export function saveActiveNoteTypeId(id: string): void {
  try {
    vestaConfig.setItem(ACTIVE_NOTE_TYPE_ID_KEY, id);
    dispatchWindowEvent(ACTIVE_NOTE_TYPE_CHANGED_EVENT);
  } catch { /* ignore */ }
}

export function newCustomNoteTypeId(): string {
  return `custom:${Date.now().toString(36)}`;
}

/** Map a note type's active fields onto the backend `output_fields` payload. */
export function noteTypeOutputFields(nt: NoteTypeDef): OutputFieldsPayload {
  const i = nt.included;
  return {
    include_subs1: i.expression,
    include_subs2: i.meaning,
    include_audio: i.audio,
    include_snapshot: i.snapshot,
    include_video: i.video,
    include_tag: i.tags,
    include_sequence: i.sequenceMarker,
    include_reading: i.reading,
    include_notes: i.notes,
  };
}

export const defaultCardTemplates: CardTemplateConfig = {
  frontHtml: `<div id="tags-container"></div>
<div id="tags-source" style="display: none;">{{Tags}}</div>
<div id="timestamp-source" style="display: none;">{{SequenceMarker}}</div>
<div class='expression'>{{Expression}}</div>
<hr>`,
  backHtml: `<div id="tags-container"></div>
<div id="tags-source" style="display: none;">{{Tags}}</div>
<div id="timestamp-source" style="display: none;">{{SequenceMarker}}</div>
<span class='media'>{{Audio}}</span>
<div class="expression">{{Expression}}</div>
<hr>
<br>
<div class='reading'>{{Reading}}</div>
<div class='meaning'>{{Meaning}}</div>
<br>
<div class='media'>{{Snapshot}}</div>
<span class='media'>{{Video}}</span>
<br />`,
  css: `.card {
  font-family: arial;
  font-size: 20px;
  text-align: center;
  color: black;
  background-color: white;
}
#tags-container {
  text-align: left;
  margin-bottom: 8px;
  min-height: 20px;
}
.tag-pill {
  display: inline-block;
  font-size: 11px;
  font-family: arial, sans-serif;
  font-weight: 600;
  color: #333;
  background-color: #f0f0f0;
  padding: 4px 8px;
  border-radius: 8px;
  margin-right: 4px;
  margin-bottom: 4px;
  border: 1px solid #ddd;
  box-shadow: 0 1px 1px rgba(0,0,0,0.05);
}
.card video,
.card iframe {
  width: 600px;
  height: 400px;
  max-width: 100%;
  display: block;
  margin: 10px auto;
  border: 1px solid #eee;
}`,
  noteTypeName: "Vesta_Default",
};

export function loadCardTemplates(): CardTemplateConfig {
  try {
    const raw = vestaConfig.getItem(CARD_TEMPLATE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return sanitizeCardTemplateConfig({
        frontHtml: parsed.frontHtml || defaultCardTemplates.frontHtml,
        backHtml: parsed.backHtml || defaultCardTemplates.backHtml,
        css: parsed.css || defaultCardTemplates.css,
        noteTypeName: parsed.noteTypeName || defaultCardTemplates.noteTypeName,
      });
    }
  } catch { /* ignore */ }
  return sanitizeCardTemplateConfig({ ...defaultCardTemplates });
}

export function saveCardTemplates(config: CardTemplateConfig): void {
  const sanitized = sanitizeCardTemplateConfig(config);
  vestaConfig.setItem(CARD_TEMPLATE_KEY, JSON.stringify(sanitized));
  dispatchWindowEvent(CARD_TEMPLATES_UPDATED_EVENT);
}

export function resetCardTemplates(): CardTemplateConfig {
  vestaConfig.removeItem(CARD_TEMPLATE_KEY);
  dispatchWindowEvent(CARD_TEMPLATES_UPDATED_EVENT);
  return sanitizeCardTemplateConfig({ ...defaultCardTemplates });
}
