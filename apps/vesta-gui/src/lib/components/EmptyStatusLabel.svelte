<script lang="ts">
  let { message } = $props<{ message: string }>();
</script>

<div class="pointer-events-none absolute inset-0 z-10 flex items-center justify-center p-4" role="status" aria-live="polite">
  <span class="rounded-lg bg-gray-900/95 px-4 py-2 text-center text-xs italic text-gray-400">{message}</span>
</div>
