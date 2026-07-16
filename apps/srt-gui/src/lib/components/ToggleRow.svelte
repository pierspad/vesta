<script lang="ts">
  /**
   * Riga standard "icona + titolo/descrizione + switch". Oggi ogni tab la
   * reimplementa con dimensioni diverse (w-10 h-6 bg-indigo-600 in un posto,
   * w-11 h-6 bg-violet-500/60 in un altro...): estratta qui per farla
   * combaciare sempre, indipendentemente dall'accent.
   */
  type Accent = "amber" | "sky" | "emerald" | "violet" | "rose" | "indigo" | "gray";

  interface Props {
    label: string;
    description?: string;
    checked: boolean;
    accent?: Accent;
    /** Path SVG (24x24, stroke) per l'icona nel pallino a sinistra. Se omesso, niente icona. */
    iconPath?: string;
    disabled?: boolean;
    onchange?: (checked: boolean) => void;
  }

  let {
    label,
    description = "",
    checked = $bindable(),
    accent = "gray",
    iconPath = "",
    disabled = false,
    onchange,
  }: Props = $props();

  const iconBg: Record<Accent, string> = {
    amber: "bg-amber-500/15 text-amber-300",
    sky: "bg-sky-500/15 text-sky-300",
    emerald: "bg-emerald-500/15 text-emerald-300",
    violet: "bg-violet-500/15 text-violet-300",
    rose: "bg-rose-500/15 text-rose-300",
    indigo: "bg-indigo-500/15 text-indigo-300",
    gray: "bg-white/10 text-gray-300",
  };
  const switchOn: Record<Accent, string> = {
    amber: "bg-amber-600",
    sky: "bg-sky-600",
    emerald: "bg-emerald-600",
    violet: "bg-violet-600",
    rose: "bg-rose-600",
    indigo: "bg-indigo-600",
    gray: "bg-indigo-600",
  };

  function toggle() {
    if (disabled) return;
    checked = !checked;
    onchange?.(checked);
  }
</script>

<div class="flex items-center justify-between gap-4">
  <div class="flex items-center gap-3.5 min-w-0">
    {#if iconPath}
      <div class="w-9 h-9 rounded-lg {iconBg[accent]} flex items-center justify-center shrink-0">
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={iconPath} />
        </svg>
      </div>
    {/if}
    <div class="flex-1 min-w-0">
      <span class="text-sm font-semibold text-white block truncate">{label}</span>
      {#if description}
        <span class="text-xs text-gray-400 block leading-relaxed">{description}</span>
      {/if}
    </div>
  </div>
  <button
    type="button"
    onclick={toggle}
    {disabled}
    class="w-10 h-6 rounded-full p-1 transition-colors duration-200 shrink-0 cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed {checked ? switchOn[accent] : 'bg-white/10'}"
    role="switch"
    aria-checked={checked}
    aria-label={label}
  >
    <div class="bg-white w-4 h-4 rounded-full shadow-md transform transition-transform duration-200 {checked ? 'translate-x-4' : 'translate-x-0'}"></div>
  </button>
</div>
