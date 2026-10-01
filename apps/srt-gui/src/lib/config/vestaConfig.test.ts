import { beforeEach, expect, it, vi } from "vitest";
import { invokeCommand } from "$lib/services/tauriClient";
import * as config from "./vestaConfig";
const invoke = vi.mocked(invokeCommand);
beforeEach(async () => { invoke.mockResolvedValue(undefined); await config.replaceAll({}); invoke.mockClear(); });
it("flush waits for durable writes before setup can reload", async () => {
  let release!: () => void;
  invoke.mockImplementationOnce(() => new Promise<void>((resolve) => release = resolve));
  config.setItem("vesta-first-run-setup-complete", "true");
  let completed = false;
  const saving = config.flush().then(() => completed = true);
  await Promise.resolve(); await Promise.resolve();
  expect(completed).toBe(false);
  expect(invoke).toHaveBeenCalledTimes(1);
  release(); await saving;
  expect(invoke).toHaveBeenLastCalledWith("config_replace_all", { values: { "vesta-first-run-setup-complete": "true" } });
});
it("flush exposes a disk failure instead of reporting setup complete", async () => {
  invoke.mockRejectedValueOnce(new Error("disk full"));
  await expect(config.flush()).rejects.toThrow("disk full");
});
