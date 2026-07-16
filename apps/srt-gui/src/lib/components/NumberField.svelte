<script lang="ts">
  /**
   * Input numerico standard: label sopra + input + suffisso unità opzionale
   * (ms, kb/s, ecc.). Ricalca il pattern già in uso in ExperimentalTab
   * (padding/mergeGap/bitrate) e in una dozzina di varianti leggermente
   * diverse sparse fra FlashcardsTab/SettingsTab — estratto qui così tutti
   * i campi numerici condividono dimensioni, colori e comportamento.
   */
  interface Props {
    label: string;
    value: number;
    /** Suffisso unità mostrato a destra dell'input (es. "ms", "kb/s"). */
    unit?: string;
    min?: number;
    max?: number;
    step?: number;
    disabled?: boolean;
    /** Mostra l'asterisco rosso di campo obbligatorio accanto alla label. */
    required?: boolean;
    class?: string;
  }

  let {
    label,
    value = $bindable(),
    unit = "",
    min,
    max,
    step,
    disabled = false,
    required = false,
    class: className = "",
  }: Props = $props();
</script>

<div class={className}>
  <span class="block text-xs text-gray-500 mb-1">
    {label}
    {#if required}<span class="text-red-400">*</span>{/if}
  </span>
  <div class="flex items-center gap-1">
    <input
      type="number"
      bind:value
      {min}
      {max}
      {step}
      {disabled}
      class="input-modern w-full text-xs"
    />
    {#if unit}
      <span class="text-xs text-gray-500 shrink-0">{unit}</span>
    {/if}
  </div>
</div>
