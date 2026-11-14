// Sites whose Content Security Policy forbids WebAssembly, inline styles,
// external connections and untrusted DOM sinks
// (docs/architecture/browser-extension.md § D7).

import {
  A,
  B,
  candidates,
  chip,
  commitWord,
  expect,
  expectToolbar,
  openTyping,
  overlay,
  preedit,
  test,
} from "./helpers.mjs";

const STRICT = `${A}/strict.html`;

const NUCLEAR_CSP = [
  "default-src 'none'",
  "script-src 'self'",
  "style-src 'none'",
  "img-src 'none'",
  "font-src 'none'",
  "connect-src 'none'",
  "object-src 'none'",
  "base-uri 'none'",
  "form-action 'none'",
  "frame-ancestors 'none'",
  "require-trusted-types-for 'script'",
  "trusted-types 'none'",
].join("; ");

// Same lock-down for pages that link their own external stylesheet (`lexical.css`),
// still forbidding inline styles, WebAssembly, `eval`, fetches and Trusted Types.
const SELF_STYLE_CSP = NUCLEAR_CSP.replace(
  "style-src 'none'",
  "style-src 'self'",
);

/** @typedef {import("@playwright/test").Page} Page */

/**
 * Serves `urlPattern` with an HTTP `Content-Security-Policy` response header.
 * `route.fetch` runs in Node rather than Chromium's `--host-resolver-rules`,
 * so `*.test` origins are fetched from the local loopback server directly.
 * @param {Page} page
 * @param {string} urlPattern
 * @param {string} [policy]
 * @returns {Promise<void>}
 */
async function withStrictHeader(page, urlPattern, policy = NUCLEAR_CSP) {
  await page.route(urlPattern, async (route) => {
    const loopbackUrl = route
      .request()
      .url()
      .replace(/^http:\/\/[^/:]+:/, "http://127.0.0.1:");
    const response = await route.fetch({ url: loopbackUrl });
    await route.fulfill({
      response,
      headers: {
        ...response.headers(),
        "content-security-policy": policy,
      },
    });
  });
}

test("strict HTTP CSP and Trusted Types block WebAssembly, and the JS fallback types cleanly", async ({
  context,
  worker,
  page,
}) => {
  /** @type {string[]} */
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await withStrictHeader(page, `${A}/**`);
  await page.addInitScript(() => {
    window.cspViolations = [];
    document.addEventListener("securitypolicyviolation", (event) => {
      window.cspViolations?.push({
        directive: event.effectiveDirective,
        blockedURI: event.blockedURI,
      });
    });
  });

  const field = await openTyping(context, worker, page, STRICT, "#text");
  await page.keyboard.type("taiuan");
  await candidates(page).filter({ hasText: "台灣" }).first().click();
  await page.keyboard.press("Enter");
  await expect(field).toHaveValue("台灣");
  await expect(preedit(page)).toBeEmpty();
  expect(pageErrors).toEqual([]);

  // Only the initial WASM compile probe may trip `script-src`; styles, shadow
  // stylesheets, `DOMParser` and dictionary fetches trip nothing.
  const violations = (await page.evaluate(() => window.cspViolations)) ?? [];
  for (const violation of violations) {
    expect(violation.blockedURI).toBe("wasm-eval");
  }
});

test("a rich-text Lexical editor composes, swaps spaces, and undoes under strict CSP", async ({
  context,
  worker,
  page,
}) => {
  await worker.evaluate(() =>
    chrome.storage.sync.set({
      "taigi.settings": JSON.stringify({
        revision: 1,
        values: { autoSpaceEnabled: true },
      }),
    }),
  );
  await withStrictHeader(page, `${A}/**`, SELF_STYLE_CSP);

  await openTyping(context, worker, page, `${A}/lexical.html`, "#editor");
  await commitWord(page);
  await expect
    .poll(() => page.evaluate(() => window.lexicalFixture.text()))
    .toBe("台灣");

  await page.keyboard.press("ControlOrMeta+z");
  await expect
    .poll(() => page.evaluate(() => window.lexicalFixture.text()))
    .toBe("");

  await page.keyboard.press("`");
  await commitWord(page);
  await page.keyboard.press(".");
  await expect
    .poll(() => page.evaluate(() => window.lexicalFixture.text()))
    .toBe("tâi-uân. ");
});

test("expandable candidate measurement and unfolding work across wasm2js under strict CSP", async ({
  context,
  worker,
  page,
}) => {
  await worker.evaluate(() =>
    chrome.storage.sync.set({
      "taigi.settings": JSON.stringify({
        revision: 1,
        values: { candidateLayout: "expandable" },
      }),
    }),
  );
  const field = await openTyping(context, worker, page, STRICT, "#text");
  await page.keyboard.type("ti");
  const unfold = overlay(page).locator("#unfold");
  await expect(unfold).toBeVisible();
  const collapsedCount = await candidates(page).count();

  await page.keyboard.press("ArrowDown");
  await expect
    .poll(() => candidates(page).count())
    .toBeGreaterThan(collapsedCount);
  await page.keyboard.press("Enter");
  await expect(field).not.toHaveValue("");
});

test("learned picks persist and sync across strict-CSP tabs running wasm2js", async ({
  context,
  worker,
  page,
}) => {
  const other = await context.newPage();
  await openTyping(context, worker, other, `${B}/strict.html`, "#text");

  await page.bringToFront();
  const field = await openTyping(context, worker, page, STRICT, "#text");
  for (let round = 0; round < 3; round += 1) {
    await page.keyboard.type("kausiu");
    await candidates(page).filter({ hasText: "狗岫" }).first().click();
    await page.keyboard.press("Enter");
  }
  await expect(field).toHaveValue("狗岫狗岫狗岫");

  // Wait for the service worker's IndexedDB generation to advance, then bring
  // the second strict-CSP tab forward so it swaps to a fresh wasm2js instance.
  await expect
    .poll(() =>
      worker.evaluate(
        () =>
          new Promise((resolve, reject) => {
            const opening = indexedDB.open("taigi-extension-user-data", 1);
            opening.onupgradeneeded = () =>
              opening.result.createObjectStore("files");
            opening.onerror = () => reject(opening.error);
            opening.onsuccess = () => {
              const reading = opening.result
                .transaction("files")
                .objectStore("files")
                .get("snapshot");
              reading.onerror = () => reject(reading.error);
              reading.onsuccess = () => {
                opening.result.close();
                resolve(reading.result?.generation ?? 0);
              };
            };
          }),
      ),
    )
    .toBeGreaterThan(0);

  await other.bringToFront();
  await other.evaluate(() => window.dispatchEvent(new Event("focus")));
  await other.locator("#text").click();

  const topCell = candidates(other)
    .filter({ has: other.locator(".index", { hasText: /^.+$/ }) })
    .first()
    .locator(".text");
  await expect(async () => {
    await other.keyboard.type("kausiu");
    try {
      await expect(topCell).toHaveText("狗岫", { timeout: 500 });
    } finally {
      await other.keyboard.press("Escape");
    }
  }).toPass();
});

test("English mode toggle and reload retention work under strict CSP", async ({
  context,
  worker,
  page,
}) => {
  const field = await openTyping(context, worker, page, STRICT, "#text");
  await page.keyboard.press("Shift");
  await expect(chip(page)).toHaveAttribute("data-state", "english");
  await expectToolbar(worker, page, "A");
  await page.keyboard.type("abc");
  await expect(field).toHaveValue("abc");

  await page.reload();
  await expectToolbar(worker, page, "A");
  const reloadedField = page.locator("#text");
  await reloadedField.click();
  await expect(chip(page)).toHaveAttribute("data-state", "english");

  await page.keyboard.press("Shift");
  await expect(chip(page)).toHaveAttribute("data-state", "taigi");
  await commitWord(page);
  await expect(reloadedField).toHaveValue("台灣");
});
