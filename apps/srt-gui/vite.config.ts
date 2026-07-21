import { fileURLToPath, URL } from "node:url";

import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [svelte(), tailwindcss()],
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
  // Scope kept to `lib/utils` and `lib/config` on purpose: those are the pure,
  // DOM-free modules (see seriesFileMatching.ts docstring) — components and
  // Tauri-backed services aren't unit-testable without a much heavier harness
  // (jsdom + mocked `invoke`), which isn't worth it yet for a desktop app
  // that's manually smoke-tested before every release.
  test: {
    include: ["src/lib/{utils,config}/**/*.test.ts"],
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
