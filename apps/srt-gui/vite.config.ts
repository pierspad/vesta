import { fileURLToPath, URL } from "node:url";

import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig, type Plugin } from "vite";

// `Component.svelte?svelte&type=style&lang.css` — the virtual module
// vite-plugin-svelte serves for a component's `<style>` block.
const SVELTE_VIRTUAL_STYLE = /\.svelte\?.*\btype=style\b/;

/**
 * Repairs cache misses on Svelte virtual CSS modules.
 *
 * vite-plugin-svelte serves `<style>` blocks from the compiled-CSS cache it
 * fills during the component's own transform. When that module is requested
 * *before* the component has been compiled in the current dev-server run (HMR
 * of a style-only edit, a reload where the component JS came from the browser
 * cache), its `load` hook returns nothing — and Vite falls back to reading the
 * raw `.svelte` file from disk. The id still ends in `&lang.css`, so
 * @tailwindcss/vite parses the component *source* as CSS and the dev server
 * dies on the first TypeScript line (`Invalid declaration: \`Snippet\``).
 *
 * Transforming the component first fills that cache, so we can answer with the
 * real scoped CSS. Runs before the Svelte CSS loader, so cold requests are repaired before its warning
 * (`this.load()` is not enough — in dev it resolves without transforming).
 */
function svelteVirtualCssFallback(): Plugin {
  return {
    name: "vesta:svelte-virtual-css-fallback",
    enforce: "pre",
    load: {
      filter: { id: SVELTE_VIRTUAL_STYLE },
      async handler(id) {
        const filename = id.split("?")[0];
        const env = this.environment as { transformRequest?: (url: string) => Promise<unknown> };
        if (!this.getModuleInfo(filename)?.meta?.svelte?.css) {
          await env.transformRequest?.(filename);
        }
        const cached = this.getModuleInfo(filename)?.meta?.svelte?.css;
        // Empty CSS keeps the server alive if the component genuinely has none
        // (or its compile failed and is reported elsewhere) — anything else here
        // would be handed to Tailwind as CSS.
        if (!cached) return "";
        const { hasGlobal, ...css } = cached;
        if (hasGlobal === false) {
          css.meta ??= {};
          css.meta.vite ??= {};
          css.meta.vite.cssScopeTo = [filename, "default"];
        }
        css.moduleType = "css";
        return css;
      },
    },
  };
}

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [svelte(), svelteVirtualCssFallback(), tailwindcss()],
  clearScreen: false,
  resolve: {
    alias: {
      $lib: fileURLToPath(new URL("./src/lib", import.meta.url)),
    },
  },
  server: {
    port: process.env.PORT ? parseInt(process.env.PORT) : 5175,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  // `vitest` reads its config from this same file (single source of truth).
  // Pure utilities, stores, configuration and mocked IPC services run in Node.
  // Desktop/webview interactions still require integration and release smoke tests.
  test: {
    include: ["src/lib/**/*.test.{ts,js}"],
    environment: "node",
    setupFiles: ["src/lib/test-setup.ts"],
  },
  build: {
    target: "es2022",
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (!id.includes("node_modules")) return undefined;
          // Big self-contained editor dependency: only CodeEditor.svelte uses it.
          if (id.includes("codemirror") || id.includes("@lezer")) return "codemirror";
          if (id.includes("@tauri-apps")) return "tauri";
          return "vendor";
        },
      },
    },
  },
});
