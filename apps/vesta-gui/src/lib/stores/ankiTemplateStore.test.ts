import { describe, it, expect, beforeEach, vi } from "vitest";

const { storage } = vi.hoisted(() => {
  return { storage: new Map<string, string>() };
});

vi.mock("$lib/config/vestaConfig", () => ({
  getItem: (key: string) => storage.get(key) ?? null,
  setItem: (key: string, val: string) => {
    storage.set(key, val);
  },
  removeItem: (key: string) => {
    storage.delete(key);
  },
}));

vi.mock("$lib/stores/snackbarStore.svelte", () => ({
  snackbar: {
    show: vi.fn(),
  },
}));

import {
  defaultFieldNames,
  defaultCardTemplates,
  loadLastUsedNoteTypeId,
  saveActiveNoteTypeId,
  loadFieldNames,
  saveFieldNames,
  loadCardTemplates,
  saveCardTemplates,
  resetCardTemplates,
  noteTypeOutputFields,
  predefinedNoteTypeForLanguage,
  limitNoteTypeFieldValue,
  NOTE_TYPE_FIELD_SOFT_LIMIT,
} from "$lib/types/noteTypes";

describe("ankiTemplate & noteTypes logic", () => {
  beforeEach(() => {
    storage.clear();
  });

  describe("limitNoteTypeFieldValue", () => {
    it("clamps string lengths to NOTE_TYPE_FIELD_SOFT_LIMIT", () => {
      const longName = "A".repeat(50);
      const limited = limitNoteTypeFieldValue(longName);
      expect(limited.length).toBe(NOTE_TYPE_FIELD_SOFT_LIMIT);
      expect(limited).toBe("A".repeat(NOTE_TYPE_FIELD_SOFT_LIMIT));
    });

    it("leaves short strings unmodified", () => {
      expect(limitNoteTypeFieldValue("Expression")).toBe("Expression");
      expect(limitNoteTypeFieldValue("")).toBe("");
    });
  });

  describe("field names persistence", () => {
    it("loads default field names when nothing is stored", () => {
      const fields = loadFieldNames();
      expect(fields.expression).toBe("Expression");
      expect(fields.audio).toBe("Audio");
      expect(fields.snapshot).toBe("Snapshot");
      expect(fields.video).toBe("Video");
    });

    it("saves and reloads custom field names", () => {
      const custom = {
        ...defaultFieldNames,
        meaning: "Translation",
        audio: "Sound",
        snapshot: "Picture",
      };
      saveFieldNames(custom);

      const reloaded = loadFieldNames();
      expect(reloaded.meaning).toBe("Translation");
      expect(reloaded.audio).toBe("Sound");
      expect(reloaded.snapshot).toBe("Picture");
    });
  });

  describe("card templates persistence", () => {
    it("loads defaults when no template is saved", () => {
      const templates = loadCardTemplates();
      expect(templates.frontHtml).toBe(defaultCardTemplates.frontHtml);
      expect(templates.backHtml).toBe(defaultCardTemplates.backHtml);
      expect(templates.css).toBe(defaultCardTemplates.css);
    });

    it("saves, reloads and resets custom templates", () => {
      saveCardTemplates({
        frontHtml: "<div>Front Custom</div>",
        backHtml: "<div>Back Custom</div>",
        css: ".card { color: red; }",
        noteTypeName: "Custom_Model",
      });

      const custom = loadCardTemplates();
      expect(custom.frontHtml).toBe("<div>Front Custom</div>");
      expect(custom.css).toBe(".card { color: red; }");
      expect(custom.noteTypeName).toBe("Custom_Model");

      resetCardTemplates();
      const reset = loadCardTemplates();
      expect(reset.frontHtml).toBe(defaultCardTemplates.frontHtml);
      expect(reset.css).toBe(defaultCardTemplates.css);
    });
  });

  describe("noteTypeOutputFields payload generation", () => {
    it("correctly flags all fields for Default Vesta note type", () => {
      const defaultNT = predefinedNoteTypeForLanguage("");
      const payload = noteTypeOutputFields(defaultNT);

      expect(payload.include_subs1).toBe(true);
      expect(payload.include_subs2).toBe(true);
      expect(payload.include_audio).toBe(true);
      expect(payload.include_snapshot).toBe(true);
      expect(payload.include_video).toBe(true);
      expect(payload.include_tag).toBe(true);
      expect(payload.include_sequence).toBe(true);
      expect(payload.include_reading).toBe(true);
      expect(payload.include_notes).toBe(true);
    });

    it("selectively omits disabled fields", () => {
      const minimalNT = {
        ...predefinedNoteTypeForLanguage(""),
        included: {
          expression: true,
          meaning: false,
          reading: false,
          audio: true,
          snapshot: false,
          video: false,
          tags: true,
          sequenceMarker: false,
          notes: false,
        },
      };

      const payload = noteTypeOutputFields(minimalNT);
      expect(payload.include_subs1).toBe(true);
      expect(payload.include_subs2).toBe(false);
      expect(payload.include_audio).toBe(true);
      expect(payload.include_snapshot).toBe(false);
      expect(payload.include_video).toBe(false);
      expect(payload.include_tag).toBe(true);
    });
  });
});


it("preserves the last explicit note type when automatic mode is selected", () => {
  storage.clear();
  saveActiveNoteTypeId("predef:en");
  expect(loadLastUsedNoteTypeId()).toBe("predef:en");
  saveActiveNoteTypeId("default");
  expect(loadLastUsedNoteTypeId()).toBe("predef:en");
  saveActiveNoteTypeId("predef:ja");
  expect(loadLastUsedNoteTypeId()).toBe("predef:ja");
  // A deleted custom type must not strand new films on a nonexistent template.
  storage.set("vesta-last-used-note-type-id", "custom:deleted");
  expect(loadLastUsedNoteTypeId()).toBe("predef:ja");
});
