// A stock CodeMirror 5 editor (`inputStyle: "textarea"`, pinned and served
// beside the fixture): keys focus a `1000px` `<textarea>` clipped inside a
// `3px × 0px` wrapper, `TextareaInput` diffs `textarea.value` against its
// `prevInput`, and the visible caret is `.CodeMirror-cursor`.

import {
  A,
  candidates,
  chip,
  commitWord,
  expect,
  openTyping,
  preedit,
  test,
} from "./helpers.mjs";

const CM5 = `${A}/codemirror5.html`;
const EDITOR = ".CodeMirror";

const text = (page) => page.evaluate(() => window.cm5Fixture.text());
const cursor = (page) => page.evaluate(() => window.cm5Fixture.cursor());

test("a word composes at CodeMirror 5's cursor and the chip sits under .CodeMirror", async ({
  context,
  worker,
  page,
}) => {
  const editor = await openTyping(context, worker, page, CM5, EDITOR);
  await page.evaluate(() =>
    window.cm5Fixture.setDocument("function f() {\n  const x = ;\n}", 1, 12),
  );

  // The chip sits under `.CodeMirror`, never off to the right of its `1000px`
  // hidden `<textarea>`.
  const editorBox = await editor.boundingBox();
  const chipBox = await chip(page).boundingBox();
  expect(chipBox.y).toBeGreaterThanOrEqual(editorBox.y + editorBox.height);
  expect(chipBox.y).toBeLessThan(editorBox.y + editorBox.height + 12);
  expect(
    Math.abs(chipBox.x + chipBox.width - (editorBox.x + editorBox.width)),
  ).toBeLessThan(2);

  const cmCursor = await page
    .locator(".CodeMirror-cursors > .CodeMirror-cursor")
    .first()
    .boundingBox();
  await page.keyboard.type("taiuan");
  await expect(preedit(page)).toContainText("taiuan");
  await expect(candidates(page).first()).toContainText("台灣");
  expect(await text(page)).toBe("function f() {\n  const x = ;\n}");

  const drawn = await preedit(page).boundingBox();
  expect(Math.abs(drawn.x - cmCursor.x)).toBeLessThan(4);
  expect(Math.abs(drawn.y - cmCursor.y)).toBeLessThan(6);

  // The composition takes `.CodeMirror-code`'s monospace font, not the browser
  // default on the hidden `<textarea>`.
  const codeFamily = await page
    .locator(".CodeMirror-code")
    .evaluate((el) => getComputedStyle(el).fontFamily);
  const preeditFamily = await preedit(page).evaluate(
    (el) => getComputedStyle(el).fontFamily,
  );
  expect(preeditFamily).toBe(codeFamily);

  await page.keyboard.press("Enter");
  await expect
    .poll(() => text(page))
    .toBe("function f() {\n  const x = 台灣;\n}");
  expect(await cursor(page)).toEqual({ line: 1, ch: 14 });
});

test("a commit replaces a selection in CodeMirror 5", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, CM5, EDITOR);
  await page.evaluate(() =>
    window.cm5Fixture.setSelection(
      "const s = 'abcd';",
      { line: 0, ch: 12 },
      { line: 0, ch: 14 },
    ),
  );
  await page.keyboard.type("tai");
  const selectedBox = await page
    .locator(".CodeMirror-selected")
    .first()
    .boundingBox();
  const drawn = await preedit(page).boundingBox();
  expect(Math.abs(drawn.x - (selectedBox.x + selectedBox.width))).toBeLessThan(
    4,
  );
  await page.keyboard.type("uan");
  await page.keyboard.press("Enter");
  await expect.poll(() => text(page)).toBe("const s = 'a台灣d';");
  expect(await cursor(page)).toEqual({ line: 0, ch: 14 });
});

test("the auto-space swap replaces the trailing space in one edit", async ({
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
  await openTyping(context, worker, page, CM5, EDITOR);
  await page.keyboard.press("`");
  await commitWord(page);
  await expect.poll(() => text(page)).toBe("tâi-uân ");
  await page.keyboard.press(".");
  await expect.poll(() => text(page)).toBe("tâi-uân. ");

  // Moving CodeMirror 5's cursor disarms the swap.
  await commitWord(page);
  await expect.poll(() => text(page)).toBe("tâi-uân. tâi-uân ");
  await page.keyboard.press("ArrowLeft");
  await page.keyboard.press("ArrowLeft");
  await page.keyboard.press(".");
  await expect.poll(() => text(page)).toBe("tâi-uân. tâi-uâ.n ");
});

test("undo and redo are CodeMirror 5's own", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, CM5, EDITOR);
  await commitWord(page);
  await expect.poll(() => text(page)).toBe("台灣");
  await page.keyboard.press("ControlOrMeta+z");
  await expect.poll(() => text(page)).toBe("");
  await page.keyboard.press("ControlOrMeta+Shift+z");
  await expect.poll(() => text(page)).toBe("台灣");
});

test("read-only CodeMirror 5 is left alone and a dialog input inside it still composes", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, CM5, EDITOR);
  await commitWord(page);
  await page.keyboard.press("Enter");
  await commitWord(page);
  await expect.poll(() => text(page)).toBe("台灣\n台灣");

  // A visible `<input>` inside `.CodeMirror` (such as `addon/dialog`) is an
  // ordinary `"input"`, not the hidden editor `<textarea>`.
  await page.evaluate(() => window.cm5Fixture.openDialogInput());
  await expect(chip(page)).toHaveAttribute("data-state", "taigi");
  await commitWord(page);
  await expect(page.locator("#cm-find")).toHaveValue("台灣");

  await page.evaluate(() => window.cm5Fixture.setReadOnly(true));
  await expect(chip(page)).toBeHidden();
});
