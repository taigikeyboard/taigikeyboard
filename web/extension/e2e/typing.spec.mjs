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
  // The composition lies on the field's own line, the list right under it.
  expect(drawn.y).toBeGreaterThanOrEqual(box.y - 1);
  expect(drawn.y + drawn.height).toBeLessThanOrEqual(box.y + box.height + 1);
  expect(drawn.x).toBeGreaterThanOrEqual(box.x);
  expect(list.y).toBeGreaterThanOrEqual(drawn.y + drawn.height - 1);
});

test("a slot key and a click each pick their candidate", async ({
  context,
  worker,
  page,
}) => {
  const field = await openTyping(context, worker, page, FIELDS, "#text");
  await page.keyboard.type("taiuan");
  const second = candidates(page).nth(1);
  const label = await second.locator(".index").textContent();
  const hanji = await second.locator(".text").textContent();
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
  await field.evaluate((element) => element.setSelectionRange(1, 1));
  await commitWord(page);
  await expect(field).toHaveValue("a台灣b");
  // The caret follows the commit.
  expect(await field.evaluate((element) => element.selectionStart)).toBe(3);

  await field.evaluate((element) => element.setSelectionRange(0, 3));
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

  // A romanized commit and its auto space join into one undo step too.
  await worker.evaluate(() =>
    chrome.storage.sync.set({
      "taigi.settings": JSON.stringify({
        revision: 1,
        values: { autoSpaceEnabled: true },
      }),
    }),
  );
  await page.keyboard.press("`");
  await commitWord(page);
  await expect(field).toHaveValue("台灣tâi-uân ");
  await page.keyboard.press("ControlOrMeta+z");
  await expect(field).toHaveValue("台灣");
});

test("the site's own input handlers hear a commit", async ({
  context,
  worker,
  page,
}) => {
  const field = await openTyping(context, worker, page, FIELDS, "#text");
  await page.evaluate(() => {
    window.inputs = [];
    document.getElementById("text").addEventListener("input", (event) => {
      window.inputs.push([event.inputType, event.data]);
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
