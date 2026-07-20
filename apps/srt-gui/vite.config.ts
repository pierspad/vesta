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
