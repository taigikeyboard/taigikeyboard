// Quill's Snow editor, pinned and served beside the fixture: the rich-text
// editor the extension is held to (docs/architecture/browser-extension.md).
// The document is read from Quill itself, not from the DOM.

import {
  A,
  candidates,
  commitWord,
  expect,
  openTyping,
  preedit,
  test,
} from "./helpers.mjs";

const QUILL = `${A}/quill.html`;
const EDITOR = ".ql-editor";

const contents = (page) => page.evaluate(() => window.quill.getContents().ops);
const text = (page) => page.evaluate(() => window.quill.getText());

test("a word composes beside the caret and commits into the document", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, QUILL, EDITOR);
  await page.keyboard.type("taiuan");
  await expect(preedit(page)).toContainText("taiuan");
  await expect(candidates(page).first()).toContainText("台灣");
  expect(await text(page)).toBe("\n");
  await page.keyboard.press("Enter");
  // Enter picked the candidate: Quill made no new line of it.
  await expect.poll(() => text(page)).toBe("台灣\n");
});

test("a commit takes the format armed at the caret", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, QUILL, EDITOR);
  await page.locator(".ql-bold").click();
  await commitWord(page);
  await expect
    .poll(() => contents(page))
    .toEqual([
      { insert: "台灣", attributes: { bold: true } },
      { insert: "\n" },
    ]);
});

test("a commit leaves the document around it as it was", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, QUILL, EDITOR);
  await page.evaluate(() => {
    window.quill.setContents([
      { insert: "Title" },
      { insert: "\n", attributes: { header: 1 } },
      { insert: "plain " },
      { insert: "bold", attributes: { bold: true } },
      { insert: " end\n" },
    ]);
    // Inside the bold word.
    window.quill.setSelection(14, 0);
  });
  await commitWord(page);
  await expect
    .poll(() => contents(page))
    .toEqual([
      { insert: "Title" },
      { insert: "\n", attributes: { header: 1 } },
      { insert: "plain " },
      { insert: "bo台灣ld", attributes: { bold: true } },
      { insert: " end\n" },
    ]);
  // The caret follows the commit.
  expect(await page.evaluate(() => window.quill.getSelection())).toEqual({
    index: 16,
    length: 0,
  });
});

test("a commit replaces the selection", async ({ context, worker, page }) => {
  await openTyping(context, worker, page, QUILL, EDITOR);
  await page.evaluate(() => {
    window.quill.setText("abcd\n");
    window.quill.setSelection(1, 2);
  });
  await commitWord(page);
  await expect.poll(() => text(page)).toBe("a台灣d\n");
});

test("undo and redo are Quill's own", async ({ context, worker, page }) => {
  await openTyping(context, worker, page, QUILL, EDITOR);
  await commitWord(page);
  await expect.poll(() => text(page)).toBe("台灣\n");
  await page.keyboard.press("ControlOrMeta+z");
  await expect.poll(() => text(page)).toBe("\n");
  await page.keyboard.press("ControlOrMeta+Shift+z");
  await expect.poll(() => text(page)).toBe("台灣\n");
  // Quill's history holds the commit as a step of its own.
  await page.evaluate(() => window.quill.history.undo());
  expect(await text(page)).toBe("\n");
});

test("Quill's own keys still work around a composition", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, QUILL, EDITOR);
  await commitWord(page);
  // With nothing composing, Enter is Quill's.
  await page.keyboard.press("Enter");
  await commitWord(page);
  await expect.poll(() => text(page)).toBe("台灣\n台灣\n");
  await page.keyboard.press("Backspace");
  await expect.poll(() => text(page)).toBe("台灣\n台\n");
});
