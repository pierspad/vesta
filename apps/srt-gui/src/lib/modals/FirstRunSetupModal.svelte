<script lang="ts">
  import { availableUILanguages, currentLanguage } from "$lib/i18n";
  import { languages, getLanguageSearchTerms } from "$lib/config/languages";
  import SearchableSelect from "$lib/components/SearchableSelect.svelte";
  import { loadMediaSettings, saveMediaSettings } from "$lib/utils/mediaSettings";
  import { transcribeDownloadModel, transcribeDownloadVad } from "$lib/services/transcribe";
  import { fontStore } from "$lib/stores/fontStore.svelte";
  import type { ExportFormat, ExportFallbackFormat } from "$lib/stores/exportFormatStore.svelte";
  import * as vestaConfig from "$lib/config/vestaConfig";

  let { onComplete }: { onComplete: (wantsTranscription: boolean) => void } = $props();
  let mode = $state<"quick" | "custom" | null>(null);
  let step = $state<"languages" | "export" | "transcription">("languages");
  let uiLanguage = $state($currentLanguage);
  let nativeLanguage = $state(languages.some((l) => l.code === $currentLanguage) ? $currentLanguage : "en");
  let studyLanguage = $state("en");
  let exportFormat = $state<ExportFormat>("apkg");
  let fallbackFormat = $state<ExportFallbackFormat>("apkg");
  let compactAudio = $state(false);
  let wantsTranscription = $state(false);
  let qualityTranscription = $state(false);
  let useVad = $state(true);
  let whisperModel = $state("small");
  let installing = $state(false);
  let installMessage = $state("");
  let installError = $state("");
  let it = $derived(uiLanguage === "it");

  const uiLanguageOptions = availableUILanguages.map((l) => ({ value: l.code, label: `${l.nativeName} — ${l.name}`, icon: l.flag, searchTerms: `${l.code} ${l.name} ${l.nativeName}` }));
  const languageOptions = languages.map((l) => ({ value: l.code, label: l.name === l.nameEn ? l.name : `${l.name} — ${l.nameEn}`, icon: l.flag, searchTerms: getLanguageSearchTerms(l.code) }));
  const modelIcon = (path: string) => `<svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="${path}" /></svg>`;
  const whisperOptions = [
    { value: "tiny", label: "Tiny · ~75 MB", icon: modelIcon("M13 3L4 14h7l-1 7 9-12h-7l1-6z"), searchTerms: "fast smallest" },
    { value: "base", label: "Base · ~142 MB", icon: modelIcon("M5 15v2m4-6v6m4-10v10m4-7v7m4-4v4"), searchTerms: "balanced basic" },
    { value: "small", label: "Small · ~466 MB · Recommended", icon: modelIcon("M4 12h3l2-5 4 10 2-5h5"), searchTerms: "recommended quality" },
    { value: "medium", label: "Medium · ~1.5 GB", icon: modelIcon("M4 6h16M7 12h10M10 18h4"), searchTerms: "high quality" },
    { value: "large", label: "Large · ~3.1 GB", icon: modelIcon("M12 6v6l4 2m5-2a9 9 0 11-18 0 9 9 0 0118 0z"), searchTerms: "best accuracy slow" },
  ];

  const customStepNumber = $derived(step === "languages" ? 1 : step === "export" ? 2 : 3);

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
        await transcribeDownloadModel(whisperModel);
        if (useVad) {
          installMessage = it ? "Download Silero VAD 6.2.0…" : "Downloading Silero VAD 6.2.0…";
          await transcribeDownloadVad("v6.2.0");
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
      vestaConfig.setItem("vesta-default-target-language", nativeLanguage);
      vestaConfig.setItem("vesta-default-flashcards-language", studyLanguage);
      vestaConfig.setItem("vesta-default-transcribe-language", studyLanguage);
      vestaConfig.setItem("vesta-export-format", mode === "quick" ? "apkg" : exportFormat);
      vestaConfig.setItem("vesta-export-fallback", fallbackFormat);
      vestaConfig.setItem("vesta-expert-mode", String(mode === "custom"));
      vestaConfig.setItem("vesta-transcribe-quality", String(mode === "custom" && qualityTranscription));
      vestaConfig.setItem("vesta-transcribe-vad", String(wantsTranscription && useVad));
      vestaConfig.setItem("srt-default-whisper-model", whisperModel);
      vestaConfig.setItem("vesta-transcribe-vad-selection", JSON.stringify({ modelId: "v6.2.0", customPath: null }));
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
  <div class="w-full max-w-5xl overflow-visible rounded-2xl border border-indigo-400/30 bg-gray-900 p-7 shadow-2xl">
    <div class="flex items-start justify-between gap-5">
      <div><p class="text-xs font-bold uppercase tracking-[0.2em] text-indigo-300">Vesta</p><h1 class="mt-2 text-2xl font-bold text-white">{it ? "Configura la tua esperienza" : "Set up your experience"}</h1><p class="mt-2 text-sm text-gray-400">{it ? "Tre scelte essenziali, poi Vesta prepara il resto." : "Three essential choices, then Vesta prepares the rest."}</p></div>
      {#if mode}<span class="rounded-full border border-indigo-400/25 bg-indigo-500/10 px-3 py-1 text-xs text-indigo-200">{mode === "quick" ? (it ? "Rapido" : "Quick") : "Expert"} · {mode === "quick" ? 1 : customStepNumber}/{mode === "custom" ? 3 : 1}</span>{/if}
    </div>

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
      <div class="mt-7 grid grid-cols-1 gap-4 lg:grid-cols-3">
        <div class="rounded-xl border border-white/10 bg-white/[0.03] p-4"><div class="mb-3 flex items-center gap-2"><span class="flex h-8 w-8 items-center justify-center rounded-lg bg-sky-500/15 text-sky-300">⌘</span><div><p class="text-sm font-semibold text-white">{it ? "Interfaccia" : "Interface"}</p><p class="text-[11px] text-gray-500">{it ? "Lingua dei menu" : "Language for menus"}</p></div></div><SearchableSelect options={uiLanguageOptions} value={uiLanguage} onchange={(v) => uiLanguage = v} /></div>
        <div class="rounded-xl border border-white/10 bg-white/[0.03] p-4"><div class="mb-3 flex items-center gap-2"><span class="flex h-8 w-8 items-center justify-center rounded-lg bg-emerald-500/15 text-emerald-300">A</span><div><p class="text-sm font-semibold text-white">{it ? "La tua lingua madre" : "Your native language"}</p><p class="text-[11px] text-gray-500">{it ? "Traduzioni e significati" : "Translations and meanings"}</p></div></div><SearchableSelect options={languageOptions} value={nativeLanguage} onchange={(v) => nativeLanguage = v} /></div>
        <div class="rounded-xl border border-white/10 bg-white/[0.03] p-4"><div class="mb-3 flex items-center gap-2"><span class="flex h-8 w-8 items-center justify-center rounded-lg bg-violet-500/15 text-violet-300">文</span><div><p class="text-sm font-semibold text-white">{it ? "Lingua studiata" : "Study language"}</p><p class="text-[11px] text-gray-500">{it ? "Audio e sottotitoli originali" : "Audio and original subtitles"}</p></div></div><SearchableSelect options={languageOptions} value={studyLanguage} onchange={(v) => studyLanguage = v} /></div>
      </div>
      <div class="mt-7 flex justify-between"><button class="btn-secondary px-4 py-2" onclick={() => mode = null}>{it ? "Indietro" : "Back"}</button>{#if mode === "custom"}<button class="rounded-lg bg-indigo-500 px-5 py-2 font-semibold text-white" onclick={() => step = "export"}>{it ? "Continua" : "Continue"}</button>{:else}<button class="rounded-lg bg-indigo-500 px-5 py-2 font-semibold text-white" onclick={finish}>{it ? "Completa setup" : "Finish setup"}</button>{/if}</div>
    {:else if step === "export"}
      <div class="mt-7 rounded-xl border border-white/10 bg-white/[0.03] p-4">
        <p class="mb-3 text-sm font-semibold text-white">{it ? "Come vuoi esportare le flashcard?" : "How do you want to export flashcards?"}</p>
        <div class="grid grid-cols-3 gap-1 rounded-lg bg-black/20 p-1" role="switch" aria-checked={exportFormat !== "apkg"} tabindex="0" onkeydown={(event) => (event.key === "Enter" || event.key === " ") && cycleExportFormat()}>
          <button class="flex items-center justify-center gap-2 rounded-md px-3 py-3 text-sm font-semibold transition-colors {exportFormat === 'apkg' ? 'bg-emerald-500/20 text-emerald-200 ring-1 ring-emerald-400/40' : 'text-gray-400'}" onclick={cycleExportFormat}><svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" /></svg> APKG</button>
          <button class="flex items-center justify-center gap-2 rounded-md px-3 py-3 text-sm font-semibold transition-colors {exportFormat === 'tsv' ? 'bg-sky-500/20 text-sky-200 ring-1 ring-sky-400/40' : 'text-gray-400'}" onclick={cycleExportFormat}><svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 5h16v14H4zM8 9h8M8 13h8M8 17h5" /></svg> TSV</button>
          <button class="flex items-center justify-center gap-2 rounded-md px-3 py-3 text-sm font-semibold transition-colors {exportFormat === 'anki' ? 'bg-violet-500/20 text-violet-200 ring-1 ring-violet-400/40' : 'text-gray-400'}" onclick={cycleExportFormat}><svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M13 3 4 14h7v7l9-11h-7V3z" /></svg> AnkiConnect</button>
        </div>
        <p class="mt-3 text-xs text-gray-400">{exportFormat === "apkg" ? (it ? "Un pacchetto completo, consigliato per quasi tutti." : "A complete package, recommended for most people.") : exportFormat === "tsv" ? (it ? "File tabellare e cartella media per importazione manuale." : "Tabular file and media folder for manual import.") : (it ? "Invio diretto ad Anki; usa un formato di riserva se Anki è chiuso." : "Send directly to Anki; use a fallback format when Anki is closed.")}</p>
        {#if exportFormat === "anki"}
          <div class="mt-4 flex items-center justify-between gap-3 border-t border-white/10 pt-4"><span class="text-xs text-gray-300">{it ? "Formato di riserva" : "Fallback format"}</span><div class="grid grid-cols-2 rounded-lg bg-black/30 p-1" role="switch" aria-checked={fallbackFormat === "tsv"}><button class="rounded-md px-4 py-2 text-xs font-semibold {fallbackFormat === 'apkg' ? 'bg-emerald-500/20 text-emerald-200' : 'text-gray-400'}" onclick={toggleFallbackFormat}>APKG</button><button class="rounded-md px-4 py-2 text-xs font-semibold {fallbackFormat === 'tsv' ? 'bg-sky-500/20 text-sky-200' : 'text-gray-400'}" onclick={toggleFallbackFormat}>TSV</button></div></div>
        {/if}
      </div>
      <button class="mt-4 w-full rounded-xl border p-4 text-left {compactAudio ? 'border-cyan-400/50 bg-cyan-500/10' : 'border-white/10 bg-white/[0.03]'}" onclick={() => compactAudio = !compactAudio}><div class="flex items-center justify-between gap-4"><div><p class="font-semibold text-white">♫ {it ? "Audio compatto" : "Compact audio"}</p><p class="mt-1 text-xs text-gray-400">{it ? "Opus dimezza circa lo spazio, ma non è compatibile con AnkiMobile su iOS." : "Opus roughly halves storage, but is not compatible with AnkiMobile on iOS."}</p></div><span class="rounded-md bg-black/25 px-3 py-1 text-xs font-semibold {compactAudio ? 'text-cyan-200' : 'text-gray-400'}">{compactAudio ? "OPUS" : "MP3"}</span></div></button>
      <div class="mt-7 flex justify-between"><button class="btn-secondary px-4 py-2" onclick={() => step = "languages"}>{it ? "Indietro" : "Back"}</button><button class="rounded-lg bg-indigo-500 px-5 py-2 font-semibold text-white" onclick={() => step = "transcription"}>{it ? "Continua" : "Continue"}</button></div>
    {:else}
      <div class="mt-4 rounded-xl border border-white/10 bg-white/[0.03] p-5"><div class="flex items-center justify-between gap-4"><div><p class="font-semibold text-white">{it ? "Trascrizione locale" : "Local transcription"}</p><p class="mt-1 text-xs text-gray-400">{it ? "Scarica ora i modelli necessari." : "Download the required models now."}</p></div><button aria-label={it ? "Attiva trascrizione locale" : "Enable local transcription"} class="relative h-6 w-12 rounded-full {wantsTranscription ? 'bg-indigo-500' : 'bg-gray-600'}" onclick={() => wantsTranscription = !wantsTranscription}><span class="absolute top-0.5 h-5 w-5 rounded-full bg-white transition-all {wantsTranscription ? 'left-6' : 'left-0.5'}"></span></button></div>
        <div class="mt-5 grid gap-4 md:grid-cols-3 {wantsTranscription ? '' : 'opacity-45'}"><div><p class="mb-2 text-xs text-gray-400">Whisper model</p><SearchableSelect disabled={!wantsTranscription} options={whisperOptions} value={whisperModel} onchange={(v) => whisperModel = v} /></div><button disabled={!wantsTranscription} class="rounded-lg border p-3 text-left disabled:cursor-not-allowed {useVad ? 'border-emerald-400/40 bg-emerald-500/10' : 'border-white/10'}" onclick={() => useVad = !useVad}><p class="text-sm font-semibold text-white">◉ Silero VAD 6.2.0</p><p class="text-[11px] text-gray-400">{useVad ? (it ? "Verrà scaricato" : "Will be downloaded") : "OFF"}</p></button><button disabled={!wantsTranscription} class="rounded-lg border p-3 text-left disabled:cursor-not-allowed {qualityTranscription ? 'border-violet-400/40 bg-violet-500/10' : 'border-white/10'}" onclick={() => qualityTranscription = !qualityTranscription}><p class="text-sm font-semibold text-white">✦ {it ? "Qualità alta" : "High quality"}</p><p class="text-[11px] text-gray-400">Beam search · {qualityTranscription ? "ON" : "OFF"}</p></button></div>
      </div>
      {#if installMessage}<p class="mt-4 text-sm text-cyan-300">{installMessage}</p>{/if}{#if installError}<p class="mt-4 text-sm text-red-300">{installError}</p>{/if}
      <div class="mt-7 flex justify-between"><button class="btn-secondary px-4 py-2" disabled={installing} onclick={() => step = "export"}>{it ? "Indietro" : "Back"}</button><button class="rounded-lg bg-indigo-500 px-5 py-2 font-semibold text-white disabled:opacity-50" disabled={installing} onclick={finish}>{installing ? (it ? "Installazione…" : "Installing…") : (it ? "Completa setup" : "Finish setup")}</button></div>
    {/if}
  </div>
</div>
