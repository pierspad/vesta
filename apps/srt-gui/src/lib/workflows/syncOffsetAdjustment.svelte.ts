/** Coalesces keyboard adjustments while keeping reset/unmount free of pending frames. */
export class SyncOffsetAdjustment {
  value = $state(0);
  private target = 0;
  private frame: number | null = null;
  private disposed = false;
  constructor(
    private readonly schedule = (callback: FrameRequestCallback) => requestAnimationFrame(callback),
    private readonly cancel = (id: number) => cancelAnimationFrame(id),
  ) {}
  update(delta: number) {
    if (this.disposed) return;
    this.target += delta;
    this.cancelPending();
    this.frame = this.schedule(() => {
      this.frame = null;
      if (!this.disposed) this.value = this.target;
    });
  }
  private cancelPending() {
    if (this.frame !== null) this.cancel(this.frame);
    this.frame = null;
  }
  reset() {
    this.cancelPending();
    this.target = 0;
    this.value = 0;
  }
  dispose() {
    this.disposed = true;
    this.cancelPending();
  }
}
