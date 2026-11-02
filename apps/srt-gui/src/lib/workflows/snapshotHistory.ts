/** Bounded undo snapshots with one capture per typing burst. */
export class SnapshotHistory<T> {
  private snapshots: string[] = [];
  private future: string[] = [];
  private timer: ReturnType<typeof setTimeout> | null = null;
  constructor(
    private readonly read: () => T,
    private readonly restore: (value: T) => void,
    private readonly limit = 50,
    private readonly delay = 500,
  ) {}
  push() {
    this.cancelTimer();
    this.future = [];
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
  get canUndo() { return this.snapshots.length > 0; }
  get canRedo() { return this.future.length > 0; }
  undo(): boolean {
    this.cancelTimer();
    const current = JSON.stringify(this.read());
    // A grouped input callback can capture a snapshot without changing its value.
    while (this.snapshots.at(-1) === current) this.snapshots.pop();
    const snapshot = this.snapshots.pop();
    if (snapshot === undefined) return false;
    this.future.push(current);
    this.restore(JSON.parse(snapshot) as T);
    return true;
  }
  redo(): boolean {
    this.cancelTimer();
    const snapshot = this.future.pop();
    if (snapshot === undefined) return false;
    this.snapshots = [...this.snapshots, JSON.stringify(this.read())].slice(-this.limit);
    this.restore(JSON.parse(snapshot) as T);
    return true;
  }
  private cancelTimer() {
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = null;
  }
  reset() { this.cancelTimer(); this.snapshots = []; this.future = []; }
  dispose() { this.cancelTimer(); }
}
