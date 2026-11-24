// One learned history for the whole profile, whichever tab and site it was
// learned on (docs/architecture/browser-extension.md § D6). The word is
// the web page's (`web/e2e/learning.spec.mjs`).

import {
  A,
  B,
  candidates,
  expect,
  LEARN_HANJI,
  LEARN_RAW,
  launch,
  openTyping,
  pickThrice,
  test,
} from "./helpers.mjs";

/** @typedef {import("@playwright/test").Page} Page */

/**
 * The first cell a slot key picks, past the literal roman row.
 * @param {Page} page
 */
const topCell = (page) =>
  candidates(page)
    .filter({ has: page.locator(".index", { hasText: /^.+$/ }) })
    .first()
    .locator(".text");

/**
 * @param {Page} page
 * @param {string} hanji
 * @returns {Promise<void>}
 */
async function expectTop(page, hanji) {
  await expect(async () => {
    await page.keyboard.type(LEARN_RAW);
    try {
      await expect(topCell(page)).toHaveText(hanji, { timeout: 500 });
    } finally {
      await page.keyboard.press("Escape");
    }
  }).toPass();
}

const FIELDS = "/fields.html";

test("a pick on one site ranks first in a new tab on another", async ({
  context,
  worker,
  page,
}) => {
  const field = await openTyping(context, worker, page, A + FIELDS, "#text");
  await expectTop(page, "教授");
  await pickThrice(page, field, worker);
  await expectTop(page, LEARN_HANJI);

  const other = await context.newPage();
  await openTyping(context, worker, other, B + FIELDS, "#text");
  await expectTop(other, LEARN_HANJI);
});

test("a tab already typing takes up what another learned since", async ({
  context,
  worker,
  page,
}) => {
  const other = await context.newPage();
  await openTyping(context, worker, other, B + FIELDS, "#text");
  await expectTop(other, "教授");

  await page.bringToFront();
  const field = await openTyping(context, worker, page, A + FIELDS, "#text");
  await pickThrice(page, field, worker);

  await other.bringToFront();
  // Playwright keeps every tab visible and focused, so the browser never
  // says this one came back to the front: the test says it.
  await other.evaluate(() => window.dispatchEvent(new Event("focus")));
  await other.locator("#text").click();
  await expectTop(other, LEARN_HANJI);
  // And what it learns in turn continues the same copy, not its old one.
  await pickThrice(other, other.locator("#text"), worker);
  await page.bringToFront();
  await page.evaluate(() => window.dispatchEvent(new Event("focus")));
  await field.click();
  await expectTop(page, LEARN_HANJI);
});

test("the history outlives the browser, and stays in its profile", async ({
  context,
  worker,
  page,
  extension,
  profile,
}) => {
  const field = await openTyping(context, worker, page, A + FIELDS, "#text");
  // A new profile knows nothing.
  await expectTop(page, "教授");
  await pickThrice(page, field, worker);
  await context.close();

  const again = await launch(extension, profile);
  try {
    const reopened = await again.context.newPage();
    await openTyping(
      again.context,
      again.worker,
      reopened,
      A + FIELDS,
      "#text",
    );
    await expectTop(reopened, LEARN_HANJI);
  } finally {
    await again.context.close();
  }
});
