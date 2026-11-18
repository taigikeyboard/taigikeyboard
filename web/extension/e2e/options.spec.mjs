// The options page's Shortcuts section: the page's own session draws it, so it
// is there before typing has ever been turned on, and follows the form.

import { expect, test, toggleShortcut } from "./helpers.mjs";

test("the Shortcuts section is drawn on a new install and follows the form", async ({
  context,
  worker,
}) => {
  const id = new URL(worker.url()).host;
  const shortcut = await toggleShortcut(worker);
  const page = await context.newPage();
  await page.goto(`chrome-extension://${id}/options.html`);
  const section = page.locator("#shortcuts");

  // trace: `shortcuts(false)` keeps Switch Phonetic Symbols and drops the
  // Telex guide and TPS key panel rows; the browser command joins Other.
  await expect(section).toContainText("Switch Phonetic Symbols");
  await expect(section).not.toContainText("Open Telex Guide");
  await expect(section).not.toContainText("Mini Keyboard");
  const toggle = section
    .locator(".setting")
    .filter({ hasText: "Start or stop typing Taigi in this tab" });
  await expect(toggle.locator(".setting-keys")).toHaveText(shortcut);

  // trace: under TPS a cell is the Hanji alone (`fixed_keys_label` → None),
  // so the other-script row loses its Shift run.
  const other = section
    .locator(".setting")
    .filter({ hasText: "Output the Other Script" });
  await expect(other).toContainText("QWDFZXVY");
  await page.locator('select[name="inputMode"]').selectOption("tps");
  await expect(other).not.toContainText("QWDFZXVY");
});
