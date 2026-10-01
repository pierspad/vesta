/** Warm one tab per idle slot; cancellation stops future mounts after app teardown. */
export function preloadTabs<T>(tabs: readonly T[], load: (tab: T) => Promise<void>, schedule: (callback: () => void) => () => void): () => void {
  let stopped = false;
  let index = 0;
  let cancelScheduled: (() => void) | undefined;
  function next() {
    if (stopped || index >= tabs.length) return;
    cancelScheduled = schedule(() => {
      if (stopped) return;
      void load(tabs[index++]).catch(() => {}).finally(next);
    });
  }
  next();
  return () => { stopped = true; cancelScheduled?.(); };
}
