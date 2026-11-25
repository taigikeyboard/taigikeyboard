// The options page's Shortcuts section: the page's own session draws it, so it
// is there before typing has ever been turned on, and follows the form.

import { STRINGS } from "../../src/generated/strings.js";
import { expect, test, toggleShortcut } from "./helpers.mjs";

/** @type {ReadonlyArray<readonly [string, string]>} */
const CAPTIONED_GENERAL_ROWS = [
  ["toneInputScheme", "settings.toneSchemeStandardInfo"],
  ["hanLoMixedEnabled", "settings.hanLoMixedInfo"],
  ["literalRomanCandidateEnabled", "desktop.literalRomanCandidateInfo"],
  ["outputBothScripts", "settings.outputBothScriptsInfo"],
  ["khinsiannInHanjiEnabled", "settings.khinsiannInHanjiInfo"],
  ["syllableSeparator", "settings.syllableSeparatorInfo"],
  ["autoSpaceEnabled", "settings.autoSpaceInfo"],
];

/**
 * @param {keyof typeof STRINGS} lang
 * @param {string} key
 * @returns {string}
 */
function stringFor(lang, key) {
  return /** @type {Record<string, string>} */ (STRINGS[lang])[key];
}

test("the Shortcuts section is drawn on a new install and follows the form", async ({
  context,
  worker,
}) => {
  const id = new URL(worker.url()).host;
  const shortcut = await toggleShortcut(worker);
  const page = await context.newPage();
  await page.goto(`chrome-extension://${id}/options.html`);
  const section = page.locator("#shortcuts");

  // trace: `shortcuts(false)` keeps Switch Phonetic Symbols and drops the
  // Telex guide and TPS key panel rows; the browser command joins Other.
  await expect(section).toContainText("Switch Phonetic Symbols");
  await expect(section).not.toContainText("Open Telex Guide");
  await expect(section).not.toContainText("Mini Keyboard");
  const toggle = section
    .locator(".setting")
    .filter({ hasText: "Start or stop typing Taigi in this tab" });
  await expect(toggle.locator(".setting-keys")).toHaveText(shortcut);

  // trace: under TPS a cell is the Hanji alone (`fixed_keys_label` → None),
  // so the other-script row loses its Shift run.
  const other = section
    .locator(".setting")
    .filter({ hasText: "Output the Other Script" });
  await expect(other).toContainText("QWDFZXVY");
  await page.locator('select[name="inputMode"]').selectOption("tps");
  await expect(other).not.toContainText("QWDFZXVY");
});

test("inline captions render, follow Tone Keys and Display Language, and dim when disabled", async ({
  context,
  worker,
}) => {
  const id = new URL(worker.url()).host;
  const page = await context.newPage();
  await page.goto(`chrome-extension://${id}/options.html`);

  /** @param {string} name */
  const rowOf = (name) =>
    page.locator("#settings .setting", {
      has: page.locator(`[name="${name}"]`),
    });

  for (const [name, captionKey] of CAPTIONED_GENERAL_ROWS) {
    const note = rowOf(name).locator("span[data-i18n] > small");
    await expect(note).toHaveCount(1);
    await expect(note).toHaveText(stringFor("en", captionKey));
  }
  for (const name of ["inputMode", "cAffricateEnabled", "displayLanguage"]) {
    await expect(rowOf(name).locator("span[data-i18n] > small")).toHaveCount(0);
  }
  await expect(page.locator("#shortcuts")).toContainText("QWDFZXVY");

  const toneSelect = page.locator('select[name="toneInputScheme"]');
  const toneNote = rowOf("toneInputScheme").locator("span[data-i18n] > small");
  await toneSelect.selectOption("telex");
  await expect(toneNote).toHaveText(
    stringFor("en", "settings.toneSchemeTelexInfo"),
  );
  await toneSelect.selectOption("standard");
  await expect(toneNote).toHaveText(
    stringFor("en", "settings.toneSchemeStandardInfo"),
  );

  for (const lang of /** @type {const} */ ([
    "hanji",
    "tailo",
    "poj",
    "ja",
    "en",
  ])) {
    await page.locator('select[name="displayLanguage"]').selectOption(lang);
    for (const [name, captionKey] of CAPTIONED_GENERAL_ROWS) {
      await expect(rowOf(name).locator("span[data-i18n] > small")).toHaveText(
        stringFor(lang, captionKey),
      );
    }
    await toneSelect.selectOption("telex");
    await expect(toneNote).toHaveText(
      stringFor(lang, "settings.toneSchemeTelexInfo"),
    );
    await toneSelect.selectOption("standard");
  }

  await page
    .locator('select[name="candidateDisplayMode"]')
    .selectOption("romanOnly");
  for (const name of [
    "hanLoMixedEnabled",
    "outputBothScripts",
    "khinsiannInHanjiEnabled",
  ]) {
    const row = rowOf(name);
    await expect(row.locator("input")).toBeDisabled();
    const opacity = await row
      .locator("span[data-i18n]")
      .evaluate((el) => Number.parseFloat(getComputedStyle(el).opacity));
    expect(opacity).toBeLessThan(1);
  }
  const enabledOpacity = await rowOf("autoSpaceEnabled")
    .locator("span[data-i18n]")
    .evaluate((el) => Number.parseFloat(getComputedStyle(el).opacity));
  expect(enabledOpacity).toBe(1);
});
