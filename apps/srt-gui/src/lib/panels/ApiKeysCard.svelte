<script lang="ts">
  import { locale } from "$lib/i18n";
  import ProviderIcon from "$lib/components/ProviderIcon.svelte";
  import type { ApiKeyConfig } from "$lib/config/apiKeys";

  interface Props {
    title: string;
    addButtonLabel: string;
    defaultProvider: string;
    apiKeys: ApiKeyConfig[];
    onAddKey: (defaultProvider: string) => void;
    onEditKey: (id: string) => void;
    onDeleteKey: (id: string) => void;
    onSetDefault: (id: string) => void;
  }

  let { title, addButtonLabel, defaultProvider, apiKeys, onAddKey, onEditKey, onDeleteKey, onSetDefault }: Props =
    $props();

  let t = $derived($locale);

  // Purely local display state — nothing outside this card reads it, so it
  // stays here rather than in a shared store (see [[vesta-flashcards-refactor]]
  // for the "only lift state that's read from outside" rule).
  let visibleKeyIds = $state<Set<string>>(new Set());

  function toggleKeyVisibility(keyId: string) {
    const newSet = new Set(visibleKeyIds);
    if (newSet.has(keyId)) {
      newSet.delete(keyId);
    } else {
      newSet.add(keyId);
    }
    visibleKeyIds = newSet;
  }

  function copyApiKey(key: string) {
    navigator.clipboard.writeText(key);
  }

  function maskApiKey(key: string): string {
    if (!key || key.length <= 8) return "••••••••";
    return key.substring(0, 4) + "••••" + key.substring(key.length - 4);
  }

  function formatApiKeyForDisplay(key: string, isVisible: boolean): string {
    if (!key) return "—";
    if (isVisible) {
      return key
        .split("")
        .map((char) => {
          if (char === " ") return "␣"; // Space indicator
          if (char === "\t") return "→"; // Tab indicator
          if (char === "\n") return "↵"; // Newline indicator
          return char;
        })
        .join("");
    }
    return maskApiKey(key);
  }

  function hasSpecialChars(key: string): boolean {
    return /[\s\t\n]/.test(key);
  }
</script>

<div class="glass-card flex flex-col h-auto">
  <div class="p-4 border-b border-white/5 flex items-center justify-between gap-3 w-full">
    <div class="flex items-center gap-3">
      <div class="w-9 h-9 rounded-lg bg-violet-500/20 text-violet-300 flex items-center justify-center shrink-0">
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m-2 4a2 2 0 012 2m-3-4a3 3 0 11-6 0 3 3 0 016 0zM8 21a4 4 0 014-4h4a4 4 0 014 4H8z" />
        </svg>
      </div>
      <div>
        <h3 class="text-sm font-bold text-white tracking-wide">
          {title}
        </h3>
      </div>
    </div>
    <button
      type="button"
      onclick={() => onAddKey(defaultProvider)}
      class="inline-flex items-center justify-center gap-2 rounded-lg bg-indigo-500 px-3.5 py-2 text-xs font-bold text-white shadow-lg shadow-indigo-500/20 hover:bg-indigo-400 transition-colors cursor-pointer"
    >
      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
      </svg>
      {addButtonLabel}
    </button>
  </div>

  <div
    class="p-2 space-y-2 overflow-y-auto"
    style={apiKeys.length <= 1
      ? "height: 78px;"
      : apiKeys.length === 2
        ? "height: 164px;"
        : "height: 250px;"}
  >
    {#each apiKeys as key}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        role="button"
        tabindex="0"
        onclick={() => onEditKey(key.id)}
        onkeydown={(event) => {
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            onEditKey(key.id);
          }
        }}
        class="p-3 rounded-xl border transition-all group cursor-pointer bg-white/[0.035] opacity-90 hover:opacity-100 hover:bg-white/[0.075] hover:border-white/30
          {key.isDefault
          ? 'ring-1 ring-indigo-500/50 border-indigo-400/30 bg-indigo-500/10'
          : 'border-white/10'}"
      >
        <div class="flex items-start gap-3">
          <ProviderIcon provider={key.apiType} />

          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2 mb-0.5">
              <span class="font-medium text-gray-200 text-sm truncate"
                >{key.name}</span
              >
            </div>
            <div class="flex items-center gap-1.5">
              <button
                onclick={(event) => {
                  event.stopPropagation();
                  toggleKeyVisibility(key.id);
                }}
                class="text-[10px] text-gray-500 font-mono truncate hover:text-gray-300 transition-colors flex items-center gap-1"
                title={t("settings.toggleVisibility")}
              >
                {#if visibleKeyIds.has(key.id)}
                  <svg
                    class="w-3 h-3 flex-shrink-0"
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
                    class="w-3 h-3 flex-shrink-0"
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
                <span class="truncate"
                  >{formatApiKeyForDisplay(
                    key.apiKey,
                    visibleKeyIds.has(key.id),
                  )}</span
                >
              </button>
              <button
                onclick={(event) => {
                  event.stopPropagation();
                  copyApiKey(key.apiKey);
                }}
                class="p-1 text-gray-500 hover:text-gray-300 transition-colors flex-shrink-0"
                title={t("common.copy")}
              >
                <svg
                  class="w-3 h-3"
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
              {#if hasSpecialChars(key.apiKey)}
                <span
                  class="text-[9px] bg-amber-500/20 text-amber-400 px-1 py-0.5 rounded flex-shrink-0"
                  title={t("settings.hasSpecialChars")}
                >
                  ⚠
                </span>
              {/if}
            </div>
          </div>

          <div class="flex items-center gap-1.5">
            <button
              onclick={(event) => {
                event.stopPropagation();
                if (!key.isDefault) onSetDefault(key.id);
              }}
              class="p-1.5 rounded transition-colors {key.isDefault
                ? 'text-indigo-300 bg-indigo-500/15 cursor-default'
                : 'text-gray-500 hover:text-indigo-400 hover:bg-white/10'}"
              title={key.isDefault ? t("settings.default") : t("settings.setAsDefault")}
              aria-pressed={key.isDefault}
            >
              <svg
                class="w-4 h-4"
                fill={key.isDefault ? "currentColor" : "none"}
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M5 5a2 2 0 012-2h10a2 2 0 012 2v4.38l1.71 1.71c.18.18.29.43.29.7V14a2 2 0 01-2 2h-5v4l-2 2-2-2v-4H5a2 2 0 01-2-2v-2.21c0-.27.11-.52.29-.71L5 9.38V5z"
                />
              </svg>
            </button>
            <button
              onclick={(event) => {
                event.stopPropagation();
                onEditKey(key.id);
              }}
              class="p-2.5 text-amber-400 hover:text-amber-300 hover:bg-amber-500/10 rounded-lg transition-colors"
              title={t("settings.edit")}
            >
              <svg
                class="w-5 h-5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
                />
              </svg>
            </button>
            <button
              onclick={(event) => {
                event.stopPropagation();
                onDeleteKey(key.id);
              }}
              class="p-2.5 text-red-400 hover:text-red-300 hover:bg-red-500/10 rounded-lg transition-colors"
              title={t("settings.delete")}
            >
              <svg
                class="w-5 h-5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                />
              </svg>
            </button>
          </div>
        </div>
      </div>
    {/each}

    {#if apiKeys.length === 0}
      <div
        class="h-full flex flex-col items-center justify-center text-gray-500 p-2 text-center opacity-50"
      >
        <svg
          class="w-6 h-6 mb-1"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"
          />
        </svg>
        <p class="text-xs">{t("settings.noApiKeys")}</p>
      </div>
    {/if}
  </div>
</div>
