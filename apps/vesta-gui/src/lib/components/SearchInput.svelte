<script lang="ts">
  import type { Snippet } from "svelte";
  import { locale } from "$lib/i18n";

  let t = $derived($locale);

  interface Props {
    value?: string;
    placeholder?: string;
    size?: "sm" | "md";
    class?: string;
    inputClass?: string;
    disabled?: boolean;
    autofocus?: boolean;
    ariaLabel?: string;
    oninput?: (value: string) => void;
    onclear?: () => void;
    children?: Snippet;
  }

  let {
    value = $bindable(""),
    placeholder = "",
    size = "sm",
    class: containerClass = "w-64",
    inputClass = "",
    disabled = false,
    autofocus = false,
    ariaLabel = undefined,
    oninput = undefined,
    onclear = undefined,
    children = undefined,
  }: Props = $props();

  function handleInput(e: Event) {
    const val = (e.currentTarget as HTMLInputElement).value;
    value = val;
    oninput?.(val);
  }

  function handleClear() {
    value = "";
    onclear?.();
    oninput?.("");
  }
</script>

<div class="relative flex items-center {containerClass}">
  <!-- Left magnifying glass icon -->
  <div class="pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3 text-gray-400 z-10">
    <svg class="{size === 'md' ? 'w-4 h-4' : 'w-3.5 h-3.5'}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
    </svg>
  </div>

  <!-- svelte-ignore a11y_autofocus -->
  <input
    type="text"
    bind:value
    {placeholder}
    {disabled}
    {autofocus}
    aria-label={ariaLabel || placeholder}
    oninput={handleInput}
    class="{size === 'md' ? 'h-10 text-sm pl-9' : 'h-9 text-xs pl-9'} {value && !disabled ? (children ? 'pr-20' : 'pr-8') : (children ? 'pr-14' : 'pr-3')} w-full rounded-xl border border-white/10 bg-white/[0.04] text-gray-100 placeholder-gray-500 focus:border-indigo-500/60 focus:bg-white/[0.06] focus:ring-1 focus:ring-indigo-500/40 outline-none transition-colors disabled:opacity-50 {inputClass}"
  />

  {#if value && !disabled}
    <button
      type="button"
      onclick={handleClear}
      class="absolute {children ? 'right-12' : 'right-2.5'} top-1/2 -translate-y-1/2 text-gray-400 hover:text-white transition-colors p-1 rounded cursor-pointer leading-none z-10 flex items-center justify-center text-sm"
      aria-label={t("common.clearField")}
      title={t("common.clearField")}
    >
      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
      </svg>
    </button>
  {/if}

  {#if children}
    <div class="absolute right-1 top-1/2 -translate-y-1/2 flex items-center z-10">
      {@render children()}
    </div>
  {/if}
</div>
