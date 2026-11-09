// @vitest-environment jsdom
import { mount, tick, unmount } from "svelte";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import PluginRenderer from "./PluginRenderer.svelte";
import type { WidgetNode } from "$lib/services/pluginClient";
import { uiMode } from "$lib/stores/uiModeStore.svelte";

const mockDispatch = vi.fn();
const mockRender = vi.fn();

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
  expect(search.title).toBe("Save your key");
  expect(search.getAttribute("aria-disabled")).toBe("true");
  search.click();
  expect(document.activeElement).toBe(settings);
  expect(mockDispatch).not.toHaveBeenCalled();

  renderer.applyPatch({ setProps: { search: { disabled: false } } });
  await tick();
  expect(search.title).toBe("Enter an ID");
  input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
  expect(mockDispatch).not.toHaveBeenCalled();
  input.value = "123";
  input.dispatchEvent(new Event("input", { bubbles: true }));
  await tick();
  expect(search.hasAttribute("aria-disabled")).toBe(false);
  input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
  expect(mockDispatch).toHaveBeenCalledOnce();
});
