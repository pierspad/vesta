<script lang="ts">
  import { locale, currentLanguage } from "./i18n";
  import { snackbar } from "./snackbarStore.svelte";
  import ProviderIcon from "./ProviderIcon.svelte";
  import { providers } from "./llmProviders";
  import {
    apiKeyEditorStore,
    llmProviderIds,
    whisperProviderIds,
    llmApiKeyUrls,
    llmDocUrls,
    whisperApiKeyUrls,
    whisperDocUrls,
  } from "./apiKeyEditorStore.svelte";

  interface Props {
    onSave: () => void;
  }

  let { onSave }: Props = $props();

  let t = $derived($locale);
  let store = apiKeyEditorStore;

  function copyToClipboard(text: string) {
    navigator.clipboard.writeText(text);
    snackbar.show(t("settings.keyCopied"), "success", 1300);
  }
</script>

{#if store.showAddKey}
  <div
    class="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-50 p-4"
    role="dialog"
    tabindex="-1"
    onmousedown={(e) => {
      if (e.target === e.currentTarget) store.close();
    }}
  >
    <div
      class="w-full max-w-4xl max-h-[92vh] overflow-hidden animate-fade-in shadow-2xl border border-white/20 bg-gray-900/98 backdrop-blur-xl rounded-xl flex flex-col"
      role="presentation"
      onmousedown={(e) => e.stopPropagation()}
    >
      <div class="p-6 border-b border-white/5 bg-white/5">
        <h3 class="text-xl font-bold text-white flex items-center gap-2">
          {store.editKeyId
            ? t("settings.modal.editApiKey")
            : t("settings.modal.addCustomApiKey")}
        </h3>
      </div>

      <div class="p-6 flex-1 overflow-hidden flex flex-col">
        <div class="space-y-5 overflow-y-auto custom-scrollbar pr-1">
        <div>
          <span
            class="block text-xs font-bold text-gray-500 uppercase tracking-wide mb-2"
            >{t("settings.modal.provider")}</span
          >
          <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2.5 mb-3">
            {#each (store.modalContext === "whisper" ? whisperProviderIds : llmProviderIds) as pid (pid)}
              {@const prov = providers[pid]}
              {@const isCustom = pid === "custom"}
              <button
                type="button"
                onclick={() => store.selectProvider(pid)}
                class="flex items-center gap-2.5 p-2.5 rounded-lg transition-all duration-200 border text-left
                  {store.newKeyType === pid
                  ? 'bg-indigo-500/20 border-indigo-500/50 text-white'
                  : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400'}"
              >
                <ProviderIcon provider={pid} />
                <div class="flex flex-col min-w-0">
                  <span class="text-sm font-bold truncate"
                    >{pid === "openai" ? "Open AI" : (isCustom ? t("provider.custom") : prov?.name || pid)}</span
                  >
                  <span class="text-[10px] opacity-70 leading-tight line-clamp-2"
                    >{isCustom ? t("provider.custom.desc") : (pid === "openai" ? (store.modalContext === "whisper" ? ($currentLanguage === "it" ? "API OpenAI speech-to-text (Whisper)" : "OpenAI speech-to-text API (Whisper)") : ($currentLanguage === "it" ? "Modelli OpenAI (GPT-4o, GPT-4...)" : "OpenAI models (GPT-4o, GPT-4...)")) : prov?.description || "")}</span
                  >
                </div>
              </button>
            {/each}
          </div>
        </div>

        <div class="space-y-4">
          <div>
            <label
              for="key-name"
              class="block text-xs font-bold text-gray-500 uppercase tracking-wide mb-1.5"
              >{t("settings.modal.configName")}</label
            >
            <input
              id="key-name"
              type="text"
              bind:value={store.newKeyName}
              placeholder={t("settings.modal.configNamePlaceholder")}
              class="w-full bg-black/40 border border-white/10 rounded-lg px-4 py-2.5 text-sm text-white focus:ring-2 focus:ring-indigo-500/50 focus:border-indigo-500/50 outline-none transition-all placeholder-gray-600"
            />
          </div>

          {#if store.newKeyType === "local" || store.newKeyType === "custom"}
            <div>
              <label
                for="api-url"
                class="block text-xs font-bold text-gray-500 uppercase tracking-wide mb-1.5"
              >
                {t("settings.modal.apiEndpoint")}
              </label>
              <input
                id="api-url"
                type="text"
                bind:value={store.newKeyUrl}
                placeholder={store.newKeyType === "local"
                  ? providers[store.newKeyType]?.defaultApiUrl || "https://..."
                  : "https://api.example.com/v1"}
                class="w-full bg-black/40 border border-white/10 rounded-lg px-4 py-2.5 text-sm text-white focus:ring-2 focus:ring-indigo-500/50 focus:border-indigo-500/50 outline-none transition-all placeholder-gray-600 font-mono"
              />
            </div>
          {/if}

          {#if store.newKeyType !== "local"}
            <div>
              <label
                for="api-key"
                class="block text-xs font-bold text-gray-500 uppercase tracking-wide mb-1.5"
                >{t("settings.modal.apiKey")}
                {#if store.newKeyType === "custom"}<span
                    class="text-gray-600 normal-case font-normal"
                    >({t("settings.modal.optional")})</span
                  >{/if}</label
              >
              <div class="relative">
                <input
                  id="api-key"
                  type={store.showNewKeyPassword ? "text" : "password"}
                  bind:value={store.newKeyValue}
                  placeholder={store.newKeyType === "custom"
                    ? t("settings.modal.notRequiredForLocal")
                    : providers[store.newKeyType]?.keyPlaceholder || "API key"}
                  class="w-full bg-black/40 border border-white/10 rounded-lg px-4 py-2.5 pr-20 text-sm text-white focus:ring-2 focus:ring-indigo-500/50 focus:border-indigo-500/50 outline-none transition-all placeholder-gray-600 font-mono"
                />
                <div
                  class="absolute right-2 top-1/2 -translate-y-1/2 flex gap-1"
                >
                  <button
                    type="button"
                    onclick={() => (store.showNewKeyPassword = !store.showNewKeyPassword)}
                    class="p-1.5 text-gray-500 hover:text-gray-300 transition-colors"
                    title={t("settings.toggleVisibility")}
                  >
                    {#if store.showNewKeyPassword}
                      <svg
                        class="w-4 h-4"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21"
                        />
                      </svg>
                    {:else}
                      <svg
                        class="w-4 h-4"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                        />
                        <path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"
                        />
                      </svg>
                    {/if}
                  </button>
                  <button
                    type="button"
                    onclick={() => copyToClipboard(store.newKeyValue)}
                    class="p-1.5 text-gray-500 hover:text-gray-300 transition-colors"
                    title="Copy"
                  >
                    <svg
                      class="w-4 h-4"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
                      />
                    </svg>
                  </button>
                </div>
              </div>

              {#if (store.modalContext === "whisper" ? whisperApiKeyUrls[store.newKeyType] : llmApiKeyUrls[store.newKeyType]) || (store.modalContext === "whisper" ? whisperDocUrls[store.newKeyType] : llmDocUrls[store.newKeyType])}
                <div class="mt-2.5 flex flex-wrap gap-x-4 gap-y-1 text-xs">
                  {#if store.modalContext === "whisper" ? whisperApiKeyUrls[store.newKeyType] : llmApiKeyUrls[store.newKeyType]}
                    <a
                      href={store.modalContext === "whisper" ? whisperApiKeyUrls[store.newKeyType] : llmApiKeyUrls[store.newKeyType]}
                      target="_blank"
                      rel="noopener noreferrer"
                      class="text-indigo-400 hover:text-indigo-300 hover:underline flex items-center gap-1.5 font-semibold transition-colors"
                    >
                      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m-2 4a2 2 0 012 2m-3-4a3 3 0 11-6 0 3 3 0 016 0zM8 21a4 4 0 014-4h4a4 4 0 014 4H8z" />
                      </svg>
                      {t("settings.modal.getApiKey")}
                    </a>
                  {/if}
                  {#if store.modalContext === "whisper" ? whisperDocUrls[store.newKeyType] : llmDocUrls[store.newKeyType]}
                    <a
                      href={store.modalContext === "whisper" ? whisperDocUrls[store.newKeyType] : llmDocUrls[store.newKeyType]}
                      target="_blank"
                      rel="noopener noreferrer"
                      class="text-cyan-400 hover:text-cyan-300 hover:underline flex items-center gap-1.5 font-semibold transition-colors"
                    >
                      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5s3.332.477 4.5 1.253v13C19.832 18.477 18.246 18 16.5 18s-3.332.477-4.5 1.253" />
                      </svg>
                      {t("settings.modal.documentation")}
                    </a>
                  {/if}
                </div>
              {/if}
            </div>
          {/if}

        </div>

        </div>

        <div class="flex gap-3 pt-4 mt-auto border-t border-white/5 shrink-0">
          <button
            onclick={() => store.close()}
            class="flex-1 py-2.5 rounded-lg border border-white/10 text-gray-400 hover:bg-white/5 hover:text-white transition-all text-sm font-medium"
          >
            {t("settings.modal.cancel")}
          </button>
          <button
            onclick={onSave}
            class="flex-1 py-2.5 rounded-lg bg-indigo-500 hover:bg-indigo-400 text-white shadow-lg shadow-indigo-500/20 transition-all text-sm font-bold"
          >
            {t("settings.modal.save")}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
