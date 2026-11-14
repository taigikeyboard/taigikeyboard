// What the user sees of the extension's state: the toolbar button, the chip
// beside the focused field, and the popup
// (docs/architecture/browser-extension.md § Status).

import { cpSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  A,
  C,
  chip,
  commitWord,
  expect,
  expectToolbar,
  launch,
  openPopup,
  openTyping,
  preedit,
  removeSiteMenuEnabled,
  STORE,
  test,
  toggleShortcut,
  toolbar,
  toolbarPopup,
  toolbarTitle,
  turnOn,
} from "./helpers.mjs";

const FIELDS = `${A}/fields.html`;

test("the toolbar button and the chip name the same state", async ({
  context,
  worker,
  page,
}) => {
  const shortcut = await toggleShortcut(worker);
  await page.goto(FIELDS);
  await expect
    .poll(() => toolbar(worker, page))
    .toEqual(["", toolbarTitle("TaigiKeyboard: Off", shortcut)]);
  await expect.poll(() => toolbarPopup(worker, page)).toBe("");

  await turnOn(context, worker, page);
  // No field has the focus yet: the mode shows on the button alone.
  await expect(chip(page)).toBeHidden();
  expect(await toolbar(worker, page)).toEqual([
    "台",
    toolbarTitle("TaigiKeyboard: Taigi · Tâi-lô", shortcut),
  ]);

  await page.locator("#text").click();
  await expect(chip(page)).toHaveText("Taigi · Tâi-lô");
  await expect(chip(page)).toHaveAttribute("data-state", "taigi");

  await page.keyboard.press("Shift");
  await expect(chip(page)).toHaveText("English");
  await expect(chip(page)).toHaveAttribute("data-state", "english");
  await expect
    .poll(() => toolbar(worker, page))
    .toEqual(["A", toolbarTitle("TaigiKeyboard: English", shortcut)]);

  await page.locator("#password").click();
  await expect(chip(page)).toHaveText("Field not supported");
  await expect
    .poll(() => toolbar(worker, page))
    .toEqual([
      "×",
      toolbarTitle("TaigiKeyboard: Field not supported", shortcut),
    ]);
});

test("the chip sits beside the focused field", async ({
  context,
  worker,
  page,
}) => {
  const field = await openTyping(context, worker, page, FIELDS, "#area");
  const box = await field.boundingBox();
  const drawn = await chip(page).boundingBox();
  expect(box).not.toBeNull();
  expect(drawn).not.toBeNull();
  const b = /** @type {NonNullable<typeof box>} */ (box);
  const d = /** @type {NonNullable<typeof drawn>} */ (drawn);
  expect(d.y).toBeGreaterThanOrEqual(b.y + b.height);
  expect(d.y).toBeLessThan(b.y + b.height + 12);
  expect(Math.abs(d.x + d.width - (b.x + b.width))).toBeLessThan(2);
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
  const shortcut = await toggleShortcut(worker);
  await page.goto(FIELDS);
  const off = await openPopup(context, worker, page);
  await expect(off.locator("#site")).toHaveText("a.test");
  await expect(off.locator("#status")).toHaveText("Off");
  await expect(off.locator("#turn-on")).toBeVisible();
  await expect(off.locator("#turn-off")).toBeHidden();
  await expect(off.locator("#enable")).toBeHidden();
  if (shortcut) {
    await expect(off.locator("#turn-on .popup-shortcut")).toBeVisible();
    await expect(off.locator("#turn-on .popup-shortcut")).toHaveText(shortcut);
  } else {
    await expect(off.locator("#turn-on .popup-shortcut")).toBeHidden();
  }
  await off.close();

  await turnOn(context, worker, page);
  const on = await openPopup(context, worker, page);
  await expect(on.locator("#status")).toHaveText("Taigi · Tâi-lô");
  await expect(on.locator("#turn-off")).toBeVisible();
  await expect(on.locator("#turn-on")).toBeHidden();
  await expect(on.locator("#remove")).toBeVisible();
  if (shortcut) {
    await expect(on.locator("#turn-off .popup-shortcut")).toBeVisible();
    await expect(on.locator("#turn-off .popup-shortcut")).toHaveText(shortcut);
  } else {
    await expect(on.locator("#turn-off .popup-shortcut")).toBeHidden();
  }
  await on.close();

  const ungranted = await context.newPage();
  await ungranted.goto(`${C}/fields.html`);
  await expect
    .poll(() => toolbarPopup(worker, ungranted))
    .toMatch(/\/popup\.html$/);
  const ungrantedTabId = await worker.evaluate(async () => {
    const [tab] = await chrome.tabs.query({ active: true });
    return tab.id;
  });
  // Opening popup.html in a tab does not trigger activeTab, so without this
  // stub Chrome withholds the ungranted tab's URL from chrome.tabs.get().
  const enablePopup = await context.newPage();
  await enablePopup.addInitScript(() => {
    const send = chrome.runtime.sendMessage.bind(chrome.runtime);
    Object.assign(chrome.runtime, {
      /**
       * @param {unknown} message
       * @param {...unknown} rest
       */
      sendMessage(message, ...rest) {
        if (
          message &&
          typeof message === "object" &&
          "type" in message &&
          message.type === "taigi.popup.read"
        ) {
          return Promise.resolve({
            site: "c.test",
            pattern: "*://c.test/*",
            hasAccess: false,
            mode: "off",
            state: "off",
            label: "",
          });
        }
        return Reflect.apply(send, chrome.runtime, [message, ...rest]);
      },
    });
  });
  const id = new URL(worker.url()).host;
  await enablePopup.goto(
    `chrome-extension://${id}/popup.html?tab=${ungrantedTabId}`,
  );
  await expect(enablePopup.locator("#status")).toHaveText(
    "Not enabled on this site",
  );
  await expect(enablePopup.locator("#enable")).toBeVisible();
  if (shortcut) {
    await expect(enablePopup.locator("#enable .popup-shortcut")).toBeVisible();
    await expect(enablePopup.locator("#enable .popup-shortcut")).toHaveText(
      shortcut,
    );
  } else {
    await expect(enablePopup.locator("#enable .popup-shortcut")).toBeHidden();
  }
  await enablePopup.close();
});

test("the popup says so on a page no extension may enter", async ({
  context,
  worker,
}) => {
  // The popup as a page is such a page itself.
  const id = new URL(worker.url()).host;
  const page = await context.newPage();
  await page.goto(`chrome-extension://${id}/options.html`);
  await expect.poll(() => toolbarPopup(worker, page)).toMatch(/\/popup\.html$/);
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
  await expect
    .poll(() => toolbarPopup(worker, storePage))
    .toMatch(/\/popup\.html$/);
  const storePopup = await openPopup(context, worker, storePage);
  await expect(storePopup.locator("#status")).toHaveText(
    "TaigiKeyboard cannot run on this page",
  );
  await expect(storePopup.locator("#enable")).toBeHidden();
});

test("the action context menu tracks whether the active tab's site is enabled", async ({
  context,
  worker,
  page,
}) => {
  const id = new URL(worker.url()).host;
  await page.goto(FIELDS);
  await expect.poll(() => removeSiteMenuEnabled(worker, page)).toBe(true);

  const ungranted = await context.newPage();
  await ungranted.goto(`${C}/fields.html`);
  await expect.poll(() => removeSiteMenuEnabled(worker, ungranted)).toBe(false);

  const storePage = await context.newPage();
  await storePage.goto(`${STORE}/fields.html`);
  await expect.poll(() => removeSiteMenuEnabled(worker, storePage)).toBe(false);

  const optionsPage = await context.newPage();
  await optionsPage.goto(`chrome-extension://${id}/options.html`);
  await expect
    .poll(() => removeSiteMenuEnabled(worker, optionsPage))
    .toBe(false);

  await expect.poll(() => removeSiteMenuEnabled(worker, page)).toBe(true);
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
