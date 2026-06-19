export type SnackbarVariant = "success" | "info" | "warning" | "error";

class SnackbarStore {
  message = $state<string | null>(null);
  variant = $state<SnackbarVariant>("info");
  key = $state(0);
  duration = $state(1800);
  private timeout: ReturnType<typeof setTimeout> | null = null;

  show(msg: string, v: SnackbarVariant = "info", d = 1800) {
    if (this.timeout) clearTimeout(this.timeout);
    this.key += 1;
    this.message = msg;
    this.variant = v;
    this.duration = 1800;
    this.timeout = setTimeout(() => {
      this.message = null;
    }, 1800);
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
