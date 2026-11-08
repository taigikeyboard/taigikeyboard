// docs/architecture/web-roadmap.md § Text surface, one test per row.

import { expect, test } from "@playwright/test";

import { commitWord, open, panel, preedit } from "./helpers.mjs";

const AUTO_SPACE = { autoSpaceEnabled: true };

/// Under the Hanji/romanization swap, the output that earns an auto space.
async function commitRomanization(editor) {
  await editor.press("`");
  await commitWord(editor);
}

function caretTo(editor, position) {
  return editor.evaluate((element, caret) => {
    element.setSelectionRange(caret, caret);
  }, position);
}

test("a click in the textarea commits, then moves the caret", async ({
  page,
}) => {
  const editor = await open(page);
  await commitWord(editor);
  await editor.pressSequentially("tai");
  await editor.click({ position: { x: 4, y: 4 } });
  await expect(panel(page)).toBeHidden();
  // Committed after the click, it would land at the start.
  await expect(editor).toHaveValue("台灣tai");
  const caret = await editor.evaluate((element) => element.selectionStart);
  expect(caret).toBeLessThan(5);
});

test("a click elsewhere commits before focus leaves", async ({ page }) => {
  const editor = await open(page);
  await editor.pressSequentially("tai");
  await page.getByRole("heading", { level: 1 }).click();
  await expect(editor).toHaveValue("tai");
  await expect(panel(page)).toBeHidden();
});

test("the auto-space swap survives the page's own write", async ({ page }) => {
  // trace: only romanization earns the space (`earns_auto_space`); `.` then
  // `swap_preceding_space`. A write of the page's own must not disarm it.
  const editor = await open(page, { settings: AUTO_SPACE });
  await commitRomanization(editor);
  await expect(editor).toHaveValue("tâi-uân ");
  await editor.press(".");
  await expect(editor).toHaveValue("tâi-uân. ");
});

// The swap would delete the character before the caret, unseen.
async function expectNothingLost(editor) {
  const typed = await editor.inputValue();
  expect(typed.replace(/[.。]/, "")).toBe("tâi-uân ");
  expect(typed).toMatch(/[.。]/);
}

test("a click that moves the caret, then punctuation: no text is lost", async ({
  page,
}) => {
  const editor = await open(page, { settings: AUTO_SPACE });
  await commitRomanization(editor);
  await editor.click({ position: { x: 4, y: 4 } });
  await editor.press(".");
  await expectNothingLost(editor);
});

test("a click that leaves the caret, then punctuation: no swap", async ({
  page,
}) => {
  // Caret and text compare equal here: only `pointerdown` disarms the swap.
  const editor = await open(page, { settings: AUTO_SPACE });
  await commitRomanization(editor);
  await page.getByRole("heading", { level: 1 }).click();
  await editor.focus();
  await editor.press(".");
  await expect(editor).toHaveValue(/^tâi-uân [.。]/);
});

test("a text change under a still caret, then punctuation: no swap", async ({
  page,
}) => {
  const editor = await open(page, { settings: AUTO_SPACE });
  await commitRomanization(editor);
  await editor.evaluate((element) => {
    const end = element.value.length;
    element.setRangeText("x", end - 1, end, "end");
  });
  await editor.press(".");
  await expect(editor).toHaveValue(/^tâi-uânx[.。]/);
});

test("an arrow key, then punctuation: no text is lost", async ({ page }) => {
  const editor = await open(page, { settings: AUTO_SPACE });
  await commitRomanization(editor);
  await editor.press("ArrowLeft");
  await editor.press(".");
  await expectNothingLost(editor);
});

test("a caret moved by script, then punctuation: no text is lost", async ({
  page,
}) => {
  // No key or pointer announces this one: only the comparison at the next key
  // sees it.
  const editor = await open(page, { settings: AUTO_SPACE });
  await commitRomanization(editor);
  await caretTo(editor, 3);
  await editor.press(".");
  await expectNothingLost(editor);
});

test("a commit replaces the selection", async ({ page }) => {
  const editor = await open(page);
  await commitWord(editor);
  await commitWord(editor);
  await expect(editor).toHaveValue("台灣台灣");
  await editor.evaluate((element) => element.setSelectionRange(0, 2));
  await commitWord(editor);
  await expect(editor).toHaveValue("台灣台灣");
  await editor.evaluate((element) => element.select());
  await editor.pressSequentially("tai");
  await editor.press("q");
  await expect(editor).not.toHaveValue(/台灣/);
});

test("the panel follows the caret and the scroll", async ({ page }) => {
  // The textarea takes the window's room left over: a tall window gives the
  // list room below the caret at both scroll positions.
  await page.setViewportSize({ width: 1280, height: 1000 });
  const editor = await open(page);
  await editor.fill("\n".repeat(60));
  await editor.evaluate((element) => element.setSelectionRange(30, 30));
  await editor.pressSequentially("tai");
  const scrollTo = (position) =>
    editor.evaluate(async (element, top) => {
      element.scrollTop = top;
      await new Promise(requestAnimationFrame);
      await new Promise(requestAnimationFrame);
    }, position);
  const panelTop = () => panel(page).evaluate((element) => element.offsetTop);
  // trace: line 30 at 16 + 30 × 30 = 916px; 856 and 806 put it 60px and
  // 110px down the box.
  await scrollTo(856);
  const before = await panelTop();
  await scrollTo(806);
  expect(await panelTop()).toBe(before + 50);
});

test("the panel moves along the line as text is committed", async ({
  page,
}) => {
  const editor = await open(page);
  await editor.pressSequentially("tai");
  const left = () => panel(page).evaluate((element) => element.offsetLeft);
  const start = await left();
  await editor.press("Enter");
  await editor.pressSequentially("tai");
  expect(await left()).toBeGreaterThan(start);
});

test("a refused write says what it could not write", async ({ page }) => {
  const editor = await open(page);
  await page.evaluate(() => {
    document.execCommand = () => false;
  });
  await commitWord(editor);
  await expect(page.locator("#message")).toContainText("台灣");
  await expect(panel(page)).toBeHidden();
});

test("a Shift-click is no tap", async ({ page }) => {
  const editor = await open(page);
  await page.keyboard.down("Shift");
  await editor.click({ position: { x: 4, y: 4 } });
  await page.keyboard.up("Shift");
  // Still composing: a tap would have handed the key to the page.
  await editor.pressSequentially("a");
  await expect(preedit(page)).toContainText("a");
});

test("a Shift held across the window losing focus is no tap", async ({
  page,
}) => {
  const editor = await open(page);
  await page.keyboard.down("Shift");
  await page.evaluate(() => window.dispatchEvent(new Event("blur")));
  await page.keyboard.up("Shift");
  await expect(editor).toBeFocused();
  await editor.pressSequentially("a");
  await expect(preedit(page)).toContainText("a");
});

test("the window losing focus writes nothing", async ({ page }) => {
  const editor = await open(page);
  await editor.pressSequentially("tai");
  await page.evaluate(() => window.dispatchEvent(new Event("blur")));
  await expect(editor).toHaveValue("");
  await expect(preedit(page)).toContainText("tai");
  await editor.pressSequentially("uan");
  await expect(preedit(page)).toContainText("taiuan");
});
