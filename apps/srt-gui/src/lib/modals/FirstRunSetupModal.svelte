<script lang="ts">
  import { onMount } from "svelte";
  import { availableUILanguages, currentLanguage } from "$lib/i18n";
  import { languages, getLanguageSearchTerms } from "$lib/config/languages";
  import SearchableSelect from "$lib/components/SearchableSelect.svelte";
  import WhisperModelSelector from "$lib/components/WhisperModelSelector.svelte";
  import { loadMediaSettings, saveMediaSettings } from "$lib/utils/mediaSettings";
  import { transcribeDownloadModel, transcribeDownloadVad } from "$lib/services/transcribe";
  import { whisperModelsStore } from "$lib/stores/whisperModelsStore.svelte";
  import { fontStore } from "$lib/stores/fontStore.svelte";
  import type { ExportFormat, ExportFallbackFormat } from "$lib/stores/exportFormatStore.svelte";
  import * as vestaConfig from "$lib/config/vestaConfig";

  let { onComplete }: { onComplete: (wantsTranscription: boolean) => void } = $props();
  let mode = $state<"quick" | "custom" | null>(null);
  let step = $state<"languages" | "export" | "transcription">("languages");
  let uiLanguage = $state($currentLanguage);
  let nativeLanguage = $state(languages.some((l) => l.code === $currentLanguage) ? $currentLanguage : "en");
  let studyLanguage = $state("en");
  let targetLanguage = $state(languages.some((l) => l.code === $currentLanguage) ? $currentLanguage : "en");
  let transcribeLanguage = $state("en");
  let exportFormat = $state<ExportFormat>("apkg");
  let fallbackFormat = $state<ExportFallbackFormat>("apkg");
  let compactAudio = $state(false);
  let wantsTranscription = $state(false);
  let useVad = $state(true);
  let vadChoice = $state<"silero" | "custom">("silero");
  let whisperModel = $state("small");
  let installing = $state(false);
  let installMessage = $state("");
  let installError = $state("");
  let it = $derived(uiLanguage === "it");

  const uiLanguageOptions = availableUILanguages.map((l) => ({ value: l.code, label: `${l.nativeName} — ${l.name}`, icon: l.flag, searchTerms: `${l.code} ${l.name} ${l.nativeName}` }));
  const languageOptions = languages.map((l) => ({ value: l.code, label: l.name === l.nameEn ? l.name : `${l.name} — ${l.nameEn}`, icon: l.flag, searchTerms: getLanguageSearchTerms(l.code) }));
  let selectedWhisperDownloaded = $derived(whisperModelsStore.whisperModels.find((model) => model.id === whisperModel)?.downloaded ?? false);
  let sileroDownloaded = $derived(whisperModelsStore.vadModels.find((model) => model.id === "v6.2.0")?.downloaded ?? false);

  const customStepNumber = $derived(step === "languages" ? 1 : step === "export" ? 2 : 3);

  onMount(async () => {
    whisperModelsStore.defaultWhisperModel = vestaConfig.getItem("srt-default-whisper-model") || "small";
    await Promise.all([whisperModelsStore.refreshModels(), whisperModelsStore.refreshAddons()]);
    const preferred = whisperModelsStore.whisperModels.find((model) => model.id === whisperModelsStore.defaultWhisperModel && model.downloaded)
      ?? whisperModelsStore.whisperModels.find((model) => model.downloaded);
    if (preferred) whisperModel = preferred.id;
    if (whisperModelsStore.vadSelection.customPath && whisperModelsStore.vadCustomValid) vadChoice = "custom";
  });

  function cycleExportFormat() {
    exportFormat = exportFormat === "apkg" ? "tsv" : exportFormat === "tsv" ? "anki" : "apkg";
  }

  function toggleFallbackFormat() {
    fallbackFormat = fallbackFormat === "apkg" ? "tsv" : "apkg";
  }

  async function finish() {
    if (!mode || installing) return;
    installing = true;
    installError = "";
    try {
      if (mode === "custom" && wantsTranscription) {
        installMessage = it ? `Download Whisper ${whisperModel}…` : `Downloading Whisper ${whisperModel}…`;
        if (!selectedWhisperDownloaded) await transcribeDownloadModel(whisperModel);
        if (useVad) {
          if (vadChoice === "silero" && !sileroDownloaded) {
            installMessage = it ? "Download Silero VAD 6.2.0…" : "Downloading Silero VAD 6.2.0…";
            await transcribeDownloadVad("v6.2.0");
          }
        }
      }
      installMessage = it ? "Controllo dei font per le lingue scelte…" : "Checking fonts for the selected languages…";
      await fontStore.loadFonts();
      const requiredFonts = [studyLanguage, nativeLanguage]
        .map((language) => fontStore.getFontForLanguage(language))
        .filter((font, index, all) => font && all.findIndex((candidate) => candidate?.id === font.id) === index);
      for (const font of requiredFonts) {
        if (font && !font.downloaded) {
          installMessage = it ? `Download ${font.name} (${font.approx_size})…` : `Downloading ${font.name} (${font.approx_size})…`;
          await fontStore.downloadFont(font.id);
        }
      }
      vestaConfig.setItem("vesta-first-run-setup-complete", "true");
      vestaConfig.removeItem("vesta-first-run-force");
      vestaConfig.setItem("srt-tools-ui-language", uiLanguage);
      vestaConfig.setItem("vesta-default-native-language", nativeLanguage);
      vestaConfig.setItem("vesta-default-target-language", mode === "quick" ? nativeLanguage : targetLanguage);
      vestaConfig.setItem("vesta-default-flashcards-language", studyLanguage);
      vestaConfig.setItem("vesta-default-transcribe-language", mode === "quick" ? studyLanguage : transcribeLanguage);
      vestaConfig.setItem("vesta-export-format", mode === "quick" ? "apkg" : exportFormat);
      vestaConfig.setItem("vesta-export-fallback", fallbackFormat);
      vestaConfig.setItem("vesta-expert-mode", String(mode === "custom"));
      vestaConfig.setItem("vesta-transcribe-vad", String(wantsTranscription && useVad));
      vestaConfig.setItem("srt-default-whisper-model", whisperModel);
      if (wantsTranscription && useVad && vadChoice === "silero") whisperModelsStore.selectVadModel("v6.2.0");
      const media = loadMediaSettings();
      media.audioFormat = mode === "custom" && compactAudio ? "opus" : "mp3";
      media.audioBitrate = media.audioFormat === "opus" ? 64 : 128;
      saveMediaSettings(media);
      onComplete(wantsTranscription);
    } catch (error) { installError = String(error); }
    finally { installing = false; installMessage = ""; }
  }
</script>

<div class="fixed inset-0 z-[100] flex items-center justify-center overflow-hidden bg-black/80 p-6" role="dialog" aria-modal="true">
  <div class="flex h-[720px] max-h-[calc(100vh-3rem)] w-full max-w-5xl flex-col overflow-visible rounded-2xl border border-indigo-400/30 bg-gray-900 p-7 shadow-2xl">
    <div class="flex items-start justify-between gap-5">
      <div><p class="text-xs font-bold uppercase tracking-[0.2em] text-indigo-300">Vesta</p><h1 class="mt-2 text-2xl font-bold text-white">{it ? "Configura la tua esperienza" : "Set up your experience"}</h1><p class="mt-2 text-sm text-gray-400">{it ? "Tre scelte essenziali, poi Vesta prepara il resto." : "Three essential choices, then Vesta prepares the rest."}</p></div>
      {#if mode}<span class="rounded-full border border-indigo-400/25 bg-indigo-500/10 px-3 py-1 text-xs text-indigo-200">{mode === "quick" ? (it ? "Rapido" : "Quick") : "Expert"} · {mode === "quick" ? 1 : customStepNumber}/{mode === "custom" ? 3 : 1}</span>{/if}
    </div>

    <div class="min-h-0 flex-1">
    {#if mode === null}
      <div class="mt-7 grid gap-4 sm:grid-cols-2">
        <button class="rounded-xl border border-indigo-400/30 bg-indigo-500/10 p-5 text-left hover:bg-indigo-500/20" onclick={() => { mode = "custom"; step = "languages"; }}>
          <span class="mb-3 flex h-9 w-9 items-center justify-center rounded-lg bg-indigo-400/15 text-indigo-200"><svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M4 7h10M18 7h2M4 17h2m4 0h10M14 5v4M8 15v4M4 12h4m4 0h8M10 10v4" /></svg></span>
          <strong class="text-white">{it ? "Setup personalizzato" : "Custom setup"}</strong><p class="mt-2 text-xs text-gray-400">{it ? "Modalità Expert, esportazione, audio e trascrizione locale." : "Expert mode, export, audio and local transcription."}</p>
        </button>
        <button class="rounded-xl border border-emerald-400/30 bg-emerald-500/10 p-5 text-left hover:bg-emerald-500/20" onclick={() => { mode = "quick"; step = "languages"; }}>
          <span class="mb-3 flex h-9 w-9 items-center justify-center rounded-lg bg-emerald-400/15 text-emerald-200"><svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M13 2 4 14h7l-1 8 10-13h-7V2z" /></svg></span>
          <strong class="text-white">{it ? "Setup rapido" : "Quick setup"}</strong><p class="mt-2 text-xs text-gray-400">{it ? "Lingue, interfaccia semplice e impostazioni conservative." : "Languages, a simple interface and conservative defaults."}</p>
        </button>
      </div>
    {:else if step === "languages"}
      <div class="flex h-full flex-col">
        <div class="mt-5 grid grid-cols-1 gap-3 {mode === 'custom' ? 'lg:grid-cols-3' : 'lg:grid-cols-3'}">
          <div class="rounded-xl border border-white/10 bg-white/[0.03] p-4"><div class="mb-3 flex items-center gap-2"><span class="flex h-8 w-8 items-center justify-center rounded-lg bg-emerald-500/15 text-emerald-300"><svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M12 3a4 4 0 110 8 4 4 0 010-8zM5 21a7 7 0 0114 0"/></svg></span><div><p class="text-sm font-semibold text-white">{it ? "La tua lingua madre" : "Your native language"}</p><p class="text-[11px] text-gray-500">{it ? "Significati e riferimenti" : "Meanings and references"}</p></div></div><SearchableSelect options={languageOptions} value={nativeLanguage} onchange={(v) => { nativeLanguage = v; if (mode === 'quick') targetLanguage = v; }} /></div>
          <div class="rounded-xl border border-white/10 bg-white/[0.03] p-4"><div class="mb-3 flex items-center gap-2"><span class="flex h-8 w-8 items-center justify-center rounded-lg bg-violet-500/15 text-violet-300">文</span><div><p class="text-sm font-semibold text-white">{it ? "Lingua studiata" : "Study language"}</p><p class="text-[11px] text-gray-500">{it ? "Originali delle flashcard" : "Flashcard originals"}</p></div></div><SearchableSelect options={languageOptions} value={studyLanguage} onchange={(v) => { studyLanguage = v; if (mode === 'quick') transcribeLanguage = v; }} /></div>
          <div class="rounded-xl border border-white/10 bg-white/[0.03] p-4"><div class="mb-3 flex items-center gap-2"><span class="flex h-8 w-8 items-center justify-center rounded-lg bg-sky-500/15 text-sky-300"><svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M4 6h16M4 12h10M4 18h7"/></svg></span><div><p class="text-sm font-semibold text-white">{it ? "Interfaccia" : "Interface"}</p><p class="text-[11px] text-gray-500">{it ? "Lingua dei menu" : "Language for menus"}</p></div></div><SearchableSelect options={uiLanguageOptions} value={uiLanguage} onchange={(v) => uiLanguage = v} /></div>
          {#if mode === "custom"}
            <div class="rounded-xl border border-white/10 bg-white/[0.03] p-4"><div class="mb-3 flex items-center gap-2"><span class="flex h-8 w-8 items-center justify-center rounded-lg bg-cyan-500/15 text-cyan-300"><svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M7 7h11m0 0-3-3m3 3-3 3M17 17H6m0 0 3 3m-3-3 3-3"/></svg></span><div><p class="text-sm font-semibold text-white">{it ? "Lingua di traduzione" : "Translation language"}</p><p class="text-[11px] text-gray-500">{it ? "Destinazione di Translate" : "Translate destination"}</p></div></div><SearchableSelect options={languageOptions} value={targetLanguage} onchange={(v) => targetLanguage = v} /></div>
            <div class="rounded-xl border border-white/10 bg-white/[0.03] p-4"><div class="mb-3 flex items-center gap-2"><span class="flex h-8 w-8 items-center justify-center rounded-lg bg-amber-500/15 text-amber-300"><svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M12 3a3 3 0 00-3 3v6a3 3 0 006 0V6a3 3 0 00-3-3zM5 11a7 7 0 0014 0M12 18v3m-4 0h8"/></svg></span><div><p class="text-sm font-semibold text-white">{it ? "Lingua di trascrizione" : "Transcription language"}</p><p class="text-[11px] text-gray-500">{it ? "Lingua parlata per Whisper" : "Spoken language for Whisper"}</p></div></div><SearchableSelect options={languageOptions} value={transcribeLanguage} onchange={(v) => transcribeLanguage = v} /></div>
          {/if}
        </div>
        <div class="mt-auto flex justify-between"><button class="btn-secondary px-4 py-2" onclick={() => mode = null}>{it ? "Indietro" : "Back"}</button>{#if mode === "custom"}<button class="rounded-lg bg-indigo-500 px-5 py-2 font-semibold text-white" onclick={() => step = "export"}>{it ? "Continua" : "Continue"}</button>{:else}<button class="rounded-lg bg-indigo-500 px-5 py-2 font-semibold text-white" onclick={finish}>{it ? "Completa setup" : "Finish setup"}</button>{/if}</div>
      </div>
    {:else if step === "export"}
      <div class="flex h-full flex-col">
        <div class="mt-5 rounded-xl border border-white/10 bg-white/[0.03] p-4">
          <div class="mb-3 flex items-center gap-3"><span class="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-cyan-500/15 text-cyan-300"><svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M9 18V5l10-2v13M9 9l10-2M6 18a3 2 0 110 4 3 2 0 010-4zm10-2a3 2 0 110 4 3 2 0 010-4z"/></svg></span><div><p class="text-sm font-semibold text-white">{it ? "Formato audio" : "Audio format"}</p><p class="text-xs text-gray-400">{it ? "MP3 mantiene la compatibilità più ampia; Opus comprime meglio." : "MP3 keeps the widest compatibility; Opus compresses more efficiently."}</p></div></div>
          <div class="relative grid grid-cols-2 rounded-lg bg-black/25 p-1">
            <span class="absolute bottom-1 top-1 w-[calc(50%-4px)] rounded-md border border-cyan-400/40 bg-cyan-500/20 transition-transform duration-200 ease-out {compactAudio ? 'translate-x-[calc(100%+4px)]' : 'translate-x-0'}"></span>
            <button class="relative z-10 px-3 py-2 text-left" onclick={() => compactAudio = false}><span class="block text-sm font-semibold {compactAudio ? 'text-gray-400' : 'text-cyan-100'}">MP3 · {it ? "Alta compatibilità" : "Full compatibility"}</span><span class="block text-[10px] text-gray-500">128 kb/s · Anki Desktop, AnkiDroid, AnkiMobile</span></button>
            <button class="relative z-10 px-3 py-2 text-left" onclick={() => compactAudio = true}><span class="block text-sm font-semibold {compactAudio ? 'text-cyan-100' : 'text-gray-400'}">Opus · {it ? "Compatto" : "Compressed"}</span><span class="block text-[10px] text-gray-500">64 kb/s · {it ? "circa metà spazio" : "roughly half the storage"}</span></button>
          </div>
          <p class="mt-3 text-[11px] leading-relaxed text-gray-400">{it ? "Entrambi sono formati con perdita: Opus a 64 kb/s elimina dati audio non percepibili e di solito conserva molto bene il parlato, ma non è riproducibile su AnkiMobile per iOS. MP3 non è audio compatto ed è il valore sicuro predefinito." : "Both formats are lossy: Opus at 64 kb/s removes inaudible audio data and usually preserves speech very well, but AnkiMobile on iOS cannot play it. MP3 is not compact audio and remains the safe default."}</p>
        </div>

        <div class="mt-4 min-h-[238px] rounded-xl border border-white/10 bg-white/[0.03] p-4">
          <p class="mb-3 flex items-center gap-2 text-sm font-semibold text-white"><svg class="h-5 w-5 text-violet-300" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.7" d="M4 7l8-4 8 4-8 4-8-4zm0 5 8 4 8-4m-16 5 8 4 8-4"/></svg>{it ? "Come vuoi esportare le flashcard?" : "How do you want to export flashcards?"}</p>
          <div class="relative grid grid-cols-3 rounded-lg bg-black/20 p-1" role="switch" aria-checked={exportFormat !== "apkg"} tabindex="0" onkeydown={(event) => (event.key === "Enter" || event.key === " ") && cycleExportFormat()}>
            <span class="absolute bottom-1 top-1 w-[calc(33.333%-4px)] rounded-md border border-violet-400/40 bg-violet-500/20 transition-transform duration-200 ease-out" style="transform: translateX(calc({exportFormat === 'apkg' ? 0 : exportFormat === 'tsv' ? 100 : 200}% + {exportFormat === 'apkg' ? 0 : exportFormat === 'tsv' ? 4 : 8}px));"></span>
            <button class="relative z-10 flex items-center justify-center gap-2 px-3 py-3 text-sm font-semibold {exportFormat === 'apkg' ? 'text-emerald-200' : 'text-gray-400'}" onclick={cycleExportFormat}>APKG</button><button class="relative z-10 flex items-center justify-center gap-2 px-3 py-3 text-sm font-semibold {exportFormat === 'tsv' ? 'text-sky-200' : 'text-gray-400'}" onclick={cycleExportFormat}>TSV</button><button class="relative z-10 flex items-center justify-center gap-2 px-3 py-3 text-sm font-semibold {exportFormat === 'anki' ? 'text-violet-200' : 'text-gray-400'}" onclick={cycleExportFormat}>AnkiConnect</button>
          </div>
          <p class="mt-3 text-xs text-gray-400">{exportFormat === "apkg" ? (it ? "Pacchetto completo, consigliato per quasi tutti." : "Complete package, recommended for most people.") : exportFormat === "tsv" ? (it ? "File tabellare e cartella media per importazione manuale." : "Tabular file and media folder for manual import.") : (it ? "Invio diretto ad Anki; usa il formato di riserva se Anki è chiuso." : "Send directly to Anki; use the fallback if Anki is closed.")}</p>
          <div class="mt-4 flex min-h-10 items-center justify-between gap-3 border-t border-white/10 pt-4 {exportFormat === 'anki' ? 'visible' : 'invisible'}"><span class="text-xs text-gray-300">{it ? "Formato di riserva" : "Fallback format"}</span><div class="relative grid grid-cols-2 rounded-lg bg-black/30 p-1"><span class="absolute bottom-1 top-1 w-[calc(50%-4px)] rounded-md bg-emerald-500/20 transition-transform duration-200 {fallbackFormat === 'tsv' ? 'translate-x-[calc(100%+4px)]' : ''}"></span><button class="relative z-10 px-4 py-2 text-xs font-semibold {fallbackFormat === 'apkg' ? 'text-emerald-200' : 'text-gray-400'}" onclick={toggleFallbackFormat}>APKG</button><button class="relative z-10 px-4 py-2 text-xs font-semibold {fallbackFormat === 'tsv' ? 'text-sky-200' : 'text-gray-400'}" onclick={toggleFallbackFormat}>TSV</button></div></div>
        </div>
        <div class="mt-auto flex justify-between"><button class="btn-secondary px-4 py-2" onclick={() => step = "languages"}>{it ? "Indietro" : "Back"}</button><button class="rounded-lg bg-indigo-500 px-5 py-2 font-semibold text-white" onclick={() => step = "transcription"}>{it ? "Continua" : "Continue"}</button></div>
      </div>
    {:else}
      <div class="flex h-full flex-col">
        <div class="mt-4 rounded-xl border border-white/10 bg-white/[0.03] p-5"><div class="flex items-center justify-between gap-4"><div><p class="font-semibold text-white">{it ? "Trascrizione locale" : "Local transcription"}</p><p class="mt-1 text-xs text-gray-400">{it ? "Usa Whisper sul dispositivo; i modelli presenti non saranno riscaricati." : "Run Whisper on this device; existing models will not be downloaded again."}</p></div><button aria-label={it ? "Attiva trascrizione locale" : "Enable local transcription"} class="relative h-6 w-12 rounded-full {wantsTranscription ? 'bg-indigo-500' : 'bg-gray-600'}" onclick={() => wantsTranscription = !wantsTranscription}><span class="absolute top-0.5 h-5 w-5 rounded-full bg-white transition-all {wantsTranscription ? 'left-6' : 'left-0.5'}"></span></button></div>
          <div class="mt-5 {wantsTranscription ? '' : 'pointer-events-none opacity-45'}"><p class="mb-2 text-xs font-semibold text-gray-300">Whisper model</p><WhisperModelSelector models={whisperModelsStore.whisperModels} value={whisperModel} disabled={!wantsTranscription} onselect={(model) => whisperModel = model.id} /></div>
          <div class="mt-4 grid grid-cols-2 gap-3 {wantsTranscription ? '' : 'pointer-events-none opacity-45'}">
            <button disabled={!wantsTranscription} class="rounded-xl border p-4 text-left {useVad && vadChoice === 'silero' ? 'border-emerald-400/40 bg-emerald-500/10' : 'border-white/10 bg-white/[0.03]'}" onclick={() => { useVad = true; vadChoice = 'silero'; }}><p class="flex items-center gap-2 text-sm font-semibold text-white"><svg class="h-4 w-4 text-emerald-300" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M4 12h3l2-5 4 10 2-5h5"/></svg>Silero VAD 6.2.0</p><p class="mt-1 text-[11px] {sileroDownloaded ? 'text-emerald-400' : 'text-gray-400'}">{sileroDownloaded ? (it ? "Scaricato · pronto" : "Downloaded · ready") : (it ? "Sarà scaricato al termine" : "Will download when setup finishes")}</p></button>
            <button disabled={!wantsTranscription} class="rounded-xl border p-4 text-left {useVad && vadChoice === 'custom' ? 'border-sky-400/40 bg-sky-500/10' : 'border-white/10 bg-white/[0.03]'}" onclick={async () => { await whisperModelsStore.pickCustomVad(); if (whisperModelsStore.vadCustomValid) { useVad = true; vadChoice = 'custom'; } }}><p class="flex items-center gap-2 text-sm font-semibold text-white"><svg class="h-4 w-4 text-sky-300" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M12 16V4m0 0-4 4m4-4 4 4M5 15v4h14v-4"/></svg>{it ? "Modello VAD personalizzato" : "Custom VAD model"}</p><p class="mt-1 truncate text-[11px] text-gray-400">{whisperModelsStore.vadSelection.customPath ?? (it ? "Carica un file .bin" : "Load a .bin file")}</p></button>
          </div>
        </div>
        {#if installMessage}<p class="mt-4 text-sm text-cyan-300">{installMessage}</p>{/if}{#if installError}<p class="mt-4 text-sm text-red-300">{installError}</p>{/if}
        <div class="mt-auto flex justify-between"><button class="btn-secondary px-4 py-2" disabled={installing} onclick={() => step = "export"}>{it ? "Indietro" : "Back"}</button><button class="rounded-lg bg-indigo-500 px-5 py-2 font-semibold text-white disabled:opacity-50" disabled={installing} onclick={finish}>{installing ? (it ? "Installazione…" : "Installing…") : (it ? "Completa setup" : "Finish setup")}</button></div>
      </div>
    {/if}
    </div>
  </div>
</div>
