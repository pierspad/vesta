<script lang="ts">
  import SearchableSelect from "$lib/components/SearchableSelect.svelte";

  type LanguageOption = {
    value: string;
    label: string;
    searchTerms?: string;
    icon?: string;
  };

  let { kind, title, description, options, value, onchange } = $props<{
    kind: "native" | "study";
    title: string;
    description: string;
    options: LanguageOption[];
    value: string;
    onchange: (value: string) => void;
  }>();
</script>

<div class="rounded-xl border border-white/10 bg-white/[0.03] p-5">
  <div class="mb-4 flex items-center gap-3">
    <span class="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg {kind === 'native' ? 'bg-emerald-500/15 text-emerald-300' : 'bg-violet-500/15 text-violet-300'}">
      {#if kind === "native"}
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M12 3a4 4 0 110 8 4 4 0 010-8zM5 21a7 7 0 0114 0"/></svg>
      {:else}
        <span aria-hidden="true">文</span>
      {/if}
    </span>
    <div>
      <p class="text-sm font-semibold text-white">{title}</p>
      <p class="text-[11px] text-gray-500">{description}</p>
    </div>
  </div>
  <SearchableSelect {options} {value} {onchange} />
</div>
