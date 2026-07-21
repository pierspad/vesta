<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { locale } from "$lib/i18n";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  import CodeEditor from "$lib/components/CodeEditor.svelte";
  import SearchableSelect from "$lib/components/SearchableSelect.svelte";
  import {
    CARD_TEMPLATES_UPDATED_EVENT,
    FIELD_NAMES_UPDATED_EVENT,
    ACTIVE_NOTE_TYPE_CHANGED_EVENT,
    saveActiveNoteTypeId,
    limitNoteTypeFieldValue,
  } from "$lib/types/noteTypes";
  import { ankiTemplateStore, type AnkiFieldKey, type TemplateCodeTab } from "$lib/stores/ankiTemplateStore.svelte";

  interface Props {
    /** SettingsTab.svelte's own settingsCopy translation helper -- kept local
     * to that file (its keys are a closed union typed against settingsCopy.en),
     * so passed through rather than duplicated or re-typed here. */
    s: (key: any) => string;
  }

  let { s }: Props = $props();

  let t = $derived($locale);
  let store = ankiTemplateStore;

  const templateCodeTabs: { id: TemplateCodeTab; label: string; hint: string }[] = [
    { id: "front", label: "Front HTML", hint: "Modifica il file front.html di questo template" },
    { id: "back", label: "Back HTML", hint: "Modifica il file back.html di questo template" },
    { id: "css", label: "Style CSS", hint: "Modifica il file style.css condiviso da questo template" },
  ];

  const ankiFieldDefinitions: {
    key: AnkiFieldKey;
    variable: string;
    colorClass: string;
    iconClass: string;
    iconPath: string;
  }[] = [
    {
      key: "expression",
      variable: "{{Expression}}",
      colorClass: "border-sky-400/30 bg-sky-400/10 text-sky-200 hover:bg-sky-400/15",
      iconClass: "text-sky-300",
      iconPath: "M4 6h16M4 12h10M4 18h7",
    },
    {
      key: "meaning",
      variable: "{{Meaning}}",
      colorClass: "border-emerald-400/30 bg-emerald-400/10 text-emerald-200 hover:bg-emerald-400/15",
      iconClass: "text-emerald-300",
      iconPath: "M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10",
    },
    {
      key: "reading",
      variable: "{{Reading}}",
      colorClass: "border-violet-400/30 bg-violet-400/10 text-violet-200 hover:bg-violet-400/15",
      iconClass: "text-violet-300",
      iconPath: "M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5s3.332.477 4.5 1.253v13C19.832 18.477 18.246 18 16.5 18s-3.332.477-4.5 1.253",
    },
    {
      key: "audio",
      variable: "{{Audio}}",
      colorClass: "border-rose-400/30 bg-rose-400/10 text-rose-200 hover:bg-rose-400/15",
      iconClass: "text-rose-300",
      iconPath: "M11 5L6 9H3v6h3l5 4V5zm4.5 4.5a4 4 0 010 5m2.5-7.5a8 8 0 010 10",
    },
    {
      key: "snapshot",
      variable: "{{Snapshot}}",
      colorClass: "border-amber-400/30 bg-amber-400/10 text-amber-200 hover:bg-amber-400/15",
      iconClass: "text-amber-300",
      iconPath: "M3 7h4l2-3h6l2 3h4v13H3V7zm9 10a4 4 0 100-8 4 4 0 000 8z",
    },
    {
      key: "video",
      variable: "{{Video}}",
      colorClass: "border-orange-400/30 bg-orange-400/10 text-orange-200 hover:bg-orange-400/15",
      iconClass: "text-orange-300",
      iconPath: "M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 6h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V8a2 2 0 012-2z",
    },
    {
      key: "tags",
      variable: "{{Tags}}",
      colorClass: "border-lime-400/30 bg-lime-400/10 text-lime-200 hover:bg-lime-400/15",
      iconClass: "text-lime-300",
      iconPath: "M7 7h.01M3 11l8.586-8.586A2 2 0 0113 2h6a2 2 0 012 2v6a2 2 0 01-.586 1.414L11.828 20a2 2 0 01-2.828 0L3 14a2 2 0 010-3z",
    },
    {
      key: "sequenceMarker",
      variable: "{{SequenceMarker}}",
      colorClass: "border-cyan-400/30 bg-cyan-400/10 text-cyan-200 hover:bg-cyan-400/15",
      iconClass: "text-cyan-300",
      iconPath: "M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01",
    },
    {
      key: "notes",
      variable: "{{Notes}}",
      colorClass: "border-fuchsia-400/30 bg-fuchsia-400/10 text-fuchsia-200 hover:bg-fuchsia-400/15",
      iconClass: "text-fuchsia-300",
      iconPath: "M11 5H6a2 2 0 00-2 2v11a1 1 0 001 1h11a2 2 0 002-2v-5M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z",
    },
  ];

  function fieldVariableName(field: { variable: string }): string {
    return field.variable.replace(/^\{\{|\}\}$/g, "");
  }

  function syncLimitedInput(
    event: Event,
    applyValue: (value: string) => void,
    save: () => void,
  ) {
    const target = event.currentTarget;
    if (!(target instanceof HTMLInputElement)) return;

    const limitedValue = limitNoteTypeFieldValue(target.value);
    if (target.value !== limitedValue) {
      target.value = limitedValue;
    }

    applyValue(limitedValue);
    save();
  }

  onMount(() => {
    // Re-sync from storage when a save dispatches these events -- see
    // ankiTemplateStore for why the events fire (saveCardTemplates /
    // saveFieldNames dispatch them) even though, today, this panel is the
    // only listener in the app.
    window.addEventListener(CARD_TEMPLATES_UPDATED_EVENT, store.syncTemplateStateFromStorage);
    window.addEventListener(FIELD_NAMES_UPDATED_EVENT, store.syncFieldStateFromStorage);
    window.addEventListener(ACTIVE_NOTE_TYPE_CHANGED_EVENT, store.refreshActiveNoteTypeId);
    // Catch up in case something changed while this panel was unmounted
    // (it only mounts while the "anki" section is active).
    store.syncTemplateStateFromStorage();
    store.syncFieldStateFromStorage();
    store.refreshActiveNoteTypeId();
  });

  onDestroy(() => {
    window.removeEventListener(CARD_TEMPLATES_UPDATED_EVENT, store.syncTemplateStateFromStorage);
    window.removeEventListener(FIELD_NAMES_UPDATED_EVENT, store.syncFieldStateFromStorage);
    window.removeEventListener(ACTIVE_NOTE_TYPE_CHANGED_EVENT, store.refreshActiveNoteTypeId);
  });
</script>

<div class="mt-6 space-y-4">

  <div class="glass-card p-5">
    <div class="flex flex-col lg:flex-row lg:items-start lg:justify-between gap-4 mb-5">
      <div class="flex items-center gap-3">
        <div class="w-9 h-9 rounded-lg bg-emerald-500/20 text-emerald-300 flex items-center justify-center shrink-0">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 5a1 1 0 011-1h14a1 1 0 011 1v4H4V5zm0 8h8v7H5a1 1 0 01-1-1v-6zm12 0h4v6a1 1 0 01-1 1h-3v-7z" />
          </svg>
        </div>
        <div>
          <h3 class="text-sm font-bold text-white">{s("fieldPanelKicker")}</h3>
        </div>
      </div>
      <div class="flex flex-wrap gap-2">
        <button
          type="button"
          onclick={() => store.saveCurrentAnkiFieldPreset()}
          disabled={!store.noteTypeName.trim() || !store.getFieldValue("expression").trim()}
          class="px-3 py-2 rounded-lg border border-emerald-500/30 bg-emerald-500/10 text-emerald-200 hover:bg-emerald-500/20 transition-colors text-xs font-semibold flex items-center gap-2 disabled:opacity-40 disabled:cursor-not-allowed"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
          </svg>
          {t("settings.modal.save")}
        </button>
        <button
          type="button"
          onclick={() => store.deleteCurrentAnkiFieldPreset()}
          disabled={store.selectedAnkiFieldPresetId === "default"}
          class="px-3 py-2 rounded-lg border border-red-500/30 bg-red-500/10 text-red-300 hover:bg-red-500/20 hover:border-red-500/50 disabled:opacity-40 disabled:cursor-not-allowed disabled:bg-white/5 disabled:border-white/10 disabled:text-gray-400 transition-colors text-xs font-semibold flex items-center gap-2"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
          </svg>
          {t("settings.delete")}
        </button>

      </div>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-[1fr_1fr_1.2fr] gap-4 mb-5">
      <div>
        <label for="anki-field-preset-select" class="block text-xs font-semibold text-gray-400 mb-2">{s("savedTemplate")}</label>
        <SearchableSelect
          className="settings-template-select"
          noResultsText={t("common.noResults")}
          options={store.allAnkiFieldPresets.map((preset) => ({
            value: preset.id,
            label: preset.name,
          }))}
          value={store.selectedAnkiFieldPresetId}
          onchange={(v) => store.applyAnkiFieldPreset(v)}
          placeholder={s("savedTemplate")}
        />
      </div>
      <div>
        <label for="anki-field-preset-name" class="block text-xs font-semibold text-gray-400 mb-2">{s("templateName")}</label>
        <input
          id="anki-field-preset-name"
          type="text"
          bind:value={store.ankiFieldPresetName}
          maxlength="25"
          class="input-modern w-full text-sm"
          placeholder="vesta_modificato"
        />
      </div>
      <div>
        <label for="active-flashcards-template-select" class="block text-xs font-semibold text-gray-400 mb-2">{s("activeTemplate")}</label>
        <SearchableSelect
          className="settings-active-template-select"
          noResultsText={t("common.noResults")}
          options={store.allAnkiFieldPresets.map((preset) => ({
            value: preset.id,
            label: preset.id === "default" ? preset.name : `★ ${preset.name}`,
          }))}
          value={store.activeNoteTypeId}
          onchange={(v) => {
            store.activeNoteTypeId = v;
            saveActiveNoteTypeId(v);
          }}
          placeholder="Seleziona il template attivo..."
        />
      </div>
    </div>

    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5 gap-3">
      <div>
        <label for="note-type-name-inline" class="mb-1 flex items-center gap-1.5 text-xs font-semibold text-gray-400">
          <svg class="h-3.5 w-3.5 text-emerald-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 5a1 1 0 011-1h14a1 1 0 011 1v4H4V5zm0 8h8v7H5a1 1 0 01-1-1v-6zm12 0h4v6a1 1 0 01-1 1h-3v-7z" />
          </svg>
          <span>{t("settings.noteType")}</span>
        </label>
        <input
          id="note-type-name-inline"
          type="text"
          bind:value={store.noteTypeName}
          maxlength="25"
          disabled={store.selectedAnkiFieldPresetId === "default"}
          oninput={(event) =>
            syncLimitedInput(event, (value) => (store.noteTypeName = value), () => store.saveTemplates())}
          class="input-modern w-full text-sm disabled:opacity-50 disabled:cursor-not-allowed"
          placeholder="Vesta_Default"
        />
      </div>
      {#each ankiFieldDefinitions as field}
        {@const isLocked = field.key === "expression" || field.key === "sequenceMarker"}
        <div>
          <label for={`anki-field-${field.key}`} class="mb-1 flex items-center justify-between gap-1.5 text-xs font-semibold text-gray-400">
            <div class="flex items-center gap-1.5">
              <svg class={`h-3.5 w-3.5 ${field.iconClass}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={field.iconPath} />
              </svg>
              <span>{fieldVariableName(field)}</span>
            </div>
            {#if isLocked}
              <span class="text-[9px] text-amber-500/80 font-semibold uppercase tracking-wider bg-amber-500/10 px-1.5 py-0.5 rounded border border-amber-500/20 flex items-center gap-1" title={t("settings.essentialFieldLocked")}>
                <svg class="w-2.5 h-2.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
                </svg>
                 <span>{s("locked")}</span>
              </span>
            {/if}
          </label>
          <input
            id={`anki-field-${field.key}`}
            aria-label={fieldVariableName(field)}
            type="text"
            value={store.getFieldValue(field.key)}
            maxlength="25"
            disabled={store.selectedAnkiFieldPresetId === "default" || isLocked}
            oninput={(event) =>
              syncLimitedInput(event, (value) => store.setFieldValue(field.key, value), () => store.saveFields())}
            class="input-modern w-full text-sm disabled:opacity-50 disabled:cursor-not-allowed {isLocked ? 'border-amber-500/20 bg-amber-500/5 text-amber-200/90' : !store.getFieldValue(field.key).trim() ? 'opacity-40 border-dashed border-gray-600' : ''}"
            placeholder={fieldVariableName(field)}
          />
        </div>
      {/each}
    </div>
  </div>

  <div class="glass-card p-5">
    <div class="flex flex-col xl:flex-row xl:items-start xl:justify-between gap-4 mb-4">
      <div class="flex items-center gap-3">
        <div class="w-9 h-9 rounded-lg bg-cyan-500/20 text-cyan-300 flex items-center justify-center shrink-0">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l3 3-3 3m5 0h3M5 5h14a2 2 0 012 2v10a2 2 0 01-2 2H5a2 2 0 01-2-2V7a2 2 0 012-2z" />
          </svg>
        </div>
        <div>
          <h3 class="text-sm font-bold text-white">{s("cardPanelKicker")}</h3>
        </div>
      </div>
      <div class="flex flex-wrap gap-2">
        {#each templateCodeTabs as tab}
          <button
            type="button"
            onclick={() => (store.activeTemplateCodeTab = tab.id)}
            title={tab.hint}
            class="h-9 px-3 rounded-lg border text-xs font-semibold transition-colors {store.activeTemplateCodeTab === tab.id ? 'bg-cyan-500/20 border-cyan-400/40 text-cyan-100' : 'bg-black/20 border-white/10 text-gray-400 hover:text-white hover:bg-white/10'}"
          >
            {tab.label}
          </button>
        {/each}


      </div>
    </div>

    {#if store.activeTemplateCodeTab === "front"}
      <CodeEditor bind:value={store.templateFrontHtml} language="html" onchange={() => store.saveTemplates()} />
    {:else if store.activeTemplateCodeTab === "back"}
      <CodeEditor bind:value={store.templateBackHtml} language="html" onchange={() => store.saveTemplates()} />
    {:else}
      <CodeEditor bind:value={store.templateCss} language="css" onchange={() => store.saveTemplates()} />
    {/if}

    <div class="mt-4 rounded-lg border border-white/10 bg-black/20 p-4">
      <div class="flex flex-col lg:flex-row lg:items-center gap-3">
        <div class="lg:w-48 shrink-0">
          <p class="text-xs uppercase tracking-wide text-cyan-300/80">{t("settings.availableVars")}</p>
          <p class="text-xs text-gray-500 mt-1">{s("clickToCopy")}</p>
        </div>
        <div class="flex flex-wrap gap-2 text-[11px] font-mono">
        {#each ankiFieldDefinitions as field}
          <button
            type="button"
            onclick={() => {
              navigator.clipboard.writeText(store.getFieldVariable(field));
              snackbar.show(t("settings.keyCopied"), "success", 1300);
            }}
            class="px-2.5 py-1.5 rounded-lg border transition-colors {field.colorClass}"
            title="Copia variabile"
          >
            {store.getFieldVariable(field)}
          </button>
        {/each}
        </div>
      </div>
    </div>
  </div>

</div>
