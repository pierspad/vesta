<script lang="ts">
  import { writeClipboard } from "$lib/utils/clipboard";
  import { locale } from "$lib/i18n";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  import SearchableSelect from "$lib/components/SearchableSelect.svelte";
  import { hasProviderNameConflict, loadAndValidateApiKeys } from "$lib/config/apiKeys";
  import ProviderIcon from "$lib/components/ProviderIcon.svelte";
  import { providers } from "$lib/config/llmProviders";
  import { transcribeProviders } from "$lib/config/transcribeProviders";
  import {
    apiKeyEditorStore,
    llmProviderIds,
    whisperProviderIds,
    llmApiKeyUrls,
    llmDocUrls,
    whisperApiKeyUrls,
    whisperDocUrls,
  } from "$lib/stores/apiKeyEditorStore.svelte";

  interface Props {
    onSave: () => void;
    onDelete?: (id: string) => void;
  }

  let { onSave, onDelete }: Props = $props();

  const usageIcon = (path: string) => `<svg class="w-5 h-5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="${path}" /></svg>`;
  const usageOptions = [
    { value: "llm", label: "LLM", icon: usageIcon("M4 4h16v12H9l-5 4V4zm4 4h8m-8 4h5") },
    { value: "stt", label: "STT", icon: usageIcon("M9 5a3 3 0 016 0v6a3 3 0 01-6 0V5zm-3 6a6 6 0 0012 0m-6 6v4m-3 0h6") },
    { value: "both", label: "LLM + STT", icon: usageIcon("M3 4h12v9H8l-5 4V4zm4 4h4m7 3a2 2 0 014 0v4a2 2 0 01-4 0v-4zm-2 4a4 4 0 008 0m-4 4v3m-2 0h4") },
  ];

  let t = $derived($locale);
  let store = apiKeyEditorStore;
  let canDelete = $derived(!!onDelete && !!store.editKeyId && !["local", "local_stt_server"].includes(store.editKeyId));
  let duplicateName = $derived(hasProviderNameConflict(loadAndValidateApiKeys(), store.newKeyName, store.editKeyId));

  function copyToClipboard(text: string) {
    void writeClipboard(text, t("settings.keyCopied"));
  }
</script>

{#snippet icon(path: string)}
  <svg aria-hidden="true" class="h-4 w-4 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d={path} /></svg>
{/snippet}

{#if store.showAddKey}
  <div
    class="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-50 p-4"
    role="dialog"
    aria-modal="true"
    aria-labelledby="provider-dialog-title"
    tabindex="-1"
    onmousedown={(e) => {
      if (e.target === e.currentTarget) store.close();
    }}
  >
    <div
      class="provider-dialog w-full max-h-[92dvh] overflow-hidden animate-fade-in shadow-2xl border border-white/20 bg-gray-900/98 backdrop-blur-xl rounded-xl flex flex-col"
      class:editing={!!store.editKeyId}
      role="presentation"
      onmousedown={(e) => e.stopPropagation()}
    >
      <div class="p-4 sm:p-5 border-b border-white/5 bg-white/5">
        <h3 id="provider-dialog-title" class="text-xl font-bold text-white flex items-center gap-2">
          {@render icon("M12 3H5a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7M16 3l5 5-9 9-5 1 1-5z")}
          {store.editKeyId
            ? t("settings.modal.editApiKey")
            : t("settings.modal.addCustomApiKey")}
        </h3>
      </div>

      <div class="p-4 sm:p-5 flex-1 min-h-0 overflow-hidden flex flex-col">
        <div class="space-y-4 overflow-y-auto custom-scrollbar pr-1 pb-1 min-h-0">
        <div>
          <span
            class="flex items-center gap-2 text-xs font-semibold text-gray-400 mb-2"
            >{@render icon("M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h6v6h-6z")} {t("settings.modal.provider")}</span
          >
          {#if store.editKeyId}
            <div class="flex items-center gap-3 rounded-xl border border-white/10 bg-white/5 p-4" data-testid="editing-provider">
              <ProviderIcon provider={store.newKeyType} />
              <span class="text-sm font-semibold text-white">{store.newKeyType === "custom" ? t("provider.custom") : (store.modalContext === "whisper" ? transcribeProviders[store.newKeyType]?.name : providers[store.newKeyType]?.name) || store.newKeyType}</span>
            </div>
          {:else}
          <div class="grid grid-cols-[repeat(auto-fit,minmax(min(100%,12rem),1fr))] gap-2.5 mb-3">
            {#each (store.modalContext === "whisper" ? whisperProviderIds : llmProviderIds) as pid (pid)}
              {@const prov = providers[pid]}
              {@const speechProv = transcribeProviders[pid]}
              {@const isCustom = pid === "custom"}
              <button
                type="button"
                disabled={(store.editKeyId === "local" && pid !== "local") || (store.editKeyId === "local_stt_server" && pid !== "custom")}
                onclick={() => store.selectProvider(pid)}
                class="flex items-center gap-2.5 p-2.5 rounded-lg transition-all duration-200 border text-left disabled:opacity-35 disabled:cursor-not-allowed
                  {store.newKeyType === pid
                  ? 'bg-indigo-500/20 border-indigo-500/50 text-white'
                  : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400'}"
              >
                <ProviderIcon provider={pid} />
                <div class="flex flex-col min-w-0">
                  <span class="text-sm font-bold truncate"
                    >{isCustom ? t("provider.custom") : (store.modalContext === "whisper" ? speechProv?.name : prov?.name) || pid}</span
                  >
                  <span class="text-[10px] opacity-70 leading-tight line-clamp-2"
                    >{isCustom ? t("provider.custom.desc") : t(store.modalContext === "whisper" && pid === "openai" ? "provider.openai.whisperDesc" : `provider.${pid}.desc`)}</span
                  >
                </div>
              </button>
            {/each}
          </div>
          {/if}
        </div>

        <div class="space-y-4">
          <div>
            <label
              for="key-name"
              class="flex items-center gap-2 text-xs font-semibold text-gray-400 mb-1.5"
              >{@render icon("M4 5h16M12 5v14M8 19h8")} {t("settings.modal.configName")}</label
            >
            <input
              id="key-name"
              type="text"
              bind:value={store.newKeyName}
              placeholder={t("settings.modal.configNamePlaceholder")}
              class="w-full bg-black/40 border border-white/10 rounded-lg px-4 py-2.5 text-sm text-white focus:ring-2 focus:ring-indigo-500/50 focus:border-indigo-500/50 outline-none transition-all placeholder-gray-600"
            />
          </div>

          {#if duplicateName}
            <p role="alert" class="text-xs text-amber-300">{t("settings.errorDuplicateCustomName")}</p>
          {/if}
          {#if store.newKeyType === "custom"}
            <div>
              <span class="flex items-center gap-2 text-xs font-semibold text-gray-400 mb-1.5">{t("settings.endpointCapabilities")}</span>
              <SearchableSelect
                options={usageOptions}
                value={store.newKeyUsage}
                onchange={(value) => { if (value === "llm" || value === "stt" || value === "both") store.newKeyUsage = value; }}
                placement="down"
                portal={true}
                placeholder={t("settings.endpointCapabilities")}
                ariaLabel={t("settings.endpointCapabilities")}
                noResultsText={t("common.noResults")}
              />
            </div>
          {/if}
          {#if store.newKeyType === "local" || store.newKeyType === "custom"}
            <div>
              <label
                for="api-url"
                class="flex items-center gap-2 text-xs font-semibold text-gray-400 mb-1.5"
              >
                {@render icon("M10 13a5 5 0 007 0l3-3a5 5 0 00-7-7l-2 2M14 11a5 5 0 00-7 0l-3 3a5 5 0 007 7l2-2")} {t("settings.modal.apiEndpoint")}
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

          {#if true}
            <div>
              <label
                for="api-key"
                class="flex items-center gap-2 text-xs font-semibold text-gray-400 mb-1.5"
                >{@render icon("M15 7a4 4 0 114 4h-2v3h-3v3h-3v4H3v-4l8-8a4 4 0 014-2")} {t("settings.modal.apiKey")}
                {#if store.newKeyType === "custom" || store.newKeyType === "local"}<span
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
                    : (store.modalContext === "whisper"
                        ? transcribeProviders[store.newKeyType]?.keyPlaceholder
                        : providers[store.newKeyType]?.keyPlaceholder) || t("settings.modal.apiKey")}
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
                    title={t("common.copy")}
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
                <div class="mt-3 mb-3 flex flex-wrap gap-x-5 gap-y-2 text-xs">
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

        <div class="grid grid-cols-2 gap-2 pt-4 mt-4 border-t border-white/5 shrink-0">
          <button type="button" onclick={() => store.close()}
            class="flex items-center justify-center gap-2 py-2.5 rounded-lg border border-white/10 text-gray-300 hover:bg-white/5 hover:text-white transition-colors text-sm font-medium" class:col-span-2={!canDelete}>
            {@render icon("M6 6l12 12M6 18 18 6")} {t("settings.modal.cancel")}
          </button>
          {#if canDelete}
            <button type="button" data-provider-action="delete" onclick={() => { if (store.editKeyId) onDelete?.(store.editKeyId); }}
              class="flex items-center justify-center gap-2 py-2.5 px-3 rounded-lg border border-red-500/30 bg-red-500/10 text-red-300 hover:bg-red-500/20 transition-colors text-sm font-medium">
              {@render icon("M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7M14 10v7")} {t("settings.modal.deleteProvider")}
            </button>
          {/if}
          <button type="button" onclick={onSave} disabled={duplicateName || !store.newKeyName.trim()}
            title={duplicateName ? t("settings.errorDuplicateCustomName") : ""}
            class="col-span-2 flex items-center justify-center gap-2 disabled:opacity-40 disabled:cursor-not-allowed py-3 rounded-lg bg-indigo-500 hover:bg-indigo-400 text-white transition-colors text-sm font-bold">
            {@render icon("M4 3h13l4 4v14H3V3zM7 3v6h10V3M7 21v-8h10v8")} {t("settings.modal.save")}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .provider-dialog { max-width: 48rem; }
  .provider-dialog.editing { max-width: 36rem; }
</style>
