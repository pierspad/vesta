import { expect, test } from "./browser-fixture.mjs";

const callsFor = (page, command) =>
  page.evaluate(
    (name) => window.browserFixture.calls.filter((call) => call.command === name),
    command,
  );

test("Diagnostics UI, Support Logs, and AlignTab parse/save work end-to-end", async ({ page }) => {
  await page.goto("/browser-tests/harness.html");
  await expect(page.getByRole("navigation")).toBeVisible();

  // Open Settings -> Overview shows MCP card (hidden when AI Kill Switch is active) -> Diagnostics section
  await page.locator("aside > div button").first().click();
  const mcpCard = page.locator("#settings-mcp-card");
  await expect(mcpCard).toBeVisible();
  await expect(mcpCard).toContainText("/usr/bin/vesta-cli");
  await expect(mcpCard).toContainText('"mcp"');

  const killSwitch = page.getByRole("switch", { name: "AI Kill Switch", exact: true });
  await killSwitch.click();
  await expect(mcpCard).toBeHidden();
  await killSwitch.click();
  await expect(mcpCard).toBeVisible();

  await page.getByRole("button", { name: "Diagnostics" }).click();

  await expect.poll(() => callsFor(page, "get_system_diagnostics")).toHaveLength(1);
  await expect(page.getByText("linux · x86_64")).toBeVisible();

  const startLogsBtn = page.getByRole("button", { name: "Record", exact: true });
  await startLogsBtn.click();
  await expect.poll(() => callsFor(page, "support_log_start")).toHaveLength(1);

  const stopLogsBtn = page.getByRole("button", { name: "Stop recording", exact: true });
  await stopLogsBtn.click();
  await expect.poll(() => callsFor(page, "support_log_stop")).toHaveLength(1);

  // Exit Settings back to main sidebar navigation -> Revise (AlignTab) -> load + save via Rust parse/save IPC
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Revise", exact: true }).click();
  const pickers = page.getByRole("button", { name: "Open SRT", exact: true });
  await page.evaluate(() => window.browserFixture.selectFile("/fixtures/Episode01.ja.srt"));
  await pickers.first().click();
  await page.evaluate(() => window.browserFixture.selectFile("/fixtures/reference.srt"));
  await pickers.last().click();

  await expect.poll(() => callsFor(page, "parse_subtitle_file")).toHaveLength(2);
  await page.evaluate(() => window.browserFixture.selectFile("/exports/Episode01_aligned.srt"));
  await page.getByRole("button", { name: /Save/i }).first().click();
  await expect.poll(() => callsFor(page, "save_subtitle_file")).toHaveLength(1);
});

test("All registered IPC commands accept documented payloads and pass browser contract checks", async ({
  page,
}) => {
  await page.goto("/browser-tests/harness.html");
  await expect(page.getByRole("navigation")).toBeVisible();

  const results = await page.evaluate(async () => {
    const inv = (command, args = {}) => window.__TAURI_INTERNALS__.invoke(command, args);
    const out = {};

    const snapshot = await inv("config_load_all");
    out.configLoaded = Boolean(snapshot?.entries);
    await inv("config_remove", { key: "vesta-ephemeral-test-key" });
    await inv("config_replace_all", { values: snapshot.entries });

    out.mediaInfo = await inv("get_media_server_info");
    out.mcpCliInfo = await inv("get_mcp_cli_info");
    out.diagnostics = await inv("get_system_diagnostics");
    out.supportStatus = await inv("support_log_status");
    out.supportAppend = await inv("support_log_append", {
      path: "/exports/vesta-support.jsonl",
      entries: [{ ts: 1, kind: "test", message: "coverage check" }],
    });
    out.supportExport = await inv("support_log_export", {
      destination: "/exports/vesta-support-export.jsonl",
    });

    out.httpFetch = await inv("http_fetch", {
      req: { url: "https://example.invalid/catalog.json", method: "GET" },
    });
    out.updateInstall = await inv("install_release_update", { version: "0.34.0" });
    out.openPath = await inv("open_output_path", { path: "/exports/deck.apkg", folder: true });

    out.rawSrt = await inv("read_subtitle_file", { path: "/fixtures/Episode01.ja.srt" });
    out.parsedSrt = await inv("parse_subtitle_file", { path: "/fixtures/Episode01.ja.srt" });
    await inv("save_subtitle_file", {
      path: "/exports/saved.srt",
      entries: out.parsedSrt,
    });

    out.tracks = await inv("embedded_subtitle_tracks", { path: "/fixtures/movie.mkv" });
    out.existingOutputs = await inv("existing_subtitle_outputs", {
      directory: "/exports",
      names: ["Episode01.ja.srt"],
    });
    out.ocrEmbeddedPreview = await inv("preview_ocr_embedded_subtitle", {
      path: "/fixtures/movie.mkv",
      index: 0,
      tiers: [],
      language: "en",
    });
    out.ocrHardcodedPreview = await inv("preview_ocr_hardcoded_subtitle", {
      path: "/fixtures/movie.mkv",
      roi: { x: 0.1, y: 0.7, width: 0.8, height: 0.2 },
      tiers: [],
      sampleFps: 2,
      language: "en",
    });
    out.ocrModels = await inv("ocr_list_models");
    out.ocrCancel = await inv("ocr_cancel");
    out.ocrUninstall = await inv("ocr_uninstall_model", { modelId: "ppocr-en" });

    out.translateInfo = await inv("load_srt_for_translate", { path: "/fixtures/Episode01.ja.srt" });
    out.latestPairs = await inv("get_latest_translated_subtitles", {
      inputPath: "/fixtures/Episode01.ja.srt",
      outputPath: "/exports/translated.srt",
      count: 5,
    });

    out.suggestMedia = await inv("sync_suggest_media_for_srt", {
      srtPath: "/fixtures/Episode01.ja.srt",
    });
    out.suggestCompanion = await inv("sync_suggest_companion_subtitle_for_srt", {
      srtPath: "/fixtures/Episode01.ja.srt",
    });
    out.suggestSubs = await inv("sync_suggest_subtitles_for_media", {
      mediaPath: "/fixtures/movie.mkv",
      defaultTargetLang: "ja",
      defaultNativeLang: "en",
    });
    out.syncStatus = await inv("sync_get_status");
    out.syncRange = await inv("sync_get_subtitles_range", { startId: 1, count: 10 });
    out.syncOne = await inv("sync_get_subtitle", { id: 1 });
    out.syncAnchors = await inv("sync_get_anchors");
    out.syncPlayback = await inv("sync_prepare_media_for_playback", { path: "/fixtures/movie.mkv" });
    out.syncCancelAuto = await inv("sync_cancel_auto_sync");

    out.fcParsed = await inv("flashcard_parse_subtitles", { path: "/fixtures/Episode01.ja.srt" });
    out.fcMerged = await inv("flashcard_merge_apkg", {
      apkgPaths: ["/exports/ep1.apkg", "/exports/ep2.apkg"],
      outputPath: "/exports/merged.apkg",
    });
    out.fcTracks = await inv("flashcard_list_audio_tracks", { path: "/fixtures/movie.mkv" });
    out.fcDeps = await inv("flashcard_check_deps");
    out.fcFfmpeg = await inv("flashcard_download_ffmpeg");
    out.mediaSetup = await inv("setup_media_status");
    out.fcDirExists = await inv("flashcard_check_dir_exists", { path: "/exports" });
    out.cpuCount = await inv("flashcard_get_cpu_count");
    out.ramMb = await inv("flashcard_get_total_memory_mb");
    out.fonts = await inv("flashcard_list_fonts");

    out.trBackends = await inv("transcribe_check_backends");
    out.trModels = await inv("transcribe_list_models");
    out.trUninstallModel = await inv("transcribe_uninstall_model", { modelId: "base" });
    out.trAddons = await inv("transcribe_addons_status");
    out.trDlVad = await inv("transcribe_download_vad", { modelId: "v6.2.0" });
    out.trRmVad = await inv("transcribe_uninstall_vad", { modelId: "v6.2.0" });
    out.trPathExists = await inv("transcribe_path_exists", { path: "/fixtures/movie.mkv" });
    out.trFileExists = await inv("transcribe_check_file_exists", {
      path: "/fixtures/Episode01.ja.srt",
    });

    out.refineCancel = await inv("refine_cancel");
    out.ankiDecks = await inv("ankiconnect_deck_names", { url: "http://127.0.0.1:8765" });
    out.ankiImport = await inv("ankiconnect_import_package", {
      path: "/exports/deck.apkg",
      url: "http://127.0.0.1:8765",
    });

    out.inspected = await inv("plugin_inspect", {
      url: "https://example.invalid/demo-plugin.git",
      gitRef: "main",
    });
    const installedPlugin = await inv("plugin_install", {
      url: "https://example.invalid/demo-plugin.git",
      gitRef: "main",
    });
    const pluginId = installedPlugin.id;
    out.pluginList = await inv("plugin_list");
    await inv("plugin_disable", { id: pluginId });
    await inv("plugin_enable", { id: pluginId });
    await inv("plugin_save_settings", { id: pluginId, settings: { limit: 10 } });
    out.pluginSettings = await inv("plugin_get_settings", { id: pluginId });
    out.pluginRendered = await inv("plugin_render", {
      pluginId,
      screen: "main",
      locale: "en",
      expertMode: false,
    });
    out.pluginUpdated = await inv("plugin_update", { id: pluginId, releaseTag: "v1.0.0" });
    await inv("plugin_uninstall", { id: pluginId });

    return out;
  });

  expect(results.configLoaded).toBe(true);
  expect(results.mediaInfo).toEqual([12345, "fixture"]);
  expect(results.mcpCliInfo).toEqual({
    command: "/usr/bin/vesta-cli",
    bundled: true,
    default_read_root: "/fixtures",
    default_output_root: "/exports",
  });
  expect(results.cpuCount).toBe(4);
  expect(results.ramMb).toBe(16384);
  expect(results.pluginSettings).toEqual({ limit: 10 });
  expect(results.pluginRendered?.root).toBeTruthy();
});

test("media server token handling (MS-01) and http_fetch proxy SSRF error surface cleanly (MS-02)", async ({ page }) => {
  const pageErrors = [];
  page.on("pageerror", (err) => pageErrors.push(err.message));

  const seenMediaRequests = [];
  await page.route("http://127.0.0.1:12345/media**", async (route) => {
    seenMediaRequests.push(route.request().url());
    await route.fulfill({
      status: 403,
      contentType: "text/plain",
      body: "Forbidden",
      headers: { "access-control-allow-origin": "*" },
    });
  });

  await page.goto("/browser-tests/harness.html");
  await page.getByRole("button", { name: "Upload media" }).click();
  await expect.poll(() => callsFor(page, "flashcard_preview_snapshot").then((c) => c.length)).toBeGreaterThan(0);

  await page.getByRole("button", { name: /^Play$/ }).click();
  await expect.poll(() => seenMediaRequests.some((url) => url.includes("token=fixture"))).toBe(true);
  await expect(page.getByRole("alert").first()).toBeVisible();

  await page.locator("aside > div button").first().click();
  const checkNowBtn = page.getByRole("button", { name: "Check now", exact: true });
  await expect(checkNowBtn).toBeVisible();
  await checkNowBtn.click();
  await expect.poll(() => callsFor(page, "http_fetch").then((c) => c.length)).toBeGreaterThan(0);
  await expect(page.getByRole("status").filter({ hasText: /v0\.34\.0|up to date/i }).first()).toBeVisible();

  await page.evaluate(() => window.browserFixture.failHttpFetch("Blocked SSRF target"));
  await checkNowBtn.click();
  await expect(page.getByRole("status").filter({ hasText: /Failed|error|Could not/i }).first()).toBeVisible();
  expect(pageErrors).toEqual([]);
});
