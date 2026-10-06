<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import ProviderIcon from "$lib/components/ProviderIcon.svelte";

  interface Option {
    value: string;
    label: string;
    searchTerms?: string; // Additional terms to search by (e.g., English name)
    icon?: string; // Flag emoji or icon
    provider?: string; // Provider ID for showing provider logo
  }

  interface Props {
    options: Option[];
    value: string;
    onchange: (value: string) => void;
    placeholder?: string;
    className?: string;
    noResultsText?: string;
    onfocus?: () => void;
    disabled?: boolean;
    placement?: "up" | "down";
  }

  let {
    options,
    value,
    onchange,
    placeholder = "Select...",
    className = "",
    noResultsText = "No results",
    onfocus,
    disabled = false,
    placement = "down",
  }: Props = $props();

  let isOpen = $state(false);
  let searchQuery = $state("");
  let highlightedIndex = $state(0);
  let inputElement = $state<HTMLInputElement | null>(null);
  let dropdownElement = $state<HTMLDivElement | null>(null);
  let containerElement = $state<HTMLDivElement | null>(null);

  let selectedOption = $derived(options.find((opt) => opt.value === value));

  function iconMarkup(icon: string | undefined): string | null {
    if (!icon) return null;
    const trimmed = icon.trim();
    if (trimmed.startsWith("<")) return trimmed;
    // Region-indicator flag emoji depend on the host OS emoji font. Windows
    // WebView2 can render these as their fallback letters (for example GB).
    // Draw flags from bundled inline SVG instead, so every packaged build is stable.
    const regionalIndicators = [...trimmed].filter((char) => /\p{Regional_Indicator}/u.test(char));
    if (regionalIndicators.length !== 2) return null;
    const countryCode = regionalIndicators
      .map((char) => String.fromCharCode(char.codePointAt(0)! - 0x1f1e6 + 65))
      .join("")
      .toLowerCase();
    return `<img src="/flags/${countryCode}.svg" alt="" class="language-flag" />`;
  }

  let displayValue = $derived.by(() => {
    if (isOpen) return searchQuery;
    if (!selectedOption) return "";
    return iconMarkup(selectedOption.icon)
      ? selectedOption.label
      : `${selectedOption.icon || ""} ${selectedOption.label}`.trim();
  });

  let filteredOptions = $derived.by(() => {
    if (!searchQuery.trim()) return options;
    const query = searchQuery.toLowerCase().trim();
    return options.filter((opt) => {
      const labelMatch = opt.label.toLowerCase().includes(query);
      const searchMatch = opt.searchTerms?.toLowerCase().includes(query);
      return labelMatch || searchMatch;
    });
  });

  $effect(() => {
    if (filteredOptions.length > 0) {
      highlightedIndex = 0;
    }
  });

  async function handleFocus() {
    if (disabled) return;
    isOpen = true;
    searchQuery = "";
    // Start with the currently selected option highlighted and visible
    const selectedIdx = options.findIndex((opt) => opt.value === value);
    highlightedIndex = selectedIdx >= 0 ? selectedIdx : 0;
    if (onfocus) {
      onfocus();
    }
    await tick();
    scrollToHighlighted();
  }

  function handleBlur(e: FocusEvent) {
    if (disabled) return;
    // Delay to allow click on option
    setTimeout(() => {
      if (!containerElement?.contains(document.activeElement)) {
        isOpen = false;
        searchQuery = "";
      }
    }, 150);
  }

  function handleInput(e: Event) {
    if (disabled) return;
    const target = e.target as HTMLInputElement;
    searchQuery = target.value;
    isOpen = true;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (disabled) return;
    if (!isOpen) {
      if (e.key === "Enter" || e.key === "ArrowDown" || e.key === "ArrowUp") {
        isOpen = true;
        e.preventDefault();
      }
      return;
    }

    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        highlightedIndex = Math.min(
          highlightedIndex + 1,
          filteredOptions.length - 1,
        );
        scrollToHighlighted();
        break;
      case "ArrowUp":
        e.preventDefault();
        highlightedIndex = Math.max(highlightedIndex - 1, 0);
        scrollToHighlighted();
        break;
      case "Enter":
        e.preventDefault();
        if (filteredOptions[highlightedIndex]) {
          selectOption(filteredOptions[highlightedIndex]);
        }
        break;
      case "Escape":
        isOpen = false;
        searchQuery = "";
        inputElement?.blur();
        break;
      case "Tab":
        isOpen = false;
        searchQuery = "";
        break;
    }
  }

  function scrollToHighlighted() {
    if (dropdownElement) {
      const highlighted = dropdownElement.querySelector(
        `[data-index="${highlightedIndex}"]`,
      );
      highlighted?.scrollIntoView({ block: "nearest" });
    }
  }

  function selectOption(opt: Option) {
    onchange(opt.value);
    isOpen = false;
    searchQuery = "";
    inputElement?.blur();
  }

  function handleClickOutside(e: MouseEvent) {
    if (containerElement && !containerElement.contains(e.target as Node)) {
      isOpen = false;
      searchQuery = "";
    }
  }

  onMount(() => {
    document.addEventListener("click", handleClickOutside);
  });

  onDestroy(() => {
    document.removeEventListener("click", handleClickOutside);
  });
</script>

<div
  class="relative {className}"
  style:z-index={isOpen ? 50 : "auto"}
  bind:this={containerElement}
>
  <div class="relative">
    {#if selectedOption?.provider && !isOpen}
      <div class="absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none z-10">
        <ProviderIcon provider={selectedOption.provider} size="w-5.5 h-5.5" glyph="w-3.5 h-3.5" rounded="rounded-md" />
      </div>
    {/if}
    {#if selectedOption?.icon && !isOpen}
      {#if iconMarkup(selectedOption.icon)}
        <div class="absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none z-10 text-gray-400 flex items-center justify-center">
          {@html iconMarkup(selectedOption.icon)!}
        </div>
      {/if}
    {/if}
    <input
      bind:this={inputElement}
      type="text"
      value={displayValue}
      oninput={handleInput}
      onfocus={handleFocus}
      onblur={handleBlur}
      onkeydown={handleKeydown}
      {placeholder}
      disabled={disabled}
      class="searchable-select-input w-full disabled:opacity-50 disabled:cursor-not-allowed disabled:bg-white/[0.02]"
      style:padding-left={(selectedOption?.provider || (selectedOption?.icon && iconMarkup(selectedOption.icon))) && !isOpen ? "38px" : ""}
      autocomplete="off"
    />
    <div
      class="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-gray-400 transition-transform duration-200 {isOpen
        ? 'rotate-180'
        : ''}"
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
          d="M19 9l-7 7-7-7"
        />
      </svg>
    </div>
  </div>

  {#if isOpen}
    <div
      bind:this={dropdownElement}
      class="searchable-select-dropdown"
      class:opens-up={placement === "up"}
    >
      {#if filteredOptions.length === 0}
        <div class="px-4 py-3 text-gray-500 text-sm text-center">
          {noResultsText}
        </div>
      {:else}
        {#each filteredOptions as option, index}
          <button
            type="button"
            data-index={index}
            onclick={() => selectOption(option)}
            onmouseenter={() => (highlightedIndex = index)}
            class="searchable-select-option w-full text-left
              {index === highlightedIndex
              ? 'highlighted text-white'
              : 'text-gray-300'}
              {option.value === value ? 'selected' : ''}"
          >
            {#if option.provider}
              <span class="mr-2 shrink-0">
                <ProviderIcon provider={option.provider} size="w-5.5 h-5.5" glyph="w-3.5 h-3.5" rounded="rounded-md" />
              </span>
            {/if}
            {#if option.icon}
              {#if iconMarkup(option.icon)}
                <span class="mr-2 flex items-center justify-center">{@html iconMarkup(option.icon)!}</span>
              {:else}
                <span class="mr-2">{option.icon}</span>
              {/if}
            {/if}
            <span>{option.label}</span>
            {#if option.value === value}
              <svg
                class="w-4 h-4 ml-auto text-indigo-400"
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
            {/if}
          </button>
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .searchable-select-input {
    appearance: none;
    -webkit-appearance: none;
    background-image: none;
    box-shadow: none;
    background: rgba(24, 24, 42, 0.98) !important;
    border: 1px solid rgba(148, 163, 184, 0.22);
    border-radius: 10px;
    padding: 11px 40px 11px 14px;
    color: white;
    transition: border-color 0.12s ease, background-color 0.12s ease;
    font-size: 0.875rem;
  }

  :global(.language-flag) {
    display: block;
    width: 1.25rem;
    height: 0.875rem;
    object-fit: cover;
    border-radius: 2px;
  }

  .searchable-select-input:focus {
    outline: none;
    background: rgba(27, 28, 47, 1) !important;
    border-color: rgba(129, 140, 248, 0.58);
  }

  .searchable-select-input::placeholder {
    color: rgba(255, 255, 255, 0.4);
  }

  .searchable-select-dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    margin-top: 4px;
    max-height: 280px;
    overflow-y: auto;
    overflow-x: hidden;
    background: #0f172a !important;
    border: 1px solid rgba(148, 163, 184, 0.25);
    border-radius: 10px;
    box-shadow: none;
    z-index: 9999;
    opacity: 1 !important;
    isolation: isolate;
  }

  .searchable-select-dropdown.opens-up {
    top: auto;
    bottom: 100%;
    margin-top: 0;
    margin-bottom: 4px;
  }

  .searchable-select-option {
    display: flex;
    align-items: center;
    min-height: 42px;
    gap: 8px;
    padding: 10px 14px;
    font-size: 0.875rem;
    transition: background-color 0.08s ease, color 0.08s ease;
    cursor: pointer;
    border: none;
    background: #0f172a !important;
  }

  .searchable-select-option:hover {
    background: #1e293b !important;
  }

  .searchable-select-option.highlighted {
    background: rgba(67, 56, 202, 0.8) !important;
  }

  .searchable-select-option.selected {
    background: rgba(79, 70, 229, 0.6) !important;
  }

  :global(.compact-select) .searchable-select-input {
    min-height: 42px;
    padding: 10px 38px 10px 12px;
    font-size: 0.75rem;
  }

  :global(.compact-select) .searchable-select-option {
    min-height: 38px;
    padding: 9px 12px;
    font-size: 0.75rem;
  }

  .searchable-select-option:first-child {
    border-radius: 9px 9px 0 0;
  }

  .searchable-select-option:last-child {
    border-radius: 0 0 9px 9px;
  }

  .searchable-select-option:only-child {
    border-radius: 9px;
  }

  .searchable-select-dropdown::-webkit-scrollbar {
    width: 6px;
  }

  .searchable-select-dropdown::-webkit-scrollbar-track {
    background: transparent;
  }

  .searchable-select-dropdown::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.1);
    border-radius: 3px;
  }

  .searchable-select-dropdown::-webkit-scrollbar-thumb:hover {
    background: rgba(255, 255, 255, 0.2);
  }
</style>
