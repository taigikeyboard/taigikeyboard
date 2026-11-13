// The staged extension in a real Chromium profile. Each test gets a new
// profile; a worker shares one copy of the extension.
//
// The browser's own question about a site ("Use on This Site") is a native
// dialog no harness can answer, so the copy under test is given its sites in
// the manifest: `a.test` and `b.test` are enabled, `c.test` is not. All three
// are this machine (`--host-resolver-rules`).

import {
  cpSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { test as base, chromium, expect } from "@playwright/test";

import { PORT } from "../playwright.config.mjs";

const STAGED = fileURLToPath(
  new URL("../../target/extension", import.meta.url),
);

/// Sites the copy under test may type on. Quill's own page is for the smoke
/// test against the public editor (`quill-public.spec.mjs`).
const ENABLED_SITES = ["*://a.test/*", "*://b.test/*", "*://quilljs.com/*"];

export const A = `http://a.test:${PORT}`;
export const B = `http://b.test:${PORT}`;
export const C = `http://c.test:${PORT}`;
export const STORE = `http://chromewebstore.google.com:${PORT}`;

/// Waits for any force-installed MV2 background page (such as Helium's bundled
/// uBlock Origin, which reloads tabs opened before its filter lists finish
/// compiling) to settle before tests navigate. Stock Chromium has none.
async function settleBackgroundPages(context) {
  const cdp = await context.browser().newBrowserCDPSession();
  try {
    const { targetInfos } = await cdp.send("Target.getTargets");
    const pages = targetInfos.filter((info) => info.type === "background_page");
    for (const { targetId } of pages) {
      const { sessionId } = await cdp.send("Target.attachToTarget", {
        targetId,
        flatten: false,
      });
      let nextId = 1;
      const evaluate = (expression) =>
        new Promise((resolve, reject) => {
          const id = nextId++;
          const onMessage = (event) => {
            if (event.sessionId !== sessionId) {
              return;
            }
            const data = JSON.parse(event.message);
            if (data.id !== id) {
              return;
            }
            cdp.off("Target.receivedMessageFromTarget", onMessage);
            if (data.error) {
              reject(new Error(data.error.message));
            } else {
              resolve(data.result?.result?.value);
            }
          };
          cdp.on("Target.receivedMessageFromTarget", onMessage);
          cdp
            .send("Target.sendMessageToTarget", {
              sessionId,
              message: JSON.stringify({
                id,
                method: "Runtime.evaluate",
                params: { expression, returnByValue: true },
              }),
            })
            .catch((error) => {
              cdp.off("Target.receivedMessageFromTarget", onMessage);
              reject(error);
            });
        });
      await expect
        .poll(
          () =>
            evaluate(
              'location.protocol === "chrome-extension:" && document.readyState === "complete" && (!("µBlock" in self) || (Boolean(self.µBlock?.readyToFilter) && (self.vAPI?.net?.suspendDepth ?? 0) === 0))',
            ),
          { timeout: 15_000 },
        )
        .toBe(true);
      await cdp.send("Target.detachFromTarget", { sessionId });
    }
  } finally {
    await cdp.detach();
  }
}

/// Opens the profile at `profile` with the extension at `extension` loaded.
/// `EXTENSION_BROWSER`: another Chromium's executable, instead of
/// Playwright's own.
export async function launch(extension, profile) {
  const context = await chromium.launchPersistentContext(profile, {
    channel: "chromium",
    executablePath: process.env.EXTENSION_BROWSER || undefined,
    args: [
      `--disable-extensions-except=${extension}`,
      `--load-extension=${extension}`,
      "--host-resolver-rules=MAP *.test 127.0.0.1, MAP chromewebstore.google.com 127.0.0.1",
    ],
  });
  const worker =
    context.serviceWorkers()[0] ??
    (await context.waitForEvent("serviceworker"));
  // A page opened before the content script is registered would not get it.
  await expect
    .poll(() =>
      worker.evaluate(async () => {
        const scripts = await chrome.scripting.getRegisteredContentScripts();
        return scripts.length;
      }),
    )
    .toBe(1);
  await settleBackgroundPages(context);
  return { context, worker };
}

export const test = base.extend({
  extension: [
    // biome-ignore lint/correctness/noEmptyPattern: Playwright reads the pattern.
    async ({}, use) => {
      const directory = mkdtempSync(join(tmpdir(), "taigi-extension-"));
      cpSync(STAGED, directory, { recursive: true });
      const path = join(directory, "manifest.json");
      const manifest = JSON.parse(readFileSync(path, "utf8"));
      manifest.host_permissions = ENABLED_SITES;
      writeFileSync(path, JSON.stringify(manifest));
      await use(directory);
      rmSync(directory, { recursive: true, force: true });
    },
    { scope: "worker" },
  ],

  /// A directory for the test's profile, kept for the whole test so that a
  /// test can close the browser and open the same profile again.
  // biome-ignore lint/correctness/noEmptyPattern: Playwright reads the pattern.
  profile: async ({}, use) => {
    const directory = mkdtempSync(join(tmpdir(), "taigi-profile-"));
    await use(directory);
    rmSync(directory, { recursive: true, force: true });
  },

  launched: async ({ extension, profile }, use) => {
    const launched = await launch(extension, profile);
    await use(launched);
    await launched.context.close();
  },

  context: async ({ launched }, use) => {
    await use(launched.context);
  },

  worker: async ({ launched }, use) => {
    await use(launched.worker);
  },

  page: async ({ context }, use) => {
    await use(await context.newPage());
  },
});

export { expect };

/// The id the browser gave `page`'s tab. A tab's URL is no name for it: two
/// tabs can share one, and a site that is not enabled hides its own. The
/// profile has one window, so the tab in front is `page`'s.
export async function tabIdOf(worker, page) {
  await page.bringToFront();
  return worker.evaluate(async () => {
    const [tab] = await chrome.tabs.query({ active: true });
    return tab.id;
  });
}

/// The toolbar popup, opened as a page about `page`'s tab.
export async function openPopup(context, worker, page) {
  const tabId = await tabIdOf(worker, page);
  const popup = await context.newPage();
  const id = new URL(worker.url()).host;
  await popup.goto(`chrome-extension://${id}/popup.html?tab=${tabId}`);
  await expect(popup.locator("#status")).not.toBeEmpty();
  return popup;
}

/// Clicks the popup's button `name` (`turn-on`, `turn-off`) for `page`'s tab,
/// and brings `page` back to the front.
export async function clickPopup(context, worker, page, name) {
  const popup = await openPopup(context, worker, page);
  // The popup closes itself once the service worker has answered.
  await Promise.all([
    popup.waitForEvent("close"),
    popup.locator(`#${name}`).click(),
  ]);
  await page.bringToFront();
}

/// Starts typing in `page`'s tab, as the popup's button does, and waits for
/// the dictionaries.
export async function turnOn(context, worker, page) {
  await clickPopup(context, worker, page, "turn-on");
  await expectToolbar(worker, page, "台");
}

/// The toolbar button of `page`'s tab: `[badge, title]`.
export async function toolbar(worker, page) {
  const tabId = await tabIdOf(worker, page);
  return worker.evaluate(
    async (tabId) => [
      await chrome.action.getBadgeText({ tabId }),
      await chrome.action.getTitle({ tabId }),
    ],
    tabId,
  );
}

export function expectToolbar(worker, page, badge) {
  return expect.poll(async () => (await toolbar(worker, page))[0]).toBe(badge);
}

/// What the extension draws on `page`. Playwright's locators see into its
/// open shadow tree.
export const overlay = (page) => page.locator("taigi-keyboard");
export const preedit = (page) => overlay(page).locator("#preedit");
export const candidates = (page) => overlay(page).locator("#candidates li");
export const chip = (page) => overlay(page).locator("#chip");

/// `page` on `url`, typing on, its field `selector` focused.
export async function openTyping(context, worker, page, url, selector) {
  await page.goto(url);
  await turnOn(context, worker, page);
  const field = page.locator(selector);
  await field.click();
  await expect(chip(page)).toHaveAttribute("data-state", "taigi");
  return field;
}

/// What the page's own listeners heard (`fixtures/fields.js`).
export const heard = (page) => page.locator("#log").textContent();

/// Types the web page's test word and takes its first candidate, 台灣
/// (`web/e2e/helpers.mjs` `commitWord`).
export async function commitWord(page) {
  await page.keyboard.type("taiuan");
  await page.keyboard.press("Enter");
}
