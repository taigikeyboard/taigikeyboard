// The layout simulation (`make -C web layout-sim`): types corpus sentences
// into the staged page and asserts the candidate window's layout after every
// word. Not part of `make e2e` or CI — its testDir is outside `e2e/`.

import { fileURLToPath } from "node:url";
import { defineConfig } from "@playwright/test";

// Its own port (e2e 4173, extension e2e 4174). Never run `make e2e` during a
// run: it deletes all of test-results/, this run's output with it.
const PORT = 4175;

export default defineConfig({
  testDir: ".",
  // Cleaned at the start of every run; the spec writes its screenshots and
  // reports here, `summary.mjs` the summary.
  outputDir: "../test-results/layout-sim",
  forbidOnly: true,
  fullyParallel: true,
  // One combination types every sentence: 10–20 s each. More workers starve
  // the browsers on a laptop also running simulators.
  workers: 4,
  timeout: 180_000,
  globalTeardown: "./summary.mjs",
  webServer: {
    command: `node scripts/serve.mjs target/site ${PORT}`,
    cwd: fileURLToPath(new URL("..", import.meta.url)),
    url: `http://127.0.0.1:${PORT}/index.html`,
    stdout: "ignore",
    stderr: "ignore",
    reuseExistingServer: false,
  },
  use: { baseURL: `http://127.0.0.1:${PORT}`, actionTimeout: 10_000 },
  projects: [
    { name: "chromium", use: { browserName: "chromium" } },
    { name: "firefox", use: { browserName: "firefox" } },
    { name: "webkit", use: { browserName: "webkit" } },
  ],
});
