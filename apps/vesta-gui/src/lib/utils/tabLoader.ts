/** Creates a deduplicating lazy-tab loader with retryable, observable failures. */
export function createTabLoader<T extends string, C>(options: {
  loaders: Record<T, () => Promise<{ default: C }>>;
  getLoaded: () => Partial<Record<T, C>>;
  setLoaded: (tab: T, component: C) => void;
  clearError: (tab: T) => void;
  setError: (tab: T, message: string) => void;
  onError?: (tab: T, error: unknown) => void;
}): (tab: T) => Promise<void> {
  const pending = new Map<T, Promise<void>>();
  return (tab) => {
    if (options.getLoaded()[tab]) return Promise.resolve();
    const existing = pending.get(tab);
    if (existing) return existing;
    options.clearError(tab);
    const load = Promise.resolve().then(options.loaders[tab])
      .then(({ default: component }) => options.setLoaded(tab, component))
      .catch((error: unknown) => {
        options.onError?.(tab, error);
        options.setError(tab, error instanceof Error ? error.message : String(error));
      })
      .finally(() => pending.delete(tab));
    pending.set(tab, load);
    return load;
  };
}
