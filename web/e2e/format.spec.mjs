// The format bubble over a selection: bold, italic, quote and link as Markdown
// markers, the Command / Control shortcuts, and Save's HTML reading them.

import { expect, test } from "@playwright/test";
import { open, preedit, saveAs, seedText } from "./helpers.mjs";

const TEXT = "我欲去台北揣朋友";

const bubble = (page) => page.locator("#format-bubble");
const button = (page, kind) =>
  page.locator(`#format-bubble button[data-markup="${kind}"]`);

/// The page with `TEXT` stored by a past visit, its undo history empty (WebKit
/// folds consecutive edits into one undo step).
async function openWithText(page) {
  await seedText(page, TEXT);
  const editor = await open(page, { intro: true });
  await expect(editor).toHaveValue(TEXT);
  return editor;
}

function select(editor, part) {
  return editor.evaluate((element, part) => {
    const start = element.value.indexOf(part);
    element.setSelectionRange(start, start + part.length);
  }, part);
}

const selected = (editor) =>
  editor.evaluate((element) =>
    element.value.slice(element.selectionStart, element.selectionEnd),
  );

test("a selection shows the bubble inside the editor, and bold toggles", async ({
  page,
}) => {
  const editor = await openWithText(page);
  await expect(bubble(page)).toBeHidden();
  await select(editor, "台北");
  await expect(bubble(page)).toBeVisible();
  const box = await bubble(page).boundingBox();
  const area = await page.locator("#editor").boundingBox();
  expect(box.x).toBeGreaterThanOrEqual(area.x);
  expect(box.y).toBeGreaterThanOrEqual(area.y);
  expect(box.x + box.width).toBeLessThanOrEqual(area.x + area.width + 1);

  await button(page, "bold").click();
  await expect(editor).toHaveValue("我欲去**台北**揣朋友");
  expect(await selected(editor)).toBe("台北");
  await expect(editor).toBeFocused();
  await expect(bubble(page)).toBeVisible();

  await button(page, "bold").click();
  await expect(editor).toHaveValue(TEXT);
});

test("one undo takes a format back", async ({ page }) => {
  const editor = await openWithText(page);
  await select(editor, "台北");
  await button(page, "italic").click();
  await expect(editor).toHaveValue("我欲去*台北*揣朋友");
  await editor.press("ControlOrMeta+z");
  await expect(editor).toHaveValue(TEXT);
});

test("quote marks the paragraph", async ({ page }) => {
  const editor = await openWithText(page);
  await select(editor, "台北");
  await button(page, "quote").click();
  await expect(editor).toHaveValue(`> ${TEXT}`);
});

test("Command or Control + B and I format the selection", async ({ page }) => {
  const editor = await openWithText(page);
  await select(editor, "朋友");
  await editor.press("ControlOrMeta+b");
  await expect(editor).toHaveValue("我欲去台北揣**朋友**");
  await editor.press("ControlOrMeta+i");
  await expect(editor).toHaveValue("我欲去台北揣***朋友***");
});

test("with nothing selected the shortcut does nothing to the text", async ({
  page,
}) => {
  const editor = await openWithText(page);
  await editor.press("ControlOrMeta+b");
  await expect(editor).toHaveValue(TEXT);
});

test("the link field takes a URL with slashes and writes the link", async ({
  page,
}) => {
  const editor = await openWithText(page);
  await select(editor, "台北");
  await editor.press("ControlOrMeta+k");
  const url = page.locator("#format-bubble input");
  await expect(url).toBeFocused();
  await expect(button(page, "bold")).toBeHidden();
  await url.pressSequentially("taigikeyboard.tw/a/b");
  await expect(url).toHaveValue("taigikeyboard.tw/a/b");
  await url.press("Enter");
  await expect(editor).toHaveValue(
    "我欲去[台北](https://taigikeyboard.tw/a/b)揣朋友",
  );
  await expect(editor).toBeFocused();
  expect(await selected(editor)).toBe("台北");

  // On the link, the field holds its URL; emptied, it takes the link off.
  await button(page, "link").click();
  await expect(url).toHaveValue("https://taigikeyboard.tw/a/b");
  await url.fill("");
  await url.press("Enter");
  await expect(editor).toHaveValue(TEXT);
});

test("Escape leaves the link field with the text as it was", async ({
  page,
}) => {
  const editor = await openWithText(page);
  await select(editor, "台北");
  await button(page, "link").click();
  await page.locator("#format-bubble input").press("Escape");
  await expect(editor).toBeFocused();
  await expect(editor).toHaveValue(TEXT);
  expect(await selected(editor)).toBe("台北");
  await expect(button(page, "bold")).toBeVisible();
});

test("a composition hides the bubble", async ({ page }) => {
  const editor = await openWithText(page);
  await select(editor, "台北");
  await expect(bubble(page)).toBeVisible();
  await editor.press("t");
  await expect(preedit(page)).not.toBeEmpty();
  await expect(bubble(page)).toBeHidden();
});

test("Save's HTML reads the markers", async ({ page }) => {
  const editor = await openWithText(page);
  await select(editor, "台北");
  await button(page, "bold").click();
  await expect(editor).toHaveValue("我欲去**台北**揣朋友");
  const { text } = await saveAs(page, "html");
  expect(text).toContain("<p>我欲去<strong>台北</strong>揣朋友</p>");
});

test("headings, lists and underline from the bubble", async ({ page }) => {
  const editor = await openWithText(page);
  await select(editor, "台北");
  await button(page, "h2").click();
  await expect(editor).toHaveValue(`## ${TEXT}`);
  await button(page, "bulletList").click();
  await expect(editor).toHaveValue(`- ${TEXT}`);
  await button(page, "numberedList").click();
  await expect(editor).toHaveValue(`1. ${TEXT}`);
  await button(page, "numberedList").click();
  await expect(editor).toHaveValue(TEXT);
  await select(editor, "台北");
  await button(page, "underline").click();
  await expect(editor).toHaveValue("我欲去<u>台北</u>揣朋友");
});

test("a click on the bubble's padding keeps it and the selection", async ({
  page,
}) => {
  const editor = await openWithText(page);
  await select(editor, "台北");
  // The left padding, halfway down: clear of the rounded corner.
  await bubble(page).click({ position: { x: 2, y: 20 } });
  await expect(bubble(page)).toBeVisible();
  await expect(editor).toBeFocused();
  expect(await selected(editor)).toBe("台北");
});

test("the link button again closes the URL field", async ({ page }) => {
  const editor = await openWithText(page);
  await select(editor, "台北");
  await button(page, "link").click();
  await expect(page.locator("#format-bubble input")).toBeVisible();
  await button(page, "link").click();
  await expect(page.locator("#format-bubble input")).toBeHidden();
  await expect(editor).toBeFocused();
  await expect(editor).toHaveValue(TEXT);
});

test("typing hides the bubble", async ({ page }) => {
  const editor = await openWithText(page);
  await select(editor, "台北");
  await expect(bubble(page)).toBeVisible();
  await editor.press("Backspace");
  await expect(bubble(page)).toBeHidden();
});
