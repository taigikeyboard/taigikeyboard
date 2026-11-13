// What the user sees of the extension's state: the toolbar button, the chip
// beside the focused field, and the popup
// (docs/architecture/browser-extension.md § Status).

import { cpSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  A,
  chip,
  commitWord,
  expect,
  expectToolbar,
  launch,
  openPopup,
  openTyping,
  preedit,
  STORE,
  test,
  toolbar,
  turnOn,
} from "./helpers.mjs";

const FIELDS = `${A}/fields.html`;

test("the toolbar button and the chip name the same state", async ({
  context,
  worker,
  page,
}) => {
  await page.goto(FIELDS);
  await expect
    .poll(() => toolbar(worker, page))
    .toEqual(["", "TaigiKeyboard: Off"]);

  await turnOn(context, worker, page);
  // No field has the focus yet: the mode shows on the button alone.
  await expect(chip(page)).toBeHidden();
  expect(await toolbar(worker, page)).toEqual([
    "台",
    "TaigiKeyboard: Taigi · Tâi-lô",
  ]);

  await page.locator("#text").click();
  await expect(chip(page)).toHaveText("Taigi · Tâi-lô");
  await expect(chip(page)).toHaveAttribute("data-state", "taigi");

  await page.keyboard.press("Shift");
  await expect(chip(page)).toHaveText("English");
  await expect(chip(page)).toHaveAttribute("data-state", "english");
  await expect
    .poll(() => toolbar(worker, page))
    .toEqual(["A", "TaigiKeyboard: English"]);

  await page.locator("#password").click();
  await expect(chip(page)).toHaveText("Field not supported");
  await expect
    .poll(() => toolbar(worker, page))
    .toEqual(["×", "TaigiKeyboard: Field not supported"]);
});

test("the chip sits beside the focused field", async ({
  context,
  worker,
  page,
}) => {
  const field = await openTyping(context, worker, page, FIELDS, "#area");
  const box = await field.boundingBox();
  const drawn = await chip(page).boundingBox();
  expect(drawn.y).toBeGreaterThanOrEqual(box.y + box.height);
  expect(drawn.y).toBeLessThan(box.y + box.height + 12);
  expect(Math.abs(drawn.x + drawn.width - (box.x + box.width))).toBeLessThan(2);
  // It never takes a click meant for the site.
  expect(
    await chip(page).evaluate(
      (element) => getComputedStyle(element).pointerEvents,
    ),
  ).toBe("none");
});

test("the popup names the tab's state and offers the way out of it", async ({
  context,
  worker,
  page,
}) => {
  await page.goto(FIELDS);
  const off = await openPopup(context, worker, page);
  await expect(off.locator("#site")).toHaveText("a.test");
  await expect(off.locator("#status")).toHaveText("Off");
  await expect(off.locator("#turn-on")).toBeVisible();
  await expect(off.locator("#turn-off")).toBeHidden();
  await expect(off.locator("#enable")).toBeHidden();
  await off.close();

  await turnOn(context, worker, page);
  const on = await openPopup(context, worker, page);
  await expect(on.locator("#status")).toHaveText("Taigi · Tâi-lô");
  await expect(on.locator("#turn-off")).toBeVisible();
  await expect(on.locator("#turn-on")).toBeHidden();
  await expect(on.locator("#remove")).toBeVisible();
});

test("the popup says so on a page no extension may enter", async ({
  context,
  worker,
}) => {
  // The popup as a page is such a page itself.
  const id = new URL(worker.url()).host;
  const page = await context.newPage();
  await page.goto(`chrome-extension://${id}/options.html`);
  const popup = await openPopup(context, worker, page);
  await expect(popup.locator("#status")).toHaveText(
    "TaigiKeyboard cannot run on this page",
  );
  for (const name of ["enable", "turn-on", "turn-off", "remove"]) {
    await expect(popup.locator(`#${name}`)).toBeHidden();
  }
  await expect(popup.locator("#settings")).toBeVisible();
  await popup.close();

  // The web store is `http(s):` yet restricted all the same. Served by the
  // fixture server rather than `page.route`: `Fetch.fulfillRequest` on the
  // web store host crashes Chrome for Testing.
  const storePage = await context.newPage();
  await storePage.goto(`${STORE}/fields.html`);
  const storePopup = await openPopup(context, worker, storePage);
  await expect(storePopup.locator("#status")).toHaveText(
    "TaigiKeyboard cannot run on this page",
  );
  await expect(storePopup.locator("#enable")).toBeHidden();
});

test("a broken package keeps the site's keys, and says why not", async ({
  extension,
  profile,
}) => {
  const broken = mkdtempSync(join(tmpdir(), "taigi-broken-"));
  cpSync(extension, broken, { recursive: true });
  rmSync(join(broken, "pkg", "taigi_web_bg.wasm"));
  const { context, worker } = await launch(broken, profile);
  try {
    const page = await context.newPage();
    await page.goto(FIELDS);
    const popup = await openPopup(context, worker, page);
    await popup.locator("#turn-on").click();
    await page.bringToFront();
    await expectToolbar(worker, page, "!");
    const field = page.locator("#text");
    await field.click();
    await expect(chip(page)).toHaveAttribute("data-state", "failed");
    await expect(chip(page)).toHaveText("Could not load");
    await commitWord(page);
    await expect(field).toHaveValue("taiuan");
    await expect(preedit(page)).toBeEmpty();

    // Turning off clears the failure badge and chip.
    const offPopup = await openPopup(context, worker, page);
    await Promise.all([
      offPopup.waitForEvent("close"),
      offPopup.locator("#turn-off").click(),
    ]);
    await expectToolbar(worker, page, "");
    await expect(chip(page)).toBeHidden();
  } finally {
    await context.close();
    rmSync(broken, { recursive: true, force: true });
  }
});

test("a settings change reaches a tab that is typing", async ({
  context,
  worker,
  page,
}) => {
  await openTyping(context, worker, page, FIELDS, "#text");
  // As the options page writes it.
  await worker.evaluate(() =>
    chrome.storage.sync.set({
      "taigi.settings": JSON.stringify({
        revision: 1,
        values: { inputMode: "poj", displayLanguage: "ja" },
      }),
    }),
  );
  await expect(chip(page)).toHaveText("台湾語 · Pe̍h-ōe-jī");
});
