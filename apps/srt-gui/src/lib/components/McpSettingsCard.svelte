<script lang="ts">
  import { onMount } from "svelte";
  import { locale } from "$lib/i18n";
  import * as vestaConfig from "$lib/config/vestaConfig";
  import { invokeCommand } from "$lib/services/tauriClient";
  import { writeClipboard } from "$lib/utils/clipboard";
  import { guardedOpen } from "$lib/utils/dialogGuard";

  interface McpCliInfo {
    command: string;
    bundled: boolean;
    default_read_root: string;
    default_output_root: string;
  }

  const MCP_READ_ROOT_KEY = "vesta-mcp-read-root";
  const MCP_OUTPUT_ROOT_KEY = "vesta-mcp-output-root";

  const FALLBACK_INFO: McpCliInfo = {
    command: "vesta-cli",
    bundled: false,
    default_read_root: "/path/to/media",
    default_output_root: "/path/to/exports",
  };

  let t = $derived($locale);

  let info = $state<McpCliInfo>(FALLBACK_INFO);
  let readRoot = $state(vestaConfig.getItem(MCP_READ_ROOT_KEY) || "");
  let outputRoot = $state(vestaConfig.getItem(MCP_OUTPUT_ROOT_KEY) || "");
  let copiedState = $state<"idle" | "copied" | "failed">("idle");
  let copyTimer: ReturnType<typeof setTimeout> | null = null;

  let effectiveReadRoot = $derived(readRoot.trim() || info.default_read_root);
  let effectiveOutputRoot = $derived(outputRoot.trim() || info.default_output_root);

  let mcpConfigJson = $derived(
    JSON.stringify(
      {
        mcpServers: {
          vesta: {
            command: info.command,
            args: [
              "mcp",
              "--read-root",
              effectiveReadRoot,
              "--output-root",
              effectiveOutputRoot,
            ],
          },
        },
      },
      null,
      2,
    ),
  );

  onMount(() => {
    let mounted = true;
    void invokeCommand<McpCliInfo>("get_mcp_cli_info")
      .then((resolved) => {
        if (!mounted || !resolved?.command) return;
        info = resolved;
        if (!readRoot.trim() && resolved.default_read_root) {
          readRoot = resolved.default_read_root;
        }
        if (!outputRoot.trim() && resolved.default_output_root) {
          outputRoot = resolved.default_output_root;
        }
      })
      .catch(() => {
        // Non-Tauri or early startup fallback keeps the default command.
      });

    return () => {
      mounted = false;
      if (copyTimer) {
        clearTimeout(copyTimer);
        copyTimer = null;
      }
    };
  });

  function updateReadRoot(value: string) {
    readRoot = value;
    vestaConfig.setItem(MCP_READ_ROOT_KEY, value);
  }

  function updateOutputRoot(value: string) {
    outputRoot = value;
    vestaConfig.setItem(MCP_OUTPUT_ROOT_KEY, value);
  }

  async function pickFolder(kind: "read" | "output") {
    const selected = await guardedOpen({
      directory: true,
      multiple: false,
      defaultPath: kind === "read" ? effectiveReadRoot : effectiveOutputRoot,
    });
    if (typeof selected !== "string" || !selected) return;
    if (kind === "read") updateReadRoot(selected);
    else updateOutputRoot(selected);
  }

  async function copyMcpConfig() {
    const ok = await writeClipboard(mcpConfigJson, t("settings.mcp.copied"));
    copiedState = ok ? "copied" : "failed";
    if (copyTimer) clearTimeout(copyTimer);
    copyTimer = setTimeout(() => {
      copiedState = "idle";
      copyTimer = null;
    }, 2500);
  }
</script>

<section id="settings-mcp-card" class="glass-card p-5 mb-6 flex flex-col gap-4" aria-label={t("settings.mcp.title")}>
  <div class="flex flex-wrap items-start justify-between gap-3">
    <div class="flex items-start gap-3 min-w-0 flex-1">
      <span class="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-indigo-500/15 text-indigo-300">
        <svg aria-hidden="true" class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"
          />
        </svg>
      </span>
      <div class="min-w-0 flex-1">
        <div class="flex flex-wrap items-center gap-2">
          <h3 class="text-sm font-bold text-white">{t("settings.mcp.title")}</h3>
          <span
            class="text-[11px] px-2 py-0.5 rounded-full font-medium border {info.bundled
              ? 'bg-emerald-500/15 text-emerald-300 border-emerald-500/30'
              : 'bg-white/5 text-gray-300 border-white/10'}"
          >
            {info.bundled ? t("settings.mcp.cliDetected") : t("settings.mcp.cliFallback")}
          </span>
        </div>
        <p class="mt-1 text-xs leading-relaxed text-gray-400">{t("settings.mcp.description")}</p>
      </div>
    </div>

    <button
      type="button"
      onclick={copyMcpConfig}
      class="btn-secondary shrink-0 px-3.5 py-2 text-xs font-semibold flex items-center gap-1.5 cursor-pointer"
    >
      <svg aria-hidden="true" class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        {#if copiedState === "copied"}
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
        {:else}
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
        {/if}
      </svg>
      {t("settings.mcp.copyConfig")}
    </button>
  </div>

  <div class="grid grid-cols-1 gap-3 md:grid-cols-2">
    <div class="flex flex-col gap-1.5 min-w-0">
      <label for="mcp-read-root" class="text-xs font-medium text-gray-300">
        {t("settings.mcp.readRootLabel")}
      </label>
      <div class="flex gap-2 min-w-0">
        <input
          id="mcp-read-root"
          type="text"
          value={readRoot}
          placeholder={info.default_read_root}
          oninput={(e) => updateReadRoot((e.target as HTMLInputElement).value)}
          class="min-w-0 flex-1 rounded-lg border border-white/10 bg-black/30 px-3 py-1.5 text-xs font-mono text-gray-200 focus:border-indigo-400/50 focus:outline-none"
        />
        <button
          type="button"
          onclick={() => pickFolder("read")}
          class="btn-secondary shrink-0 px-3 py-1.5 text-xs cursor-pointer"
        >
          {t("settings.mcp.browseFolder")}
        </button>
      </div>
    </div>

    <div class="flex flex-col gap-1.5 min-w-0">
      <label for="mcp-output-root" class="text-xs font-medium text-gray-300">
        {t("settings.mcp.outputRootLabel")}
      </label>
      <div class="flex gap-2 min-w-0">
        <input
          id="mcp-output-root"
          type="text"
          value={outputRoot}
          placeholder={info.default_output_root}
          oninput={(e) => updateOutputRoot((e.target as HTMLInputElement).value)}
          class="min-w-0 flex-1 rounded-lg border border-white/10 bg-black/30 px-3 py-1.5 text-xs font-mono text-gray-200 focus:border-indigo-400/50 focus:outline-none"
        />
        <button
          type="button"
          onclick={() => pickFolder("output")}
          class="btn-secondary shrink-0 px-3 py-1.5 text-xs cursor-pointer"
        >
          {t("settings.mcp.browseFolder")}
        </button>
      </div>
    </div>
  </div>

  <pre class="overflow-x-auto rounded-xl border border-white/10 bg-black/35 p-3.5 text-xs font-mono text-gray-200 leading-relaxed select-all">{mcpConfigJson}</pre>

  <div class="flex flex-wrap items-center justify-between gap-2 text-xs">
    <p class="text-gray-400 leading-relaxed">{t("settings.mcp.hint")}</p>
    {#if copiedState === "failed"}
      <p role="status" class="text-red-300">{t("settings.mcp.copyFailed")}</p>
    {:else if copiedState === "copied"}
      <p role="status" class="text-emerald-300">{t("settings.mcp.copied")}</p>
    {/if}
  </div>
</section>
