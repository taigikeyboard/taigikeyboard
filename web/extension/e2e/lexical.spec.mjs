// A stock third-party Lexical rich-text editor, pinned and served beside the
// fixture: a `beforeinput` + virtual-DOM reconciler editor, complementing
// Quill's `MutationObserver` + Parchment architecture (`quill.spec.mjs`).
// The document is read from Lexical's EditorState, not from the DOM.

import {
  A,
  candidates,
  commitWord,
  expect,
  openTyping,
  preedit,
  test,
} from "./helpers.mjs";

const LEXICAL = `${A}/lexical.html`;
const EDITOR = "#editor";

const blocks = (page) => page.evaluate(() => window.lexicalFixture.blocks());
const text = (page) => page.evaluate(() => window.lexicalFixture.text());

test("a word composes beside the caret and commits into the document", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, LEXICAL, EDITOR);
  await page.keyboard.type("taiuan");
  await expect(preedit(page)).toContainText("taiuan");
  await expect(candidates(page).first()).toContainText("台灣");
  expect(await text(page)).toBe("");
  await page.keyboard.press("Enter");
  // Enter picked the candidate: Lexical split no paragraph.
  await expect
    .poll(() => blocks(page))
    .toEqual([
      {
        type: "paragraph",
        runs: [{ text: "台灣", bold: false, italic: false, underline: false }],
      },
    ]);
});

test("a commit takes the format armed at the caret", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, LEXICAL, EDITOR);
  await page.locator("#bold").click();
  await commitWord(page);
  await expect
    .poll(() => blocks(page))
    .toEqual([
      {
        type: "paragraph",
        runs: [{ text: "台灣", bold: true, italic: false, underline: false }],
      },
    ]);
});

test("a commit leaves the document around it as it was", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, LEXICAL, EDITOR);
  await page.evaluate(() => window.lexicalFixture.setFormattedDocument());
  await commitWord(page);
  await expect
    .poll(() => blocks(page))
    .toEqual([
      {
        type: "h1",
        runs: [{ text: "Title", bold: false, italic: false, underline: false }],
      },
      {
        type: "paragraph",
        runs: [
          { text: "plain ", bold: false, italic: false, underline: false },
          { text: "bo台灣ld", bold: true, italic: false, underline: false },
          { text: " end", bold: false, italic: false, underline: false },
        ],
      },
    ]);
  // The caret sits right after the committed two-character word inside the bold run.
  expect(
    await page.evaluate(() => window.lexicalFixture.selectionOffsets()),
  ).toEqual({
    anchor: 4,
    focus: 4,
  });
});

test("a commit replaces the selection", async ({ context, worker, page }) => {
  await openTyping(context, worker, page, LEXICAL, EDITOR);
  await page.evaluate(() =>
    window.lexicalFixture.setSelectedText("abcd", 1, 3),
  );
  await commitWord(page);
  await expect.poll(() => text(page)).toBe("a台灣d");
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
  await openTyping(context, worker, page, LEXICAL, EDITOR);
  await page.keyboard.press("`");
  await commitWord(page);
  await expect.poll(() => text(page)).toBe("tâi-uân ");
  await page.keyboard.press(".");
  await expect.poll(() => text(page)).toBe("tâi-uân. ");

  // Moving the caret disarms the swap: no character is deleted unseen.
  await commitWord(page);
  await expect.poll(() => text(page)).toBe("tâi-uân. tâi-uân ");
  await page.keyboard.press("ArrowLeft");
  await page.keyboard.press("ArrowLeft");
  await page.keyboard.press(".");
  await expect.poll(() => text(page)).toBe("tâi-uân. tâi-uâ.n ");
});

test("undo and redo are Lexical's own", async ({ context, worker, page }) => {
  await openTyping(context, worker, page, LEXICAL, EDITOR);
  await commitWord(page);
  await expect.poll(() => text(page)).toBe("台灣");
  await page.keyboard.press("ControlOrMeta+z");
  await expect.poll(() => text(page)).toBe("");
  await page.keyboard.press("ControlOrMeta+Shift+z");
  await expect.poll(() => text(page)).toBe("台灣");
});

test("Lexical's own keys still work around a composition", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, LEXICAL, EDITOR);
  await commitWord(page);
  // With nothing composing, Enter is Lexical's.
  await page.keyboard.press("Enter");
  await commitWord(page);
  await expect
    .poll(() => blocks(page).then((list) => list.map((b) => b.runs[0]?.text)))
    .toEqual(["台灣", "台灣"]);
  await page.keyboard.press("Backspace");
  await expect
    .poll(() => blocks(page).then((list) => list.map((b) => b.runs[0]?.text)))
    .toEqual(["台灣", "台"]);
});
