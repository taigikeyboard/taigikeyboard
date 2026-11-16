// A stock Slate editor shaped like a chat message box (Discord's is Slate).
// Slate edits its model from `beforeinput` alone, which Chromium's
// `execCommand` does not fire: `EditableWrites` dispatches it. The message is
// read from Slate's model, not from the DOM.

import {
  A,
  candidates,
  commitWord,
  expect,
  openTyping,
  preedit,
  test,
} from "./helpers.mjs";

/** @typedef {import("@playwright/test").Page} Page */

const SLATE = `${A}/slate.html`;
const EDITOR = "#editor";

/** @param {Page} page */
const text = (page) => page.evaluate(() => window.slateFixture.text());
/** @param {Page} page */
const sent = (page) => page.evaluate(() => window.slateFixture.sent());

test("a word composes beside the caret and commits into Slate's model", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, SLATE, EDITOR);
  await page.keyboard.type("taiuan");
  await expect(preedit(page)).toContainText("taiuan");
  await expect(candidates(page).first()).toContainText("台灣");
  expect(await text(page)).toBe("");
  await page.keyboard.press("Enter");
  await expect.poll(() => text(page)).toBe("台灣");
  // Enter picked the candidate: nothing was sent.
  expect(await sent(page)).toEqual([]);
  // The model's caret follows the commit.
  await expect
    .poll(() => page.evaluate(() => window.slateFixture.selection()))
    .toEqual({ anchor: 2, focus: 2 });
  // The DOM is Slate's: one copy of the text, and no placeholder over it.
  await expect(page.locator(EDITOR)).toHaveText("台灣");
});

test("commits in a row and punctuation all reach the message that is sent", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, SLATE, EDITOR);
  await commitWord(page);
  await commitWord(page);
  await page.keyboard.press(",");
  await expect.poll(() => text(page)).toBe("台灣台灣，");
  // With nothing composing, Enter is the site's: it sends the message.
  await page.keyboard.press("Enter");
  await expect.poll(() => sent(page)).toEqual(["台灣台灣，"]);
  await expect.poll(() => text(page)).toBe("");
  // The emptied box takes the next commit.
  await commitWord(page);
  await expect.poll(() => text(page)).toBe("台灣");
});

test("a commit replaces the selection", async ({ context, worker, page }) => {
  await openTyping(context, worker, page, SLATE, EDITOR);
  await page.evaluate(() => window.slateFixture.setSelectedText("abcd", 1, 3));
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
  await openTyping(context, worker, page, SLATE, EDITOR);
  await page.keyboard.press("`");
  await commitWord(page);
  await expect.poll(() => text(page)).toBe("tâi-uân ");
  await page.keyboard.press(".");
  await expect.poll(() => text(page)).toBe("tâi-uân. ");
});

test("undo and redo are Slate's own", async ({ context, worker, page }) => {
  await openTyping(context, worker, page, SLATE, EDITOR);
  await commitWord(page);
  await expect.poll(() => text(page)).toBe("台灣");
  await page.keyboard.press("ControlOrMeta+z");
  await expect.poll(() => text(page)).toBe("");
  await page.keyboard.press("ControlOrMeta+Shift+z");
  await expect.poll(() => text(page)).toBe("台灣");
});

test("Slate's own keys still work around a composition", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, SLATE, EDITOR);
  await commitWord(page);
  await page.keyboard.press("Shift+Enter");
  await commitWord(page);
  await expect.poll(() => text(page)).toBe("台灣\n台灣");
  await page.keyboard.press("Backspace");
  await expect.poll(() => text(page)).toBe("台灣\n台");
});
