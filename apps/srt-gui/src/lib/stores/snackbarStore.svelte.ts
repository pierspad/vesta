export type SnackbarVariant = "success" | "info" | "warning" | "error";

/** Default auto-dismiss duration (ms) for snackbars. */
export const SNACKBAR_DEFAULT_DURATION = 1700;

/**
 * Single global snackbar. All transient notifications in the app must go
 * through `snackbar.show(...)` — do not build ad-hoc toast markup in
 * components; the one `<Snackbar>` instance lives in App.svelte.
 */
class SnackbarStore {
  message = $state<string | null>(null);
  variant = $state<SnackbarVariant>("info");
  key = $state(0);
  duration = $state(SNACKBAR_DEFAULT_DURATION);
  private timeout: ReturnType<typeof setTimeout> | null = null;

  show(msg: string, variant: SnackbarVariant = "info", duration?: number) {
    const dismissAfter = duration ?? (variant === "error" || variant === "warning" ? 3500 : SNACKBAR_DEFAULT_DURATION);
    if (this.timeout) clearTimeout(this.timeout);
    this.key += 1;
    this.message = msg;
    this.variant = variant;
    this.duration = dismissAfter;
    this.timeout = setTimeout(() => {
      this.message = null;
      this.timeout = null;
    }, dismissAfter);
  }

  close() {
    this.message = null;
    if (this.timeout) {
      clearTimeout(this.timeout);
      this.timeout = null;
    }
  }
}

export const snackbar = new SnackbarStore();

/** Optional per-call-site override; omitted durations use the variant default. */
export function createSnackbarNotifier(defaultDuration?: number) {
  return (message: string, variant: SnackbarVariant = "info", duration: number | undefined = defaultDuration) => {
    snackbar.show(message, variant, duration);
  };
}
