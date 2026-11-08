import { expect } from "@playwright/test";

/// The global tier on the machine running the test: a Mac's page takes the
/// Mac app's chords (`key.rs`).
export const TIER =
  process.platform === "darwin" ? "Control+Meta" : "Control+Alt";

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
/// them, rather than open for a test that works on their rows. `docked`: the
/// sidebar open, as a past visit that opened it left it (it starts closed).
/// `intro`: keep the intro text a first visit finds, rather than an empty box
/// for a test that asserts what it typed.
export async function open(
  page,
  { settings, ready = true, folded = false, docked = true, intro = false } = {},
) {
  if (settings) {
    await seedSettings(page, settings);
  }
  if (!folded) {
    await unfoldSections(page);
  }
  if (docked) {
    // The first load only: a test that closes it and reloads sees it closed.
    await page.addInitScript(() => {
      if (sessionStorage.getItem("test.docked") === null) {
        sessionStorage.setItem("test.docked", "");
        localStorage.setItem("taigi.sidebar", "open");
      }
    });
  }
  await page.goto("/index.html");
  const editor = page.locator("#text");
  await expect(editor).toBeEnabled();
  await expect(editor).toBeFocused();
  if (!intro) {
    await emptyBox(editor);
  }
  if (ready) {
    await expectState(page, "ready");
  }
  return editor;
}

/// Takes the intro an empty box opens on out of `editor`, for a test that
/// asserts what it types. No `input`: nothing is stored, as for a box the user
/// has not touched, so a reload brings the intro back.
export function emptyBox(editor) {
  return editor.evaluate((element) => {
    element.value = "";
  });
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
