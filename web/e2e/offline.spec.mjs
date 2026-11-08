// Loading and offline: `docs/architecture/web-roadmap.md` § Phase 6. A request
// from a page under a service worker never reaches `page.route`, so these go by
// what the page and the caches report.

import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import {
  cpSync,
  mkdtempSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "@playwright/test";
import {
  candidates,
  commitWord,
  emptyBox,
  expectState,
  open,
} from "./helpers.mjs";

const SITE = fileURLToPath(new URL("../target/site", import.meta.url));
const SERVE = fileURLToPath(new URL("../scripts/serve.mjs", import.meta.url));
const DICTIONARY_CACHE = "taigi-web-dictionaries";
const NEXT_MARK = "<!-- next -->";

function workerReady(page) {
  return page.evaluate(async () => {
    await navigator.serviceWorker.ready;
  });
}

function downloads(page) {
  return page.evaluate(
    () =>
      performance
        .getEntriesByType("resource")
        .filter((entry) => entry.name.endsWith(".gz")).length,
  );
}

function storedDictionaries(page) {
  return page.evaluate(async (name) => {
    const cache = await caches.open(name);
    return (await cache.keys()).map((request) => request.url);
  }, DICTIONARY_CACHE);
}

async function storedDictionaryText(page) {
  return (await storedDictionaries(page)).join();
}

test("a visit types Hanji offline from then on", async ({ page, context }) => {
  await open(page);
  expect(await storedDictionaries(page)).toHaveLength(4);
  await workerReady(page);

  await context.setOffline(true);
  await page.reload();
  const editor = page.locator("#text");
  await expectState(page, "ready");
  await emptyBox(editor);
  await commitWord(editor);
  await expect(editor).toHaveValue("台灣");
});

test("a second visit downloads nothing", async ({ page }) => {
  await open(page);
  expect(await downloads(page)).toBe(4);
  await workerReady(page);
  await page.reload();
  await expectState(page, "ready");
  expect(await downloads(page)).toBe(0);
});

test("progress shows only during a download", async ({ page }) => {
  let release;
  const held = new Promise((resolve) => {
    release = resolve;
  });
  await page.route("**/dictionaries/dictionary.fst.*", async (route) => {
    await held;
    await route.continue();
  });
  await open(page, { ready: false });
  const progress = page.locator("#progress");
  await expect(progress).toBeVisible();
  const box = await page.locator(".actions").boundingBox();
  release();
  await expectState(page, "ready");
  await expect(progress).toBeHidden();
  expect(await page.locator(".actions").boundingBox()).toEqual(box);
});

test("a corrupt download keeps romanization and offers retry", async ({
  page,
}) => {
  await page.route("**/dictionaries/dictionary.bin.*", (route) =>
    route.fulfill({ body: Buffer.alloc(1024) }),
  );
  const editor = await open(page, { ready: false });
  await expectState(page, "failed");
  const retry = page.locator("#retry");
  await expect(retry).toBeVisible();
  await editor.press("a");
  await expect(page.locator("#preedit")).toContainText("a");
  await editor.press("Enter");
  await expect(editor).toHaveValue(/^a/);
  expect(await storedDictionaries(page)).not.toContainEqual(
    expect.stringContaining("dictionary.bin"),
  );

  await page.unroute("**/dictionaries/dictionary.bin.*");
  await retry.click();
  await expectState(page, "ready");
  await expect(retry).toBeHidden();
  await editor.fill("");
  await commitWord(editor);
  await expect(editor).toHaveValue("台灣");
});

test("a damaged stored dictionary downloads again", async ({ page }) => {
  await open(page);
  await workerReady(page);
  await page.evaluate(async (name) => {
    const cache = await caches.open(name);
    const [request] = await cache.keys();
    await cache.put(request, new Response("damaged"));
  }, DICTIONARY_CACHE);

  await page.reload();
  await expectState(page, "failed");
  await page.locator("#retry").click();
  await expectState(page, "ready");
  await emptyBox(page.locator("#text"));
  await commitWord(page.locator("#text"));
  await expect(page.locator("#text")).toHaveValue("台灣");
});

test("the policy blocks an inline script and nothing the page uses", async ({
  page,
}) => {
  await page.addInitScript(() => {
    window.violations = [];
    document.addEventListener("securitypolicyviolation", (event) => {
      window.violations.push(
        `${event.violatedDirective} ${event.sourceFile}:${event.lineNumber} ${event.sample}`,
      );
    });
  });
  await open(page);
  await page.locator("select[name=fontType]").selectOption("openHuninn");
  await expect(page.locator("select[name=fontType]")).toHaveCSS(
    "background-image",
    /chevron\.svg/,
  );
  expect(await page.evaluate(() => window.violations)).toEqual([]);

  const ran = await page.evaluate(async () => {
    const script = document.createElement("script");
    script.textContent = "window.inlineRan = true;";
    document.head.append(script);
    await new Promise((resolve) => setTimeout(resolve, 50));
    return window.inlineRan === true;
  });
  expect(ran).toBe(false);
});

test("the page installs from its manifest", async ({ page, request }) => {
  await open(page, { ready: false });
  const href = await page.locator("link[rel=manifest]").getAttribute("href");
  const manifest = await (await request.get(`/${href}`)).json();
  for (const icon of manifest.icons) {
    expect((await request.get(`/${icon.src}`)).ok()).toBe(true);
  }
});

test.describe("a new version", () => {
  let directory;
  let server;
  let url;

  const sha256 = (path) =>
    createHash("sha256").update(readFileSync(path)).digest("base64");

  /// What a deploy changes: the page, one dictionary's name and the build.
  function deployNextVersion(root) {
    const manifestPath = join(root, "assets-manifest.json");
    const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
    const html = join(root, "index.html");
    writeFileSync(
      html,
      readFileSync(html, "utf8").replace("<head>", `<head>${NEXT_MARK}`),
    );
    manifest.shell.find(({ path }) => path === "index.html").sha256 =
      sha256(html);

    const entry = manifest.dictionaries[0];
    const renamed = entry.file.replace(/\.gz$/, ".next.gz");
    renameSync(join(root, entry.file), join(root, renamed));
    const module = join(root, "generated/dictionaries.js");
    writeFileSync(
      module,
      readFileSync(module, "utf8").replace(entry.file, renamed),
    );
    manifest.shell.find(
      ({ path }) => path === "generated/dictionaries.js",
    ).sha256 = sha256(module);
    entry.file = renamed;

    const worker = join(root, "sw.js");
    const stamped = readFileSync(worker, "utf8");
    writeFileSync(
      worker,
      stamped.replace(manifest.build, `${manifest.build}-next`),
    );
    manifest.build = `${manifest.build}-next`;
    writeFileSync(manifestPath, JSON.stringify(manifest));
    return { old: entry.file.replace(".next.gz", ".gz"), renamed };
  }

  test.beforeEach(async ({ browserName }, testInfo) => {
    directory = mkdtempSync(join(tmpdir(), `taigi-web-${browserName}-`));
    cpSync(SITE, join(directory, "web"), {
      recursive: true,
      // The default font is in the shell; the rest come in on use.
      filter: (source) =>
        !source.endsWith(".woff2") || source.endsWith("iansui_regular.woff2"),
    });
    const port = 4300 + testInfo.workerIndex;
    server = spawn("node", [SERVE, directory, String(port)], {
      stdio: ["ignore", "pipe", "inherit"],
    });
    await new Promise((resolve) => server.stdout.once("data", resolve));
    url = `http://127.0.0.1:${port}/web/`;
  });

  test.afterEach(() => {
    server.kill();
    rmSync(directory, { recursive: true, force: true });
  });

  test("waits for the tabs to close, then replaces the old one", async ({
    context,
    page,
  }) => {
    await page.goto(url);
    await expectState(page, "ready");
    await workerReady(page);

    const { old, renamed } = deployNextVersion(join(directory, "web"));
    await page.reload();
    await expectState(page, "ready");
    expect(await page.content()).not.toContain(NEXT_MARK);
    await expect
      .poll(() =>
        page.evaluate(async () => {
          const registration = await navigator.serviceWorker.ready;
          await registration.update().catch(() => {});
          return registration.waiting !== null;
        }),
      )
      .toBe(true);
    expect(await storedDictionaryText(page)).toContain(renamed);

    await page.close();
    // A tab left open would hold the old worker: a reload is served by it.
    await expect(async () => {
      const probe = await context.newPage();
      await probe.goto(url);
      const html = await probe.content();
      await probe.close();
      expect(html).toContain(NEXT_MARK);
    }).toPass({ intervals: [200, 500, 1000] });
    const next = await context.newPage();
    await next.goto(url);
    await expectState(next, "ready");
    const stored = await storedDictionaryText(next);
    expect(stored).toContain(renamed);
    expect(stored).not.toContain(old);
    await emptyBox(next.locator("#text"));
    await commitWord(next.locator("#text"));
    await expect(next.locator("#text")).toHaveValue("台灣");
    await expect(candidates(next)).toHaveCount(0);
  });
});
