<script lang="ts">
  import { uiMode } from "./uiModeStore.svelte.ts";
  import RefineTabNew from "./RefineTabNew.svelte";
  import RefineTabOld from "./RefineTabOld.svelte";

  interface Props {
    active?: boolean;
    onGoToSettings?: (section: "overview" | "llm" | "whisper" | "language" | "anki" | "shortcuts", highlightId?: string) => void;
  }

  let { active = true, onGoToSettings }: Props = $props();
</script>

{#if uiMode.useOldReviseUi}
  <RefineTabOld {active} {onGoToSettings} />
{:else}
  <RefineTabNew {active} {onGoToSettings} />
{/if}
