export type SnackbarVariant = "success" | "info" | "warning" | "error";
export const SNACKBAR_DEFAULT_DURATION = 1700;
export const SNACKBAR_MAX_PENDING = 8;
const severity: Record<SnackbarVariant, number> = { info: 0, success: 0, warning: 1, error: 2 };
interface Notification { message: string; variant: SnackbarVariant; duration: number }

/** One notification surface. Errors retain their reading time and concurrent
 * completions queue rather than overwrite each other. Duplicate messages coalesce. */
export class SnackbarStore {
  message = $state<string | null>(null);
  variant = $state<SnackbarVariant>("info");
  key = $state(0);
  duration = $state(SNACKBAR_DEFAULT_DURATION);
  paused = $state(false);
  private queue: Notification[] = [];
  private timeout: ReturnType<typeof setTimeout> | null = null;
  private deadline = 0;
  private remaining = 0;
  private pauseReasons = new Set<string>();

  show(msg: string, variant: SnackbarVariant = "info", duration?: number) {
    const message = msg.trim();
    if (!message) return;
    if ((this.message === message && this.variant === variant) || this.queue.some(item => item.message === message && item.variant === variant)) return;
    const minimum = variant === "error" ? 5000 : variant === "warning" ? 3500 : SNACKBAR_DEFAULT_DURATION;
    const requested = duration !== undefined && Number.isFinite(duration) && duration >= 0 ? Math.min(duration, 2147483647) : minimum;
    const notification = { message, variant, duration: requested === 0 ? 0 : Math.max(minimum, requested, Math.min(15000, minimum + Math.max(0, Array.from(message).length - 80) * 20)) };
    if (this.message === null) { this.activate(notification); return; }
    if (this.queue.length >= SNACKBAR_MAX_PENDING) {
      const weakest = this.queue.reduce((index, item, i, all) => severity[item.variant] < severity[all[index].variant] ? i : index, 0);
      if (severity[this.queue[weakest].variant] > severity[variant]) return;
      this.queue.splice(weakest, 1);
    }
    this.queue.push(notification);
    this.queue.sort((a, b) => severity[b.variant] - severity[a.variant]);
  }

  private activate(notification: Notification) {
    this.clearTimer();
    this.pauseReasons = new Set([...this.pauseReasons].filter(reason => reason === "visibility"));
    this.paused = this.pauseReasons.size > 0;
    this.key += 1;
    this.message = notification.message;
    this.variant = notification.variant;
    this.duration = notification.duration;
    this.remaining = notification.duration;
    if (!this.paused) this.schedule();
  }

  private schedule() {
    if (this.duration === 0 || this.message === null || this.paused) return;
    const key = this.key;
    this.deadline = Date.now() + this.remaining;
    this.timeout = setTimeout(() => this.close(key), this.remaining);
  }
  private clearTimer() {
    if (this.timeout !== null) clearTimeout(this.timeout);
    this.timeout = null;
  }
  pause(reason = "interaction") {
    if (!this.pauseReasons.size && this.message !== null) {
      this.remaining = Math.max(0, this.deadline - Date.now());
      this.clearTimer();
    }
    this.pauseReasons.add(reason);
    this.paused = true;
  }
  resume(reason = "interaction") {
    this.pauseReasons.delete(reason);
    this.paused = this.pauseReasons.size > 0;
    if (!this.paused && this.message !== null && this.timeout === null) this.schedule();
  }
  /** A stale timeout or close handler cannot dismiss a newer notification. */
  close(expectedKey = this.key) {
    if (expectedKey !== this.key) return;
    this.clearTimer();
    const next = this.queue.shift();
    if (next) this.activate(next);
    else { this.message = null; this.pauseReasons.clear(); this.paused = false; }
  }
  clear() {
    this.queue = [];
    this.close();
  }
}
export const snackbar = new SnackbarStore();
export function createSnackbarNotifier(defaultDuration?: number) {
  return (message: string, variant: SnackbarVariant = "info", duration: number | undefined = defaultDuration) => snackbar.show(message, variant, duration);
}
