// @vitest-environment jsdom
import { mount, tick, unmount } from "svelte";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import PluginRenderer from "./PluginRenderer.svelte";
import type { WidgetNode } from "$lib/services/pluginClient";

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
    expect.any(String)
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
