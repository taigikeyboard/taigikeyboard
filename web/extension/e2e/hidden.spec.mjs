// Canvas and hidden-field editors that draw their own text (`EditContext`,
// 1px transparent `.inputarea`, zero-opacity and clipped `<textarea>`s,
// off-screen `<iframe>` and Google Sheets' off-screen cell singleton): every
// one is detected as unsupported with its keys left to the site, and its chip
// sits beside the visible editor container or in the viewport's bottom-right
// corner — never clamped to `(0, 0)` (docs/architecture/browser-extension.md
// § Fields).

import {
  A,
  chip,
  commitWord,
  expect,
  expectToolbar,
  heard,
  openTyping,
  preedit,
  test,
  turnOn,
} from "./helpers.mjs";

const HIDDEN = `${A}/hidden.html`;

async function expectChipUnder(page, selector) {
  const box = await page.locator(selector).boundingBox();
  const drawn = await chip(page).boundingBox();
  expect(drawn.y).toBeGreaterThanOrEqual(box.y + box.height);
  expect(drawn.y).toBeLessThan(box.y + box.height + 12);
  expect(Math.abs(drawn.x + drawn.width - (box.x + box.width))).toBeLessThan(2);
}

async function expectChipInCorner(page) {
  const viewport = page.viewportSize();
  const drawn = await chip(page).boundingBox();
  expect(drawn.x).toBeGreaterThan(viewport.width - drawn.width - 16);
  expect(drawn.y).toBeGreaterThan(viewport.height - drawn.height - 16);
}

test("an EditContext editor (Monaco in Chromium) is unsupported, keeps its keys, and anchors its chip to the editor", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, HIDDEN, "#docs-title");
  await page.evaluate(() => {
    window.hiddenFixture.clearLog();
    document.getElementById("editcontext-target").focus();
  });

  await expect(chip(page)).toHaveAttribute("data-state", "unsupported");
  await expect(chip(page)).toHaveText("Field not supported");
  await expectToolbar(worker, page, "×");
  await expectChipUnder(page, "#monaco-editcontext");

  await page.keyboard.type("taiuan");
  await expect(preedit(page)).toBeEmpty();
  expect(
    await page.evaluate(() => window.hiddenFixture.editContextText()),
  ).toBe("taiuan");
  expect(await heard(page)).toContain("keydown:t\nkeyup:t\n");

  // A Shift tap in an unsupported EditContext field does not flip Taigi mode.
  await page.keyboard.press("Shift");
  await page.locator("#docs-title").click();
  await expect(chip(page)).toHaveAttribute("data-state", "taigi");

  // A read-only EditContext viewer draws no chip at all.
  await page.evaluate(() => {
    window.hiddenFixture.setEditContextReadOnly(true);
    document.getElementById("editcontext-target").focus();
  });
  await expect(chip(page)).toBeHidden();
});

test("hidden textareas (1px transparent, zero-opacity, zero-height clip) are unsupported and anchor to their editor boxes", async ({
  context,
  worker,
  page,
}) => {
  await page.goto(HIDDEN);
  await turnOn(context, worker, page);

  for (const [target, box] of [
    ["#monaco-inputarea", "#monaco-textarea"],
    ["#opacity-zero", "#ace-box"],
    ["#clipped-area", "#clipped-box"],
  ]) {
    await page.evaluate((selector) => {
      window.hiddenFixture.clearLog();
      document.querySelector(selector).focus();
    }, target);
    await expect(chip(page)).toHaveAttribute("data-state", "unsupported");
    await expectToolbar(worker, page, "×");
    await expectChipUnder(page, box);

    await page.keyboard.type("tai");
    await expect(preedit(page)).toBeEmpty();
    await expect(page.locator(target)).toHaveValue("tai");
    expect(await heard(page)).toContain("keydown:t\nkeyup:t\n");
  }
});

test("an off-screen iframe (Google Docs) places its chip in the corner while top-frame inputs compose", async ({
  context,
  worker,
  page,
}) => {
  await page.goto(HIDDEN);
  await turnOn(context, worker, page);

  await page.evaluate(() => window.hiddenFixture.focusDocsIframe());
  await expect(chip(page)).toHaveAttribute("data-state", "unsupported");
  await expectToolbar(worker, page, "×");
  await expectChipInCorner(page);

  const title = page.locator("#docs-title");
  await title.click();
  await expect(chip(page)).toHaveAttribute("data-state", "taigi");
  await commitWord(page);
  await expect(title).toHaveValue("台灣");
});

test("Google Sheets' singleton cell editor stays unsupported across off-screen navigation and cell editing", async ({
  context,
  worker,
  page,
}) => {
  await page.goto(HIDDEN);
  await turnOn(context, worker, page);

  // While navigating cells, `contenteditable` is `"false"` and the singleton
  // sits at `top: -10000px`: even the very first key must reach the grid raw.
  await page.evaluate(() => {
    window.hiddenFixture.clearLog();
    window.hiddenFixture.focusSheetsGrid();
  });
  await expect(chip(page)).toHaveAttribute("data-state", "unsupported");
  await expectToolbar(worker, page, "×");
  await expectChipInCorner(page);

  await page.keyboard.type("taiuan");
  await expect(preedit(page)).toBeEmpty();
  await expectChipUnder(page, "#waffle-rich-text-editor");
  await page.keyboard.press("Enter");
  expect(await page.evaluate(() => window.hiddenFixture.sheetsCellText())).toBe(
    "taiuan",
  );
  // Enter parked the singleton back at `top: -10000px` without blurring it:
  // the chip returns to the viewport corner instead of clamping to `(0, 0)`.
  await expectChipInCorner(page);
});

test("a focused field reflects dynamic visibility changes, while visible fixed and transparent-fill editors compose", async ({
  context,
  worker,
  page,
}) => {
  await page.goto(HIDDEN);
  await turnOn(context, worker, page);

  // Focused while off-screen -> unsupported; moved on-screen -> composes on
  // the next key; moved off-screen mid-composition -> drops composition.
  const dynamic = page.locator("#dynamic-input");
  await page.evaluate(() => document.getElementById("dynamic-input").focus());
  await expect(chip(page)).toHaveAttribute("data-state", "unsupported");
  await page.evaluate(() =>
    document.getElementById("dynamic-input").classList.remove("offscreen"),
  );
  await commitWord(page);
  await expect(dynamic).toHaveValue("台灣");
  await expect(chip(page)).toHaveAttribute("data-state", "taigi");

  await page.keyboard.type("tai");
  await expect(preedit(page)).toContainText("tai");
  await page.evaluate(() =>
    document.getElementById("dynamic-input").classList.add("offscreen"),
  );
  await expect(preedit(page)).toBeEmpty();
  await expect(chip(page)).toHaveAttribute("data-state", "unsupported");

  // A `position: fixed` search bar inside an `overflow: hidden` header is not
  // clipped by that header.
  const palette = page.locator("#palette-input");
  await palette.click();
  await expect(chip(page)).toHaveAttribute("data-state", "taigi");
  await commitWord(page);
  await expect(palette).toHaveValue("台灣");

  // A syntax-highlight overlay `<textarea>` (`color: transparent` with a
  // visible `caret-color`) composes normally.
  const overlayCode = page.locator("#overlay-code");
  await overlayCode.click();
  await expect(chip(page)).toHaveAttribute("data-state", "taigi");
  await commitWord(page);
  await expect(overlayCode).toHaveValue("台灣");
});
