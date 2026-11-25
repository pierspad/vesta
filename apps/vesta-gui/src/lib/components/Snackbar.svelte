<script lang="ts">
  import { onMount } from "svelte";
  import { locale } from "$lib/i18n";
  let t = $derived($locale);
  import { SNACKBAR_DEFAULT_DURATION, type SnackbarIcon, type SnackbarVariant } from "$lib/stores/snackbarStore.svelte";

  interface Props {
    message: string;
    onclose: () => void;
    variant?: SnackbarVariant;
    icon?: SnackbarIcon;
    duration?: number;
    animationKey?: string | number;
    paused?: boolean;
    onpause?: (reason: string) => void;
    onresume?: (reason: string) => void;
  }

  let {
    message,
    onclose,
    variant = "info",
    icon = "default",
    duration = SNACKBAR_DEFAULT_DURATION,
    animationKey = "",
    paused = false,
    onpause = () => {},
    onresume = () => {},
  }: Props = $props();

  const variantClasses = {
    success: {
      container:
        "border-emerald-400/35",
      icon: "text-emerald-300",
      close: "text-emerald-300 hover:text-emerald-100",
      progress: "bg-emerald-400",
      path: "M5 13l4 4L19 7",
    },
    info: {
      container:
        "border-indigo-400/35",
      icon: "text-blue-400",
      close: "text-blue-400 hover:text-blue-200",
      progress: "bg-blue-500",
      path: "M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z",
    },
    warning: {
      container:
        "border-amber-400/35",
      icon: "text-amber-400",
      close: "text-amber-400 hover:text-amber-200",
      progress: "bg-amber-500",
      path: "M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z",
    },
    error: {
      container: "border-red-400/35",
      icon: "text-red-400",
      close: "text-red-400 hover:text-red-200",
      progress: "bg-red-500",
      path: "M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z",
    },
  } as const;

  let current = $derived(variantClasses[variant]);
  onMount(() => {
    const visibilityChanged = () => document.hidden ? onpause("visibility") : onresume("visibility");
    document.addEventListener("visibilitychange", visibilityChanged);
    visibilityChanged();
    return () => { document.removeEventListener("visibilitychange", visibilityChanged); onresume("visibility"); };
  });
</script>

{#key `${animationKey}:${variant}:${icon}:${duration}:${message}`}
  <div
    role={variant === "error" ? "alert" : "status"}
    aria-atomic="true"
    onmouseenter={() => onpause?.("hover")}
    onmouseleave={() => onresume?.("hover")}
    onfocusin={() => onpause?.("focus")}
    onfocusout={(event) => {
      if (!event.currentTarget.contains(event.relatedTarget as Node | null)) onresume?.("focus");
    }}
    class={`snackbar fixed left-1/2 -translate-x-1/2 ${current.container} border bg-slate-900 text-gray-100 rounded-xl shadow-xl flex flex-col overflow-hidden animate-fade-in`}
  >
    <div class="snackbar-content px-4 py-3 flex items-start gap-3">
      <svg
        class={`w-5 h-5 ${icon === "settings" ? "text-amber-300" : current.icon} flex-shrink-0`}
        data-icon={icon}
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
        aria-hidden="true"
      >
        {#if icon === "settings"}
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
          />
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
          />
        {:else}
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d={current.path}
          />
        {/if}
      </svg>
      <span class="snackbar-message min-w-0 flex-grow text-sm font-medium leading-snug" dir="auto">{message}</span>
      <button
        type="button"
        onclick={onclose}
        class={`${current.close} shrink-0 rounded-lg p-1 -m-1 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-400`}
        aria-label={t("common.close")}
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
    {#if duration > 0}
      <div
        aria-hidden="true"
        class={`h-0.5 w-full shrink-0 ${current.progress}`}
        style:animation={`vesta-snackbar-shrink ${duration}ms linear forwards`}
        style:animation-play-state={paused ? "paused" : "running"}
      ></div>
    {/if}
  </div>
{/key}

<style>
  .snackbar {
    z-index: 1000;
    bottom: max(1rem, env(safe-area-inset-bottom, 0px));
    width: max-content;
    min-width: min(15rem, calc(100vw - 2rem));
    max-width: min(32rem, calc(100vw - 2rem - env(safe-area-inset-left, 0px) - env(safe-area-inset-right, 0px)));
    max-height: calc(100dvh - 2rem - env(safe-area-inset-top, 0px) - env(safe-area-inset-bottom, 0px));
  }
  .snackbar-content { min-height: 0; overflow-y: auto; }
  .snackbar-message { overflow-wrap: anywhere; white-space: pre-wrap; }
  @media (prefers-reduced-motion: reduce) {
    .snackbar { animation: none; }
  }
  @keyframes -global-vesta-snackbar-shrink {
    from {
      width: 100%;
    }
    to {
      width: 0%;
    }
  }
</style>
