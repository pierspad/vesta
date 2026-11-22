import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { expect, test as base } from "@playwright/test";

const script = fileURLToPath(new URL("../../../build-scripts/browser_ipc_contract.py", import.meta.url));
const contract = JSON.parse(execFileSync("python3", [script], { encoding: "utf8" }));

export const test = base.extend({
  page: async ({ page }, use) => {
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await use(page);
    expect(errors).toEqual([]);

    const calls = await page.evaluate(() => window.browserFixture?.calls ?? []);
    for (const { command, args } of calls) {
      if (command.startsWith("plugin:")) continue;
      const signature = contract[command];
      expect(signature, command).toBeDefined();
      const keys = Object.keys(args ?? {});
      expect(keys, command).toEqual(expect.arrayContaining(signature.required));
      expect([...signature.required, ...signature.optional], command).toEqual(expect.arrayContaining(keys));
    }
  },
});

export { expect };
