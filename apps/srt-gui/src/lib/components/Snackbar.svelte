<script lang="ts">
  import { SNACKBAR_DEFAULT_DURATION, type SnackbarVariant } from "$lib/stores/snackbarStore.svelte";

  interface Props {
    message: string;
    onclose: () => void;
    variant?: SnackbarVariant;
    bottomClass?: string;
    duration?: number;
    animationKey?: string | number;
  }

  let {
    message,
    onclose,
    variant = "info",
    bottomClass = "bottom-4",
    duration = SNACKBAR_DEFAULT_DURATION,
    animationKey = "",
  }: Props = $props();

  const variantClasses = {
    success: {
      container:
        "bg-emerald-950 border border-emerald-600/70 text-emerald-50 shadow-emerald-950/40",
      icon: "text-emerald-300",
      close: "text-emerald-300 hover:text-emerald-100",
      progress: "bg-emerald-400",
      path: "M5 13l4 4L19 7",
    },
    info: {
      container:
        "bg-blue-900 border border-blue-700 text-blue-100",
      icon: "text-blue-400",
      close: "text-blue-400 hover:text-blue-200",
      progress: "bg-blue-500",
      path: "M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z",
    },
    warning: {
      container:
        "bg-amber-900 border border-amber-700 text-amber-100",
      icon: "text-amber-400",
      close: "text-amber-400 hover:text-amber-200",
      progress: "bg-amber-500",
      path: "M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z",
    },
    error: {
      container: "bg-red-900 border border-red-700 text-red-100",
      icon: "text-red-400",
      close: "text-red-400 hover:text-red-200",
      progress: "bg-red-500",
      path: "M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z",
    },
  } as const;

  let current = $derived(variantClasses[variant]);
</script>

{#key `${animationKey}:${variant}:${duration}:${message}`}
  <div
    class={`fixed ${bottomClass} left-1/2 -translate-x-1/2 ${current.container} rounded-xl shadow-xl flex flex-col overflow-hidden animate-fade-in z-50 min-w-[240px] max-w-[min(92vw,420px)]`}
  >
    <div class="px-6 py-3 flex items-center gap-3">
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
      <span class="flex-grow text-sm font-medium leading-snug">{message}</span>
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
    <div 
      class={`h-1 w-full ${current.progress}`} 
      style="animation: vesta-snackbar-shrink {duration}ms linear forwards;"
    ></div>
  </div>
{/key}

<style>
  @keyframes -global-vesta-snackbar-shrink {
    from {
      width: 100%;
    }
    to {
      width: 0%;
    }
  }
</style>
