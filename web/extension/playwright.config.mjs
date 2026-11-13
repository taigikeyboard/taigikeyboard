// The browser extension's gate: the staged extension, loaded into Chromium's
// own extension runtime, typing into the fixtures `make extension-e2e` staged.

import { defineConfig } from "@playwright/test";

export const PORT = 4174;

export default defineConfig({
  testDir: "./e2e",
  outputDir: "../test-results/extension",
  forbidOnly: true,
  fullyParallel: true,
  workers: process.env.CI ? 2 : undefined,
  webServer: {
    command: `node ../scripts/serve.mjs ../target/extension-fixtures ${PORT}`,
    url: `http://127.0.0.1:${PORT}/fields.html`,
    stdout: "ignore",
    stderr: "ignore",
    reuseExistingServer: !process.env.CI,
  },
  use: { actionTimeout: 5_000 },
});
