<script lang="ts">
  let { value, accent = "cyan" }: { value: number; accent?: "cyan" | "emerald" } = $props();
  let percent = $derived(Number.isFinite(value) ? Math.max(0, Math.min(100, value)) : 0);
</script>
<div aria-hidden="true" class="pointer-events-none absolute inset-y-0 left-0 rounded-[inherit] transition-[width] duration-300 motion-reduce:transition-none {accent === 'emerald' ? 'bg-emerald-400/20' : 'bg-cyan-400/20'}" style:width={`${percent}%`}></div>
