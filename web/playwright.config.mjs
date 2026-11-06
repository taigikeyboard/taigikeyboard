import { defineConfig } from "@playwright/test";

const PORT = 4173;

export default defineConfig({
  testDir: "./e2e",
  forbidOnly: true,
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
