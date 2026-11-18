<script lang="ts">
  import { locale } from "$lib/i18n";
  import { supportsCapability, type ApiKeyConfig } from "$lib/config/apiKeys";
  import { DEFAULT_REFINEMENT_PROMPT } from "$lib/config/refinementPrompt";
  import ApiKeysCard from "$lib/panels/ApiKeysCard.svelte";
  import TranslationTiers from "$lib/components/TranslationTiers.svelte";
  import CodeEditor from "$lib/components/CodeEditor.svelte";

  interface Props {
    active: boolean;
    apiKeys: ApiKeyConfig[];
    highlightedModelId: string | null;
    defaultRefinementPrompt: string;
    onAddKey: (providerId?: string) => void;
    onEditKey: (id: string) => void;
    onDeleteKey: (id: string) => void;
    onSetDefault: (id: string) => void;
    onPersistPrompt: () => void;
  }

  let {
    active,
    apiKeys,
    highlightedModelId,
    defaultRefinementPrompt = $bindable(),
    onAddKey,
    onEditKey,
    onDeleteKey,
    onSetDefault,
    onPersistPrompt,
  }: Props = $props();

  let t = $derived($locale);
  let translationTiersRef = $state<{ triggerAddTier: () => void } | null>(null);
</script>

<div id="notification-llm" class:notification-target-pulse={highlightedModelId === "notification-llm"}>
  <ApiKeysCard
    title={t("settings.copy.apiKeysSaved")}
    addButtonLabel={t("settings.copy.addProviderButton")}
    defaultProvider="custom"
    apiKeys={apiKeys.filter((key) => supportsCapability(key, "llm"))}
    {onAddKey}
    {onEditKey}
    {onDeleteKey}
    {onSetDefault}
  />

  <div class="glass-card flex flex-col mb-6 mt-4 relative z-20">
    <div class="p-4 border-b border-white/5 flex items-center justify-between gap-3 w-full">
      <div class="flex items-center gap-3">
        <div class="w-9 h-9 rounded-lg bg-indigo-500/20 text-indigo-300 flex items-center justify-center shrink-0">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h10M4 18h6" />
          </svg>
        </div>
        <div>
          <h3 class="text-sm font-bold text-white">{t("tiers.cardTitle")}</h3>
        </div>
      </div>
      <button
        type="button"
        onclick={() => translationTiersRef?.triggerAddTier()}
        class="inline-flex items-center justify-center gap-2 rounded-lg bg-indigo-500 px-3.5 py-2 text-xs font-bold text-white shadow-lg shadow-indigo-500/20 hover:bg-indigo-400 transition-colors cursor-pointer"
      >
        + {t("tiers.addTier")}
      </button>
    </div>
    <div class="p-4">
      <TranslationTiers bind:this={translationTiersRef} {active} />
    </div>
  </div>
</div>

<div class="glass-card p-5 mt-10 mb-6 relative z-10">
  <div class="flex items-center gap-3 mb-4">
    <div class="w-9 h-9 rounded-lg bg-indigo-500/20 text-indigo-300 flex items-center justify-center shrink-0">
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
      </svg>
    </div>
    <div>
      <h3 class="text-sm font-bold text-white">{t("settings.refinementPromptTitle")}</h3>
    </div>
  </div>
  <div class="space-y-3">
    <CodeEditor
      id="default-refinement-prompt"
      bind:value={defaultRefinementPrompt}
      onchange={onPersistPrompt}
      language="prompt"
      placeholder={DEFAULT_REFINEMENT_PROMPT}
      heightClass="h-[450px]"
      class={highlightedModelId === "default-refinement-prompt" ? "editor-highlight-pulse" : ""}
    />
  </div>
</div>
