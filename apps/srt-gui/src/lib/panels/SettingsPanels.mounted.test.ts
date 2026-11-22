// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createRawSnippet, mount, tick, unmount } from "svelte";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn<(cmd: string) => Promise<unknown>>(async (cmd: string) => {
    if (cmd === "flashcard_list_fonts" || cmd === "ocr_list_models" || cmd === "transcribe_list_models") return [];
    if (cmd === "transcribe_addons_status") return { ffmpeg_ready: true, vad_ready: false, vad_models: [] };
    if (cmd === "setup_media_status") return { ready: true, download_bytes: 0 };
    if (cmd === "flashcard_get_cpu_count") return 4;
    if (cmd === "flashcard_get_total_memory_mb") return 8192;
    return null;
  }),
  rawInvoke: vi.fn<(cmd: string) => Promise<unknown>>(),
  save: vi.fn(),
  open: vi.fn(),
  copy: vi.fn(),
}));

vi.mock("$lib/services/tauriClient", () => ({
  invokeCommand: mocks.invoke,
}));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: mocks.rawInvoke,
}));
vi.mock("$lib/utils/dialogGuard", () => ({
  guardedOpen: mocks.open,
  guardedSave: mocks.save,
}));
vi.mock("$lib/utils/clipboard", () => ({
  writeClipboard: mocks.copy,
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("@tauri-apps/plugin-fs", () => ({
  readTextFile: vi.fn().mockResolvedValue("{}"),
  writeTextFile: vi.fn().mockResolvedValue(undefined),
}));

import AnkiSettingsPanel from "./AnkiSettingsPanel.svelte";
import ApiKeysCard from "./ApiKeysCard.svelte";
import DiagnosticsSettingsPanel from "./DiagnosticsSettingsPanel.svelte";
import IttSettingsPanel from "./IttSettingsPanel.svelte";
import OverviewSettingsPanel from "./OverviewSettingsPanel.svelte";
import SupportLogsPanel from "./SupportLogsPanel.svelte";
import WhisperSettingsPanel from "./WhisperSettingsPanel.svelte";

const mounted: ReturnType<typeof mount>[] = [];
const sCopy = (key: string) => key;

function targetEl() {
  const el = document.createElement("div");
  document.body.append(el);
  return el;
}

async function settle(turns = 6) {
  for (let i = 0; i < turns; i++) {
    await Promise.resolve();
    await tick();
  }
}

beforeEach(() => {
  vi.clearAllMocks();
  const handleSupport = async (cmd: string) => {
    if (cmd === "support_log_status" || cmd === "support_log_start" || cmd === "support_log_stop") {
      return { recording: cmd === "support_log_start", path: "/tmp/vesta-support.jsonl", lines: [] };
    }
    if (cmd === "support_log_export") return "/tmp/exported.jsonl";
    return null;
  };
  mocks.rawInvoke.mockImplementation(handleSupport);
  mocks.invoke.mockImplementation(async (cmd: string) => {
    if (cmd === "get_system_diagnostics") {
      return {
        os: "linux",
        arch: "x86_64",
        compute_backend: "vulkan",
        gpu_compiled: true,
        gpu_device: "Mock GPU",
        ffmpeg_available: true,
        video_encoder: "libx264",
        hardware_video_encoder: false,
        gstreamer_available: true,
        gstreamer_h264: true,
        gstreamer_h265: true,
      };
    }
    if (cmd.startsWith("support_log_")) return handleSupport(cmd);
    if (cmd === "ankiconnect_ping") return 6;
    if (cmd === "ankiconnect_deck_names") return ["Default"];
    if (cmd === "ocr_list_models") return [];
    if (cmd === "transcribe_list_models") return [{ id: "tiny", name: "Tiny", size_mb: 75, downloaded: false }];
    if (cmd === "transcribe_addons_status") return { ffmpeg_ready: true, vad_ready: false, vad_models: [] };
    if (cmd === "setup_media_status") return { ready: true, download_bytes: 0 };
    if (cmd === "flashcard_list_fonts") {
      return [{ id: "noto-sans", name: "Noto Sans", language: "en", downloaded: false, file_name: "NotoSans.ttf" }];
    }
    if (cmd === "flashcard_get_cpu_count") return 4;
    if (cmd === "flashcard_get_total_memory_mb") return 8192;
    return null;
  });
});

afterEach(async () => {
  for (const component of mounted.splice(0)) {
    await unmount(component);
  }
  document.body.replaceChildren();
});

describe("Settings panels", () => {
  it("ApiKeysCard masks keys and dispatches add/edit callbacks", async () => {
    const onAddKey = vi.fn();
    const target = targetEl();
    mounted.push(
      mount(ApiKeysCard, {
        target,
        props: {
          title: "Saved API Keys",
          addButtonLabel: "Add Provider",
          defaultProvider: "google",
          apiKeys: [
            {
              id: "k1",
              name: "My Gemini",
              apiType: "google",
              apiKey: "AIzaSecret1234567890",
              isDefault: true,
            },
          ],
          onAddKey,
          onEditKey: vi.fn(),
          onDeleteKey: vi.fn(),
          onSetDefault: vi.fn(),
        },
      }),
    );
    await tick();

    expect(target.textContent).toContain("My Gemini");
    expect(target.textContent).not.toContain("AIzaSecret1234567890");
    const addBtn = Array.from(target.querySelectorAll<HTMLButtonElement>("button")).find((b) =>
      b.textContent?.includes("Add Provider"),
    )!;
    addBtn.click();
    expect(onAddKey).toHaveBeenCalledWith("google");
  });

  it("DiagnosticsSettingsPanel refreshes and renders system diagnostics and SupportLogsPanel", async () => {
    const target = targetEl();
    mounted.push(mount(DiagnosticsSettingsPanel, { target }));
    await settle();

    expect(mocks.invoke).toHaveBeenCalledWith("get_system_diagnostics");
    expect(target.textContent).toContain("linux");
    expect(target.textContent).toContain("Mock GPU");
  });

  it("SupportLogsPanel toggles recording via support_log_start IPC", async () => {
    const target = targetEl();
    mounted.push(mount(SupportLogsPanel, { target }));
    await settle();

    const recordBtn = target.querySelector<HTMLButtonElement>("button")!;
    recordBtn.click();
    await settle();
    expect(mocks.rawInvoke).toHaveBeenCalledWith("support_log_start");
  });

  it("AnkiSettingsPanel renders note type templates and field definitions", async () => {
    const target = targetEl();
    mounted.push(mount(AnkiSettingsPanel, { target, props: { s: sCopy } }));
    await settle();

    expect(target.textContent).toContain("{{Expression}}");
    expect(target.textContent).toContain("{{Meaning}}");
  });

  it("OverviewSettingsPanel renders snippet content and export backup controls", async () => {
    const defaultLanguagesCard = createRawSnippet(() => ({
      render: () => `<div data-default-langs="1">Default Languages Slot</div>`,
    }));
    const target = targetEl();
    mounted.push(mount(OverviewSettingsPanel, { target, props: { defaultLanguagesCard } }));
    await settle();

    expect(target.querySelector('[data-default-langs="1"]')).not.toBeNull();
  });

  it("IttSettingsPanel renders Vision endpoints and priority tiers", async () => {
    const target = targetEl();
    mounted.push(
      mount(IttSettingsPanel, {
        target,
        props: {
          active: true,
          s: sCopy,
          apiKeys: [],
          highlightedModelId: null,
          onAddKey: vi.fn(),
          onEditKey: vi.fn(),
          onDeleteKey: vi.fn(),
          onSetDefault: vi.fn(),
        },
      }),
    );
    await settle();

    expect(target.querySelector("#itt-endpoints")).not.toBeNull();
    expect(target.querySelector("#itt-tiers")).not.toBeNull();
  });

  it("WhisperSettingsPanel renders Whisper endpoints and model list", async () => {
    const target = targetEl();
    mounted.push(
      mount(WhisperSettingsPanel, {
        target,
        props: {
          s: sCopy,
          apiKeys: [],
          onAddKey: vi.fn(),
          onEditKey: vi.fn(),
          onDeleteKey: vi.fn(),
          onSetDefault: vi.fn(),
          highlightedModelId: null,
        },
      }),
    );
    await settle();

    expect(target.textContent).toBeTruthy();
  });
});
