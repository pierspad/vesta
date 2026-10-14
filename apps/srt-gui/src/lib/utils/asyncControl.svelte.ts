/** Invalidates stale async responses, including same-input retries and close/reopen. */
export class LatestRequest {
  private version = 0;
  begin() {
    const version = ++this.version;
    return { isCurrent: () => version === this.version };
  }
  invalidate() { this.version += 1; }
}

/** Holds an action lock until BOTH the original work and its cancellation
 * request settle. A late cancellation can never cancel the next run. */
export class AsyncOperation {
  busy = $state(false);
  activeAction = $state<string | null>(null);
  cancelling = $state(false);
  cancelRequested = $state(false);
  private sequence = 0;
  private workSettled = false;
  private cancelSettled = true;
  private disposed = false;

  begin(action = "work") {
    if (this.busy || this.disposed) return null;
    const sequence = ++this.sequence;
    this.busy = true;
    this.activeAction = action;
    this.workSettled = false;
    this.cancelSettled = true;
    this.cancelRequested = false;
    return {
      isCurrent: () => !this.disposed && sequence === this.sequence && !this.workSettled,
      isCancelled: () => this.disposed || sequence !== this.sequence || this.cancelRequested,
      finish: () => { if (sequence === this.sequence) { this.workSettled = true; this.release(); } },
    };
  }
  async cancel(action: () => Promise<unknown>) {
    if (!this.busy || this.workSettled || this.cancelling || this.cancelRequested) return false;
    const sequence = this.sequence;
    this.cancelling = true;
    this.cancelRequested = true;
    this.cancelSettled = false;
    try { await action(); return true; }
    catch (error) { if (sequence === this.sequence) this.cancelRequested = false; throw error; }
    finally {
      if (sequence === this.sequence) { this.cancelling = false; this.cancelSettled = true; this.release(); }
    }
  }
  private release() {
    if (this.workSettled && this.cancelSettled) { this.busy = false; this.activeAction = null; }
  }
  dispose() { this.disposed = true; }
}

/** Whisper inference, model downloads and removal share a native resource. */
export const localWhisperOperation = new AsyncOperation();
