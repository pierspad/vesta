<script lang="ts">
  import type { Snippet } from "svelte";
  import ToggleRow from "./components/ToggleRow.svelte";
  import { uiMode } from "./uiModeStore.svelte";
  import { availableUILanguages, currentLanguage, locale, setLanguage } from "./i18n";
  import { cpuRamStore } from "./cpuRamStore.svelte";
  import { exportFormatStore } from "./exportFormatStore.svelte";
  import { ankiStore } from "./ankiStore.svelte";
  import { updateCheckerStore } from "./updateCheckerStore.svelte";

  let { defaultLanguagesCard }: { defaultLanguagesCard: Snippet } = $props();

  let t = $derived($locale);
</script>

<div class="glass-card p-6 mb-6 flex flex-col gap-5 shrink-0">
  <div class="ui-language-grid w-full">
    {#each availableUILanguages as lang}
      <button
        onclick={() => setLanguage(lang.code)}
        class="ui-language-button flex items-center gap-3 px-4 py-3 rounded-lg transition-all duration-200 border text-left min-w-0
          {$currentLanguage === lang.code
          ? 'bg-gradient-to-r from-indigo-500/20 to-purple-500/20 border-indigo-500/50 text-white shadow-sm'
          : 'bg-white/5 hover:bg-white/10 text-gray-400 hover:text-gray-200 border-transparent hover:border-white/10'}"
      >
        <span class="text-2xl leading-none shrink-0">{lang.flag}</span>
        <span class="min-w-0 flex flex-col leading-tight">
          <span class="block truncate text-sm font-bold text-white">{lang.name}</span>
          <span class="block truncate text-[11px] font-medium text-gray-400 opacity-80">{lang.nativeName}</span>
        </span>
      </button>
    {/each}
  </div>
</div>

{#if uiMode.easyMode}
  <div class="mb-6">
    {@render defaultLanguagesCard()}
  </div>
{/if}

<!-- Export Format Card (Expert Mode Only) -->
{#if uiMode.expertMode}
  <div class="glass-card p-6 flex flex-col justify-center mb-6">
    <div class="relative bg-black/20 border border-white/5 rounded-xl p-1 flex gap-2 w-full select-none">
      <!-- Sliding indicator background -->
      <div
        class="absolute top-1 bottom-1 left-1 rounded-lg shadow-md transition-all duration-200 ease-out z-0
          {exportFormatStore.exportFormat === 'apkg'
            ? 'bg-emerald-500/15 border border-emerald-500/30'
            : exportFormatStore.exportFormat === 'tsv'
              ? 'bg-sky-500/15 border border-sky-500/30'
              : 'bg-violet-500/15 border border-violet-500/30'}"
        style="width: calc(33.333% - 6px); transform: translateX(calc({exportFormatStore.activeIdx * 100}% + {exportFormatStore.activeIdx * 8}px)); left: 4px;"
      ></div>

      <!-- APKG Option Button -->
      <button
        type="button"
        onclick={() => exportFormatStore.setExportFormat('apkg')}
        class="flex-1 text-left p-4 rounded-lg transition-all duration-200 select-none relative z-10 flex items-center justify-between gap-4 cursor-pointer"
      >
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-2 flex-wrap">
            <span class="text-sm font-bold transition-colors duration-200 {exportFormatStore.exportFormat === 'apkg' ? 'text-white' : 'text-gray-400 hover:text-gray-200'}">
              {$currentLanguage === 'it' ? 'Esportazione APKG' : 'APKG export'}
            </span>
            <span class="text-[10px] px-1.5 py-0.5 rounded-full font-bold transition-all duration-200
              {exportFormatStore.exportFormat === 'apkg'
                ? 'bg-emerald-500/30 text-emerald-300 border border-emerald-500/40'
                : 'bg-gray-700/60 text-gray-400 border border-gray-700'}">
              {t("flashcards.exportAPKGBadge")}
            </span>
          </div>
          <p class="text-xs text-gray-400 mt-1 leading-relaxed">{t("flashcards.exportAPKGDesc")}</p>
        </div>

        <!-- Anki Package / File Zip SVG Icon on the right -->
        <svg class="w-8 h-8 transition-colors duration-200 shrink-0 {exportFormatStore.exportFormat === 'apkg' ? 'text-emerald-400' : 'text-gray-500'}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" />
        </svg>
      </button>

      <!-- TSV Option Button -->
      <button
        type="button"
        onclick={() => exportFormatStore.setExportFormat('tsv')}
        class="flex-1 text-left p-4 rounded-lg transition-all duration-200 select-none relative z-10 flex items-center justify-between gap-4 cursor-pointer"
      >
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-2 flex-wrap">
            <span class="text-sm font-bold transition-colors duration-200 {exportFormatStore.exportFormat === 'tsv' ? 'text-white' : 'text-gray-400 hover:text-gray-200'}">
              {$currentLanguage === 'it' ? 'Esportazione TSV' : 'TSV export'}
            </span>
            <span class="text-[10px] px-1.5 py-0.5 rounded-full font-bold transition-all duration-200
              {exportFormatStore.exportFormat === 'tsv'
                ? 'bg-sky-500/30 text-sky-300 border border-sky-500/40'
                : 'bg-gray-700/60 text-gray-400 border border-gray-700'}">
              {t("flashcards.exportTSVBadge")}
            </span>
          </div>
          <p class="text-xs text-gray-400 mt-1 leading-relaxed">{t("flashcards.exportTSVDesc")}</p>
        </div>

        <!-- Folder SVG Icon on the right -->
        <svg class="w-8 h-8 transition-colors duration-200 shrink-0 {exportFormatStore.exportFormat === 'tsv' ? 'text-sky-400' : 'text-gray-500'}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
        </svg>
      </button>

      <!-- Anki Connect Option Button (Permanently visible) -->
      <button
        type="button"
        onclick={() => exportFormatStore.setExportFormat('anki')}
        class="flex-1 text-left p-4 rounded-lg transition-all duration-200 select-none relative z-10 flex items-center justify-between gap-4 cursor-pointer"
      >
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-2 flex-wrap">
            <span class="text-sm font-bold transition-colors duration-200 {exportFormatStore.exportFormat === 'anki' ? 'text-white' : 'text-gray-400 hover:text-gray-200'}">
              {$currentLanguage === 'it' ? 'Esportazione Anki Connect' : 'Anki Connect export'}
            </span>
            <span class="text-[10px] px-1.5 py-0.5 rounded-full font-bold transition-all duration-200
              {exportFormatStore.exportFormat === 'anki'
                ? 'bg-violet-500/30 text-violet-300 border border-violet-500/40'
                : 'bg-gray-700/60 text-gray-400 border border-gray-700'}">
              {t("flashcards.exportAnkiConnectBadge")}
            </span>
            {#if ankiStore.status === "online"}
              <span class="text-[10px] px-1.5 py-0.5 rounded-full font-bold bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 flex items-center gap-1">
                🟢 {$currentLanguage === 'it' ? 'Anki rilevato' : 'Anki detected'}
              </span>
            {:else}
              <span class="text-[10px] px-1.5 py-0.5 rounded-full font-bold bg-amber-500/20 text-amber-300 border border-amber-500/30 flex items-center gap-1">
                🟠 {$currentLanguage === 'it' ? 'Anki non rilevato' : 'Anki offline'}
              </span>
            {/if}
          </div>
          <p class="text-xs text-gray-400 mt-1 leading-relaxed">{t("flashcards.exportAnkiConnectDesc")}</p>
        </div>

        <!-- Anki Connect/Flash SVG Icon on the right -->
        <svg class="w-8 h-8 transition-colors duration-200 shrink-0 {exportFormatStore.exportFormat === 'anki' ? 'text-violet-400' : 'text-gray-500'}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M13 10V3L4 14h7v7l9-11h-7z" />
        </svg>
      </button>
    </div>

    <!-- Secondary Fallback Hierarchy Selector when AnkiConnect is selected -->
    {#if exportFormatStore.exportFormat === 'anki'}
      <div class="mt-4 pt-4 border-t border-white/10 flex items-center justify-between gap-4 flex-wrap">
        <div class="flex items-center gap-2">
          <svg class="w-4 h-4 text-amber-400 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <span class="text-xs font-semibold text-gray-300">
            {$currentLanguage === 'it' ? 'Formato di ripiego se Anki è chiuso o disconnesso:' : 'Fallback format if Anki is closed or offline:'}
          </span>
        </div>
        <div class="flex items-center gap-2 bg-black/40 p-1 rounded-lg border border-white/10 shrink-0">
          <button
            type="button"
            onclick={() => exportFormatStore.setFallbackFormat('apkg')}
            class="px-3 py-1.5 rounded-md text-xs font-bold transition-all duration-150 cursor-pointer flex items-center gap-1.5
              {exportFormatStore.fallbackFormat === 'apkg'
                ? 'bg-emerald-500/30 text-emerald-300 border border-emerald-500/50 shadow-sm'
                : 'text-gray-400 hover:text-gray-200 border border-transparent'}"
          >
            <span>APKG (.apkg)</span>
            <span class="text-[9px] uppercase px-1 py-0.2 rounded bg-emerald-500/20 text-emerald-300">{$currentLanguage === 'it' ? 'Consigliato' : 'Recommended'}</span>
          </button>
          <button
            type="button"
            onclick={() => exportFormatStore.setFallbackFormat('tsv')}
            class="px-3 py-1.5 rounded-md text-xs font-bold transition-all duration-150 cursor-pointer flex items-center gap-1.5
              {exportFormatStore.fallbackFormat === 'tsv'
                ? 'bg-sky-500/30 text-sky-300 border border-sky-500/50 shadow-sm'
                : 'text-gray-400 hover:text-gray-200 border border-transparent'}"
          >
            <span>TSV (.tsv)</span>
          </button>
        </div>
      </div>
    {/if}
  </div>
{/if}

<div class="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6 items-stretch">
  <!-- CPU Cores Card -->
  <div class="glass-card p-6 flex flex-col justify-between h-full">
    {#if uiMode.expertMode}
      <div>
        <div class="flex items-center gap-3 mb-4">
          <div class="w-9 h-9 rounded-lg bg-orange-500/20 text-orange-300 flex items-center justify-center shrink-0">
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"
              />
            </svg>
          </div>
          <div>
            <h3 class="text-sm font-bold text-white">{t("settings.cpuCoresExpertTitle")}</h3>
          </div>
        </div>

        <!-- Granular CPU slider -->
        <div class="mb-5">
          <span class="block text-xs text-gray-400 mb-2 font-medium">
            {t("settings.cpuCoresSliderLabel")}
          </span>
          <div class="flex items-center gap-4">
            <div class="flex-1 min-w-0">
              <input
                type="range"
                min={cpuRamStore.minCpuCores}
                max={cpuRamStore.maxCpuCores}
                value={cpuRamStore.cpuCores}
                class="slider-resource w-full cursor-pointer"
                oninput={(e) => cpuRamStore.setCores(parseInt((e.target as HTMLInputElement).value))}
              />
              <!-- Tick marks — dynamic, proportionally distributed across core range -->
              <div class="relative mt-1.5" style="height: 22px;">
                {#each Array(cpuRamStore.maxCpuCores - cpuRamStore.minCpuCores + 1) as _, i}
                  {@const val = cpuRamStore.minCpuCores + i}
                  {@const pct = ((val - cpuRamStore.minCpuCores) / (cpuRamStore.maxCpuCores - cpuRamStore.minCpuCores)) * 100}
                  {#if val === cpuRamStore.minCpuCores || val === cpuRamStore.maxCpuCores || (val - cpuRamStore.minCpuCores) % cpuRamStore.cpuTickStep === 0}
                    <div class="absolute flex flex-col items-center gap-0.5" style="left: {pct}%; transform: translateX(-50%);">
                      <div class="w-px h-1.5 {val === cpuRamStore.cpuCores ? 'bg-white/60' : 'bg-white/20'}"></div>
                      <span class="text-[9px] {val === cpuRamStore.cpuCores ? 'text-white/70' : 'text-white/25'}">{val}</span>
                    </div>
                  {/if}
                {/each}
              </div>
            </div>
            <span class="text-white font-mono bg-white/10 px-2.5 py-1 rounded-lg text-xs shrink-0 self-start">
              {cpuRamStore.cpuCores} / {cpuRamStore.maxCpuCores}
            </span>
          </div>
        </div>

        <!-- RAM Memory slider -->
        <div class="pt-4 border-t border-white/5">
          <div class="flex items-center justify-between mb-2">
            <span class="block text-xs text-gray-400 font-medium">
              {t("settings.ramLimitLabel")}
            </span>
            <span class="text-white font-mono bg-white/10 px-2.5 py-1 rounded-lg text-xs shrink-0">
              {cpuRamStore.ramLimitMb === 0
                ? ($currentLanguage === 'it' ? 'Nessun limite' : 'No limit')
                : cpuRamStore.ramLimitMb >= 1024
                  ? `${(cpuRamStore.ramLimitMb / 1024).toFixed(cpuRamStore.ramLimitMb % 1024 === 0 ? 0 : 1)} GB`
                  : `${cpuRamStore.ramLimitMb} MB`}
            </span>
          </div>
          <input
            type="range"
            min={0}
            max={cpuRamStore.maxRamMb}
            step={64}
            value={cpuRamStore.ramLimitMb}
            class="slider-resource w-full cursor-pointer"
            oninput={(e) => cpuRamStore.setRamLimitMb(parseInt((e.target as HTMLInputElement).value))}
          />
          <!-- Tick marks: dynamic nice step — ~6 equidistant ticks based on system RAM -->
          <div class="relative mt-1.5" style="height: 22px;">
            {#each cpuRamStore.ramTicksMb as v}
              {@const pct = (v / cpuRamStore.maxRamMb) * 100}
              <div class="absolute flex flex-col items-center gap-0.5" style="left: {pct}%; transform: translateX(-50%);">
                <div class="w-px h-1.5 {cpuRamStore.ramLimitMb === v ? 'bg-white/60' : 'bg-white/20'}"></div>
                <span class="text-[9px] {cpuRamStore.ramLimitMb === v ? 'text-white/70' : 'text-white/25'} whitespace-nowrap">
                  {v === 0
                    ? ($currentLanguage === 'it' ? 'Nessuno' : 'None')
                    : v >= 1024 ? `${v / 1024}G` : `${v}M`}
                </span>
              </div>
            {/each}
          </div>
        </div>
      </div>
    {:else}
      <div>
        <div class="flex items-center gap-3 mb-3">
          <div class="w-9 h-9 rounded-lg bg-orange-500/20 text-orange-300 flex items-center justify-center shrink-0">
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"
              />
            </svg>
          </div>
          <div>
            <h3 class="text-sm font-bold text-white">{t("settings.cpuCoresEasyTitle")}</h3>
          </div>
        </div>
        <div class="grid grid-cols-2 sm:grid-cols-4 lg:grid-cols-2 xl:grid-cols-4 gap-2.5">
          <button
            onclick={() => cpuRamStore.setCpuPreset("eco")}
            class="p-3 rounded-xl text-center transition-all duration-200 border text-xs cursor-pointer {cpuRamStore.activeCpuPreset === 'eco'
              ? 'bg-orange-500/20 border-orange-500/50 text-white'
              : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
          >
            <span class="block mb-1 text-white">
              <svg class="w-4 h-4 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M5 14c0-5.523 4.477-10 10-10h4v4c0 5.523-4.477 10-10 10H5v-4z" />
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M7 17c2.5-2.5 5.5-4.5 9-6" />
              </svg>
            </span>
            <span class="font-semibold block">{t("flashcards.cpuEco")}</span>
          </button>
          <button
            onclick={() => cpuRamStore.setCpuPreset("balanced")}
            class="p-3 rounded-xl text-center transition-all duration-200 border text-xs cursor-pointer {cpuRamStore.activeCpuPreset === 'balanced'
              ? 'bg-orange-500/20 border-orange-500/50 text-white'
              : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
          >
            <span class="block mb-1 text-white">
              <svg class="w-4 h-4 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M12 4v16" />
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M6 7h12" />
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M8 7l-3 5h6L8 7z" />
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M16 7l-3 5h6l-3-5z" />
              </svg>
            </span>
            <span class="font-semibold block">{t("flashcards.cpuBalanced")}</span>
          </button>
          <button
            onclick={() => cpuRamStore.setCpuPreset("performance")}
            class="p-3 rounded-xl text-center transition-all duration-200 border text-xs cursor-pointer {cpuRamStore.activeCpuPreset === 'performance'
              ? 'bg-orange-500/20 border-orange-500/50 text-white'
              : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
          >
            <span class="block mb-1 text-white">
              <svg class="w-4 h-4 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M5 16l5-5 3 3 6-7" />
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M14 7h5v5" />
              </svg>
            </span>
            <span class="font-semibold block">{t("flashcards.cpuPerformance")}</span>
          </button>
          <button
            onclick={() => cpuRamStore.setCpuPreset("full")}
            class="p-3 rounded-xl text-center transition-all duration-200 border text-xs cursor-pointer {cpuRamStore.activeCpuPreset === 'full'
              ? 'bg-orange-500/20 border-orange-500/50 text-white'
              : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
          >
            <span class="block mb-1 text-white">
              <svg class="w-4 h-4 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M11 3L6 13h5l-1 8 8-12h-5l2-6h-4z" />
              </svg>
            </span>
            <span class="font-semibold block">{t("flashcards.cpuFullPower")}</span>
          </button>
        </div>
      </div>
    {/if}
  </div>

  <!-- Aggiornamenti Card -->
  <div class="glass-card p-6 flex flex-col justify-between h-full">
    <div class="flex flex-col gap-6">
      <ToggleRow
        label={$currentLanguage === 'it' ? 'Verifica aggiornamenti all\'avvio' : 'Check for updates on startup'}
        bind:checked={updateCheckerStore.automaticUpdateChecks}
        onchange={() => updateCheckerStore.onAutomaticUpdateChecksChange()}
        accent="indigo"
        iconPath="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M9 19l3 3m0 0l3-3m-3 3V10"
      />

      <!-- Bottom Row: Dynamic Status / Manual Check Area -->
      <div class="pt-4 border-t border-white/5 flex items-center justify-between min-h-[44px]">
        <span class="text-xs text-gray-400">
          {$currentLanguage === 'it' ? 'Stato degli aggiornamenti' : 'Update status'}
        </span>
        <div>
          {#if updateCheckerStore.updateStatus === "available"}
            <a
              href={updateCheckerStore.releaseUrl}
              target="_blank"
              class="inline-flex items-center gap-2 rounded-xl border border-amber-500/40 bg-amber-500/15 px-4 py-2 text-left transition-all duration-200 hover:border-amber-500/60 hover:bg-amber-500/25 active:scale-[0.98] cursor-pointer shadow-md shadow-amber-900/20"
            >
              <span class="w-2 h-2 rounded-full bg-amber-400 animate-pulse shrink-0"></span>
              <span class="text-[11px] font-bold text-amber-200">
                {$currentLanguage === 'it' ? `Scarica v${updateCheckerStore.latestVersion}` : `Download v${updateCheckerStore.latestVersion}`}
              </span>
              <svg class="w-4 h-4 text-amber-400 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
              </svg>
            </a>
          {:else if updateCheckerStore.updateStatus === "checking"}
            <div class="flex items-center gap-2 text-xs text-gray-400 font-semibold">
              <svg class="animate-spin h-3.5 w-3.5 text-indigo-400 shrink-0" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
              </svg>
              <span>{$currentLanguage === 'it' ? 'Verifica...' : 'Checking...'}</span>
            </div>
          {:else if updateCheckerStore.updateStatus === "current"}
            <span class="text-xs font-semibold text-emerald-400 flex items-center gap-1.5 animate-fade-in bg-emerald-500/10 border border-emerald-500/25 px-2.5 py-1.5 rounded-lg select-none">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 13l4 4L19 7" />
              </svg>
              {$currentLanguage === 'it' ? 'Aggiornato' : 'Up to date'}
            </span>
          {:else if updateCheckerStore.automaticUpdateChecks}
            <span class="text-xs text-gray-500 italic">
              {$currentLanguage === 'it' ? 'Attivo all\'avvio' : 'Active on startup'}
            </span>
          {:else}
            <button
              type="button"
              onclick={() => updateCheckerStore.checkForUpdates("manual")}
              class="px-4 py-2 bg-white/5 border border-white/10 hover:bg-white/10 hover:border-white/20 text-white rounded-xl text-xs font-semibold transition-all duration-200 active:scale-[0.98] cursor-pointer"
            >
              {$currentLanguage === 'it' ? 'Verifica ora' : 'Check now'}
            </button>
          {/if}
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .ui-language-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(11.5rem, 1fr));
    gap: 0.75rem;
    width: 100%;
  }

  @media (min-width: 1280px) {
    .ui-language-grid {
      grid-template-columns: repeat(5, minmax(0, 1fr));
    }
  }

  .ui-language-button {
    min-width: 0;
  }
</style>
