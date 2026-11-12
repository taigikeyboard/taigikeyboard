import { defineConfig } from "@playwright/test";

const PORT = 4173;

export default defineConfig({
  testDir: "./e2e",
  forbidOnly: true,
  // Tests share no state (each `a new version` worker serves its own copy on
  // its own port), so spread them per test: look.spec alone holds a quarter.
  // CI's runner has 4 vCPUs; Playwright's default would take half.
  fullyParallel: true,
  workers: process.env.CI ? 4 : undefined,
  webServer: {
    command: `node scripts/serve.mjs target/site ${PORT}`,
    url: `http://127.0.0.1:${PORT}/index.html`,
    stdout: "ignore",
    stderr: "ignore",
    reuseExistingServer: !process.env.CI,
  },
  use: { baseURL: `http://127.0.0.1:${PORT}`, actionTimeout: 5_000 },
  projects: [
    { name: "chromium", use: { browserName: "chromium" } },
    { name: "firefox", use: { browserName: "firefox" } },
    { name: "webkit", use: { browserName: "webkit" } },
  ],
});
