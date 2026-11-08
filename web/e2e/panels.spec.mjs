// The TPS key panel, the symbol picker through the candidate panel, and the
// a11y rows of docs/architecture/web-roadmap.md § Phase 5.

import { expect, test } from "@playwright/test";

import { candidates, open, panel, preedit, TIER } from "./helpers.mjs";

const keyPanel = (page) => page.locator("#tps-keyboard");
const cap = (page, glyph) =>
  page.locator(`#tps-keyboard button[data-glyph="${glyph}"]`);
const shownSetting = (page) =>
  page.getByRole("checkbox", { name: "Show Phonetic Symbols Mini Keyboard" });
const typingTps = { inputMode: "tps", tpsKeyboardShown: true };
/// The bottom half of a cap, its bare layer.
const bareLayer = { position: { x: 10, y: 35 } };

test("the chord shows and hides the key panel, and a reload keeps it", async ({
  page,
}) => {
  // No `settings`: a seed would run again on the reload. Ctrl+Alt+P = ToggleTps.
  const editor = await open(page);
  await editor.press(`${TIER}+p`);
  await expect(keyPanel(page)).toBeHidden();
  await editor.press(`${TIER}+j`);
  await expect(keyPanel(page)).toBeVisible();
  await page.reload();
  await expect(cap(page, "ㄅ")).toBeVisible();
  await editor.press(`${TIER}+j`);
  await expect(keyPanel(page)).toBeHidden();
});

test("the panel's chord leaves a composition standing", async ({ page }) => {
  const editor = await open(page, { settings: { inputMode: "tps" } });
  await editor.press("8");
  await editor.press(`${TIER}+j`);
  await expect(preedit(page)).toContainText("ㄚ");
  await expect(editor).toHaveValue("");
});

test("the panel follows the mode, and the choice waits for the way back", async ({
  page,
}) => {
  // trace: Ctrl+Alt+P = ToggleTps; `is_tps_keyboard_wanted`.
  const editor = await open(page, { settings: typingTps });
  await expect(keyPanel(page)).toBeVisible();
  await editor.press(`${TIER}+p`);
  await expect(keyPanel(page)).toBeHidden();
  await expect(shownSetting(page)).toBeDisabled();
  await expect(shownSetting(page)).toBeChecked();
  await editor.press(`${TIER}+p`);
  await expect(keyPanel(page)).toBeVisible();
  await expect(shownSetting(page)).toBeEnabled();
});

test("the sidebar row shows the panel", async ({ page }) => {
  await open(page, { settings: { inputMode: "tps" } });
  await shownSetting(page).check();
  await expect(keyPanel(page)).toBeVisible();
});

test("the panel leaves the text on screen", async ({ page }) => {
  const editor = await open(page, { settings: typingTps });
  await expect(editor).toBeInViewport();
  await expect(cap(page, "ㄅ")).toBeInViewport();
});

test("a click types the bare glyph, and the focus stays", async ({ page }) => {
  const editor = await open(page, { settings: typingTps });
  await cap(page, "ㄚ").click(bareLayer);
  await expect(preedit(page)).toContainText("ㄚ");
  await expect(editor).toBeFocused();
  await expect(editor).toHaveValue("");
});

test("the top half or a held Shift types the Shift glyph", async ({ page }) => {
  // trace: tps_layout.rs KEYS '1' → ㄅ, '!' → ㆠ, '8' → ㄚ, '*' → ㆩ.
  await open(page, { settings: typingTps });
  await cap(page, "ㄅ").click({ position: { x: 10, y: 3 } });
  await expect(preedit(page)).toContainText("ㆠ");
  await cap(page, "ㄚ").click({ modifiers: ["Shift"] });
  await expect(preedit(page)).toContainText("ㆩ");
});

test("a click on a cap keeps the running composition", async ({ page }) => {
  const editor = await open(page, { settings: typingTps });
  await editor.press("1");
  await cap(page, "ㄚ").click(bareLayer);
  await expect(preedit(page)).toContainText("ㄅㄚ");
  await expect(editor).toHaveValue("");
});

test("a key lights its cap for a moment", async ({ page }) => {
  const editor = await open(page, { settings: typingTps });
  await editor.press("8");
  await expect(cap(page, "ㄚ")).toHaveAttribute("data-lit");
  await expect(cap(page, "ㄚ")).not.toHaveAttribute("data-lit");
});

test("the caps are out of the tab order", async ({ page, browserName }) => {
  test.skip(browserName === "webkit", "Tab skips buttons in WebKit on a Mac");
  const editor = await open(page, { settings: typingTps });
  await editor.press("Tab");
  const isOnCap = await page.evaluate(
    () => document.activeElement.closest("#tps-keyboard") !== null,
  );
  expect(isOnCap).toBe(false);
});

test("a slot key writes a symbol, and it leads the next picker", async ({
  page,
}) => {
  const editor = await open(page);
  await editor.press(`${TIER}+,`);
  await editor.press("ArrowDown");
  const second = await candidates(page).nth(1).textContent();
  await editor.press("Enter");
  await expect(panel(page)).toBeHidden();
  const symbol = await editor.inputValue();
  expect(second).toContain(symbol);
  await page.reload();
  await expect(editor).toBeEnabled();
  await editor.press(`${TIER}+,`);
  await expect(candidates(page).first()).toContainText(symbol);
});

test("a click on a symbol selects it and Enter writes it", async ({ page }) => {
  const editor = await open(page);
  await editor.press(`${TIER}+,`);
  const cell = candidates(page).nth(2);
  await cell.click();
  await expect(cell).toHaveAttribute("aria-current", "true");
  await expect(editor).toHaveValue("");
  await editor.press("Enter");
  await expect(panel(page)).toBeHidden();
  await expect(editor).not.toHaveValue("");
});

/// What a cell says aloud: its drawn parts, apart.
const readCell = (cell) =>
  cell.evaluate((element) =>
    Array.from(element.children, ({ textContent }) => textContent)
      .filter(Boolean)
      .join(" "),
  );

test("the live region says one cell and goes quiet with the list", async ({
  page,
}) => {
  const editor = await open(page);
  const spoken = page.locator("#spoken");
  await expect(spoken).toHaveAttribute("aria-live", "polite");
  await editor.pressSequentially("taiuan");
  await expect(spoken).toHaveText(await readCell(candidates(page).first()));
  await editor.press("ArrowDown");
  await expect(spoken).toHaveText(
    await readCell(page.locator('#candidates [aria-current="true"]')),
  );
  await editor.press("Enter");
  await editor.press("Enter");
  await expect(spoken).toHaveText("");
});

test("reduced motion stops the sidebar sliding", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await open(page);
  const duration = await page.evaluate(
    () =>
      getComputedStyle(document.getElementById("sidebar")).transitionDuration,
  );
  expect(duration).toBe("0s");
});
