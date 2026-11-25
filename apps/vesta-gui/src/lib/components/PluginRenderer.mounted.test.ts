// @vitest-environment jsdom
import { createRawSnippet, mount, tick, unmount } from "svelte";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import PluginRenderer from "./PluginRenderer.svelte";
import PluginHeaderHarness from "$lib/test/fixtures/PluginHeaderHarness.svelte";
import type { WidgetNode } from "$lib/services/pluginClient";
import { pluginsStore } from "$lib/stores/pluginsStore.svelte";
import { uiMode } from "$lib/stores/uiModeStore.svelte";
import { invokeCommand } from "$lib/services/tauriClient";

const mockDispatch = vi.fn();
const mockRender = vi.fn();
const mockDragDrop = vi.fn<(handlers: import("$lib/utils/dragDrop").DragDropHandlers) => () => void>();
const mockDownloadDir = vi.fn<() => Promise<string>>().mockResolvedValue("/home/user/Downloads");
const mockGuardedOpen = vi.fn();

vi.mock("@tauri-apps/api/path", () => ({
  downloadDir: () => mockDownloadDir(),
}));

vi.mock("$lib/utils/dialogGuard", () => ({
  guardedOpen: (...args: any[]) => mockGuardedOpen(...args),
  guardedSave: vi.fn(),
}));

vi.mock("$lib/utils/dragDrop", () => ({
  setupWebviewDragDrop: (...args: Parameters<typeof mockDragDrop>) => mockDragDrop(...args),
}));

vi.mock("$lib/services/pluginClient", async () => {
  const actual = await vi.importActual<any>("$lib/services/pluginClient");
  return {
    ...actual,
    pluginRender: (...args: any[]) => mockRender(...args),
    pluginDispatchEvent: (...args: any[]) => mockDispatch(...args),
    pluginSaveSettings: vi.fn().mockResolvedValue(undefined),
  };
});

vi.mock("$lib/services/tauriClient", () => ({
  invokeCommand: vi.fn().mockResolvedValue(undefined),
}));

const components: ReturnType<typeof mount>[] = [];

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(async () => {
  for (const component of components.splice(0)) await unmount(component);
  pluginsStore.clearProgress("test-plugin");
  pluginsStore.clearProgress("other-plugin");
  document.body.replaceChildren();
});

it("renders a full declarative widget schema recursively", async () => {
  const schema: WidgetNode = {
    type: "column",
    gap: 4,
    children: [
      { type: "heading", text: "Vesta Test Plugin", level: 1 },
      { type: "text", text: "Welcome to declarative plugins", variant: "muted" },
      { type: "divider" },
      {
        type: "row",
        gap: 2,
        children: [
          {
            type: "textInput",
            id: "search_query",
            label: "Search Movie",
            placeholder: "Enter title…",
            defaultValue: "Inception",
          },
          {
            type: "button",
            id: "btn_search",
            label: "Search",
            variant: "primary",
          },
        ],
      },
      {
        type: "checkbox",
        id: "include_subtitles",
        label: "Include English Subtitles",
        defaultValue: true,
      },
      {
        type: "numberInput",
        id: "limit",
        label: "Max Results",
        defaultValue: 10,
        min: 1,
        max: 50,
      },
      {
        type: "select",
        id: "quality",
        label: "Video Quality",
        options: [
          { label: "1080p", value: "1080" },
          { label: "720p", value: "720" },
        ],
        defaultValue: "1080",
      },
      {
        type: "progress",
        label: "Downloading",
        value: 75,
        max: 100,
      },
      {
        type: "list",
        id: "results_list",
        items: [
          {
            id: "item_1",
            title: "Inception (2010)",
            subtitle: "Sci-Fi / Action",
            badge: "HD",
            actions: [{ id: "download", label: "Download", variant: "primary" }],
          },
        ],
      },
      {
        type: "table",
        headers: ["Track", "Language", "Codec"],
        rows: [
          ["1", "English", "AAC"],
          ["2", "Japanese", "FLAC"],
        ],
      },
      {
        type: "fileOutput",
        path: "/tmp/sample.mp4",
        label: "Saved Video",
      },
    ],
  };

  const target = document.createElement("div");
  document.body.append(target);

  components.push(
    mount(PluginRenderer, {
      target,
      props: {
        pluginId: "test-plugin",
        screen: "main",
        active: true,
        initialRoot: schema,
      },
    })
  );

  await tick();

  // Verify Heading
  expect(target.querySelector("h1")?.textContent).toBe("Vesta Test Plugin");
  // Verify Text
  expect(target.querySelector("p")?.textContent).toContain("Welcome to declarative plugins");
  // Verify Divider
  expect(target.querySelector("hr")).not.toBeNull();
  // Verify Input
  const input = target.querySelector<HTMLInputElement>("#widget-search_query")!;
  expect(input).not.toBeNull();
  expect(input.value).toBe("Inception");
  // Verify Checkbox
  const checkbox = target.querySelector<HTMLButtonElement>('[role="checkbox"]')!;
  expect(checkbox.getAttribute("aria-checked")).toBe("true");
  // Verify Button
  const btn = target.querySelector<HTMLButtonElement>("#widget-btn_search")!;
  expect(btn.textContent).toContain("Search");
  // Verify List item
  expect(target.textContent).toContain("Inception (2010)");
  expect(target.textContent).toContain("HD");
  // Verify Table
  expect(target.querySelector("table")).not.toBeNull();
  expect(target.textContent).toContain("Japanese");
  // Verify File Output
  expect(target.textContent).toContain("/tmp/sample.mp4");
});

it("dispatches actions with current screen values when a button is clicked", async () => {
  const schema: WidgetNode = {
    type: "column",
    children: [
      {
        type: "textInput",
        id: "query",
        defaultValue: "Interstellar",
      },
      {
        type: "checkbox",
        id: "hd_only",
        label: "HD Only",
        defaultValue: false,
      },
      {
        type: "button",
        id: "btn_submit",
        label: "Submit",
      },
    ],
  };

  mockDispatch.mockResolvedValueOnce({
    root: {
      type: "column",
      children: [
        { type: "heading", text: "Search Results for Interstellar", level: 2 },
      ],
    },
  });

  const target = document.createElement("div");
  document.body.append(target);

  components.push(
    mount(PluginRenderer, {
      target,
      props: {
        pluginId: "test-plugin",
        screen: "main",
        active: true,
        initialRoot: schema,
      },
    })
  );

  await tick();

  const button = target.querySelector<HTMLButtonElement>("#widget-btn_submit")!;
  button.click();
  await tick();

  expect(mockDispatch).toHaveBeenCalledWith(
    "test-plugin",
    expect.objectContaining({
      screen: "main",
      widgetId: "btn_submit",
      eventType: "click",
      values: expect.objectContaining({
        query: "Interstellar",
        hd_only: false,
      }),
    }),
    expect.any(String),
    expect.any(Boolean),
  );

  await vi.waitFor(() => {
    expect(target.querySelector("h2")?.textContent).toBe("Search Results for Interstellar");
  });
});

it("renders credential inputs with external links and hints", async () => {
  const schema: WidgetNode = {
    type: "column",
    children: [
      {
        type: "passwordInput",
        id: "demo_api_key",
        label: "Demo API Key",
        hint: {
          text: "Required for fetching external metadata.",
          link: {
            label: "Get Demo Key",
            url: "https://example.com/api-keys",
          },
        },
      },
      {
        type: "link",
        label: "Official Documentation",
        url: "https://example.com/docs",
      },
    ],
  };

  const target = document.createElement("div");
  document.body.append(target);

  components.push(
    mount(PluginRenderer, {
      target,
      props: {
        pluginId: "demo-plugin",
        screen: "settings",
        active: true,
        initialRoot: schema,
      },
    })
  );

  await tick();

  expect(target.textContent).toContain("Demo API Key");
  expect(target.textContent).toContain("Required for fetching external metadata.");

  const links = target.querySelectorAll("a");
  expect(links.length).toBeGreaterThanOrEqual(2);
  expect(links[0].textContent).toContain("Get Demo Key");
  expect(links[0].getAttribute("href")).toBe("https://example.com/api-keys");
  expect(links[1].textContent).toContain("Official Documentation");
});

it("applies dynamic ui.update patches to modify elements and visibility", async () => {
  const schema: WidgetNode = {
    type: "column",
    id: "main_col",
    children: [
      {
        type: "text",
        id: "status_text",
        text: "Initial status",
      },
      {
        type: "button",
        id: "toggle_btn",
        label: "Click to patch",
      },
      {
        type: "text",
        id: "removable_text",
        text: "Will be removed",
      },
    ],
  };

  mockDispatch.mockResolvedValueOnce({
    patch: {
      setProps: {
        status_text: { text: "Patched status successfully!" },
      },
      visibility: {
        toggle_btn: false,
      },
      remove: {
        id: "removable_text",
      },
      insert: {
        parentId: "main_col",
        node: {
          type: "heading",
          id: "inserted_heading",
          text: "Dynamically Inserted Heading",
          level: 3,
        },
      },
      values: {
        runtime_key: "runtime_value",
      },
    },
  });

  const target = document.createElement("div");
  document.body.append(target);

  components.push(
    mount(PluginRenderer, {
      target,
      props: {
        pluginId: "patch-plugin",
        screen: "main",
        active: true,
        initialRoot: schema,
      },
    })
  );

  await tick();

  expect(target.textContent).toContain("Initial status");
  expect(target.textContent).toContain("Will be removed");

  const button = target.querySelector<HTMLButtonElement>("#widget-toggle_btn")!;
  button.click();
  await tick();

  await vi.waitFor(() => {
    expect(target.textContent).toContain("Patched status successfully!");
    expect(target.textContent).not.toContain("Will be removed");
    expect(target.textContent).toContain("Dynamically Inserted Heading");
    // Button hidden
    expect(target.querySelector("#widget-toggle_btn")).toBeNull();
  });
});

it("hides expertOnly widgets in easy mode and reveals them when expertMode is enabled", async () => {
  uiMode.expertMode = false;
  const schema: WidgetNode = {
    type: "column",
    children: [
      {
        type: "text",
        id: "standard_text",
        text: "Standard Setting",
      },
      {
        type: "text",
        id: "expert_text",
        text: "Advanced Expert Tuning",
        expertOnly: true,
      },
    ],
  };

  const target = document.createElement("div");
  document.body.append(target);

  components.push(
    mount(PluginRenderer, {
      target,
      props: {
        pluginId: "expert-test-plugin",
        screen: "main",
        active: true,
        initialRoot: schema,
      },
    })
  );

  await tick();

  // In easy mode, expertOnly is hidden
  expect(target.textContent).toContain("Standard Setting");
  expect(target.textContent).not.toContain("Advanced Expert Tuning");

  // Toggle expertMode on
  uiMode.expertMode = true;
  await tick();

  // In expert mode, expertOnly is visible
  expect(target.textContent).toContain("Advanced Expert Tuning");

  // Clean up uiMode
  uiMode.expertMode = false;
});

it("renders segmented, pathInput, languageSelect and row flex/width widgets", async () => {
  const schema: WidgetNode = {
    type: "column",
    children: [
      {
        type: "heading",
        text: "Media Manager",
        level: 2,
        icon: "film",
      },
      {
        type: "segmented",
        id: "tab_mode",
        options: [
          { value: "search", label: "Search Online", icon: "search", badge: "New" },
          { value: "local", label: "Local Files", icon: "folder" },
        ],
        defaultValue: "search",
      },
      {
        type: "row",
        children: [
          {
            type: "pathInput",
            id: "folder_path",
            label: "Target Folder",
            defaultValue: "/media/movies",
            flex: 2,
          },
          {
            type: "languageSelect",
            id: "target_lang",
            label: "Language",
            defaultValue: "en",
            width: "w-48",
          },
        ],
      },
      {
        type: "button",
        id: "btn_action",
        label: "Download",
        icon: "download",
      },
    ],
  };

  const target = document.createElement("div");
  document.body.append(target);

  components.push(
    mount(PluginRenderer, {
      target,
      props: {
        pluginId: "media-test-plugin",
        screen: "main",
        active: true,
        initialRoot: schema,
      },
    })
  );

  await tick();

  expect(target.textContent).toContain("Media Manager");
  expect(target.textContent).toContain("Search Online");
  expect(target.textContent).toContain("Local Files");
  expect(target.textContent).toContain("Target Folder");
  expect(target.textContent).toContain("Language");
  expect(target.textContent).toContain("Download");

  // Check segmented interaction
  const buttons = target.querySelectorAll("button");
  const localBtn = Array.from(buttons).find((b) => b.textContent?.includes("Local Files"));
  expect(localBtn).toBeDefined();
  localBtn?.click();
  await tick();
  expect(mockDispatch).toHaveBeenCalledWith(
    "media-test-plugin",
    expect.objectContaining({
      widgetId: "tab_mode",
      eventType: "change",
      values: expect.objectContaining({ tab_mode: "local" }),
    }),
    expect.any(String),
    expect.any(Boolean)
  );
});

it("triggers native snackbar when returned from event or render", async () => {
  const { snackbar } = await import("$lib/stores/snackbarStore.svelte");
  const snackbarSpy = vi.spyOn(snackbar, "success");

  mockDispatch.mockResolvedValueOnce({
    root: {
      type: "column",
      children: [
        { type: "text", text: "Done" },
      ],
      snackbar: { message: "Saved successfully!", type: "success" },
    },
  });

  const schema: WidgetNode = {
    type: "column",
    children: [
      {
        type: "button",
        id: "btn_save",
        label: "Save",
      },
    ],
  };

  const target = document.createElement("div");
  document.body.append(target);

  components.push(
    mount(PluginRenderer, {
      target,
      props: {
        pluginId: "snackbar-test-plugin",
        screen: "main",
        active: true,
        initialRoot: schema,
      },
    })
  );

  await tick();

  const button = target.querySelector("button");
  button?.click();
  await tick();
  await tick();

  expect(snackbarSpy).toHaveBeenCalledWith("Saved successfully!");
  snackbarSpy.mockRestore();
});

it("dispatches action with action.id as widgetId and click as eventType when clicking list item action", async () => {
  const schema: WidgetNode = {
    type: "column",
    children: [
      {
        type: "list",
        id: "results_list",
        items: [
          {
            id: "movie_item_123",
            title: "Test Movie (2024)",
            actions: [
              {
                id: "select_movie_123",
                label: "Download / Details",
                variant: "primary",
              },
            ],
          },
        ],
      },
    ],
  };

  mockDispatch.mockResolvedValueOnce({
    root: {
      type: "column",
      children: [{ type: "text", text: "Detail Screen" }],
    },
  });

  const target = document.createElement("div");
  document.body.append(target);

  components.push(
    mount(PluginRenderer, {
      target,
      props: {
        pluginId: "vibravid",
        screen: "main",
        active: true,
        initialRoot: schema,
      },
    })
  );

  await tick();

  const actionBtn = Array.from(target.querySelectorAll("button")).find((b) =>
    b.textContent?.includes("Download / Details")
  );
  expect(actionBtn).toBeDefined();
  actionBtn?.click();
  await tick();

  expect(mockDispatch).toHaveBeenCalledWith(
    "vibravid",
    expect.objectContaining({
      widgetId: "select_movie_123",
      eventType: "click",
    }),
    expect.any(String),
    expect.any(Boolean)
  );

  // Double-clicking the list item row also triggers its default action and text is selectable
  expect(target.querySelector(".plugin-renderer")?.classList.contains("select-text")).toBe(true);
  mockDispatch.mockClear();
  mockDispatch.mockResolvedValueOnce({});
  const rowEl = target.querySelector(".plugin-renderer")?.querySelector(".rounded-xl.border") as HTMLElement;
  expect(rowEl).not.toBeNull();
  rowEl.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
  await tick();
  expect(mockDispatch).toHaveBeenCalledWith(
    "vibravid",
    expect.objectContaining({
      widgetId: "select_movie_123",
      eventType: "click",
    }),
    expect.any(String),
    expect.any(Boolean)
  );
});

it("guides blocked searches to settings and enforces prerequisites on Enter", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const root: WidgetNode = {
    type: "column",
    children: [
      { type: "textInput", id: "movie", submitAction: "search" },
      { type: "button", id: "settings", label: "Settings", variant: "settings", status: "attention" },
      { type: "button", id: "search", label: "Search", requiresAny: ["movie"], tooltip: "Enter an ID",
        disabled: true, disabledTooltip: "Save your key", disabledFocus: "settings" },
    ],
  };
  mockRender.mockResolvedValue({ root });
  mockDispatch.mockResolvedValue({});
  const renderer = mount(PluginRenderer, { target, props: { pluginId: "fixture", initialRoot: root } });
  components.push(renderer);
  await tick();
  const search = target.querySelector<HTMLButtonElement>("#widget-search")!;
  const settings = target.querySelector<HTMLButtonElement>("#widget-settings")!;
  const input = target.querySelector<HTMLInputElement>("#widget-movie")!;
  expect(settings.querySelector(":scope > .plugin-status-dot")).not.toBeNull();
  expect(settings.classList.contains("plugin-settings-button")).toBe(true);
  expect(search.title).toBe("Save your key");
  expect(search.getAttribute("aria-disabled")).toBe("true");
  search.click();
  expect(document.activeElement).toBe(settings);
  expect(mockDispatch).not.toHaveBeenCalled();

  renderer.applyPatch({ setProps: { search: { disabled: false, disabledFocus: undefined } } });
  await tick();
  expect(search.title).toBe("Enter an ID");
  expect(search.disabled).toBe(true);
  input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
  expect(mockDispatch).not.toHaveBeenCalled();
  input.value = "123";
  input.dispatchEvent(new Event("input", { bubbles: true }));
  await tick();
  expect(search.hasAttribute("aria-disabled")).toBe(false);
  input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
  expect(mockDispatch).toHaveBeenCalledOnce();
});

it("focuses an empty credential field without dispatching Save", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const root: WidgetNode = {
    type: "column",
    children: [
      { type: "passwordInput", id: "key", label: "API key", emptyWarning: "Add a key" },
      { type: "button", id: "save", label: "Save", requiresAny: ["key"], disabledFocus: "key" },
    ],
  };
  mockRender.mockResolvedValue({ root });
  mockDispatch.mockResolvedValue({});
  components.push(mount(PluginRenderer, { target, props: { pluginId: "fixture", initialRoot: root } }));
  await tick();

  const input = target.querySelector<HTMLInputElement>("#widget-key")!;
  const save = target.querySelector<HTMLButtonElement>("#widget-save")!;
  expect(target.querySelector(".plugin-field-dot")).not.toBeNull();
  expect(target.querySelector("#widget-key-warning")?.classList.contains("sr-only")).toBe(true);
  save.click();
  expect(document.activeElement).toBe(input);
  expect(mockDispatch).not.toHaveBeenCalled();

  input.value = "configured";
  input.dispatchEvent(new Event("input", { bubbles: true }));
  await tick();
  expect(target.querySelector(".plugin-field-dot")).toBeNull();
  save.click();
  expect(mockDispatch).toHaveBeenCalledOnce();
});

it.each(["textInput", "passwordInput"] as const)("describes an empty %s until a value is entered", async (type) => {
  const target = document.createElement("div");
  document.body.append(target);
  const root: WidgetNode = { type, id: "key", label: "API key", emptyWarning: "Add a key" };
  mockRender.mockResolvedValue({ root });
  components.push(mount(PluginRenderer, { target, props: { pluginId: "fixture", initialRoot: root } }));
  await tick();

  const input = target.querySelector<HTMLInputElement>("#widget-key")!;
  expect(input.getAttribute("aria-describedby")).toBe("widget-key-warning");
  expect(target.querySelector("#widget-key-warning")?.textContent).toBe("Add a key");
  expect(input.classList.contains("plugin-input-warning")).toBe(true);

  input.value = "configured";
  input.dispatchEvent(new Event("input", { bubbles: true }));
  await tick();
  expect(input.hasAttribute("aria-describedby")).toBe(false);
  expect(target.querySelector("#widget-key-warning")).toBeNull();
});

it("renders the localized alternative only between visible row inputs", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const root: WidgetNode = {
    type: "row", separator: "or", children: [
      { type: "textInput", id: "tmdb" },
      { type: "textInput", id: "hidden", hidden: true },
      { type: "textInput", id: "imdb" },
    ],
  };
  mockRender.mockResolvedValue({ root });
  components.push(mount(PluginRenderer, { target, props: { pluginId: "fixture", initialRoot: root } }));
  await tick();
  const children = [...target.querySelector(".plugin-renderer > div")!.children];
  expect(children.map(child => child.querySelector("input")?.id ?? child.textContent?.trim()))
    .toEqual(["widget-tmdb", "or", "widget-imdb"]);
});

it("moves a declared header action into the title bar and keeps focus guidance working", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const root: WidgetNode = {
    type: "column",
    children: [
      { type: "button", id: "settings", label: "Settings", variant: "settings" },
      { type: "button", id: "search", label: "Search", disabled: true, disabledFocus: "settings" },
    ],
  };
  mockRender.mockResolvedValue({ root });
  mockDispatch.mockResolvedValue({});
  components.push(mount(PluginHeaderHarness, { target, props: { root } }));
  await tick();
  const settings = target.querySelector<HTMLButtonElement>("header #widget-settings")!;
  expect(settings).not.toBeNull();
  expect(target.querySelectorAll("#widget-settings")).toHaveLength(1);
  target.querySelector<HTMLButtonElement>("#widget-search")!.click();
  expect(document.activeElement).toBe(settings);
  expect(mockDispatch).not.toHaveBeenCalled();
  settings.click();
  expect(mockDispatch).toHaveBeenCalledOnce();
});

it("filters the language catalog and preserves bilingual names", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const root: WidgetNode = {
    type: "languageSelect", id: "language", label: "Language",
    codes: ["it", "ja", "it", "unknown"], defaultValue: "it", names: "both", showFlags: false,
  };
  mockRender.mockResolvedValue({ root });
  components.push(mount(PluginRenderer, { target, props: { pluginId: "fixture", initialRoot: root } }));
  await tick();
  const input = target.querySelector<HTMLInputElement>('input[aria-label="Language"]')!;
  expect(input.value).toBe("Italian (Italiano)");
  input.focus();
  await tick();
  const menu = document.body.querySelector(".searchable-select-dropdown")!;
  expect([...menu.querySelectorAll("button")].map(option => option.textContent?.trim()))
    .toEqual(["Italian (Italiano)", "Japanese (日本語)"]);
});

it("dispatches a drop only on the opted-in path field and includes edited multiline text", async () => {
  const target = document.body.appendChild(document.createElement("div"));
  const cleanup = vi.fn();
  mockDragDrop.mockReturnValue(cleanup);
  const root: WidgetNode = {
    type: "column",
    children: [
      { type: "textArea", id: "notes", label: "Notes", rows: 5 },
      { type: "pathInput", id: "subtitle", pickerType: "file", drop: true,
        filters: [{ name: "Subtitles", extensions: ["srt"] }] },
    ],
  };
  mockRender.mockResolvedValue({ root });
  mockDispatch.mockResolvedValue({});
  const renderer = mount(PluginRenderer, { target, props: { pluginId: "fixture", initialRoot: root } });
  await tick();
  const textarea = target.querySelector("textarea")!;
  textarea.value = "Line one\nLine two";
  textarea.dispatchEvent(new Event("input", { bubbles: true }));
  expect(textarea.rows).toBe(5);

  const hitTest = vi.fn().mockReturnValue(document.body);
  const original = Object.getOwnPropertyDescriptor(document, "elementFromPoint");
  Object.defineProperty(document, "elementFromPoint", { configurable: true, value: hitTest });
  try {
    const { onDrop } = mockDragDrop.mock.calls[0][0];
    const position = { x: 10, y: 10 };
    onDrop(["/tmp/movie.srt"], position);
    hitTest.mockReturnValue(target.querySelector("[data-plugin-file-drop]"));
    onDrop(["/tmp/movie.mp4"], position);
    expect(mockDispatch).not.toHaveBeenCalled();

    onDrop(["/tmp/movie.SRT"], position);
    expect(mockDispatch).toHaveBeenCalledWith("fixture", expect.objectContaining({
      widgetId: "subtitle",
      eventType: "change",
      values: { subtitle: "/tmp/movie.SRT", notes: "Line one\nLine two" },
    }), expect.any(String), expect.any(Boolean));
  } finally {
    if (original) Object.defineProperty(document, "elementFromPoint", original);
    else Reflect.deleteProperty(document, "elementFromPoint");
    await unmount(renderer);
  }
  expect(cleanup).toHaveBeenCalled();
});

it("applies explicit credential resets alongside a replacement tree without overwriting other fields", async () => {
  const target = document.body.appendChild(document.createElement("div"));
  const root: WidgetNode = {
    type: "column",
    children: [
      { type: "passwordInput", id: "key", label: "API key", defaultValue: "old-key" },
      { type: "textInput", id: "name", label: "Name", defaultValue: "original" },
      { type: "list", id: "keys", items: [{ id: "saved", title: "Saved key",
        actions: [{ id: "remove", label: "Remove", icon: "trash", iconOnly: true }] }] },
    ],
  };
  mockRender.mockResolvedValue({ root });
  mockDispatch.mockResolvedValue({ root, patch: { values: { key: "" } } });
  components.push(mount(PluginRenderer, { target, props: { pluginId: "fixture", initialRoot: root } }));
  await tick();

  const name = target.querySelector<HTMLInputElement>("#widget-name")!;
  name.value = "unsaved edit";
  name.dispatchEvent(new Event("input", { bubbles: true }));
  const remove = target.querySelector<HTMLButtonElement>('button[aria-label="Remove"]')!;
  expect(remove.textContent?.trim()).toBe("");
  remove.click();
  await tick();
  await tick();
  expect(target.querySelector<HTMLInputElement>("#widget-key")?.value).toBe("");
  expect(target.querySelector<HTMLInputElement>("#widget-name")?.value).toBe("unsaved edit");
});

it("renders active plugin:progress indicators for its pluginId and clears them at 100%", async () => {
  const target = document.body.appendChild(document.createElement("div"));
  const root: WidgetNode = {
    type: "column",
    children: [{ type: "text", text: "Plugin body" }],
  };

  pluginsStore.updateProgress({
    pluginId: "test-plugin",
    id: "sync-job",
    value: 35,
    label: "Fetching subtitles",
  });
  pluginsStore.updateProgress({
    pluginId: "other-plugin",
    id: "other-job",
    value: 80,
    label: "Should not appear",
  });

  components.push(
    mount(PluginRenderer, {
      target,
      props: { pluginId: "test-plugin", initialRoot: root },
    })
  );
  await tick();

  const bar = target.querySelector<HTMLElement>('[data-plugin-progress="sync-job"]');
  expect(bar).not.toBeNull();
  expect(bar?.getAttribute("aria-valuenow")).toBe("35");
  expect(bar?.textContent).toContain("Fetching subtitles");
  expect(bar?.textContent).toContain("35%");
  expect(target.textContent).not.toContain("Should not appear");

  pluginsStore.updateProgress({
    pluginId: "test-plugin",
    id: "sync-job",
    value: 72,
    label: "Parsing lines",
  });
  await tick();
  expect(bar?.getAttribute("aria-valuenow")).toBe("72");
  expect(bar?.textContent).toContain("Parsing lines");
  expect(bar?.textContent).toContain("72%");

  pluginsStore.updateProgress({
    pluginId: "test-plugin",
    id: "sync-job",
    value: 100,
    label: "Done",
  });
  await tick();
  expect(target.querySelector('[data-testid="plugin-active-progress"]')).toBeNull();
});

it("renders widget type select using SearchableSelect and updates value on option pick", async () => {
  const schema: WidgetNode = {
    type: "column",
    children: [
      {
        type: "select",
        id: "quality",
        label: "Video Quality",
        options: [
          { label: "1080p Full HD", value: "1080" },
          { label: "720p HD", value: "720" },
        ],
        defaultValue: "1080",
      },
      { type: "button", id: "apply", label: "Apply", variant: "primary" },
    ],
  };

  mockDispatch.mockResolvedValue({ root: schema });

  const target = document.createElement("div");
  document.body.append(target);

  components.push(
    mount(PluginRenderer, {
      target,
      props: {
        pluginId: "test-plugin",
        screen: "main",
        active: true,
        initialRoot: schema,
      },
    })
  );
  await tick();

  expect(target.querySelector("#widget-quality select")).toBeNull();
  const combo = target.querySelector<HTMLElement>("#widget-quality [role='combobox']")!;
  expect(combo).not.toBeNull();
  combo.focus();
  await tick();

  const option720 = Array.from(target.querySelectorAll<HTMLElement>('[role="option"]')).find((el) =>
    el.textContent?.includes("720p HD")
  );
  expect(option720).toBeDefined();
  option720!.click();
  await tick();

  await vi.waitFor(() => {
    expect(mockDispatch).toHaveBeenCalledWith(
      "test-plugin",
      expect.objectContaining({
        widgetId: "quality",
        eventType: "change",
        values: expect.objectContaining({ quality: "720" }),
      }),
      expect.any(String),
      expect.any(Boolean)
    );
  });

  await Promise.resolve();
  await tick();
  target.querySelector<HTMLButtonElement>("#widget-apply")!.click();
  await tick();

  await vi.waitFor(() => {
    expect(mockDispatch).toHaveBeenCalledWith(
      "test-plugin",
      expect.objectContaining({
        widgetId: "apply",
        eventType: "click",
        values: expect.objectContaining({ quality: "720" }),
      }),
      expect.any(String),
      expect.any(Boolean)
    );
  });
});

it("renders card, pagination, and dispatches checkbox change events", async () => {
  const schema: WidgetNode = {
    type: "card",
    title: "Results",
    subtitle: "Showing page 1",
    icon: "film",
    accent: "cyan",
    children: [
      {
        type: "checkbox",
        id: "only_hd",
        label: "HD Only",
        defaultValue: false,
      },
      {
        type: "pagination",
        id: "pager",
        page: 1,
        totalPages: 3,
        totalItems: 42,
      },
      {
        type: "button",
        id: "btn_done",
        label: "Done",
        variant: "success",
      },
    ],
  };

  mockDispatch.mockResolvedValue({});

  const target = document.body.appendChild(document.createElement("div"));
  components.push(
    mount(PluginRenderer, {
      target,
      props: {
        pluginId: "test-plugin",
        initialRoot: schema,
      },
    })
  );
  await tick();

  const card = target.querySelector(".glass-card");
  expect(card).not.toBeNull();
  expect(card?.querySelector("h3")?.className).toContain("text-cyan-400");
  expect(card?.textContent).toContain("Results");
  expect(card?.textContent).toContain("Showing page 1");
  expect(target.querySelector("#widget-btn_done")?.className).toContain("plugin-action-success");

  const checkbox = target.querySelector<HTMLButtonElement>('[role="checkbox"]')!;
  checkbox.click();
  await vi.waitFor(() => {
    expect(mockDispatch).toHaveBeenCalledWith(
      "test-plugin",
      expect.objectContaining({
        widgetId: "only_hd",
        eventType: "change",
        values: expect.objectContaining({ only_hd: true }),
      }),
      expect.any(String),
      expect.any(Boolean)
    );
  });

  await Promise.resolve();
  await tick();
  mockDispatch.mockClear();

  const pager = target.querySelector('[data-plugin-pagination="pager"]')!;
  expect(pager.textContent).toContain("Page 1 / 3");
  expect(pager.textContent).toContain("(42)");

  const prevBtn = pager.querySelector<HTMLButtonElement>('button[aria-label="Prev"]')!;
  const nextBtn = pager.querySelector<HTMLButtonElement>('button[aria-label="Next"]')!;
  expect(prevBtn.disabled).toBe(true);
  expect(nextBtn.disabled).toBe(false);

  nextBtn.click();
  await vi.waitFor(() => {
    expect(mockDispatch).toHaveBeenCalledWith(
      "test-plugin",
      expect.objectContaining({
        widgetId: "pager",
        eventType: "change",
        values: expect.objectContaining({ pager: 2 }),
      }),
      expect.any(String),
      expect.any(Boolean)
    );
  });
});

it("renders mediaPlayer for remote URLs, local media server paths, audio, and error fallbacks", async () => {
  vi.mocked(invokeCommand).mockImplementation(async (cmd: string) => {
    if (cmd === "get_media_server_info") return [43210, "tok&1"];
    return undefined;
  });

  const schema: WidgetNode = {
    type: "column",
    children: [
      {
        type: "mediaPlayer",
        id: "remote_video",
        title: "Remote Trailer",
        caption: "HTTPS Stream",
        src: "https://cdn.example.com/trailer.mp4",
        poster: "https://cdn.example.com/poster.jpg",
        loop: true,
        muted: true,
        height: 320,
      },
      {
        type: "mediaPlayer",
        id: "local_audio",
        kind: "audio",
        title: "Local Track",
        src: "/tmp/audio clip.mp3",
      },
    ],
  };

  const target = document.body.appendChild(document.createElement("div"));
  components.push(
    mount(PluginRenderer, {
      target,
      props: {
        pluginId: "test-plugin",
        initialRoot: schema,
      },
    })
  );
  await tick();

  const video = target.querySelector("video")!;
  expect(video).not.toBeNull();
  expect(video.getAttribute("src")).toBe("https://cdn.example.com/trailer.mp4");
  expect(video.getAttribute("poster")).toBe("https://cdn.example.com/poster.jpg");
  expect(video.loop).toBe(true);
  expect(video.muted).toBe(true);
  expect(target.textContent).toContain("Remote Trailer");
  expect(target.textContent).toContain("HTTPS Stream");

  await vi.waitFor(() => {
    const audio = target.querySelector("audio");
    expect(audio?.getAttribute("src")).toBe(
      "http://127.0.0.1:43210/media?path=%2Ftmp%2Faudio%20clip.mp3&token=tok%261"
    );
  });

  video.dispatchEvent(new Event("error"));
  await tick();
  expect(target.textContent).toContain("Playback failed");
});

it("renders response footer in FooterActions instead of fallbackFooter and handles scrollTop", async () => {
  mockRender.mockResolvedValueOnce({
    root: {
      type: "column",
      children: [{ type: "button", id: "go_page_2", label: "Next View" }],
    },
    footer: {
      left: [{ type: "pathInput", id: "out_dir", compact: true, clearable: false, defaultDir: "downloads" }],
      right: [{ type: "button", id: "footer_ok", label: "Apply", variant: "primary" }],
    },
  });
  mockDispatch.mockResolvedValueOnce({
    root: {
      type: "column",
      children: [{ type: "text", text: "Page 2" }],
    },
    scrollTop: true,
  });

  const fallbackFooter = createRawSnippet(() => ({
    render: () => `<span data-fallback-footer>Fallback Footer</span>`,
  }));

  const target = document.body.appendChild(document.createElement("div"));
  components.push(
    mount(PluginRenderer, {
      target,
      props: {
        pluginId: "test-plugin",
        fallbackFooter,
      },
    })
  );

  await vi.waitFor(() => {
    expect(target.querySelector("[data-footer-actions]")).not.toBeNull();
    expect(target.querySelector("#widget-footer_ok")).not.toBeNull();
  });

  expect(target.querySelector("[data-fallback-footer]")).toBeNull();
  await vi.waitFor(() => {
    expect(target.querySelector("[data-footer-actions]")?.textContent).toContain("/home/user/Downloads");
  });

  const scrollEl = target.querySelector<HTMLElement>(".plugin-renderer")!;
  const scrollSpy = vi.fn();
  scrollEl.scrollTo = scrollSpy;

  target.querySelector<HTMLButtonElement>("#widget-go_page_2")!.click();
  await vi.waitFor(() => {
    expect(scrollSpy).toHaveBeenCalledWith({ top: 0, behavior: "smooth" });
    expect(target.querySelector("[data-fallback-footer]")).not.toBeNull();
  });
});

it("supports pathInput clearable: false, PathPreviewModal editing, and image/list fallbacks", async () => {
  const schema: WidgetNode = {
    type: "column",
    children: [
      {
        type: "pathInput",
        id: "output_dir",
        label: "Output Folder",
        defaultValue: "/tmp/out",
        clearable: false,
      },
      {
        type: "image",
        src: "https://example.com/broken.jpg",
        aspect: "poster",
        fit: "cover",
      },
      {
        type: "list",
        layout: "grid",
        imageAspect: "wide",
        items: [
          {
            id: "item-1",
            title: "Grid Item",
            image: "https://example.com/thumb.jpg",
          },
        ],
      },
    ],
  };

  mockDispatch.mockResolvedValue({});

  const target = document.body.appendChild(document.createElement("div"));
  components.push(
    mount(PluginRenderer, {
      target,
      props: {
        pluginId: "test-plugin",
        initialRoot: schema,
      },
    })
  );
  await tick();

  // clearable: false hides Clear field
  expect(target.textContent).not.toContain("Clear field");
  expect(target.querySelector('button[aria-label="Clear field"]')).toBeNull();

  // Clicking the path display button opens editable PathPreviewModal
  const pathDisplayBtn = Array.from(target.querySelectorAll("button")).find((b) =>
    b.textContent?.includes("/tmp/out")
  )!;
  expect(pathDisplayBtn.disabled).toBe(false);
  pathDisplayBtn.click();
  await tick();

  const dialog = target.querySelector('[role="dialog"]')!;
  expect(dialog).not.toBeNull();
  const editBtn = Array.from(dialog.querySelectorAll("button")).find((b) =>
    b.textContent?.includes("Edit")
  )!;
  editBtn.click();
  await tick();

  const modalInput = dialog.querySelector<HTMLInputElement>("input[type='text']")!;
  modalInput.value = "/tmp/edited-out";
  modalInput.dispatchEvent(new Event("input", { bubbles: true }));
  await tick();

  const saveBtn = Array.from(dialog.querySelectorAll("button")).find((b) =>
    b.textContent?.trim() === "Save"
  )!;
  saveBtn.click();

  await vi.waitFor(() => {
    expect(mockDispatch).toHaveBeenCalledWith(
      "test-plugin",
      expect.objectContaining({
        widgetId: "output_dir",
        eventType: "change",
        values: expect.objectContaining({ output_dir: "/tmp/edited-out" }),
      }),
      expect.any(String),
      expect.any(Boolean)
    );
  });

  // Image and list grid attributes + error fallback
  const img = target.querySelector<HTMLImageElement>('img[src="https://example.com/broken.jpg"]')!;
  expect(img.getAttribute("referrerpolicy")).toBe("no-referrer");
  expect(img.className).toContain("aspect-[2/3]");
  expect(img.className).toContain("object-cover");
  img.dispatchEvent(new Event("error"));
  await tick();
  expect(target.querySelector('img[src="https://example.com/broken.jpg"]')).toBeNull();

  const grid = target.querySelector(".grid");
  expect(grid).not.toBeNull();
  const thumb = target.querySelector<HTMLImageElement>('img[src="https://example.com/thumb.jpg"]')!;
  expect(thumb.getAttribute("referrerpolicy")).toBe("no-referrer");
  thumb.dispatchEvent(new Event("error"));
  await tick();
  expect(target.querySelector('img[src="https://example.com/thumb.jpg"]')).toBeNull();
});

it("toggles password visibility on passwordInput and settings password fields", async () => {
  const target = document.createElement("div");
  document.body.append(target);
  const root: WidgetNode = {
    type: "column",
    children: [
      { type: "passwordInput", id: "api_key", label: "API Key", defaultValue: "secret-123" },
      {
        type: "settings",
        fields: [
          { id: "user_pass", label: "Password", type: "password", defaultValue: "pw-456" },
        ],
      },
    ],
  };
  components.push(mount(PluginRenderer, { target, props: { pluginId: "fixture", initialRoot: root } }));
  await tick();

  const keyInput = target.querySelector<HTMLInputElement>("#widget-api_key")!;
  const settingInput = target.querySelector<HTMLInputElement>("#setting-user_pass")!;
  const toggleButtons = Array.from(target.querySelectorAll<HTMLButtonElement>("button[aria-pressed]"));
  expect(toggleButtons).toHaveLength(2);

  expect(keyInput.type).toBe("password");
  expect(toggleButtons[0].getAttribute("aria-pressed")).toBe("false");
  toggleButtons[0].click();
  await tick();
  expect(keyInput.type).toBe("text");
  expect(toggleButtons[0].getAttribute("aria-pressed")).toBe("true");
  toggleButtons[0].click();
  await tick();
  expect(keyInput.type).toBe("password");

  expect(settingInput.type).toBe("password");
  toggleButtons[1].click();
  await tick();
  expect(settingInput.type).toBe("text");
});
