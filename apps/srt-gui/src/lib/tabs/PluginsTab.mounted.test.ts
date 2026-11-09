// @vitest-environment jsdom
import { mount, tick, unmount } from "svelte";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import PluginsTab from "./PluginsTab.svelte";
import { pluginsStore } from "$lib/stores/pluginsStore.svelte";
import { guardedOpen } from "$lib/utils/dialogGuard";

vi.mock("$lib/utils/dialogGuard", () => ({
  guardedOpen: vi.fn(),
  guardedSave: vi.fn(),
}));

vi.mock("$lib/services/pluginClient", async () => {
  const actual = await vi.importActual<any>("$lib/services/pluginClient");
  return {
    ...actual,
    pluginList: vi.fn().mockResolvedValue([]),
    pluginInspect: vi.fn(),
    pluginInstall: vi.fn(),
    pluginEnable: vi.fn(),
    pluginDisable: vi.fn(),
    pluginUninstall: vi.fn(),
    pluginUpdate: vi.fn(),
    pluginRender: vi.fn().mockResolvedValue({
      root: { type: "text", text: "Plugin Content Loaded" },
    }),
    pluginDispatchEvent: vi.fn().mockResolvedValue({}),
    pluginSaveSettings: vi.fn().mockResolvedValue(undefined),
  };
});

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(),
}));

const components: ReturnType<typeof mount>[] = [];

beforeEach(() => {
  vi.clearAllMocks();
  pluginsStore.plugins = [
    {
      id: "demo-plugin",
      name: "Demo Plugin",
      version: "1.2.0",
      description: "A test demo plugin",
      author: "Test Author",
      enabled: true,
      sourceUrl: "/home/user/plugins/demo-plugin",
      installedAt: "2026-10-31T12:00:00Z",
      updatedAt: "2026-10-31T13:00:00Z",
      readme: "# Demo Readme\nDetailed information about Demo Plugin.",
      manifest: {
        id: "demo-plugin",
        name: "Demo Plugin",
        version: "1.2.0",
        apiVersion: 1,
        description: "A test demo plugin",
        author: "Test Author",
        homepage: "https://example.com/demo",
        repository: "https://github.com/example/demo",
        readme: "# Demo Readme\nDetailed information about Demo Plugin.",
        entry: { command: ["python3", "main.py"] },
        permissions: ["network", "filesystem:downloads"],
        sharedCredentials: [{ id: "tmdb_api_key", label: "TMDB API Key" }],
        tab: { title: "Demo Tab", icon: "film" },
        settings: [
          {
            id: "apiKey",
            label: "API Key",
            type: "password",
            description: "Secret API key",
          },
        ],
      },
      settings: { apiKey: "secret" },
    },
  ];
  pluginsStore.initialized = true;
  pluginsStore.loading = false;
});

afterEach(async () => {
  for (const component of components.splice(0)) await unmount(component);
  document.body.replaceChildren();
});

it("renders install section with Browse button, integrated search, and aligned 92px footer", async () => {
  const target = document.createElement("div");
  document.body.appendChild(target);

  const component = mount(PluginsTab, {
    target,
    props: { active: true },
  });
  components.push(component);
  await tick();

  // Install section title and segmented toggle
  expect(target.textContent).toContain("Install from Repository or Local Folder");
  expect(target.textContent).toContain("Repository URL");
  expect(target.textContent).toContain("Local Folder");

  // Switch to Local Folder mode
  const folderModeBtn = Array.from(target.querySelectorAll("button")).find(b => b.textContent?.trim() === "Local Folder");
  expect(folderModeBtn).not.toBeNull();
  folderModeBtn!.click();
  await tick();

  // Browse folder button is present in PathPickerField
  const browseBtn = target.querySelector('button[aria-label="Browse…"]') as HTMLButtonElement;
  expect(browseBtn).not.toBeNull();

  // Test Browse folder interaction
  vi.mocked(guardedOpen).mockResolvedValueOnce("/path/to/local/plugin");
  browseBtn.click();
  await Promise.resolve();
  await tick();
  expect(guardedOpen).toHaveBeenCalledWith({ directory: true, multiple: false });

  // PathPickerField should display selected folder path
  expect(target.textContent).toContain("/path/to/local/plugin");

  // Search input is integrated in the installed plugins section
  const searchInput = target.querySelector('input[placeholder="Filter installed plugins…"]');
  expect(searchInput).not.toBeNull();

  // Footer bar has exact h-[92px] and bg-gray-900
  const footer = target.querySelector(".footer-band");
  expect(footer).not.toBeNull();
  expect(footer?.className).toContain("h-[92px]");
  expect(footer?.className).toContain("bg-gray-900");
  expect(footer?.className).toContain("border-t");
});

it("displays plugin card with Open, Info, Settings, Update, Enable/Disable, and Danger Uninstall buttons", async () => {
  const target = document.createElement("div");
  document.body.appendChild(target);

  const component = mount(PluginsTab, {
    target,
    props: { active: true },
  });
  components.push(component);
  await tick();

  expect(target.textContent).toContain("Demo Plugin");
  expect(target.textContent).toContain("v1.2.0");

  // Open button is primary
  const openBtn = target.querySelector(".btn-primary.text-xs") as HTMLButtonElement;
  expect(openBtn).not.toBeNull();
  expect(openBtn.textContent).toContain("Open");

  // Info icon button is present
  const infoBtn = target.querySelector('button[title="Plugin Details"]') as HTMLButtonElement;
  expect(infoBtn).not.toBeNull();

  // Settings icon button is present
  const settingsBtn = target.querySelector('button[title="Settings"]') as HTMLButtonElement;
  expect(settingsBtn).not.toBeNull();

  // Update button is present
  const updateBtn = Array.from(target.querySelectorAll("button")).find((b) => b.textContent?.trim() === "Update");
  expect(updateBtn).not.toBeNull();

  // Disable button is neutral secondary
  const disableBtn = Array.from(target.querySelectorAll("button")).find((b) => b.textContent?.trim() === "Disable");
  expect(disableBtn).not.toBeNull();
  expect(disableBtn?.className).toContain("text-amber-400");
  expect(disableBtn?.className).toContain("text-amber-400");

  // Uninstall button is styled in red/danger
  const uninstallBtn = target.querySelector('button[title="Uninstall plugin"]') as HTMLButtonElement;
  expect(uninstallBtn).not.toBeNull();
  expect(uninstallBtn.className).toContain("text-rose-400");
});

it("only allows reopening or uninstalling a disabled plugin and opens active cards from their header", async () => {
  pluginsStore.plugins[0].enabled = false;
  const target = document.body.appendChild(document.createElement("div"));
  const onGoToTab = vi.fn();
  components.push(mount(PluginsTab, { target, props: { active: true, onGoToTab } }));
  await tick();

  const card = target.querySelector('[role="group"][aria-label="Demo Plugin"]')!;
  const button = (title: string) => card.querySelector<HTMLButtonElement>(`button[title="${title}"]`)!;
  for (const title of ["Update", "Plugin Details", "Settings"]) expect(button(title).disabled).toBe(true);
  expect(button("Enable").disabled).toBe(false);
  expect(button("Uninstall plugin").disabled).toBe(false);
  expect(card.querySelector('[aria-label="Open · Demo Plugin"]')).toBeNull();

  pluginsStore.plugins[0].enabled = true;
  await tick();
  card.querySelector<HTMLButtonElement>('[aria-label="Open · Demo Plugin"]')!.click();
  expect(onGoToTab).toHaveBeenCalledWith("plugin:demo-plugin");
});

it("opens Info modal with metadata, README markdown, and requested permissions", async () => {
  const target = document.createElement("div");
  document.body.appendChild(target);

  const component = mount(PluginsTab, {
    target,
    props: { active: true },
  });
  components.push(component);
  await tick();

  const infoBtn = target.querySelector('button[title="Plugin Details"]') as HTMLButtonElement;
  infoBtn.click();
  await tick();

  // Info modal should be visible
  expect(target.textContent).toContain("Demo Plugin");
  expect(target.textContent).toContain("Plugin Details");
  expect(target.textContent).toContain("Test Author");
  expect(target.textContent).toContain("/home/user/plugins/demo-plugin");
  expect(target.textContent).toContain("https://example.com/demo");
  expect(target.textContent).toContain("https://github.com/example/demo");
  expect(target.textContent).toContain("Network Access");
  expect(target.textContent).toContain("Downloads Folder Access");
  expect(target.textContent).toContain("Shared credential · tmdb_api_key (TMDB API Key)");
  expect(target.textContent).toContain("Detailed information about Demo Plugin.");
});

it("switches to embedded view with Back button and header bar when opened", async () => {
  const target = document.createElement("div");
  document.body.appendChild(target);

  let navigatedTab: string | null = null;
  const component = mount(PluginsTab, {
    target,
    props: {
      active: true,
      onGoToTab: (t: string) => { navigatedTab = t; },
    },
  });
  components.push(component);
  await tick();

  const openBtn = target.querySelector(".btn-primary.text-xs") as HTMLButtonElement;
  openBtn.click();
  await tick();

  expect(navigatedTab).toBe("plugin:demo-plugin");

  // Top header bar inside PluginsTab
  const backBtn = target.querySelector('button[aria-label="Back to Plugins"]') as HTMLButtonElement;
  expect(backBtn).not.toBeNull();
  expect(target.textContent).toContain("Back to Plugins");
  expect(target.textContent).toContain("Demo Plugin");

  // Clicking Back returns to list
  backBtn.click();
  await tick();
  expect(navigatedTab).toBe("plugins");
});

it("renders custom image icon when iconUrl is present", async () => {
  const target = document.createElement("div");
  document.body.appendChild(target);

  pluginsStore.plugins = [
    {
      id: "icon-plugin",
      name: "Custom Icon Plugin",
      version: "1.0.0",
      description: "Plugin with custom icon",
      enabled: true,
      sourceUrl: "/path/to/icon-plugin",
      installedAt: "2026-10-31T12:00:00Z",
      iconUrl: "data:image/svg+xml;base64,PHN2Zz48L3N2Zz4=",
      manifest: {
        id: "icon-plugin",
        name: "Custom Icon Plugin",
        version: "1.0.0",
        apiVersion: 1,
        entry: { command: ["python3", "main.py"] },
        permissions: [],
      },
      settings: {},
    },
  ];

  const component = mount(PluginsTab, {
    target,
    props: { active: true },
  });
  components.push(component);
  await tick();

  const iconImg = target.querySelector('img[src="data:image/svg+xml;base64,PHN2Zz48L3N2Zz4="]') as HTMLImageElement;
  expect(iconImg).not.toBeNull();
});
