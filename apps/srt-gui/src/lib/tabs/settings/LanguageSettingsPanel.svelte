<script lang="ts">
  import type { Snippet } from "svelte";
  import { locale } from "$lib/i18n";
  import { uiMode } from "$lib/stores/uiModeStore.svelte";
  import ToggleRow from "$lib/components/ToggleRow.svelte";
  import CodeEditor from "$lib/components/CodeEditor.svelte";

  interface Props {
    defaultLanguagesCard: Snippet;
    smartMatchingEnabled: boolean;
    rulesDraftValue: string;
    rulesDraftError: string | null;
    onToggleSmartMatching: () => void;
    onSaveSmartMatchingRules: () => void;
  }

  let {
    defaultLanguagesCard,
    smartMatchingEnabled,
    rulesDraftValue = $bindable(),
    rulesDraftError,
    onToggleSmartMatching,
    onSaveSmartMatchingRules,
  }: Props = $props();

  let t = $derived($locale);
</script>

<div class="mb-6 flex flex-col gap-4">
  {@render defaultLanguagesCard()}

  {#if uiMode.expertMode}
    <div id="smart-matching-rules" class="glass-card p-6 flex flex-col gap-4">
      <ToggleRow
        label={t("settings.smartMatching.title")}
        checked={smartMatchingEnabled}
        onchange={onToggleSmartMatching}
        accent="violet"
        iconPath="M13 10V3L4 14h7v7l9-11h-7z"
      />

      {#if smartMatchingEnabled}
        <div class="mt-4 pt-4 border-t border-white/5 space-y-3">
          <div class="relative">
            <CodeEditor
              bind:value={rulesDraftValue}
              language="jsonc"
              heightClass="h-[520px]"
              onchange={onSaveSmartMatchingRules}
            />
          </div>
          {#if rulesDraftError}
            <p class="mt-2 rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-200">
              {rulesDraftError}
            </p>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>
