// The page as ZenPen's writing surface: the intro text an empty box opens on,
// the text that survives a reload, the editable title, and Save's three formats.

import { expect, test } from "@playwright/test";
import { expectText, open, saveAs, textAfterCaret } from "./helpers.mjs";

const TITLE = "台語齒盤網頁版：台語簿仔紙";

test("an empty box opens on the intro, caret at its end", async ({ page }) => {
  const editor = await open(page, { intro: true });
  // trace: web.intro en, the browser's locale.
  await expectText(editor, /^TaigiKeyboard is a free/);
  expect(await textAfterCaret(editor)).toBe("");
  await editor.press("Backspace");
  await editor.press("Backspace");
  // trace: web.intro en ends on the signature "Bîn-hiân".
  await expectText(editor, /Bîn-hi$/);
});

test("an emptied box opens on the intro again", async ({ page }) => {
  const editor = await open(page, { intro: true });
  await editor.press("ControlOrMeta+a");
  await editor.press("Backspace");
  await expectText(editor, "");
  await page.reload();
  await expectText(editor, /^TaigiKeyboard is a free/);
});

test("the text survives a reload", async ({ page }) => {
  const editor = await open(page, { ready: false });
  await page.keyboard.insertText("台灣\n\nhó");
  await page.reload();
  await expectText(editor, "台灣\n\nhó");
  // Writing goes on where it stopped.
  await expect.poll(() => textAfterCaret(editor)).toBe("");
});

test("a tab opened earlier takes the text another tab saved", async ({
  page,
  context,
}) => {
  await open(page, { ready: false });
  const other = await context.newPage();
  await other.goto("/index.html");
  await expect(other.locator("#text")).toHaveAttribute(
    "contenteditable",
    "true",
  );
  await other.locator("#text").fill("台灣");
  await expectText(page.locator("#text"), "台灣");
});

test("the title is edited in place and kept", async ({ page }) => {
  const editor = await open(page, { ready: false });
  const title = page.locator("#title");
  await expect(title).toHaveText(TITLE);
  await title.click();
  await page.keyboard.press("ControlOrMeta+a");
  await page.keyboard.type("Lâu-bî");
  // One line: Enter moves on to the text.
  await page.keyboard.press("Enter");
  await expect(editor).toBeFocused();
  await expect(title).toHaveText("Lâu-bî");
  await page.reload();
  await expect(title).toHaveText("Lâu-bî");
});

test("an emptied title opens on the default one", async ({ page }) => {
  await open(page, { ready: false });
  const title = page.locator("#title");
  await title.click();
  await page.keyboard.press("ControlOrMeta+a");
  await page.keyboard.press("Backspace");
  await expect(title).toHaveText("");
  await page.reload();
  await expect(title).toHaveText(TITLE);
});

for (const [format, extension, expected] of [
  ["markdown", "md", `# ${TITLE}\n\n台灣\n\n<b>`],
  ["text", "txt", `${TITLE}\n\n台灣\n\n<b>`],
  ["html", "html", `<h1>${TITLE}</h1>\n<p>台灣</p>\n<p>&lt;b&gt;</p>`],
]) {
  test(`Save as ${format}: the title, then the text`, async ({ page }) => {
    await open(page, { ready: false });
    await page.keyboard.insertText("台灣\n\n<b>");
    const { name, text } = await saveAs(page, format);
    expect(name).toBe(`${TITLE}.${extension}`);
    expect(text).toContain(expected);
  });
}

test("a text the textarea stored reads as plain text, markers and all", async ({
  page,
}) => {
  await page.addInitScript(() => {
    if (sessionStorage.getItem("test.seeded") === null) {
      sessionStorage.setItem("test.seeded", "");
      localStorage.setItem("taigi.text", "1. 一\n*二* # 三\n\n- 四");
    }
  });
  const editor = await open(page, { intro: true });
  await expect(editor).toHaveText(/1\. 一\s*\*二\* # 三\s*- 四/);
  await expect(editor.locator("ol, ul, em, strong")).toHaveCount(0);
  // The old key is left as it was.
  expect(await page.evaluate(() => localStorage.getItem("taigi.text"))).toBe(
    "1. 一\n*二* # 三\n\n- 四",
  );
});
