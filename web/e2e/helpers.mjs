import { expect } from "@playwright/test";

/// The page with `settings` as if a past visit had stored them. `ready`: wait
/// for the dictionaries.
export async function open(page, { settings, ready = true } = {}) {
  if (settings) {
    await page.addInitScript(
      (document) => {
        localStorage.setItem("taigi.settings", document);
      },
      JSON.stringify({ values: settings }),
    );
  }
  await page.goto("/index.html");
  const editor = page.getByRole("textbox", { name: "Text" });
  await expect(editor).toBeEnabled();
  await expect(editor).toBeFocused();
  if (ready) {
    await expectState(page, "ready");
  }
  return editor;
}

export const preedit = (page) => page.locator("#preedit");
export const candidates = (page) => page.locator("#candidates li");
export const mode = (page) => page.locator("#mode");
export const panel = (page) => page.locator("#panel");

export function expectState(page, state) {
  return expect(page.locator("#message")).toHaveAttribute("data-state", state);
}

export async function commitWord(editor) {
  await editor.pressSequentially("taiuan");
  await editor.press("Enter");
}
