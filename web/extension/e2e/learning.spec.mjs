// One learned history for the whole profile, whichever tab and site it was
// learned on (docs/architecture/browser-extension.md § Learning). The word is
// the web page's (`web/e2e/learning.spec.mjs`).

import {
  A,
  B,
  candidates,
  expect,
  launch,
  openTyping,
  test,
} from "./helpers.mjs";

const RAW = "kausiu";
const PICKED = "狗岫";

/// The first cell a slot key picks, past the literal roman row.
const topCell = (page) =>
  candidates(page)
    .filter({ has: page.locator(".index", { hasText: /^.+$/ }) })
    .first()
    .locator(".text");

async function expectTop(page, hanji) {
  await expect(async () => {
    await page.keyboard.type(RAW);
    try {
      await expect(topCell(page)).toHaveText(hanji, { timeout: 500 });
    } finally {
      await page.keyboard.press("Escape");
    }
  }).toPass();
}

/// Clicks `PICKED` and commits it, three times: enough to rank it first.
async function pickThrice(page, field) {
  for (let round = 0; round < 3; round += 1) {
    await page.keyboard.type(RAW);
    await candidates(page).filter({ hasText: PICKED }).first().click();
    await page.keyboard.press("Enter");
  }
  await expect(field).toHaveValue(PICKED.repeat(3));
}

/// The number of the profile's copy, as the service worker keeps it.
function generation(worker) {
  return worker.evaluate(
    () =>
      new Promise((resolve, reject) => {
        const opening = indexedDB.open("taigi-extension-user-data", 1);
        opening.onupgradeneeded = () =>
          opening.result.createObjectStore("files");
        opening.onerror = () => reject(opening.error);
        opening.onsuccess = () => {
          const reading = opening.result
            .transaction("files")
            .objectStore("files")
            .get("snapshot");
          reading.onerror = () => reject(reading.error);
          reading.onsuccess = () => {
            opening.result.close();
            resolve(reading.result?.generation ?? 0);
          };
        };
      }),
  );
}

const FIELDS = "/fields.html";

test("a pick on one site ranks first in a new tab on another", async ({
  context,
  worker,
  page,
}) => {
  const field = await openTyping(context, worker, page, A + FIELDS, "#text");
  await expectTop(page, "教授");
  await pickThrice(page, field);
  await expectTop(page, PICKED);
  await expect.poll(() => generation(worker)).toBeGreaterThan(0);

  const other = await context.newPage();
  await openTyping(context, worker, other, B + FIELDS, "#text");
  await expectTop(other, PICKED);
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
  await pickThrice(page, field);
  await expect.poll(() => generation(worker)).toBeGreaterThan(0);

  await other.bringToFront();
  // Playwright keeps every tab visible and focused, so the browser never
  // says this one came back to the front: the test says it.
  await other.evaluate(() => window.dispatchEvent(new Event("focus")));
  await other.locator("#text").click();
  await expectTop(other, PICKED);
  // And what it learns in turn continues the same copy, not its old one.
  const before = await generation(worker);
  await pickThrice(other, other.locator("#text"));
  await expect.poll(() => generation(worker)).toBeGreaterThan(before);
  await page.bringToFront();
  await page.evaluate(() => window.dispatchEvent(new Event("focus")));
  await field.click();
  await expectTop(page, PICKED);
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
  await pickThrice(page, field);
  await expect.poll(() => generation(worker)).toBeGreaterThan(0);
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
    await expectTop(reopened, PICKED);
  } finally {
    await again.context.close();
  }
});
