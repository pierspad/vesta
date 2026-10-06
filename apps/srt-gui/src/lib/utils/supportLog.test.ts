import { afterEach, expect, it, vi } from "vitest";
import { redactLogText, safeLogValue, recordSupportEvent, setSupportLogSink } from "./supportLog";
afterEach(() => setSupportLogSink(null));
it("hides structured and textual credentials without leaking payloads", () => {
  const value = safeLogValue({ api_key: "private-key", nested: { authorization: "Bearer example", password: "secret password" }, prompt: "private subtitle text", message: "working" });
  expect(value).not.toMatch(/private-key|example|secret password|private subtitle text/);
  expect(value).toContain("working");
  expect(redactLogText('api_key="secret key with spaces" token=abcdef https://host/?token=foobar Authorization: Bearer xyz')).not.toMatch(/secret key|abcdef|foobar|xyz/);
  expect(redactLogText("sk-abcdefghijklmnopqrstuvwxyz")).toBe("[redacted]");
});
it("records only while a sink is active and bounds log size", () => {
  const sink = vi.fn(); recordSupportEvent("error", "ignored"); expect(sink).not.toHaveBeenCalled();
  setSupportLogSink(sink); recordSupportEvent("error", "a".repeat(10000));
  expect(sink.mock.calls[0][0].message).toHaveLength(4000);
  expect(sink.mock.calls[0][0].time).toMatch(/Z$/);
  setSupportLogSink(null); recordSupportEvent("error", "ignored"); expect(sink).toHaveBeenCalledOnce();
});
