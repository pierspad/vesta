import { expect, it } from "vitest";
import { errorMessage } from "./errorMessage";

it.each([
  [new Error("Offline"), "Offline"],
  ["Disk full", "Disk full"],
  [{ message: "Permission denied", code: 403 }, "Permission denied"],
  [null, "Fallback"],
  [{}, "Fallback"],
  ["  ", "Fallback"],
])("renders rejection %s without object coercion or stack traces", (value, expected) => {
  expect(errorMessage(value, "Fallback")).toBe(expected);
});

it("redacts credentials from visible errors", () => {
  expect(errorMessage('Request rejected: api_key="private key"')).not.toContain("private key");
});
