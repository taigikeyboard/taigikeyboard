// A stock CodeMirror 6 editor (pinned `@codemirror/*` packages bundled beside
// the fixture): `.cm-content` is a `contenteditable` host driven by
// `DOMObserver` inside a scrolling `.cm-scroller`. The document is read from
// `EditorView.state.doc`, not from the DOM.

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

/** @typedef {import("@playwright/test").Page} Page */

const CM6 = `${A}/codemirror6.html`;
const EDITOR = ".cm-content";

/** @param {Page} page */
const text = (page) => page.evaluate(() => window.cm6Fixture.text());
/** @param {Page} page */
const selection = (page) => page.evaluate(() => window.cm6Fixture.selection());

test("a word composes beside the caret and commits into CodeMirror 6's EditorState", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, CM6, EDITOR);
  await page.keyboard.type("taiuan");
  await expect(preedit(page)).toContainText("taiuan");
  await expect(candidates(page).first()).toContainText("台灣");
  expect(await text(page)).toBe("");
  await page.keyboard.press("Enter");
  await expect.poll(() => text(page)).toBe("台灣");
  expect(await selection(page)).toEqual({ anchor: 2, head: 2 });
});

test("the chip sits under .cm-editor whether .cm-content is short or overflows .cm-scroller", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, CM6, EDITOR);
  const cmBox = await page.locator(".cm-editor").boundingBox();
  const shortChip = await chip(page).boundingBox();
  expect(cmBox).not.toBeNull();
  expect(shortChip).not.toBeNull();
  const cm = /** @type {NonNullable<typeof cmBox>} */ (cmBox);
  const sc = /** @type {NonNullable<typeof shortChip>} */ (shortChip);
  expect(sc.y).toBeGreaterThanOrEqual(cm.y + cm.height);
  expect(sc.y).toBeLessThan(cm.y + cm.height + 12);
  expect(Math.abs(sc.x + sc.width - (cm.x + cm.width))).toBeLessThan(2);

  // Forty lines make `.cm-content` much taller than `.cm-scroller`: the chip
  // stays anchored to the visible `.cm-editor` box rather than running off
  // with the unclipped content height.
  const fortyLines = Array.from(
    { length: 40 },
    (_, index) => `const line${index} = ${index};`,
  ).join("\n");
  await page.evaluate(
    (doc) => window.cm6Fixture.setDocument(doc, 6),
    fortyLines,
  );
  await page.keyboard.type("tai");
  const tallChip = await chip(page).boundingBox();
  expect(tallChip).not.toBeNull();
  const tc = /** @type {NonNullable<typeof tallChip>} */ (tallChip);
  expect(tc.y).toBeGreaterThanOrEqual(cm.y + cm.height);
  expect(tc.y).toBeLessThan(cm.y + cm.height + 12);
});

test("a commit replaces the selection", async ({ context, worker, page }) => {
  await openTyping(context, worker, page, CM6, EDITOR);
  await page.evaluate(() => window.cm6Fixture.setSelectedText("abcd", 1, 3));
  await commitWord(page);
  await expect.poll(() => text(page)).toBe("a台灣d");
  expect(await selection(page)).toEqual({ anchor: 3, head: 3 });
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
  await openTyping(context, worker, page, CM6, EDITOR);
  await page.keyboard.press("`");
  await commitWord(page);
  await expect.poll(() => text(page)).toBe("tâi-uân ");
  await page.keyboard.press(".");
  await expect.poll(() => text(page)).toBe("tâi-uân. ");

  // Moving the caret disarms the swap.
  await commitWord(page);
  await expect.poll(() => text(page)).toBe("tâi-uân. tâi-uân ");
  await page.keyboard.press("ArrowLeft");
  await page.keyboard.press("ArrowLeft");
  await page.keyboard.press(".");
  await expect.poll(() => text(page)).toBe("tâi-uân. tâi-uâ.n ");
});

test("undo and redo are CodeMirror 6's own", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, CM6, EDITOR);
  await commitWord(page);
  await expect.poll(() => text(page)).toBe("台灣");
  await page.keyboard.press("ControlOrMeta+z");
  await expect.poll(() => text(page)).toBe("");
  await page.keyboard.press("ControlOrMeta+Shift+z");
  await expect.poll(() => text(page)).toBe("台灣");
});

test("CodeMirror 6's own keys still work around a composition", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, CM6, EDITOR);
  await commitWord(page);
  await page.keyboard.press("Enter");
  await commitWord(page);
  await expect.poll(() => text(page)).toBe("台灣\n台灣");
  await page.keyboard.press("Backspace");
  await expect.poll(() => text(page)).toBe("台灣\n台");
});
