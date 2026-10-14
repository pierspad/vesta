import { vi } from "vitest";

/**
 * Global vitest setup (see `vite.config.ts` → `test.setupFiles`).
 *
 * Vitest runs in Node, outside the Tauri webview. Keep command transport
 * inert by default so importing stores and workflows cannot invoke Tauri
 * during module initialization. Tests for a specific service or workflow
 * replace this mock when they need to assert command arguments, persistence,
 * or error handling. This setup does not exercise native IPC behavior.
 */
vi.mock("$lib/services/tauriClient", () => ({
  invokeCommand: vi.fn().mockResolvedValue(undefined),
}));
