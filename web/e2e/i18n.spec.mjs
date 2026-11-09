// The display language over the generated strings, and the Font setting.

import { expect, test } from "@playwright/test";

import {
  commitWord,
  firstSection,
  open,
  seedSettings,
  TIER,
  toggle,
} from "./helpers.mjs";

const rootLang = (page) => page.evaluate(() => document.documentElement.lang);
const stored = (page) =>
  page.evaluate(
    () => JSON.parse(localStorage.getItem("taigi.settings")).values,
  );

/// The control of the setting `name`, whatever the language of its label.
const setting = (page, name) => page.locator(`select[name="${name}"]`);

/// How many faces of `family` the browser takes for `text`: none when its
/// `unicode-range` leaves the text out.
const facesFor = (page, family, text) =>
  page.evaluate(
    async ([name, sample]) =>
      (await document.fonts.load(`16px "${name}"`, sample)).length,
    [family, text],
  );

for (const [locale, title, lang] of [
  ["en-US", "General", "en"],
  ["ja-JP", "一般", "ja"],
  ["zh-TW", "一般", "nan-Hant-TW"],
  ["fr-FR", "一般", "nan-Hant-TW"],
]) {
  test.describe(locale, () => {
    test.use({ locale });

    test(`Automatic follows the browser: ${title}`, async ({ page }) => {
      await page.goto("/index.html");
      await expect(firstSection(page)).toHaveText(title);
      expect(await rootLang(page)).toBe(lang);
    });
  });
}

test("Display Language redraws the page and survives a reload", async ({
  page,
}) => {
  await open(page);
  await setting(page, "displayLanguage").selectOption("tailo");
  const plainText = page.locator('[data-format="text"]');
  // trace: web.plainText / web.ready / desktop.menuSettings tailo values.
  await expect(plainText).toHaveText("Sûn bûn-jī");
  await expect(page.locator("#message")).toHaveText("Ē-sái phah-jī");
  await expect(toggle(page)).toHaveAttribute(
    "aria-label",
    "Tâi-gí Khí-puânn Siat-tīng",
  );
  expect(await rootLang(page)).toBe("nan-Latn-tailo");
  await page.reload();
  await expect(plainText).toHaveText("Sûn bûn-jī");
  await expect(setting(page, "displayLanguage")).toHaveValue("tailo");
});

test("the title stays put in every language", async ({ page }) => {
  await open(page, { settings: { displayLanguage: "ja" } });
  await expect(page).toHaveTitle("台語齒盤網頁版：台語簿仔紙");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(
    "台語齒盤網頁版：台語簿仔紙",
  );
});

test("the session speaks the page's language", async ({ page }) => {
  // trace: Ctrl+Alt+C → `switched_label`, settings.pojMode in the Hanji map.
  const editor = await open(page, { settings: { displayLanguage: "hanji" } });
  await editor.press(`${TIER}+c`);
  await expect(page.locator("#message")).toHaveText("白話字");
});

test("an unknown language tag reads as Hanji, as the session reads it", async ({
  page,
}) => {
  const editor = await open(page, { settings: { displayLanguage: "xx" } });
  await expect(firstSection(page)).toHaveText("一般");
  await editor.press(`${TIER}+c`);
  await expect(page.locator("#message")).toContainText("白話字");
});

test("a language change drops the session's text", async ({ page }) => {
  const editor = await open(page);
  await editor.press(`${TIER}+c`);
  await expect(page.locator("#message")).toContainText("Pe̍h-ōe-jī");
  await setting(page, "displayLanguage").selectOption("ja");
  await expect(page.locator("#message")).toHaveText("");
});

test("kana and Extension G reach the selected font", async ({ page }) => {
  await open(page, { settings: { fontType: "iansui" } });
  expect(await facesFor(page, "Iansui", "レ")).toBe(1);
  expect(await facesFor(page, "Iansui", "\u{308FB}")).toBe(1);
});

test("the markup's English stays unseen until it is translated", async ({
  page,
}) => {
  await seedSettings(page, { displayLanguage: "ja" });
  await page.route("**/ime.js", (route) => route.abort());
  await page.goto("/index.html");
  await expect(firstSection(page)).toBeHidden();
});

test("the default font is the only one fetched", async ({ page }) => {
  const fonts = [];
  page.on("request", (request) => {
    if (request.url().includes("/fonts/")) {
      fonts.push(new URL(request.url()).pathname);
    }
  });
  const editor = await open(page);
  await commitWord(editor);
  await expect(editor).toHaveValue("台灣");
  // None at all where the machine has Iansui installed (`local()`).
  const others = fonts.filter((path) => !path.endsWith("iansui_regular.woff2"));
  expect(others).toEqual([]);
});

for (const [value, family] of [
  ["openHuninn", "jf-openhuninn"],
  ["iansui", "Iansui"],
  ["genYoMin", "GenYoMin2 TW"],
  ["genYoGothic", "GenYoGothic2 TW"],
]) {
  test(`${value} sets Hanji and leaves romanization alone`, async ({
    page,
  }) => {
    await open(page);
    await setting(page, "fontType").selectOption(value);
    expect(await facesFor(page, family, "台")).toBe(1);
    expect(await facesFor(page, family, "tsi̍t")).toBe(0);
    await expect(page.locator("#text")).toHaveCSS(
      "font-family",
      new RegExp(family),
    );
  });
}

test("the font is restored before the page's module runs", async ({ page }) => {
  await open(page, { settings: { fontType: "iansui" } });
  await page.route("**/ime.js", (route) => route.abort());
  await page.reload();
  expect(await page.evaluate(() => document.documentElement.dataset.font)).toBe(
    "iansui",
  );
});

test("a chord's write keeps the font and the language", async ({ page }) => {
  const editor = await open(page, {
    settings: { fontType: "iansui", displayLanguage: "ja" },
  });
  await editor.press(`${TIER}+c`);
  await expect.poll(() => stored(page)).toMatchObject({ fontType: "iansui" });
  await expect(firstSection(page)).toHaveText("一般");
  // trace: desktop.menuSettings ja.
  await expect(toggle(page)).toHaveAttribute("title", "台湾語キーボードの設定");
});
