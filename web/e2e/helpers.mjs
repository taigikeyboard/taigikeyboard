import { expect } from "@playwright/test";

/// `settings` as if a past visit had stored them.
export function seedSettings(page, settings) {
  return page.addInitScript(
    (document) => {
      localStorage.setItem("taigi.settings", document);
    },
    JSON.stringify({ values: settings }),
  );
}

/// The page with `settings` as if a past visit had stored them. `ready`: wait
/// for the dictionaries. `folded`: leave the sidebar's sections as a visit finds
/// them, rather than open for a test that works on their rows.
export async function open(
  page,
  { settings, ready = true, folded = false } = {},
) {
  if (settings) {
    await seedSettings(page, settings);
  }
  if (!folded) {
    await unfoldSections(page);
  }
  await page.goto("/index.html");
  const editor = page.locator("#text");
  await expect(editor).toBeEnabled();
  await expect(editor).toBeFocused();
  if (ready) {
    await expectState(page, "ready");
  }
  return editor;
}

/// On every load, so a reload keeps them open.
const unfoldSections = (page) =>
  page.addInitScript(() => {
    document.addEventListener("DOMContentLoaded", () => {
      for (const section of document.querySelectorAll(".sidebar-section")) {
        section.open = true;
      }
    });
  });

export const firstSection = (page) => page.locator("summary").first();
export const toggle = (page) => page.locator("#sidebar-toggle");
export const preedit = (page) => page.locator("#preedit");
export const candidates = (page) => page.locator("#candidates li");
export const inputMode = (page) => page.locator('select[name="inputMode"]');
export const panel = (page) => page.locator("#panel");

export function expectState(page, state) {
  return expect(page.locator("#message")).toHaveAttribute("data-state", state);
}

export async function commitWord(editor) {
  await editor.pressSequentially("taiuan");
  await editor.press("Enter");
}
