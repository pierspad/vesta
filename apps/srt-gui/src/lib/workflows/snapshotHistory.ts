/** Bounded undo snapshots with one capture per typing burst. */
export class SnapshotHistory<T> {
  private snapshots: string[] = [];
  private timer: ReturnType<typeof setTimeout> | null = null;
  constructor(
    private readonly read: () => T,
    private readonly restore: (value: T) => void,
    private readonly limit = 50,
    private readonly delay = 500,
  ) {}
  push() {
    const snapshot = JSON.stringify(this.read());
    if (this.snapshots.at(-1) !== snapshot) {
      this.snapshots = [...this.snapshots.slice(-(this.limit - 1)), snapshot].slice(-this.limit);
    }
  }
  schedule() {
    if (this.timer === null) this.push();
    this.cancelTimer();
    this.timer = setTimeout(() => { this.timer = null; }, this.delay);
  }
  undo() {
    this.cancelTimer();
    const snapshot = this.snapshots.pop();
    if (snapshot !== undefined) this.restore(JSON.parse(snapshot) as T);
  }
  private cancelTimer() {
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = null;
  }
  reset() { this.cancelTimer(); this.snapshots = []; }
  dispose() { this.cancelTimer(); }
}
