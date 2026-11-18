<script lang="ts">
  import { locale } from "$lib/i18n";
  import { uiMode } from "$lib/stores/uiModeStore.svelte";
  import type { TranscribeTierEntry } from "$lib/config/transcribeTiers";
  import SpeechEngineSelector from "$lib/components/SpeechEngineSelector.svelte";
  import SettingsJumpButton from "$lib/components/SettingsJumpButton.svelte";
  import LanguageSelect from "$lib/components/LanguageSelect.svelte";
  import ToggleRow from "$lib/components/ToggleRow.svelte";

  const SEGMENT_PRESETS = [
    { id: "short", value: 10, labelKey: "transcribe.segmentShort", lines: "M7 10h8M7 14h5" },
    { id: "medium", value: 20, labelKey: "transcribe.segmentMedium", lines: "M6 9h12M6 13h9M6 17h7" },
    { id: "standard", value: 30, labelKey: "transcribe.segmentStandard", lines: "M5 9h14M5 13h14M5 17h14" },
    { id: "long", value: 60, labelKey: "transcribe.segmentLong", lines: "M4 8h16M4 12h16M4 16h16M4 20h16" },
  ] as const;

  interface Props {
    active: boolean;
    isTranscribing: boolean;
    modelHighlightClass: string;
    selectedEngine: TranscribeTierEntry | null | undefined;
    selectedLanguage: string;
    vadInstalled: boolean;
    vadEnabled: boolean;
    qualityMode: boolean;
    maxSegmentLength: number;
    onGoToSettings?: (section?: "overview" | "llm" | "whisper" | "language" | "anki" | "shortcuts", highlightItemId?: string) => void;
    onToggleVad: (enabled: boolean) => void;
    onToggleQuality: (enabled: boolean) => void;
  }

  let {
    active,
    isTranscribing,
    modelHighlightClass,
    selectedEngine = $bindable(),
    selectedLanguage = $bindable(),
    vadInstalled,
    vadEnabled,
    qualityMode,
    maxSegmentLength = $bindable(),
    onGoToSettings,
    onToggleVad,
    onToggleQuality,
  }: Props = $props();

  let t = $derived($locale);
  let activeSegmentPreset = $derived(SEGMENT_PRESETS.find((p) => p.value === maxSegmentLength)?.id ?? null);
  let isLocalEngine = $derived(!selectedEngine || selectedEngine.provider === "local" || selectedEngine.provider === "local_whisper");
</script>

<div class="glass-card p-5">
  <h3 class="text-lg font-semibold mb-4 flex items-center gap-2 text-blue-400">
    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
    </svg>
    {t("transcribe.options")}
  </h3>
  <div class="mb-4 space-y-2 rounded-xl {modelHighlightClass}">
    <div class="flex items-center gap-2 text-sm font-medium" style="color: var(--text)">
      <svg aria-hidden="true" class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10" />
      </svg>
      {t("transcribe.engineTitle")}
    </div>
    <SpeechEngineSelector purpose="transcribe" disabled={isTranscribing || !active} onresolve={(entry) => { selectedEngine = entry; }} />
    {#if onGoToSettings}
      <SettingsJumpButton
        label={t("transcribe.tiers.managePriority")}
        section="whisper"
        targetId="transcribe-tiers"
        onclick={() => onGoToSettings?.("whisper", "transcribe-tiers")}
        variant="subtle"
      />
    {/if}
  </div>
  <div class="space-y-4">
    <div>
      <span class="flex items-center gap-1.5 text-sm text-gray-400 mb-1 font-medium">
        <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10" />
        </svg>
        <span>{t("transcribe.sourceLanguage")}</span>
      </span>
      <LanguageSelect
        noResultsText={t("common.noResults")}
        autoDetect
        value={selectedLanguage}
        onchange={(v) => (selectedLanguage = v)}
        placeholder={t("transcribe.sourceLanguage")}
      />
    </div>
    {#if uiMode.expertMode}
      {#if isLocalEngine}
        <div class="rounded-lg bg-white/5 p-3">
          <ToggleRow label={t("transcribe.vad")} description={vadInstalled ? t("transcribe.vadHelp") : t("transcribe.vadNotInstalled")} checked={vadEnabled && vadInstalled} disabled={!vadInstalled} accent="sky" onchange={onToggleVad} />
          {#if !vadInstalled}
            <div class="mt-2.5 pt-2 border-t border-white/5 flex items-center justify-between gap-2">
              <span class="text-xs text-amber-300/80">{t("transcribe.vadNotInstalled")}</span>
              <SettingsJumpButton
                label={t("transcribe.downloadVad")}
                section="whisper"
                targetId="silero-vad"
                disabled={!onGoToSettings}
                onclick={() => onGoToSettings?.("whisper", "silero-vad")}
                variant="amber"
              />
            </div>
          {/if}
        </div>
        <div class="rounded-lg bg-white/5 p-3">
          <ToggleRow label={t("transcribe.quality")} checked={qualityMode} accent="sky" onchange={onToggleQuality} />
        </div>
      {/if}
      <div>
        <div class="flex items-center justify-between mb-2">
          <span class="text-sm text-gray-400">{t("transcribe.maxSegmentLength")}</span>
        </div>
        <div class="grid grid-cols-4 gap-2">
          {#each SEGMENT_PRESETS as preset (preset.id)}
            <button
              onclick={() => (maxSegmentLength = preset.value)}
              class="p-2 rounded-lg text-center transition-all duration-200 border text-xs {activeSegmentPreset === preset.id
                ? 'bg-cyan-500/20 border-cyan-500/50 text-white'
                : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
            >
              <span class="block mb-1">
                <svg class="w-4 h-4 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d={preset.lines} />
                </svg>
              </span>
              <span class="font-semibold block">{t(preset.labelKey)}</span>
            </button>
          {/each}
        </div>
        <div class="mt-2 flex items-center gap-2 text-xs">
          <span class="text-gray-500">{t("transcribe.segmentLengthLabel")}</span>
          <span class="text-white font-mono bg-white/10 px-2 py-0.5 rounded text-sm">{maxSegmentLength}s</span>
        </div>
      </div>
    {/if}
  </div>
</div>
