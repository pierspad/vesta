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
  paused = $state(false);
  private timeout: ReturnType<typeof setTimeout> | null = null;
  private remaining = 0;
  private deadline = 0;
  private pauseReasons = new Set<string>();

  show(msg: string, variant: SnackbarVariant = "info", duration?: number) {
    const base = variant === "error" || variant === "warning" ? 3500 : SNACKBAR_DEFAULT_DURATION;
    const readingTime = Math.min(15000, Math.max(base, Array.from(msg).length * 45));
    const dismissAfter = duration !== undefined && Number.isFinite(duration) && duration >= 0
      ? Math.min(duration, 2147483647) : readingTime;
    this.close();
    this.key += 1;
    this.message = msg;
    this.variant = variant;
    this.duration = dismissAfter;
    this.remaining = dismissAfter;
    this.schedule();
  }

  private schedule() {
    // Zero explicitly means persistent, including for errors needing attention.
    if (this.duration === 0 || !this.message || this.paused) return;
    this.deadline = Date.now() + this.remaining;
    this.timeout = setTimeout(() => this.close(), this.remaining);
  }

  pause(reason = "interaction") {
    if (!this.message || this.duration === 0) return;
    this.pauseReasons.add(reason);
    if (this.paused) return;
    this.paused = true;
    this.remaining = Math.max(0, this.deadline - Date.now());
    if (this.timeout !== null) clearTimeout(this.timeout);
    this.timeout = null;
  }

  resume(reason = "interaction") {
    this.pauseReasons.delete(reason);
    if (!this.paused || this.pauseReasons.size) return;
    this.paused = false;
    this.schedule();
  }

  close() {
    this.message = null;
    this.pauseReasons.clear();
    this.paused = false;
    this.remaining = 0;
    if (this.timeout !== null) {
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
