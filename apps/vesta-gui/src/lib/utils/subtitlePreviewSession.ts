interface PreviewJob {
  index: number;
  promise: Promise<string>;
  resolve: (text: string) => void;
  reject: (error: unknown) => void;
}

/** Dialog-local language-group cache, with two concurrent IPC calls. */
export class SubtitlePreviewSession {
  private cache = new Map<number, string>();
  private pending = new Map<number, PreviewJob>();
  private queue: PreviewJob[] = [];
  private window = new Set<number>();
  private running = 0;
  private closed = false;

  constructor(private indices: number[], private load: (index: number) => Promise<string>) {}

  select(index: number): Promise<string> {
    if (this.closed) return Promise.reject(new Error("Preview session closed"));
    const position = this.indices.indexOf(index);
    if (position < 0) return Promise.reject(new Error("Unknown preview track"));
    this.window = new Set(this.indices);
    const result = this.read(index, true);
    // Nearest variants first; retain the entire language group until dialog close.
    const neighbors = this.indices.filter(value => value !== index).sort((a, b) =>
      Math.abs(this.indices.indexOf(a) - position) - Math.abs(this.indices.indexOf(b) - position));
    for (const variant of neighbors) void this.read(variant, false).catch(() => {});
    return result;
  }

  close(): void {
    this.closed = true;
    this.cache.clear();
    this.window.clear();
    for (const job of this.pending.values()) job.reject(new Error("Preview session closed"));
    this.pending.clear();
    this.queue = [];
    // Already dispatched Tauri calls finish in the backend, but cannot repopulate this cache.
  }

  private read(index: number, foreground: boolean): Promise<string> {
    const cached = this.cache.get(index);
    if (cached !== undefined) return Promise.resolve(cached);
    const existing = this.pending.get(index);
    if (existing) {
      const position = this.queue.indexOf(existing);
      if (foreground && position >= 0) {
        this.queue.splice(position, 1);
        this.queue.unshift(existing);
      }
      return existing.promise;
    }
    let resolve!: PreviewJob["resolve"];
    let reject!: PreviewJob["reject"];
    const promise = new Promise<string>((yes, no) => { resolve = yes; reject = no; });
    const job = { index, promise, resolve, reject };
    this.pending.set(index, job);
    if (foreground) this.queue.unshift(job); else this.queue.push(job);
    this.pump();
    return promise;
  }

  private pump(): void {
    while (!this.closed && this.running < 2 && this.queue.length) {
      const job = this.queue.shift()!;
      this.running += 1;
      void Promise.resolve().then(() => {
        if (this.closed) throw new Error("Preview session closed");
        return this.load(job.index);
      }).then(text => {
        if (!this.closed && this.window.has(job.index)) this.cache.set(job.index, text);
        job.resolve(text);
      }, error => job.reject(error)).finally(() => {
        this.running -= 1;
        this.pending.delete(job.index);
        this.pump();
      });
    }
  }
}
