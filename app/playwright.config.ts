import { defineConfig } from "@playwright/test";

// UI flows against the Vite dev server with the in-browser mock backend.
export default defineConfig({
  testDir: "e2e",
  timeout: 30_000,
  use: { baseURL: "http://localhost:1420", viewport: { width: 1300, height: 820 } },
  webServer: {
    command: "pnpm exec vite --port 1420 --strictPort",
    url: "http://localhost:1420",
    reuseExistingServer: true,
    timeout: 60_000,
  },
});
