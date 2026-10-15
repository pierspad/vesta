import { expect, it, vi } from "vitest";
import { createTabLoader } from "./tabLoader";

it("surfaces a failed tab load, clears the error on retry, and installs the recovered component", async () => {
  const component = {};
  const loaded: Partial<Record<"translate", object>> = {};
  const errors: Partial<Record<"translate", string>> = {};
  const loader = vi.fn()
    .mockRejectedValueOnce(new Error("chunk unavailable"))
    .mockResolvedValueOnce({ default: component });
  const ensure = createTabLoader({
    loaders: { translate: loader },
    getLoaded: () => loaded,
    setLoaded: (tab, value) => { loaded[tab] = value; },
    clearError: (tab) => { delete errors[tab]; },
    setError: (tab, message) => { errors[tab] = message; },
  });

  await ensure("translate");
  expect(errors.translate).toBe("chunk unavailable");
  expect(loaded.translate).toBeUndefined();

  await ensure("translate");
  expect(errors.translate).toBeUndefined();
  expect(loaded.translate).toBe(component);
  expect(loader).toHaveBeenCalledTimes(2);
});

it("deduplicates concurrent loads and skips loading an installed tab", async () => {
  let resolve!: (module: { default: object }) => void;
  const loading = new Promise<{ default: object }>((yes) => { resolve = yes; });
  const component = {};
  const loaded: Partial<Record<"sync", object>> = {};
  const loader = vi.fn(() => loading);
  const ensure = createTabLoader({
    loaders: { sync: loader },
    getLoaded: () => loaded,
    setLoaded: (tab, value) => { loaded[tab] = value; },
    clearError: vi.fn(),
    setError: vi.fn(),
  });

  const first = ensure("sync");
  const second = ensure("sync");
  await Promise.resolve();
  expect(loader).toHaveBeenCalledOnce();
  resolve({ default: component });
  await Promise.all([first, second]);
  await ensure("sync");
  expect(loader).toHaveBeenCalledOnce();
  expect(loaded.sync).toBe(component);
});

it("allows retry after a loader throws before returning a promise", async () => {
  const component = {};
  const setLoaded = vi.fn();
  const setError = vi.fn();
  const loader = vi.fn()
    .mockImplementationOnce(() => { throw new Error("loader unavailable"); })
    .mockResolvedValueOnce({ default: component });
  const ensure = createTabLoader({
    loaders: { sync: loader }, getLoaded: () => ({}), setLoaded,
    clearError: vi.fn(), setError,
  });
  await ensure("sync");
  expect(setError).toHaveBeenCalledExactlyOnceWith("sync", "loader unavailable");
  await ensure("sync");
  expect(setLoaded).toHaveBeenCalledExactlyOnceWith("sync", component);
});
