<script lang="ts">
  interface Props {
    message: string;
    onclose: () => void;
    variant?: "success" | "info" | "warning" | "error";
    bottomClass?: string;
  }

  let {
    message,
    onclose,
    variant = "info",
    bottomClass = "bottom-4",
  }: Props = $props();

  const variantClasses = {
    success: {
      container:
        "glass-card bg-green-500/20 border border-green-500/30 text-green-200",
      icon: "text-green-400",
      close: "text-green-400 hover:text-green-300",
      path: "M5 13l4 4L19 7",
    },
    info: {
      container:
        "glass-card bg-blue-500/20 border border-blue-500/30 text-blue-200",
      icon: "text-blue-400",
      close: "text-blue-400 hover:text-blue-300",
      path: "M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z",
    },
    warning: {
      container:
        "glass-card bg-amber-500/20 border border-amber-500/30 text-amber-200",
      icon: "text-amber-400",
      close: "text-amber-400 hover:text-amber-300",
      path: "M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z",
    },
    error: {
      container: "glass-card bg-red-500/20 border border-red-500/30 text-red-200",
      icon: "text-red-400",
      close: "text-red-400 hover:text-red-300",
      path: "M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z",
    },
  } as const;

  let current = $derived(variantClasses[variant]);
</script>

<div
  class={`fixed ${bottomClass} left-1/2 -translate-x-1/2 ${current.container} px-6 py-3 rounded-xl shadow-xl flex items-center gap-3 animate-fade-in z-50`}
>
  <svg
    class={`w-5 h-5 ${current.icon} flex-shrink-0`}
    fill="none"
    stroke="currentColor"
    viewBox="0 0 24 24"
  >
    <path
      stroke-linecap="round"
      stroke-linejoin="round"
      stroke-width="2"
      d={current.path}
    />
  </svg>
  <span>{message}</span>
  <button
    type="button"
    onclick={onclose}
    class={`${current.close} ml-2`}
    aria-label="Close"
  >
    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        stroke-linecap="round"
        stroke-linejoin="round"
        stroke-width="2"
        d="M6 18L18 6M6 6l12 12"
      />
    </svg>
  </button>
</div>
