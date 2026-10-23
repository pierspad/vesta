<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { buildSetupPlan, installSetupPlan, resolveSetupChoices, setupTierPreferences, type SetupResources, type SetupTaskStatus, type SetupDestination } from "$lib/workflows/firstRunSetup";
  import { loadSetupResources, installSetupResource } from "$lib/services/setup";
  import { saveVadSelection } from "$lib/config/vadSelection";
  import { onMount } from "svelte";
  import { availableUILanguages, currentLanguage, loadLanguage, translateForLanguage } from "$lib/i18n";
  import { languages } from "$lib/config/languages";
  import SetupLanguageField from "$lib/components/SetupLanguageField.svelte";
  import WhisperModelSelector from "$lib/components/WhisperModelSelector.svelte";
  import ExportFallbackSelector from "$lib/components/ExportFallbackSelector.svelte";
  import { loadMediaSettings, saveMediaSettings } from "$lib/utils/mediaSettings";
  import { whisperModelsStore } from "$lib/stores/whisperModelsStore.svelte";
  import { fontStore } from "$lib/stores/fontStore.svelte";
  import type { ExportFormat, ExportFallbackFormat } from "$lib/stores/exportFormatStore.svelte";
  import * as vestaConfig from "$lib/config/vestaConfig";

  let { onComplete, preview = false, onClose }: { onComplete: (wantsTranscription: boolean, destination: SetupDestination) => void | Promise<void>; preview?: boolean; onClose?: () => void } = $props();
  let mode = $state<"quick" | "custom" | null>(null);
  let step = $state<"languages" | "export" | "transcription" | "review" | "installing" | "complete">("languages");
  const initialNativeLanguage = languages.some((l) => l.code === $currentLanguage) ? $currentLanguage : "en";
  let nativeLanguage = $state(initialNativeLanguage);
  let studyLanguage = $state("en");
  const uiLanguageCodes = new Set(availableUILanguages.map((language) => language.code));
  const resolveUiLanguage = (language: string) => uiLanguageCodes.has(language) ? language : "en";
  let uiLanguage = $state(resolveUiLanguage(initialNativeLanguage));
  let exportFormat = $state<ExportFormat>("apkg");
  let fallbackFormat = $state<ExportFallbackFormat>("apkg");
  let compactAudio = $state(false);
  let wantsTranscription = $state(true);
  let useVad = $state(true);
  let vadChoice = $state<"silero" | "custom">("silero");
  let whisperModel = $state("small");
  let installing = $state(false);
  let installError = $state("");
  let setupRevision = $state(0);
  $effect(() => { const selected = uiLanguage; void loadLanguage(selected).then((ok) => { if (ok && uiLanguage === selected) { if (!preview) currentLanguage.set(selected); setupRevision += 1; } }); });
  function setupT(key: string, params?: Record<string, string | number>) { void setupRevision; return translateForLanguage(uiLanguage, key, params); }

  let sileroDownloaded = $derived(whisperModelsStore.vadModels.find((model) => model.id === "v6.2.0")?.downloaded ?? false);

  const customStepNumber = $derived(step === "languages" ? 1 : step === "export" ? 2 : 3);

  let resources = $state<SetupResources | null>(null);
  let loadingResources = $state(false);
  let discoveryError = $state("");
  let taskStatuses = $state<Record<string, SetupTaskStatus>>({});
  let currentTaskId = $state<string | null>(null);
  let currentProgress = $state<number | null>(null);
  let openingApp = $state(false);
  let choices = $derived(resolveSetupChoices({ mode: mode ?? "quick", wantsTranscription, whisperModel, useVad, vadChoice, nativeLanguage, studyLanguage }));
  let plan = $derived(resources ? buildSetupPlan(choices, resources) : []);
  let installPlan = $state<typeof plan>([]);
  let displayedPlan = $derived(step === "installing" || step === "complete" ? installPlan : plan);
  let readyCount = $derived(displayedPlan.filter(task => taskStatuses[task.id] === "ready").length);
  let overallProgress = $derived(displayedPlan.length ? Math.round((readyCount + (currentTaskId && taskStatuses[currentTaskId] === "installing" ? (currentProgress ?? 0) / 100 : 0)) / displayedPlan.length * 100) : 0);
  let currentTask = $derived(displayedPlan.find(task => task.id === currentTaskId));

  async function refreshResources() {
    loadingResources = true;
    discoveryError = "";
    try {
      if (preview) {
        resources = { models: whisperModelsStore.whisperModels, vadModels: [{ id: "v6.2.0", size: "~0.9 MB", downloaded: false }], fonts: [], mediaReady: false, mediaDownloadBytes: null };
      } else {
        resources = await loadSetupResources();
        whisperModelsStore.whisperModels = resources.models;
        whisperModelsStore.vadModels = resources.vadModels;
        fontStore.fonts = resources.fonts;
      }
    } catch (error) { discoveryError = String(error); }
    finally { loadingResources = false; }
  }

  onMount(() => {
    void refreshResources();
    let disposed = false;
    const unlisteners: (() => void)[] = [];
    const register = (promise: Promise<() => void>) => {
      void promise.then(fn => { if (disposed) fn(); else unlisteners.push(fn); }).catch(console.error);
    };
    if (!preview) {
      register(listen<{ stage: string; percentage: number }>("transcribe-progress", event => {
        if (installing && (currentTask?.kind === "whisper" || currentTask?.kind === "vad") && event.payload.stage === "download") {
          currentProgress = Math.max(0, Math.min(100, Math.round(event.payload.percentage)));
        }
      }));
      register(listen<{ font_id: string; percentage: number }>("font-download-progress", event => {
        if (installing && currentTask?.kind === "font" && currentTask.resourceId === event.payload.font_id) {
          currentProgress = Math.max(0, Math.min(100, Math.round(event.payload.percentage)));
        }
      }));
    }
    return () => { disposed = true; unlisteners.forEach(fn => fn()); };
  });

  async function openReview() {
    step = "review";
    await refreshResources();
  }

  async function finish() {
    if (!mode || installing || loadingResources) return;
    if (preview) { installPlan = plan; taskStatuses = Object.fromEntries(plan.map(task => [task.id, "ready"])); step = "complete"; return; }
    installing = true;
    installError = "";
    try {
      await refreshResources();
      if (discoveryError || !resources) throw new Error(discoveryError || setupT("setup.resourceCheckFailed"));
      if (choices.wantsTranscription && choices.useVad && choices.vadChoice === "custom") {
        await whisperModelsStore.refreshVadCustomValid();
        if (!whisperModelsStore.vadCustomValid) throw new Error(setupT("settings.whisper.vadCustomInvalid"));
      }
      installPlan = buildSetupPlan(choices, resources);
      taskStatuses = {};
      currentTaskId = null;
      currentProgress = null;
      step = "installing";
      await installSetupPlan(installPlan, installSetupResource, (task, status) => {
        taskStatuses[task.id] = status;
        if (status === "installing") { currentTaskId = task.id; currentProgress = null; }
      });
      // Verify downloads before marking onboarding complete. A failed font or
      // missing model must not leave a seemingly successful setup.
      const verified = buildSetupPlan(choices, await loadSetupResources());
      const missing = verified.filter(task => !task.installed);
      if (missing.length) {
        missing.forEach(task => taskStatuses[task.id] = "failed");
        throw new Error(`${setupT("setup.resourceCheckFailed")}: ${missing.map(task => task.name).join(", ")}`);
      }
      currentTaskId = null;
      currentProgress = null;
      vestaConfig.setItem("srt-tools-ui-language", uiLanguage);
      vestaConfig.setItem("vesta-default-native-language", nativeLanguage);
      vestaConfig.setItem("vesta-default-target-language", nativeLanguage);
      vestaConfig.setItem("vesta-default-flashcards-language", studyLanguage);
      vestaConfig.setItem("vesta-default-transcribe-language", studyLanguage);
      vestaConfig.setItem("vesta-export-format", mode === "quick" ? "apkg" : exportFormat);
      vestaConfig.setItem("vesta-export-fallback", fallbackFormat);
      vestaConfig.setItem("vesta-expert-mode", String(mode === "custom"));
      vestaConfig.setItem("vesta-transcribe-vad", String(choices.wantsTranscription && choices.useVad));
      vestaConfig.setItem("srt-default-whisper-model", choices.whisperModel);
      vestaConfig.setItem("vesta-transcribe-tiers", setupTierPreferences(choices.whisperModel, choices.wantsTranscription));
      if (choices.wantsTranscription && choices.useVad && choices.vadChoice === "silero") saveVadSelection({ modelId: "v6.2.0", customPath: null });
      const media = loadMediaSettings();
      media.audioFormat = mode === "custom" && compactAudio ? "opus" : "mp3";
      media.audioBitrate = media.audioFormat === "opus" ? 64 : 128;
      saveMediaSettings(media);
      const preferences = vestaConfig.exportSnapshot();
      preferences["vesta-first-run-setup-complete"] = "true";
      delete preferences["vesta-first-run-force"];
      await vestaConfig.replaceAll(preferences);
      step = "complete";
    } catch (error) { installError = String(error); }
    finally { installing = false; }
  }
  async function startUsingApp(destination: SetupDestination) {
    if (openingApp) return;
    if (preview) { onClose?.(); return; }
    openingApp = true;
    installError = "";
    try { await onComplete(choices.wantsTranscription, destination); }
    catch (error) { installError = String(error); openingApp = false; }
  }
</script>

<div class="fixed inset-0 z-[100] flex items-center justify-center overflow-hidden bg-black/80 p-6" role="dialog" aria-modal="true">
  <div class="flex h-[720px] max-h-[calc(100vh-3rem)] w-full max-w-5xl flex-col overflow-visible rounded-2xl border border-indigo-400/30 bg-gray-900 p-7 shadow-2xl">
    <div class="flex items-start justify-between gap-5">
      <div><p class="text-xs font-bold uppercase tracking-[0.2em] text-indigo-300">Vesta</p><h1 class="mt-2 text-2xl font-bold text-white">{setupT("setup.setUpYourExperience")}</h1></div>
      {#if preview}<button type="button" class="btn-secondary px-4 py-2" onclick={onClose}>{setupT("common.close")}</button>{/if}
      {#if mode && step !== "installing" && step !== "complete"}<span class="rounded-full border border-indigo-400/25 bg-indigo-500/10 px-3 py-1 text-xs text-indigo-200">{mode === "quick" ? (setupT("setup.quick")) : setupT("setup.expert")}{#if mode === "custom"} · {step === "review" ? 4 : customStepNumber}/4{/if}</span>{/if}
    </div>

    <div class="min-h-0 flex-1">
    {#if mode === null}
      <div class="flex h-full items-center">
      <div class="grid w-full gap-4 sm:grid-cols-2">
        <button class="rounded-xl border border-indigo-400/30 bg-indigo-500/10 p-5 text-left hover:bg-indigo-500/20" onclick={() => { mode = "custom"; step = "languages"; }}>
          <span class="mb-3 flex h-9 w-9 items-center justify-center rounded-lg bg-indigo-400/15 text-indigo-200"><svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M4 7h10M18 7h2M4 17h2m4 0h10M14 5v4M8 15v4M4 12h4m4 0h8M10 10v4" /></svg></span>
          <strong class="text-white">{setupT("setup.customSetup")}</strong><p class="mt-2 text-xs text-gray-400">{setupT("setup.expertModeExportAudioAndLocalTranscription")}</p>
        </button>
        <button class="rounded-xl border border-emerald-400/30 bg-emerald-500/10 p-5 text-left hover:bg-emerald-500/20" onclick={() => { mode = "quick"; step = "languages"; whisperModel = "small"; }}>
          <span class="mb-3 flex h-9 w-9 items-center justify-center rounded-lg bg-emerald-400/15 text-emerald-200"><svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M13 2 4 14h7l-1 8 10-13h-7V2z" /></svg></span>
          <strong class="text-white">{setupT("setup.quickSetup")}</strong><p class="mt-2 text-xs text-gray-400">{setupT("setup.quickDescription")}</p>
        </button>
      </div>
      </div>
    {:else if step === "languages"}
      <div class="flex h-full flex-col">
        <div class="flex min-h-0 flex-1 items-center">
          <div inert={installing} class="mx-auto grid w-full max-w-3xl grid-cols-1 gap-4 sm:grid-cols-2">
            <SetupLanguageField kind="native" title={setupT("setup.yourNativeLanguage")} description={setupT("setup.interfaceTranslationsAndReferences")} value={nativeLanguage} onchange={(value) => { nativeLanguage = value; uiLanguage = resolveUiLanguage(value); }} />
            <SetupLanguageField kind="study" title={setupT("setup.studyLanguage")} description={setupT("setup.flashcardsAndTranscription")} value={studyLanguage} onchange={(value) => studyLanguage = value} />
          </div>
        </div>
        {#if installError}<p class="mb-4 text-sm text-[var(--danger)]" role="alert">{installError}</p>{/if}
        <div class="mt-auto flex justify-between"><button class="btn-secondary px-4 py-2" disabled={installing} onclick={() => mode = null}>{setupT("setup.back")}</button>{#if mode === "custom"}<button class="rounded-lg bg-indigo-500 px-5 py-2 font-semibold text-white" onclick={() => step = "export"}>{setupT("setup.continue")}</button>{:else}<button class="rounded-lg bg-indigo-500 px-5 py-2 font-semibold text-white" disabled={installing || loadingResources} onclick={finish}>{setupT(installing ? "setup.installing" : "setup.installEverything")}</button>{/if}</div>
      </div>
    {:else if step === "export"}
      <div class="flex h-full flex-col">
        <div class="min-h-0 flex-1 overflow-y-auto py-4">
        <div class="w-full">
        <div class="rounded-xl border border-white/10 bg-white/[0.03] p-4">
          <div class="mb-3 flex items-center gap-3"><span class="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-cyan-500/15 text-cyan-300"><svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M9 18V5l10-2v13M9 9l10-2M6 18a3 2 0 110 4 3 2 0 010-4zm10-2a3 2 0 110 4 3 2 0 010-4z"/></svg></span><p class="text-sm font-semibold text-white">{setupT("setup.audioFormat")}</p></div>
          <div class="relative grid grid-cols-2 rounded-lg bg-black/25 p-1">
            <span class="absolute bottom-1 top-1 w-[calc(50%-4px)] rounded-md border border-cyan-400/40 bg-cyan-500/20 transition-transform duration-200 ease-out {compactAudio ? 'translate-x-[calc(100%+4px)]' : 'translate-x-0'}"></span>
            <button class="relative z-10 px-3 py-2 text-left" aria-pressed={!compactAudio} onclick={() => compactAudio = false}><span class="block text-sm font-semibold {compactAudio ? 'text-gray-400' : 'text-cyan-100'}">MP3 · {setupT("setup.worksEverywhere")}</span><span class="block text-[10px] text-gray-500">128 kb/s · Anki Desktop, AnkiDroid, AnkiMobile</span></button>
            <button class="relative z-10 px-3 py-2 text-left" aria-pressed={compactAudio} onclick={() => compactAudio = true}><span class="block text-sm font-semibold {compactAudio ? 'text-cyan-100' : 'text-gray-400'}">Opus · {setupT("setup.compressed")}</span><span class="block text-[10px] text-gray-500">64 kb/s · Anki Desktop, AnkiDroid · {setupT("setup.notIos")}</span></button>
          </div>
        </div>

        <div class="mt-4 min-h-[238px] rounded-xl border border-white/10 bg-white/[0.03] p-4">
          <p class="mb-3 flex items-center gap-2 text-sm font-semibold text-white"><svg class="h-5 w-5 text-violet-300" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.7" d="M4 7l8-4 8 4-8 4-8-4zm0 5 8 4 8-4m-16 5 8 4 8-4"/></svg>{setupT("setup.howDoYouWantToExportFlashcards")}</p>
          <div class="relative grid grid-cols-3 rounded-lg bg-black/20 p-1" role="group" aria-label={setupT("setup.howDoYouWantToExportFlashcards")}>
            <span class="absolute bottom-1 top-1 w-[calc(33.333%-4px)] rounded-md border transition-transform duration-200 ease-out {exportFormat === 'apkg' ? 'border-emerald-400/40 bg-emerald-500/20' : exportFormat === 'tsv' ? 'border-violet-400/40 bg-violet-500/20' : 'border-cyan-400/40 bg-cyan-500/20'}" style="transform: translateX(calc({exportFormat === 'apkg' ? 0 : exportFormat === 'tsv' ? 100 : 200}% + {exportFormat === 'apkg' ? 0 : exportFormat === 'tsv' ? 4 : 8}px));"></span>
            <button class="relative z-10 flex items-center justify-center gap-2 px-3 py-3 text-sm font-semibold {exportFormat === 'apkg' ? 'text-emerald-200' : 'text-gray-400'}" aria-pressed={exportFormat === "apkg"} onclick={() => exportFormat = "apkg"}><svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 7l-8-4-8 4m16 0-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"/></svg>APKG</button><button class="relative z-10 flex items-center justify-center gap-2 px-3 py-3 text-sm font-semibold {exportFormat === 'tsv' ? 'text-violet-200' : 'text-gray-400'}" aria-pressed={exportFormat === "tsv"} onclick={() => exportFormat = "tsv"}><svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/></svg>TSV</button><button class="relative z-10 flex items-center justify-center gap-2 px-3 py-3 text-sm font-semibold {exportFormat === 'anki' ? 'text-cyan-200' : 'text-gray-400'}" aria-pressed={exportFormat === "anki"} onclick={() => exportFormat = "anki"}><svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M13 3 4 14h7v7l9-11h-7V3z"/></svg>AnkiConnect</button>
          </div>
          <p class="mt-3 text-xs text-gray-400">{exportFormat === "apkg" ? (setupT("setup.completePackageRecommendedForMostPeople")) : exportFormat === "tsv" ? (setupT("setup.tabularFileAndMediaFolderForManualImport")) : (setupT("setup.sendDirectlyToAnkiUseTheFallbackIfAnkiIsClosed"))}</p>
          <ExportFallbackSelector value={fallbackFormat} onchange={(value) => fallbackFormat = value} className="mt-4 min-h-10 border-t border-white/10 pt-4 {exportFormat === 'anki' ? 'visible' : 'invisible'}" />
        </div>
        </div>
        </div>
        <div class="mt-auto flex justify-between"><button class="btn-secondary px-4 py-2" onclick={() => step = "languages"}>{setupT("setup.back")}</button><button class="rounded-lg bg-indigo-500 px-5 py-2 font-semibold text-white" onclick={() => step = "transcription"}>{setupT("setup.continue")}</button></div>
      </div>
    {:else if step === "transcription"}
      <div class="flex h-full flex-col">
        <div class="min-h-0 flex-1 overflow-y-auto py-4">
        <div class="w-full rounded-xl border border-white/10 bg-white/[0.03] p-5"><div class="flex items-center justify-between gap-6"><div><p class="font-semibold text-white">{setupT("setup.localTranscription")}</p><p class="mt-1 text-xs text-gray-400">{setupT("setup.runWhisperOnThisDeviceExistingModelsWillNotBeDownloadedAgain")}</p></div><button aria-label={setupT("setup.toggleLocalTranscription")} aria-pressed={wantsTranscription} class="group flex h-10 min-w-24 shrink-0 items-center justify-between gap-2 rounded-full border px-2.5 transition-colors {wantsTranscription ? 'border-indigo-400/50 bg-indigo-500 text-white' : 'border-white/10 bg-gray-700 text-gray-300 hover:bg-gray-600'}" onclick={() => wantsTranscription = !wantsTranscription}><span class="pl-1 text-[10px] font-bold uppercase tracking-wider">{wantsTranscription ? setupT('common.on') : setupT('common.off')}</span><span class="relative h-6 w-11 rounded-full bg-black/25"><span class="absolute top-0.5 h-5 w-5 rounded-full bg-white shadow-sm transition-all {wantsTranscription ? 'left-5.5' : 'left-0.5'}"></span></span></button></div>
          <div class="mt-5 {wantsTranscription ? '' : 'pointer-events-none opacity-45'}"><p class="mb-2 text-xs font-semibold text-gray-300">{setupT("settings.whisper.modelLabel")}</p><WhisperModelSelector models={whisperModelsStore.whisperModels} value={whisperModel} disabled={!wantsTranscription} onselect={(model) => whisperModel = model.id} /></div>
          <div class="mt-4 grid grid-cols-2 gap-3 {wantsTranscription ? '' : 'pointer-events-none opacity-45'}">
            <button disabled={!wantsTranscription} class="rounded-xl border p-4 text-left {useVad && vadChoice === 'silero' ? 'border-emerald-400/40 bg-emerald-500/10' : 'border-white/10 bg-white/[0.03]'}" onclick={() => { useVad = true; vadChoice = 'silero'; }}><p class="flex items-center gap-2 text-sm font-semibold text-white"><svg class="h-4 w-4 text-emerald-300" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M4 12h3l2-5 4 10 2-5h5"/></svg>Silero Voice Activity Detection 6.2.0</p><p class="mt-1 text-[11px] {sileroDownloaded ? 'text-emerald-400' : 'text-gray-400'}">{sileroDownloaded ? (setupT("setup.downloadedReady")) : (setupT("setup.willDownloadWhenSetupFinishes"))}</p></button>
            <button disabled={!wantsTranscription} class="rounded-xl border p-4 text-left {useVad && vadChoice === 'custom' ? 'border-sky-400/40 bg-sky-500/10' : 'border-white/10 bg-white/[0.03]'}" onclick={async () => { await whisperModelsStore.pickCustomVad(); if (whisperModelsStore.vadCustomValid) { useVad = true; vadChoice = 'custom'; } }}><p class="flex items-center gap-2 text-sm font-semibold text-white"><svg class="h-4 w-4 text-sky-300" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M12 16V4m0 0-4 4m4-4 4 4M5 15v4h14v-4"/></svg>{setupT("setup.customVoiceActivityDetectionModel")}</p><p class="mt-1 truncate text-[11px] text-gray-400">{whisperModelsStore.vadSelection.customPath ?? (setupT("setup.loadABinFile"))}</p></button>
          </div>
          <button disabled={!wantsTranscription} aria-pressed={!useVad} class="mt-3 text-xs text-gray-400 hover:text-white disabled:opacity-50" onclick={() => useVad = !useVad}>{useVad ? setupT("setup.disableVad") : setupT("setup.enableVad")}</button>
        </div>
        </div>
        {#if installError}<p class="mt-4 text-sm text-red-300">{installError}</p>{/if}
        <div class="mt-auto flex justify-between"><button class="btn-secondary px-4 py-2" disabled={installing} onclick={() => step = "export"}>{setupT("setup.back")}</button><button class="rounded-lg bg-indigo-500 px-5 py-2 font-semibold text-white disabled:opacity-50" disabled={installing} onclick={openReview}>{setupT("setup.continue")}</button></div>
      </div>
    {:else if step === "review" || step === "installing"}
      <div class="flex h-full min-h-0 flex-col pt-6">
        <div class="min-h-0 flex-1 overflow-y-auto pr-1">
          <h2 class="text-xl font-semibold text-white">{setupT(step === "review" ? "setup.reviewTitle" : "setup.installTitle")}</h2>
          {#if step === "installing"}<p class="mt-2 text-sm text-gray-400">{setupT("setup.installDescription")}</p>{/if}
          {#if step === "review" && mode === "custom"}
            <dl class="review-preferences">
              <div><dt><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 3a4 4 0 1 0 0 8 4 4 0 0 0 0-8M5 21a7 7 0 0 1 14 0"/></svg>{setupT("setup.yourNativeLanguage")}</dt><dd><span class="language-badge"><span aria-hidden="true">{languages.find(language => language.code === nativeLanguage)?.flag ?? ""}</span> {languages.find(language => language.code === nativeLanguage)?.name ?? nativeLanguage}</span></dd></div>
              <div><dt><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m3 6 9-3 9 3-9 3-9-3m3 2v8l6 3 6-3V8m3-2v9"/></svg>{setupT("setup.studyLanguage")}</dt><dd><span class="language-badge"><span aria-hidden="true">{languages.find(language => language.code === studyLanguage)?.flag ?? ""}</span> {languages.find(language => language.code === studyLanguage)?.name ?? studyLanguage}</span></dd></div>
              <div><dt><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M9 18V5l10-2v13M9 9l10-2M9 18a3 2 0 1 1-3-2 3 2 0 0 1 3 2m10-2a3 2 0 1 1-3-2 3 2 0 0 1 3 2"/></svg>{setupT("setup.audioFormat")}</dt><dd>{compactAudio ? "Opus · 64 kb/s" : "MP3 · 128 kb/s"} · {exportFormat === "anki" ? "AnkiConnect" : exportFormat.toUpperCase()}</dd></div>
            </dl>
          {/if}
          {#if discoveryError && step === "review"}
            <p class="mt-4 text-sm text-red-300" role="alert">{setupT("setup.resourceCheckFailed")}: {discoveryError}</p>
            <button class="btn-secondary mt-3 px-4 py-2" disabled={loadingResources} onclick={refreshResources}>{setupT("common.retry")}</button>
          {/if}
          <ul class="mt-5 space-y-2">
            {#each displayedPlan as task (task.id)}
              <li class="flex items-center justify-between gap-4 rounded-xl border border-white/10 bg-white/[0.03] px-4 py-3">
                <div class="flex min-w-0 items-center gap-3">
                  <span class="component-icon" aria-hidden="true">
                    <svg width="22" height="22" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 24 24">
                      {#if task.kind === "media"}<rect x="3" y="5" width="18" height="14" rx="2"/><path d="m10 9 5 3-5 3zM7 5v14M17 5v14"/>
                      {:else if task.kind === "whisper"}<rect x="9" y="2" width="6" height="12" rx="3"/><path d="M5 10v2a7 7 0 0 0 14 0v-2M12 19v3m-4 0h8"/>
                      {:else if task.kind === "vad"}<path d="M2 12h4l3-7 6 14 3-7h4"/>
                      {:else}<path d="m4 20 7-16h2l7 16M7 14h10"/>{/if}
                    </svg>
                  </span>
                  <div class="min-w-0"><p class="text-sm font-semibold text-white">{task.name}</p><p class="mt-1 text-xs text-gray-400">{task.installed ? setupT("setup.alreadyInstalled") : task.size || setupT("setup.sizeUnavailable")}</p></div></div>
                <span class="shrink-0 text-xs {taskStatuses[task.id] === 'failed' ? 'text-red-300' : task.installed || taskStatuses[task.id] === 'ready' ? 'text-emerald-300' : 'text-indigo-300'}">
                  {setupT(task.installed || taskStatuses[task.id] === "ready" ? "setup.componentReady" : taskStatuses[task.id] === "failed" ? "setup.componentFailed" : taskStatuses[task.id] === "installing" ? "setup.componentInstalling" : "setup.componentPending")}
                </span>
              </li>
            {/each}
          </ul>
          {#if step === "review"}<p class="mt-3 text-xs text-gray-500">{setupT("setup.sizeNote")}</p>{/if}
          {#if step === "installing"}
            <div class="mt-5" role="status" aria-live="polite">
              <p class="mb-2 text-sm text-indigo-200">{setupT("setup.componentsProgress", { ready: readyCount, total: displayedPlan.length })}</p>
              <progress class="setup-progress" max="100" value={overallProgress} aria-label={setupT("setup.installTitle")}></progress>
              {#if installing && currentTask}
                <p class="mt-3 text-xs text-gray-300">{currentTask.name}{currentProgress !== null ? ` · ${currentProgress}%` : ` · ${setupT("setup.downloadAndPrepare")}`}</p>
                <progress class="setup-progress task-progress" max="100" value={currentProgress ?? undefined} aria-label={currentTask.name}></progress>
              {/if}
            </div>
          {/if}
          {#if installError}<p class="mt-4 text-sm text-red-300" role="alert">{setupT("setup.installFailed")}: {installError}</p><p class="mt-2 text-xs text-gray-400">{setupT("setup.retryDescription")}</p>{/if}
        </div>
        <div class="mt-5 flex justify-between gap-3">
          <button class="btn-secondary px-4 py-2" disabled={installing} onclick={() => { step = mode === "quick" ? "languages" : "transcription"; installError = ""; }}>{setupT("setup.back")}</button>
          <button class="rounded-lg bg-indigo-500 px-5 py-2 font-semibold text-white disabled:opacity-50" disabled={installing || loadingResources || (step === "review" && (!!discoveryError || !resources))} onclick={finish}>{setupT(installing ? "setup.installing" : installError ? "common.retry" : "setup.installEverything")}</button>
        </div>
      </div>
    {:else if step === "complete"}
      <div class="flex h-full min-h-0 flex-col justify-center overflow-y-auto py-6">
        <div class="setup-complete">
          <svg class="mb-4 h-10 w-10 text-emerald-300" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>
          <h2 class="text-2xl font-semibold text-white">{setupT("setup.readyTitle")}</h2>
          <button disabled={openingApp} class="setup-start" onclick={() => startUsingApp("flashcards")}>
            {setupT("setup.start")}
            <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="m9 5 7 7-7 7"/></svg>
          </button>
          {#if installError}<p class="mt-4 text-sm text-red-300" role="alert">{installError}</p>{/if}
        </div>
      </div>
    {/if}
    </div>
  </div>
</div>

<style>
  .language-badge { display: inline-flex; align-items: center; gap: .5rem; padding: .45rem .65rem; border: 1px solid var(--border); border-radius: .6rem; background: var(--glass-bg); }

  .setup-complete { margin-inline: auto; inline-size: 100%; max-inline-size: 28rem; text-align: center; }
  .setup-complete > svg { margin-inline: auto; inline-size: 3.5rem; block-size: 3.5rem; color: var(--done); }
  .setup-start {
    display: flex; align-items: center; justify-content: center; gap: .75rem;
    inline-size: 100%; min-block-size: 3.75rem; margin-block-start: 2rem;
    border: 1px solid var(--accent); border-radius: .9rem;
    background: var(--accent); color: var(--accent-text); font-size: 1.125rem; font-weight: 650;
    transition: background .15s ease, box-shadow .15s ease;
  }
  .setup-start:hover { background: var(--accent-strong); box-shadow: 0 6px 24px color-mix(in srgb, var(--accent) 25%, transparent); }
  .setup-start:focus-visible { outline: 2px solid var(--accent); outline-offset: 4px; }
  .setup-start:disabled { opacity: .5; }
  .setup-start svg { transform: scaleX(1); }
  :global([dir="rtl"]) .setup-start svg { transform: scaleX(-1); }
  .setup-progress {
    display: block; appearance: none; inline-size: 100%; block-size: .65rem;
    overflow: hidden; border: 1px solid var(--border); border-radius: 999px;
    background: var(--surface-3); color: var(--accent); accent-color: var(--accent);
  }
  .setup-progress::-webkit-progress-bar { background: var(--surface-3); border-radius: 999px; }
  .setup-progress::-webkit-progress-value { background: linear-gradient(90deg, var(--accent), var(--accent-strong)); border-radius: 999px; transition: width .25s ease; }
  .setup-progress::-moz-progress-bar { background: linear-gradient(90deg, var(--accent), var(--accent-strong)); border-radius: 999px; }
  .task-progress { margin-block-start: .65rem; block-size: .4rem; }
  .setup-progress:indeterminate { background: linear-gradient(90deg, var(--surface-3) 20%, var(--accent) 50%, var(--surface-3) 80%); background-size: 200% 100%; animation: download-pulse 1.8s linear infinite; }
  .setup-progress:indeterminate::-webkit-progress-bar { background: transparent; }
  @keyframes download-pulse { to { background-position: -200% 0; } }
  @media (prefers-reduced-motion: reduce) { .setup-progress:indeterminate { animation: none; } .setup-progress::-webkit-progress-value { transition: none; } }

  .review-preferences {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr));
    gap: 1rem;
    margin-block-start: 1.25rem;
    padding-block-end: 1.25rem;
    border-block-end: 1px solid var(--border);
    font-size: .8rem;
  }
  .review-preferences dt { display: flex; align-items: center; gap: .5rem; color: var(--muted); }
  .review-preferences dt svg { color: var(--accent-text); }
  .review-preferences dd { margin-block-start: .35rem; color: var(--text); font-weight: 600; }

  .component-icon {
    display: grid;
    place-items: center;
    flex-shrink: 0;
    inline-size: 2.75rem;
    block-size: 2.75rem;
    border-radius: .75rem;
    color: var(--accent-text);
    background: var(--glass-bg);
  }
</style>
