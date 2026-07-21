export type SnackbarVariant = "success" | "info" | "warning" | "error";

/** Default auto-dismiss duration (ms) for snackbars. */
export const SNACKBAR_DEFAULT_DURATION = 1800;

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

  show(msg: string, variant: SnackbarVariant = "info", duration = SNACKBAR_DEFAULT_DURATION) {
    if (this.timeout) clearTimeout(this.timeout);
    this.key += 1;
    this.message = msg;
    this.variant = variant;
    this.duration = duration;
    this.timeout = setTimeout(() => {
      this.message = null;
    }, duration);
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

/**
 * Returns a `showSnackbar(message, variant?, duration?)` function bound to a
 * given default duration.
 *
 * Several tabs/stores used to redeclare an identical one-line wrapper around
 * `snackbar.show` purely to pick a different default duration (1300ms for
 * quick confirmations, 3500ms for messages the user needs more time to
 * read). This factory keeps that per-call-site flexibility without
 * duplicating the wrapper body — call sites are unchanged, only the
 * declaration collapses to one line:
 *
 * ```ts
 * const showSnackbar = createSnackbarNotifier(1300);
 * ```
 */
export function createSnackbarNotifier(defaultDuration: number = SNACKBAR_DEFAULT_DURATION) {
  return (message: string, variant: SnackbarVariant = "info", duration: number = defaultDuration) => {
    snackbar.show(message, variant, duration);
  };
}
