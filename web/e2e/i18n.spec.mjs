// The display language over the generated strings, and the Font setting.

import { expect, test } from "@playwright/test";

import { commitWord, open, seedSettings, toggle } from "./helpers.mjs";

const heading = (page) => page.getByRole("heading", { level: 1 });
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
  ["en-US", "TaigiKeyboard", "en"],
  ["ja-JP", "台湾語キーボード", "ja"],
  ["zh-TW", "台語齒盤", "nan-Hant-TW"],
  ["fr-FR", "台語齒盤", "nan-Hant-TW"],
]) {
  test.describe(locale, () => {
    test.use({ locale });

    test(`Automatic follows the browser: ${title}`, async ({ page }) => {
      await page.goto("/index.html");
      await expect(heading(page)).toHaveText(title);
      expect(await rootLang(page)).toBe(lang);
    });
  });
}

test("Display Language redraws the page and survives a reload", async ({
  page,
}) => {
  const editor = await open(page);
  await toggle(page).click();
  await setting(page, "displayLanguage").selectOption("tailo");
  // trace: home.appHeaderTitle / web.placeholder / web.ready tailo values.
  await expect(heading(page)).toHaveText("Tâi-gí Khí-puânn");
  await expect(page).toHaveTitle("Tâi-gí Khí-puânn");
  await expect(editor).toHaveAttribute("placeholder", "Tī tsia phah-jī");
  await expect(page.locator("#message")).toHaveText("Ē-sái phah-jī");
  await expect(toggle(page)).toHaveAttribute("aria-label", "Siat-tīng");
  expect(await rootLang(page)).toBe("nan-Latn-tailo");
  await page.reload();
  await expect(heading(page)).toHaveText("Tâi-gí Khí-puânn");
  await expect(setting(page, "displayLanguage")).toHaveValue("tailo");
});

test("the session speaks the page's language", async ({ page }) => {
  // trace: Ctrl+Alt+C → mode_label, settings.pojMode + sideBySide + the output
  // script, each resolved in the Hanji map.
  const editor = await open(page, { settings: { displayLanguage: "hanji" } });
  await editor.press("Control+Alt+c");
  await expect(page.locator("#message")).toContainText("白話字 · 漢羅對應");
});

test("an unknown language tag reads as Hanji, as the session reads it", async ({
  page,
}) => {
  const editor = await open(page, { settings: { displayLanguage: "xx" } });
  await expect(heading(page)).toHaveText("台語齒盤");
  await editor.press("Control+Alt+c");
  await expect(page.locator("#message")).toContainText("白話字");
});

test("a language change drops the session's text", async ({ page }) => {
  const editor = await open(page);
  await editor.press("Control+Alt+c");
  await expect(page.locator("#message")).toContainText("Pe̍h-ōe-jī");
  await toggle(page).click();
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
  await expect(heading(page)).toBeHidden();
  await expect(page.getByRole("button", { name: "Copy" })).toBeHidden();
});

test("no font downloads until one is selected", async ({ page }) => {
  const fonts = [];
  page.on("request", (request) => {
    if (request.url().includes("/fonts/")) {
      fonts.push(request.url());
    }
  });
  const editor = await open(page);
  await commitWord(editor);
  await expect(editor).toHaveValue("台灣");
  expect(fonts).toEqual([]);
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
    await toggle(page).click();
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
  await editor.press("Control+Alt+c");
  await expect.poll(() => stored(page)).toMatchObject({ fontType: "iansui" });
  await expect(heading(page)).toHaveText("台湾語キーボード");
  await expect(toggle(page)).toHaveAttribute("title", "設定");
});
