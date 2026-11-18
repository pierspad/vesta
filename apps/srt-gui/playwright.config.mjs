import { defineConfig } from "@playwright/test";

const baseURL = "http://127.0.0.1:5176";

export default defineConfig({
  testDir: "./browser-tests",
  outputDir: "./node_modules/.cache/vesta-browser-tests",
  use: {
    baseURL,
    locale: "en-US",
    viewport: { width: 1440, height: 1000 },
    reducedMotion: "reduce",
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    launchOptions: { executablePath: process.env.VESTA_TEST_CHROMIUM },
  },
  webServer: {
    command: "npm run dev -- --host 127.0.0.1 --port 5176 --strictPort",
    url: baseURL,
  },
});
