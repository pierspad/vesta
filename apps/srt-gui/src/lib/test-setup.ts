import { vi } from "vitest";

/**
 * Global vitest setup (see `vite.config.ts` → `test.setupFiles`).
 *
 * The pure modules under `lib/utils`/`lib/config` are the only ones we unit
 * test, but some of them pull in Svelte stores (e.g. `smartMatchingStore`,
 * `i18n`) purely for their exported types/constants. Those stores call
 * `invokeCommand` (→ Tauri's `invoke`) as a side effect of module
 * initialization, which throws "window is not defined" outside a webview and
 * pollutes every test run with unrelated stderr. Mocking it here keeps that
 * noise out of `npm test` and CI logs without changing what's being tested —
 * none of the current unit tests assert on persistence or Tauri commands.
 */
vi.mock("$lib/services/tauriClient", () => ({
  invokeCommand: vi.fn().mockResolvedValue(undefined),
}));
