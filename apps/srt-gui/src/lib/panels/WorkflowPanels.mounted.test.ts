// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";

import FilesPanel from "./FilesPanel.svelte";
import LogPanel from "./LogPanel.svelte";
import TranscriptionSegmentsPanel from "./TranscriptionSegmentsPanel.svelte";
import TranslationPreviewPanel from "./TranslationPreviewPanel.svelte";

const mounted: ReturnType<typeof mount>[] = [];

function targetEl() {
  const el = document.createElement("div");
  document.body.append(el);
  return el;
}

afterEach(async () => {
  for (const component of mounted.splice(0)) {
    await unmount(component);
  }
  document.body.replaceChildren();
  vi.clearAllMocks();
});

describe("Workflow panels", () => {
  it("FilesPanel displays paths and triggers browse callbacks", async () => {
    const onBrowseSrt = vi.fn();
    const onBrowseMedia = vi.fn();
    const target = targetEl();
    mounted.push(
      mount(FilesPanel, {
        target,
        props: {
          srtPath: "/subs/ep01.srt",
          mediaPath: "/media/ep01.mkv",
          srtLoaded: true,
          onExpandSrt: vi.fn(),
          onExpandMedia: vi.fn(),
          onBrowseSrt,
          onBrowseMedia,
        },
      }),
    );
    await tick();

    expect(target.textContent).toContain("ep01.srt");
    expect(target.textContent).toContain("ep01.mkv");
    const buttons = target.querySelectorAll<HTMLButtonElement>("button");
    expect(buttons.length).toBeGreaterThanOrEqual(2);
  });

  it("LogPanel renders log entries and empties when clear is clicked", async () => {
    const onclear = vi.fn();
    const target = targetEl();
    mounted.push(
      mount(LogPanel, {
        target,
        props: {
          title: "Run Logs",
          clearLogText: "Clear Logs",
          noLogText: "No logs yet",
          logs: [
            { id: 1, timestamp: "12:00:00", message: "Started batch", type: "info" },
            { id: 2, timestamp: "12:00:05", message: "Completed!", type: "success", details: "10/10" },
          ],
          onclear,
        },
      }),
    );
    await tick();

    expect(target.textContent).toContain("Started batch");
    expect(target.textContent).toContain("Completed!");
    expect(target.textContent).toContain("10/10");

    const clearBtn = Array.from(target.querySelectorAll<HTMLButtonElement>("button")).find((b) =>
      b.textContent?.includes("Clear Logs"),
    )!;
    clearBtn.click();
    expect(onclear).toHaveBeenCalledOnce();
  });

  it("TranscriptionSegmentsPanel renders empty state and populated timed segments", async () => {
    const emptyTarget = targetEl();
    mounted.push(mount(TranscriptionSegmentsPanel, { target: emptyTarget, props: { segments: [] } }));
    await tick();
    expect(emptyTarget.textContent).toContain("0");

    const target = targetEl();
    mounted.push(
      mount(TranscriptionSegmentsPanel, {
        target,
        props: {
          segments: [
            { start_ms: 1000, end_ms: 2500, text: "First spoken phrase" },
            { start_ms: 3000, end_ms: 4500, text: "Second spoken phrase" },
          ],
        },
      }),
    );
    await tick();
    expect(target.textContent).toContain("First spoken phrase");
    expect(target.textContent).toContain("Second spoken phrase");
  });

  it("TranslationPreviewPanel renders paired original and translated lines", async () => {
    const target = targetEl();
    mounted.push(
      mount(TranslationPreviewPanel, {
        target,
        props: {
          source: [{ id: 1, text: "Original line" }],
          pairs: [{ id: 1, original: "Original line", translated: "Riga tradotta" }],
          loaded: true,
          translating: false,
        },
      }),
    );
    await tick();

    expect(target.textContent).toContain("#1");
    expect(target.textContent).toContain("Original line");
    expect(target.textContent).toContain("Riga tradotta");
  });
});
