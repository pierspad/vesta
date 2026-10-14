import { expect, it } from "vitest";
import { copyText } from "./clipboard";

it("reports success only after the clipboard write completes", async () => {
  const events: string[] = [];
  let finish!: () => void;
  const pending = copyText("path", "Copied", "Copy failed", message => events.push(message),
    () => new Promise<void>(resolve => { finish = resolve; }));
  expect(events).toEqual([]);
  finish();
  expect(await pending).toBe(true);
  expect(events).toEqual(["Copied"]);
});

it("handles clipboard rejection with a readable error rather than false success", async () => {
  const events: [string, string][] = [];
  const result = await copyText("path", "Copied", "Copy failed",
    (message, variant) => events.push([message, variant]),
    async () => { throw { message: "Permission denied" }; });
  expect(result).toBe(false);
  expect(events).toEqual([["Copy failed: Permission denied", "error"]]);
});
