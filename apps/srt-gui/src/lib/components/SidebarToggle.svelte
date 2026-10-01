<script lang="ts">
  interface Props {
    label: string;
    title: string;
    iconPath: string;
    active: boolean;
    collapsed: boolean;
    disabled?: boolean;
    activity?: boolean;
    ontoggle: () => void;
  }
  let { label, title, iconPath, active, collapsed, disabled = false, activity = false, ontoggle }: Props = $props();
  let words = $derived(label.trim().split(/\s+/));
</script>
<button type="button" role="switch" aria-checked={active} aria-label={label} {disabled}
  title={collapsed ? title : undefined} onclick={ontoggle}
  class="flex h-[60px] w-full items-center rounded-xl border border-transparent bg-white/5 text-gray-400 transition-all duration-100 ease-out hover:bg-white/10 hover:text-gray-300 disabled:cursor-not-allowed disabled:opacity-50 {collapsed ? 'justify-center px-2' : 'justify-between px-3.5'}">
  <span class="flex items-center gap-3.5">
    <span class="relative flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-white/5 bg-white/5 text-gray-400">
      <svg aria-hidden="true" class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={iconPath} /></svg>
      {#if activity}<span class="absolute -right-0.5 -top-0.5 h-2 w-2 animate-ping rounded-full bg-blue-500"></span>{/if}
    </span>
    {#if !collapsed}
      <span class="select-none text-center text-[15px] font-semibold leading-none text-gray-300">
        <span class="block">{words[0]}</span>
        {#if words.length > 1}<span class="block">{words.slice(1).join(" ")}</span>{/if}
      </span>
    {/if}
  </span>
  {#if !collapsed}
    <span aria-hidden="true" class="h-6 w-10 shrink-0 rounded-full p-1 transition-colors duration-100 {active ? 'bg-indigo-600' : 'bg-white/10'}">
      <span class="block h-4 w-4 transform rounded-full bg-white shadow-md transition-transform duration-100 {active ? 'translate-x-4' : 'translate-x-0'}"></span>
    </span>
  {/if}
</button>
