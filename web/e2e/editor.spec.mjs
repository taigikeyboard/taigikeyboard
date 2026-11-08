// The page as a writing surface: the text survives a reload, the header lines
// can be deleted for good, and Save downloads the text in three formats.

import { expect, test } from "@playwright/test";
import { open } from "./helpers.mjs";

test("the text survives a reload", async ({ page }) => {
  const editor = await open(page, { ready: false });
  await page.keyboard.insertText("台灣\n\nhó");
  await page.reload();
  await expect(editor).toHaveValue("台灣\n\nhó");
  // Writing goes on where it stopped.
  const caret = await editor.evaluate((element) => element.selectionStart);
  expect(caret).toBe("台灣\n\nhó".length);
});

test("a tab opened earlier takes the text another tab saved", async ({
  page,
  context,
}) => {
  await open(page, { ready: false });
  const other = await context.newPage();
  await other.goto("/index.html");
  await expect(other.locator("#text")).toBeEnabled();
  await other.keyboard.insertText("台灣");
  await expect(page.locator("#text")).toHaveValue("台灣");
});

test("a deleted header line stays deleted", async ({ page }) => {
  await open(page, { ready: false });
  await page.locator("#title-line").hover();
  await page.locator("#title-line .header-delete").click();
  await expect(page.locator("#title-line")).toBeHidden();
  await expect(page.locator("#subtitle-line")).toBeVisible();
  await expect(page.locator("#text")).toBeFocused();
  await page.reload();
  await expect(page.locator("#title-line")).toBeHidden();
  await expect(page.locator("#subtitle-line")).toBeVisible();
});

for (const [format, extension, expected] of [
  ["markdown", "md", "台灣\n\n<b>"],
  ["text", "txt", "台灣\n\n<b>"],
  ["html", "html", "<p>&lt;b&gt;</p>"],
]) {
  test(`Save as ${format}`, async ({ page }) => {
    await open(page, { ready: false });
    await page.keyboard.insertText("台灣\n\n<b>");
    const [download] = await Promise.all([
      page.waitForEvent("download"),
      page.locator(`#save-formats [data-format="${format}"]`).click(),
    ]);
    expect(download.suggestedFilename()).toBe(`台灣.${extension}`);
    const stream = await download.createReadStream();
    const chunks = [];
    for await (const chunk of stream) {
      chunks.push(chunk);
    }
    expect(Buffer.concat(chunks).toString("utf8")).toContain(expected);
  });
}
