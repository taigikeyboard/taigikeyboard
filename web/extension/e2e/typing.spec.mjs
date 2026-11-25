// Typing into a site's ordinary fields through the extension: real key
// events, the engine in the tab, the commit in the field
// (docs/architecture/browser-extension.md). Expected candidates are the web
// page's (`web/e2e/typing.spec.mjs`), which the same engine answers.

import {
  A,
  candidates,
  commitWord,
  expect,
  heard,
  openTyping,
  preedit,
  settled,
  storeSettings,
  test,
} from "./helpers.mjs";

const FIELDS = `${A}/fields.html`;

for (const selector of ["#text", "#area"]) {
  test(`a word composes beside the caret and commits into ${selector}`, async ({
    context,
    worker,
    page,
  }) => {
    const field = await openTyping(context, worker, page, FIELDS, selector);
    await page.keyboard.type("taiuan");
    await expect(preedit(page)).toContainText("taiuan");
    await expect(candidates(page).first()).toContainText("台灣");
    // Nothing reaches the field before the commit.
    await expect(field).toHaveValue("");
    await page.keyboard.press("Enter");
    await expect(field).toHaveValue("台灣");
    await expect(preedit(page)).toBeEmpty();
    await expect(candidates(page)).toHaveCount(0);
  });
}

test("the site hears none of the keys the engine took", async ({
  context,
  worker,
  page,
}) => {
  const field = await openTyping(context, worker, page, FIELDS, "#text");
  await commitWord(page);
  await expect(field).toHaveValue("台灣");
  expect(await heard(page)).toBe("");
});

test("the panel sits under the caret's line", async ({
  context,
  worker,
  page,
}) => {
  const field = await openTyping(context, worker, page, FIELDS, "#text");
  await page.keyboard.type("tai");
  const box = await field.boundingBox();
  const drawn = await preedit(page).boundingBox();
  const list = await page.locator("taigi-keyboard #panel").boundingBox();
  expect(box).not.toBeNull();
  expect(drawn).not.toBeNull();
  expect(list).not.toBeNull();
  const b = /** @type {NonNullable<typeof box>} */ (box);
  const d = /** @type {NonNullable<typeof drawn>} */ (drawn);
  const l = /** @type {NonNullable<typeof list>} */ (list);
  // The composition lies on the field's own line, the list right under it.
  expect(d.y).toBeGreaterThanOrEqual(b.y - 1);
  expect(d.y + d.height).toBeLessThanOrEqual(b.y + b.height + 1);
  expect(d.x).toBeGreaterThanOrEqual(b.x);
  expect(l.y).toBeGreaterThanOrEqual(d.y + d.height - 1);
});

/**
 * The panel's and the composition's boxes once the list has stopped
 * folding or unfolding. `settled` does not wait for the deferred `place`
 * that follows the animation's end, so this suits only tests whose caret
 * does not move during the fold or unfold.
 * @param {import("@playwright/test").Page} page
 */
async function settledBoxes(page) {
  await settled(page);
  const list = await page.locator("taigi-keyboard #panel").boundingBox();
  const drawn = await preedit(page).boundingBox();
  expect(list).not.toBeNull();
  expect(drawn).not.toBeNull();
  return {
    list: /** @type {NonNullable<typeof list>} */ (list),
    drawn: /** @type {NonNullable<typeof drawn>} */ (drawn),
  };
}

test("an unfolded list moves in from the window's right side", async ({
  context,
  worker,
  page,
}) => {
  await page.setViewportSize({ width: 360, height: 800 });
  await storeSettings(worker, { candidateLayout: "verticalExpandable" });
  await openTyping(context, worker, page, FIELDS, "#text");
  // The caret halfway along: the folded column fits right of it, the
  // unfolded grid does not.
  for (let i = 0; i < 5; i++) {
    await commitWord(page);
  }
  await page.keyboard.type("begua");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("ArrowRight");
  await expect(
    page.locator("taigi-keyboard #candidates[data-grid]"),
  ).toBeVisible();
  const { list, drawn } = await settledBoxes(page);
  // Left of the composition's start: the grid did not fit right of it, so
  // the window's side is what moved it.
  expect(list.x).toBeLessThan(drawn.x);
  expect(list.x).toBeGreaterThanOrEqual(0);
  expect(list.x + list.width).toBeLessThanOrEqual(360 + 0.5);
});

test("an unfolded list near the window's bottom goes above the composition", async ({
  context,
  worker,
  page,
}) => {
  await storeSettings(worker, { candidateLayout: "expandable" });
  const field = await openTyping(context, worker, page, FIELDS, "#text");
  // The field at the window's bottom: below its line there is no room, above
  // it room for the whole grid, so `Panel.place` puts the list above rather
  // than beside the composition.
  await field.evaluate((input) => {
    const gap = innerHeight - 12 - input.getBoundingClientRect().bottom;
    document.body.style.paddingTop = `${gap}px`;
  });
  await page.keyboard.type("siok");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("ArrowDown");
  await expect(
    page.locator("taigi-keyboard #candidates[data-grid]"),
  ).toBeVisible();
  const { list, drawn } = await settledBoxes(page);
  const viewport = /** @type {{ height: number }} */ (page.viewportSize());
  expect(list.y + list.height).toBeLessThanOrEqual(viewport.height + 0.5);
  expect(list.y + list.height).toBeLessThanOrEqual(drawn.y + 1);
});

test("a slot key and a click each pick their candidate", async ({
  context,
  worker,
  page,
}) => {
  const field = await openTyping(context, worker, page, FIELDS, "#text");
  await page.keyboard.type("taiuan");
  const second = candidates(page).nth(1);
  const label = (await second.locator(".index").textContent()) ?? "";
  const hanji = (await second.locator(".text").textContent()) ?? "";
  await page.keyboard.press(label);
  await expect(field).toHaveValue(hanji);

  await page.keyboard.type("taiuan");
  await candidates(page).filter({ hasText: "大員" }).first().click();
  // A click selects; the field keeps the focus, and Enter commits.
  await expect(field).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(field).toHaveValue(`${hanji}大員`);
});

test("Backspace edits the composition, then the field", async ({
  context,
  worker,
  page,
}) => {
  const field = await openTyping(context, worker, page, FIELDS, "#text");
  await commitWord(page);
  await page.keyboard.type("ta");
  await page.keyboard.press("Backspace");
  await expect(preedit(page)).toContainText("t");
  await expect(field).toHaveValue("台灣");
  await page.keyboard.press("Backspace");
  await expect(preedit(page)).toBeEmpty();
  // With nothing composing the key is the field's own.
  await page.keyboard.press("Backspace");
  await expect(field).toHaveValue("台");
});

test("a commit goes in at the caret and replaces a selection", async ({
  context,
  worker,
  page,
}) => {
  const field = await openTyping(context, worker, page, FIELDS, "#area");
  await field.fill("ab");
  await field.evaluate((element) =>
    /** @type {HTMLTextAreaElement} */ (element).setSelectionRange(1, 1),
  );
  await commitWord(page);
  await expect(field).toHaveValue("a台灣b");
  // The caret follows the commit.
  expect(
    await field.evaluate(
      (element) => /** @type {HTMLTextAreaElement} */ (element).selectionStart,
    ),
  ).toBe(3);

  await field.evaluate((element) =>
    /** @type {HTMLTextAreaElement} */ (element).setSelectionRange(0, 3),
  );
  await commitWord(page);
  await expect(field).toHaveValue("台灣b");
});

test("undo and redo are the field's own, one commit a step", async ({
  context,
  worker,
  page,
}) => {
  const field = await openTyping(context, worker, page, FIELDS, "#area");
  await commitWord(page);
  await commitWord(page);
  await expect(field).toHaveValue("台灣台灣");
  await page.keyboard.press("ControlOrMeta+z");
  await expect(field).toHaveValue("台灣");
  await page.keyboard.press("ControlOrMeta+z");
  await expect(field).toHaveValue("");
  await page.keyboard.press("ControlOrMeta+Shift+z");
  await expect(field).toHaveValue("台灣");
});

test("a romanized commit and its auto space are one undo step", async ({
  context,
  worker,
  page,
}) => {
  // A commit typed before the setting reached the tab would see auto space
  // off (`storeSettings`).
  await storeSettings(worker, { autoSpaceEnabled: true });
  const field = await openTyping(context, worker, page, FIELDS, "#area");
  await page.keyboard.press("`");
  await commitWord(page);
  await expect(field).toHaveValue("tâi-uân ");
  await page.keyboard.press("ControlOrMeta+z");
  await expect(field).toHaveValue("");
});

test("the site's own input handlers hear a commit", async ({
  context,
  worker,
  page,
}) => {
  const field = await openTyping(context, worker, page, FIELDS, "#text");
  await page.evaluate(() => {
    window.inputs = [];
    document.getElementById("text")?.addEventListener("input", (event) => {
      const inputEvent = /** @type {InputEvent} */ (event);
      window.inputs?.push([inputEvent.inputType, inputEvent.data]);
    });
  });
  await commitWord(page);
  await expect(field).toHaveValue("台灣");
  expect(await page.evaluate(() => window.inputs)).toEqual([
    ["insertText", "台灣"],
  ]);
});

test("a Shift tap switches to English, whose keys are the field's", async ({
  context,
  worker,
  page,
}) => {
  const field = await openTyping(context, worker, page, FIELDS, "#text");
  await page.keyboard.press("Shift");
  await expect(page.locator("taigi-keyboard #chip")).toHaveAttribute(
    "data-state",
    "english",
  );
  await page.keyboard.type("tai");
  await expect(field).toHaveValue("tai");
  await expect(preedit(page)).toBeEmpty();
  await page.keyboard.press("Shift");
  await page.keyboard.type("tai");
  await expect(preedit(page)).toContainText("tai");
});
