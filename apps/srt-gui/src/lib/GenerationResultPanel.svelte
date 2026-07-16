<script lang="ts">
  import { locale } from "./i18n";
  import { generationStore } from "./generationStore.svelte";

  /** Detailed progress/result/error display shown in the main panel grid.
   * Distinct from the compact GenerationStatusDisplay.svelte in the footer
   * bar (segmented progress bar + full paths vs. truncated filename +
   * copy button there) -- reads generationStore directly, no parent-domain
   * state at all, so it takes zero props. */
  let t = $derived($locale);
</script>

<div class="space-y-3">
  {#if generationStore.isProcessing || generationStore.progress > 0}
    <div
      class="glass-card p-5 {generationStore.isProcessing ? 'animate-pulse-glow' : ''}"
    >
      <div class="flex items-center gap-4">
        <div class="flex-1">
          <div class="progress-modern h-2">
            <div
              class="progress-modern-bar bg-gradient-to-r from-emerald-500 to-teal-500"
              style="width: {generationStore.progress}%"
            ></div>
          </div>
        </div>
        <span class="text-lg font-bold text-emerald-400">{generationStore.progress}%</span
        >
      </div>
      {#if generationStore.progressMessage}
        <p class="text-gray-400 text-xs mt-2">{generationStore.progressMessage}</p>
      {/if}
      {#if generationStore.progressStage}
        <div class="flex gap-1.5 mt-2">
          {#each Array(10) as _, i}
            {@const threshold = (i + 1) * 10}
            <div
              class="h-1 flex-1 rounded-full transition-colors duration-300 {generationStore.progress >=
              threshold
                ? 'bg-emerald-700'
                : generationStore.progress >= threshold - 10
                  ? 'bg-emerald-400'
                  : 'bg-gray-700'}"
            ></div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
  {#if generationStore.result}
    <div
      class="glass-card p-5 border-l-4 {generationStore.result.success
        ? 'border-green-500 bg-green-500/5'
        : 'border-red-500 bg-red-500/5'}"
    >
      {#if generationStore.result.success}
        <div class="space-y-2">
          <div class="flex items-center gap-3">
            <svg
              class="w-5 h-5 text-green-400"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M5 13l4 4L19 7"
              />
            </svg>
            <p class="text-green-400 font-medium">
              {generationStore.result.cardsGenerated}
              {t("flashcards.cardsGenerated")}
            </p>
          </div>
          <div class="flex gap-4 text-xs text-gray-400">
            {#if generationStore.result.audioClips > 0}
              <span>🔊 {generationStore.result.audioClips} {t("flashcards.countAudio")}</span>
            {/if}
            {#if generationStore.result.snapshots > 0}
              <span>📸 {generationStore.result.snapshots} {t("flashcards.countSnapshots")}</span>
            {/if}
            {#if generationStore.result.videoClips > 0}
              <span>🎬 {generationStore.result.videoClips} {t("flashcards.countVideo")}</span>
            {/if}
          </div>
          {#if generationStore.result.tsvPath}
            <p
              class="text-xs text-gray-500 break-words"
              title={generationStore.result.tsvPath}
            >
              📄 {generationStore.result.tsvPath}
            </p>
          {/if}
          {#if generationStore.result.apkgPath}
            <p
              class="text-xs text-gray-500 break-words"
              title={generationStore.result.apkgPath}
            >
              📦 {generationStore.result.apkgPath}
            </p>

          {/if}
        </div>
      {:else}
        <div class="flex items-center gap-3">
          <svg
            class="w-5 h-5 text-red-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
            />
          </svg>
          <p class="text-red-300">
            {generationStore.result.message
              ? (generationStore.result.message.includes("No active")
                ? t("flashcards.noActiveLines")
                : generationStore.result.message)
              : t("flashcards.errorGenerating")}
          </p>
        </div>
      {/if}
    </div>
  {/if}
  {#if generationStore.error}
    <div class="glass-card p-5 border border-red-500/30 bg-red-500/10">
      <div class="flex items-center gap-3">
        <svg
          class="w-5 h-5 text-red-400"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
          />
        </svg>
        <p class="text-red-300 flex-1 text-sm break-words">{generationStore.error}</p>
        <button
          onclick={() => (generationStore.error = null)}
          class="text-red-400 hover:text-red-300">✕</button
        >
      </div>
    </div>
  {/if}
</div>
