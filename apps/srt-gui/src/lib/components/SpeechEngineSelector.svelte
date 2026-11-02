<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { locale } from "$lib/i18n";
  import SearchableSelect from "$lib/components/SearchableSelect.svelte";
  import { whisperModelsStore as models } from "$lib/stores/whisperModelsStore.svelte";
  import { loadTranscribeTiers, TRANSCRIBE_TIERS_UPDATED_EVENT, type TranscribeTierEntry } from "$lib/config/transcribeTiers";
  import { loadAndValidateApiKeys } from "$lib/config/apiKeys";
  import { transcribeProviders } from "$lib/config/transcribeProviders";
  import * as config from "$lib/config/vestaConfig";
  import { availableSpeechEngines } from "$lib/workflows/speechEngines";

  let { purpose, disabled = false, onresolve, compact = false }: {
    purpose: "transcribe" | "sync";
    disabled?: boolean;
    compact?: boolean;
    onresolve: (entry: TranscribeTierEntry | null | undefined) => void;
  } = $props();
  const t = $derived($locale);
  let tiers = $state(loadTranscribeTiers());
  let keys = $state(loadAndValidateApiKeys());
  let selection = $state(untrack(() => config.getItem(`vesta-${purpose}-speech-engine`)) || "");
  let value = $derived(selection || (purpose === "sync" ? `local:${models.defaultWhisperModel}` : "chain"));
  let entries = $derived(availableSpeechEngines(tiers, models.whisperModels, keys, purpose === "sync"));
  let options = $derived([
    ...(purpose === "transcribe" ? [{ value: "chain", label: t("speech.priorityChain") }] : []),
    ...entries.map(entry => ({ value: entry.id,
      label: `${transcribeProviders[entry.provider]?.name || t("transcribe.engineLocal")} · ${entry.model}`,
      provider: entry.provider === "local" ? "local_whisper" : entry.provider,
    })),
  ]);
  $effect(() => onresolve(value === "chain" ? null : entries.find(entry => entry.id === value)));
  onMount(() => {
    const refresh = () => { tiers = loadTranscribeTiers(); keys = loadAndValidateApiKeys(); };
    const events = [TRANSCRIBE_TIERS_UPDATED_EVENT, "apikeys-updated"];
    for (const event of events) window.addEventListener(event, refresh);
    return () => { for (const event of events) window.removeEventListener(event, refresh); };
  });
</script>

<div title={compact ? t("speech.externalHint") : undefined}>
<SearchableSelect {options} {value} {disabled} {compact}
  placement={compact ? "up" : "down"}
  ariaLabel={t("transcribe.engineTitle")}
  placeholder={t("transcribe.engineTitle")}
  noResultsText={t("transcribe.blocked.noTiers")}
  onchange={(id) => { selection = id; config.setItem(`vesta-${purpose}-speech-engine`, id); }}
/>
</div>

{#if !compact}<p class="mt-2 text-sm" style="color: var(--muted)">{t("speech.externalHint")}</p>{/if}
