// Real keystrokes against the real engine and dictionaries. Words come from
// engine/composing/tests/walker_fixed_inputs_prod.rs, never from corpus/.

import { expect, test } from "@playwright/test";

import {
  candidates,
  commitWord,
  expectState,
  inputMode,
  open,
  panel,
  preedit,
  TIER,
} from "./helpers.mjs";

test("Enter commits the highlighted candidate", async ({ page }) => {
  const editor = await open(page);
  await editor.pressSequentially("taiuan");
  await expect(preedit(page)).toContainText("taiuan");
  await expect(candidates(page).first()).toBeVisible();
  await expect(editor).toHaveValue("");
  await editor.press("Enter");
  await expect(editor).toHaveValue("台灣");
  await expect(panel(page)).toBeHidden();
});

/// The cell the slot key `key` picks.
const keyedCell = (page, key) =>
  candidates(page).filter({
    has: page.locator(".index", { hasText: new RegExp(`^${key}$`) }),
  });

test("a slot key picks its candidate", async ({ page }) => {
  const editor = await open(page);
  await editor.pressSequentially("taiuan");
  const labelled = keyedCell(page, "w");
  const word = await labelled.locator(".text").textContent();
  await editor.press("w");
  await expect(editor).toHaveValue(word);
});

test("a click selects, Enter commits", async ({ page }) => {
  const editor = await open(page);
  await editor.pressSequentially("taiuan");
  const cell = keyedCell(page, "w");
  const word = await cell.locator(".text").textContent();
  await cell.click();
  await expect(editor).toBeFocused();
  await expect(cell).toHaveAttribute("aria-current", "true");
  await expect(editor).toHaveValue("");
  await editor.press("Enter");
  await expect(editor).toHaveValue(word);
});

test("the second page's cells select by position", async ({ page }) => {
  const editor = await open(page);
  await editor.pressSequentially("taiuan");
  const firstPage = await candidates(page).allTextContents();
  await editor.press("PageDown");
  await expect
    .poll(() => candidates(page).allTextContents())
    .not.toEqual(firstPage);
  // A single syllable nails a segment rather than committing, so the highlight
  // is the assertion.
  const cell = candidates(page).nth(1);
  await cell.click();
  await expect(cell).toHaveAttribute("aria-current", "true");
  await expect(editor).toHaveValue("");
});

test("undo takes a committed word back", async ({ page }) => {
  const editor = await open(page);
  await commitWord(editor);
  await expect(editor).toHaveValue("台灣");
  await editor.press("ControlOrMeta+z");
  await expect(editor).toHaveValue("");
});

test("Backspace goes through a composition and ends it", async ({ page }) => {
  const editor = await open(page);
  await editor.pressSequentially("tai");
  await editor.press("Backspace");
  await expect(preedit(page)).toContainText("ta");
  await editor.press("Backspace");
  await editor.press("Backspace");
  await expect(panel(page)).toBeHidden();
  await expect(editor).toHaveValue("");
});

test("TPS keys compose glyphs", async ({ page }) => {
  // trace: tps_layout.rs KEYS '8' → ㄚ.
  const editor = await open(page, { settings: { inputMode: "tps" } });
  await editor.press("8");
  await expect(preedit(page)).toContainText("ㄚ");
});

test("a Shift tap hands the keys to the page and back", async ({ page }) => {
  const editor = await open(page);
  await editor.press("Shift");
  await editor.pressSequentially("ab");
  await expect(editor).toHaveValue("ab");
  await expect(panel(page)).toBeHidden();
  await editor.press("Shift");
  await editor.pressSequentially("a");
  await expect(preedit(page)).toContainText("a");
  await expect(editor).toHaveValue("ab");
});

test("every default chord reaches the page", async ({ page }) => {
  // trace: shortcut_actions.rs `default_chord`.
  const editor = await open(page);
  await page.evaluate(() => {
    window.addEventListener("keydown", (event) => {
      window.lastPrevented = event.defaultPrevented;
    });
  });
  const prevented = () => page.evaluate(() => window.lastPrevented);

  for (const chord of ["s", "c", "p", "h", ",", "/", "j"]) {
    await editor.press(`${TIER}+${chord}`);
    expect(await prevented(), chord).toBe(true);
    await editor.press("Escape");
  }
  await editor.press("`");
  expect(await prevented()).toBe(true);
});

test("a chord that writes the settings survives a reload", async ({ page }) => {
  // trace: Ctrl+Alt+C = ToggleRomanization, TL → POJ.
  const editor = await open(page);
  await editor.press(`${TIER}+c`);
  await expect(inputMode(page)).toHaveValue("poj");
  await page.reload();
  await expect(inputMode(page)).toHaveValue("poj");
});

test("a switch names the new mode, then gets out of the way", async ({
  page,
}) => {
  // trace: Ctrl+Alt+C = ToggleRomanization, TL → POJ; `switched_label` is the
  // romanization alone. A Shift tap flashes `flash_label_key`. The observer
  // keeps each label, so a slow machine cannot miss the 0.8 s the card is up.
  const flash = page.locator("#mode-flash");
  const editor = await open(page);
  await flash.evaluate((card) => {
    window.flashed = [];
    new MutationObserver(() => {
      if (!card.hidden) {
        window.flashed.push(card.textContent);
      }
    }).observe(card, { attributes: true, childList: true });
  });
  const flashed = () => page.evaluate(() => window.flashed.at(-1));

  await expect(flash).toBeHidden();
  await editor.press(`${TIER}+c`);
  await expect.poll(flashed).toBe("Pe̍h-ōe-jī");
  await expect(flash).toBeHidden();

  await editor.press(`${TIER}+c`);
  await expect.poll(flashed).toBe("Tâi-lô");
  await editor.press("Shift");
  await expect.poll(flashed).not.toContain("Tâi-lô");
  await expect(flash).toBeHidden();
});

test("the picker and the guide open and close at the caret", async ({
  page,
}) => {
  const editor = await open(page);
  await editor.press(`${TIER}+,`);
  await expect(candidates(page).first()).toBeVisible();
  await editor.press("Escape");
  await expect(panel(page)).toBeHidden();

  await editor.press(`${TIER}+/`);
  await expect(candidates(page).first()).toBeVisible();
  await editor.press(`${TIER}+/`);
  await expect(panel(page)).toBeHidden();
});

test("keys before the install type romanization", async ({ page }) => {
  let release;
  const held = new Promise((resolve) => {
    release = resolve;
  });
  await page.route("**/dictionaries/**", async (route) => {
    await held;
    await route.continue();
  });
  const editor = await open(page, { ready: false });
  await editor.press("a");
  await expect(preedit(page)).toContainText("a");
  await expect(candidates(page)).toHaveCount(0);
  await editor.press("Enter");
  await expect(editor).toHaveValue(/^a/);
  release();
  await expectState(page, "ready");
  await commitWord(editor);
  await expect(editor).toHaveValue(/^a\s*台灣$/);
});

test("a stored document the session refuses leaves the defaults", async ({
  page,
}) => {
  await page.addInitScript(() => {
    localStorage.setItem("taigi.settings", "[]");
  });
  const editor = await open(page);
  await expect(inputMode(page)).toHaveValue("tl");
  await commitWord(editor);
  await expect(editor).toHaveValue("台灣");
});

test("a failed download leaves romanization working", async ({ page }) => {
  await page.route("**/dictionaries/dictionary.bin.*", (route) =>
    route.fulfill({ status: 404 }),
  );
  const editor = await open(page, { ready: false });
  await expectState(page, "failed");
  await editor.press("a");
  await expect(preedit(page)).toContainText("a");
});

test("a trap loses the composition, not the text", async ({ page }) => {
  const editor = await open(page);
  await commitWord(editor);
  await expect(editor).toHaveValue("台灣");
  await editor.pressSequentially("tai");

  // What a panic reaches the page as. Wrecking the instance's memory for a real
  // trap also crashed Chromium's renderer on Linux, through the allocator.
  await page.evaluate(async () => {
    const { Session } = await import("./pkg/taigi_web.js?instance=1");
    Session.prototype.keyDown = () => {
      throw new WebAssembly.RuntimeError("unreachable");
    };
  });
  await editor.press("a");

  await expect(panel(page)).toBeHidden();
  await expect(editor).toHaveValue("台灣");
  await expectState(page, "ready");
  await commitWord(editor);
  await expect(editor).toHaveValue("台灣台灣");
});

test("another input method's keys stay its own", async ({ page }) => {
  const editor = await open(page);
  const notice = page.locator("#notice");
  await expect(notice).toBeHidden();
  await editor.dispatchEvent("compositionstart");
  await expect(notice).toBeVisible();
  await page.evaluate(() => {
    const event = new KeyboardEvent("keydown", {
      key: "a",
      code: "KeyA",
      isComposing: true,
      bubbles: true,
    });
    document.getElementById("text").dispatchEvent(event);
  });
  await expect(panel(page)).toBeHidden();
  await editor.dispatchEvent("compositionend");
  await expect(notice).toBeHidden();
});
