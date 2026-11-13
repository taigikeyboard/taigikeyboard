// Where a composition ends, and the fields the extension leaves alone
// (docs/architecture/browser-extension.md § Fields).

import {
  A,
  chip,
  commitWord,
  expect,
  expectToolbar,
  heard,
  openTyping,
  preedit,
  test,
} from "./helpers.mjs";

const FIELDS = `${A}/fields.html`;

test("a click on another field ends the composition in its own field", async ({
  context,
  worker,
  page,
}) => {
  const field = await openTyping(context, worker, page, FIELDS, "#text");
  await page.keyboard.type("tai");
  const second = page.locator("#second");
  await second.click();
  // As on the page (`web/e2e/surface.spec.mjs`): what was typed is written.
  await expect(field).toHaveValue("tai");
  await expect(preedit(page)).toBeEmpty();
  await expect(second).toHaveValue("");
  await commitWord(page);
  await expect(second).toHaveValue("台灣");
  await expect(field).toHaveValue("tai");
});

test("a composition never follows the focus to another field", async ({
  context,
  worker,
  page,
}) => {
  const field = await openTyping(context, worker, page, FIELDS, "#text");
  await page.keyboard.type("tai");
  await expect(preedit(page)).toContainText("tai");
  // The site moves the focus itself: no pointer, no key.
  await page.evaluate(() => document.getElementById("second").focus());
  await expect(preedit(page)).toBeEmpty();
  const second = page.locator("#second");
  await expect(second).toHaveValue("");
  await page.keyboard.type("uan");
  await expect(preedit(page)).toContainText("uan");
  await expect(preedit(page)).not.toContainText("tai");
  await page.keyboard.press("Escape");
  await expect(field).toHaveValue("");
  await expect(second).toHaveValue("");
});

test("a composition ends when the site removes its field", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, FIELDS, "#text");
  await page.keyboard.type("tai");
  await expect(preedit(page)).toContainText("tai");
  await page.evaluate(() => document.getElementById("text").remove());
  await expect(preedit(page)).toBeEmpty();
  await expect(chip(page)).toBeHidden();
});

test("a password field's keys are never read", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, FIELDS, "#text");
  const password = page.locator("#password");
  await password.click();
  await expect(chip(page)).toHaveAttribute("data-state", "unsupported");
  await expectToolbar(worker, page, "×");
  await page.keyboard.type("taiuan");
  await page.keyboard.press("Enter");
  await expect(password).toHaveValue("taiuan");
  await expect(preedit(page)).toBeEmpty();
  // Every key reached the site as typed.
  expect(await heard(page)).toContain("keydown:t\nkeyup:t\n");
  expect(await heard(page)).toContain("keydown:Enter\n");
  // A Shift tap there is not a mode switch.
  await page.keyboard.press("Shift");
  await page.locator("#text").click();
  await expect(chip(page)).toHaveAttribute("data-state", "taigi");
});

test("a field that takes no text an input method writes is left alone", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, FIELDS, "#text");
  const number = page.locator("#number");
  await number.click();
  await expect(chip(page)).toHaveAttribute("data-state", "unsupported");
  await page.keyboard.type("12");
  await expect(number).toHaveValue("12");
  // A read-only or disabled field takes no typing at all: nothing is drawn.
  await page.locator("#readonly").click();
  await expect(chip(page)).toBeHidden();
  const text = page.locator("#text");
  await text.click();
  await page.keyboard.type("tai");
  await expect(preedit(page)).toContainText("tai");
  await page.evaluate(() => {
    document.getElementById("text").disabled = true;
  });
  await expect(preedit(page)).toBeEmpty();
  await expect(chip(page)).toBeHidden();
});
