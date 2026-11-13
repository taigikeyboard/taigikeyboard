// Site access and the typing mode, which are two things: a site is enabled
// for good, typing is on per tab and starts off
// (docs/architecture/browser-extension.md § Tabs).

import {
  A,
  B,
  C,
  chip,
  clickPopup,
  commitWord,
  expect,
  expectToolbar,
  openTyping,
  overlay,
  preedit,
  test,
  toolbar,
  turnOn,
} from "./helpers.mjs";

const FIELDS = `${A}/fields.html`;

/// Typing is off on `page`: the keys of the test word reach its field.
async function expectOff(page) {
  const field = page.locator("#text");
  await field.click();
  await commitWord(page);
  await expect(field).toHaveValue("taiuan");
  await expect(preedit(page)).toHaveCount(0);
}

test("a new tab starts off, on an enabled site too", async ({
  worker,
  page,
}) => {
  await page.goto(FIELDS);
  await expectOff(page);
  // Nothing of the engine or the overlay is loaded.
  await expect(overlay(page)).toHaveCount(0);
  await expectToolbar(worker, page, "");
});

test("each tab keeps its own mode", async ({ context, worker, page }) => {
  const field = await openTyping(context, worker, page, FIELDS, "#text");
  const other = await context.newPage();
  await other.goto(FIELDS);
  await expectOff(other);
  await page.bringToFront();
  await field.click();
  await commitWord(page);
  await expect(field).toHaveValue("台灣");
  await expectToolbar(worker, page, "台");
  await expectToolbar(worker, other, "");
});

test("a reload keeps the tab's mode, English too", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, FIELDS, "#text");
  await page.reload();
  const field = page.locator("#text");
  await field.click();
  await expect(chip(page)).toHaveAttribute("data-state", "taigi");
  await commitWord(page);
  await expect(field).toHaveValue("台灣");

  await page.keyboard.press("Shift");
  await expect(chip(page)).toHaveAttribute("data-state", "english");
  await expectToolbar(worker, page, "A");
  await page.reload();
  await field.click();
  await expect(chip(page)).toHaveAttribute("data-state", "english");
  await page.keyboard.type("tai");
  await expect(field).toHaveValue("tai");
});

test("another page of the same site keeps the tab's mode, back-forward too", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, FIELDS, "#text");
  await page.locator("#elsewhere").click();
  await page.waitForURL(/other\.html/);
  const field = page.locator("#text");
  await field.click();
  await expect(chip(page)).toHaveAttribute("data-state", "taigi");
  await commitWord(page);
  await expect(field).toHaveValue("台灣");

  // Switch to English on the second page, then return via back-forward cache.
  await page.keyboard.press("Shift");
  await expectToolbar(worker, page, "A");
  await page.goBack();
  await expectToolbar(worker, page, "A");
  await page.locator("#text").click();
  await expect(chip(page)).toHaveAttribute("data-state", "english");
});

test("another site starts off, and so does the way back", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, FIELDS, "#text");
  // An enabled site.
  await page.goto(`${B}/fields.html`);
  await expectOff(page);
  await expectToolbar(worker, page, "");
  await page.goBack();
  await expectOff(page);
});

test("a site that is not enabled gets nothing", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, FIELDS, "#text");
  await page.goto(`${C}/fields.html`);
  await expectOff(page);
  await expect(overlay(page)).toHaveCount(0);
  // The manifest's own button: no state was ever told for this tab.
  const [badge, title] = await toolbar(worker, page);
  expect(badge).toBe("");
  expect(title).toBe("TaigiKeyboard: not enabled on this page");
  // And the way back to the enabled site starts off.
  await page.goto(FIELDS);
  await expectOff(page);
});

test("turning typing off ends the composition and frees the keys", async ({
  context,
  worker,
  page,
}) => {
  const field = await openTyping(context, worker, page, FIELDS, "#text");
  await page.keyboard.type("tai");
  await clickPopup(context, worker, page, "turn-off");
  await expectToolbar(worker, page, "");
  await expect(preedit(page)).toBeEmpty();
  await expect(chip(page)).toBeHidden();
  await field.click();
  await field.fill("");
  await commitWord(page);
  await expect(field).toHaveValue("taiuan");
  // And on again, in the same tab, without a reload — even after turning off
  // from English, turning back on returns in Taigi.
  await turnOn(context, worker, page);
  await field.click();
  await page.keyboard.press("Shift");
  await expectToolbar(worker, page, "A");
  await clickPopup(context, worker, page, "turn-off");
  await expectToolbar(worker, page, "");
  await turnOn(context, worker, page);
  await field.fill("");
  await field.click();
  await expect(chip(page)).toHaveAttribute("data-state", "taigi");
  await commitWord(page);
  await expect(field).toHaveValue("台灣");
});
