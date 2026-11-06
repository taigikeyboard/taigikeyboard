// The sidebar, the settings form over the session's document, and the fixed
// slots of docs/architecture/web-roadmap.md § Phase 5.

import { expect, test } from "@playwright/test";

import {
  candidates,
  commitWord,
  expectState,
  mode,
  open,
  panel,
  preedit,
  toggle,
} from "./helpers.mjs";

const inputMode = (page) => page.getByLabel("Input Mode");
const theme = (page) =>
  page.evaluate(() => document.documentElement.dataset.theme);
const pageColour = (page) =>
  page.evaluate(() => getComputedStyle(document.body).backgroundColor);

test("the sidebar opens beside the page and keeps the keyboard", async ({
  page,
}) => {
  const editor = await open(page);
  await expect(inputMode(page)).toBeHidden();
  await toggle(page).click();
  await expect(inputMode(page)).toBeVisible();
  await expect(toggle(page)).toHaveAttribute("aria-expanded", "true");
  await expect(editor).toBeFocused();
  await commitWord(editor);
  await expect(editor).toHaveValue("台灣");
});

test("the docked sidebar is remembered", async ({ page }) => {
  await open(page);
  await toggle(page).click();
  await page.reload();
  await expect(inputMode(page)).toBeVisible();
  await toggle(page).click();
  await expect(inputMode(page)).toBeHidden();
  await page.reload();
  await expect(inputMode(page)).toBeHidden();
});

test("a narrow window opens the sidebar as a drawer", async ({ page }) => {
  await page.setViewportSize({ width: 500, height: 700 });
  const editor = await open(page);
  await toggle(page).click();
  await expect(inputMode(page)).toBeVisible();
  await expect(toggle(page)).toHaveAttribute("aria-expanded", "true");
  await page.getByRole("button", { name: "Close" }).click();
  await expect(inputMode(page)).toBeHidden();
  await expect(toggle(page)).toHaveAttribute("aria-expanded", "false");
  await expect(editor).toBeVisible();
});

test("the status control and the chord open the sidebar", async ({ page }) => {
  const editor = await open(page);
  await page.locator("#mode-button").click();
  await expect(inputMode(page)).toBeVisible();
  await toggle(page).click();
  await expect(inputMode(page)).toBeHidden();
  // trace: Ctrl+Alt+S = OpenLastSettingsPane.
  await editor.press("Control+Alt+s");
  await expect(inputMode(page)).toBeVisible();
});

test("Input Mode reaches the session and survives a reload", async ({
  page,
}) => {
  // trace: tps_layout.rs KEYS '8' → ㄚ; `ModeIndicator::Tps` symbol 方.
  const editor = await open(page);
  await toggle(page).click();
  await inputMode(page).selectOption("tps");
  await expect(mode(page)).toHaveText("方");
  await editor.focus();
  await editor.press("8");
  await expect(preedit(page)).toContainText("ㄚ");
  await page.reload();
  await expect(mode(page)).toHaveText("方");
  await expect(inputMode(page)).toHaveValue("tps");
});

test("Candidate Display reaches the session", async ({ page }) => {
  // trace: `RomanOnly` shows no Hanji, so the top candidate is the romanization.
  const editor = await open(page);
  await toggle(page).click();
  await page.getByLabel("Candidate Display").selectOption("romanOnly");
  await editor.focus();
  await commitWord(editor);
  await expect(editor).toHaveValue("tâi-uân");
});

test("a chord's write shows in the form", async ({ page }) => {
  // trace: Ctrl+Alt+C = ToggleRomanization, TL → POJ.
  const editor = await open(page);
  await editor.press("Control+Alt+c");
  await toggle(page).click();
  await expect(inputMode(page)).toHaveValue("poj");
});

test("a form change keeps what a chord wrote", async ({ page }) => {
  const editor = await open(page);
  await editor.press("Control+Alt+c");
  await expect(mode(page)).toHaveText("白");
  await toggle(page).click();
  await page.getByLabel("Appearance").selectOption("dark");
  await expect(mode(page)).toHaveText("白");
  await page.reload();
  await expect(mode(page)).toHaveText("白");
});

test("Appearance pins the theme before first paint", async ({ page }) => {
  await open(page);
  await toggle(page).click();
  await page.getByLabel("Appearance").selectOption("dark");
  await expect.poll(() => pageColour(page)).toBe("rgb(0, 0, 0)");
  await page.getByLabel("Appearance").selectOption("light");
  await expect.poll(() => pageColour(page)).toBe("rgb(245, 245, 247)");
  await page.getByLabel("Appearance").selectOption("dark");
  // The script runs at parse time, ahead of the module that fills the form.
  await page.route("**/ime.js", (route) => route.abort());
  await page.reload();
  expect(await theme(page)).toBe("dark");
  await expect.poll(() => pageColour(page)).toBe("rgb(0, 0, 0)");
  // WebKit once lost the `body` rules from the first reload on.
  await page.reload();
  await expect.poll(() => pageColour(page)).toBe("rgb(0, 0, 0)");
});

test("a stored value no control offers shows the default", async ({ page }) => {
  await open(page, { settings: { inputMode: "bopomofo" } });
  await toggle(page).click();
  await expect(inputMode(page)).toHaveValue("tl");
  await expect(mode(page)).toHaveText("台");
});

test("the panel stays clear of Copy and Clear", async ({ page }) => {
  const editor = await open(page);
  await editor.fill("\n".repeat(60));
  await editor.evaluate((element) => {
    element.setSelectionRange(60, 60);
    element.scrollTop = element.scrollHeight;
  });
  await editor.pressSequentially("taiuan");
  await expect(candidates(page).first()).toBeVisible();
  const box = await panel(page).boundingBox();
  const copy = await page.getByRole("button", { name: "Copy" }).boundingBox();
  const area = await editor.boundingBox();
  expect(box.y + box.height).toBeLessThanOrEqual(area.y + area.height);
  expect(box.y + box.height).toBeLessThanOrEqual(copy.y);
});

test("a short window keeps the panel inside the editor", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 600 });
  const editor = await open(page);
  await editor.fill("\n");
  await editor.pressSequentially("taiuan");
  await expect(candidates(page).first()).toBeVisible();
  const top = await panel(page).evaluate((element) => element.offsetTop);
  expect(top).toBeGreaterThanOrEqual(0);
});

test("the panel follows a window that only got shorter", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 600 });
  const editor = await open(page);
  await editor.pressSequentially("taiuan");
  await expect(candidates(page).first()).toBeVisible();
  await page.setViewportSize({ width: 1280, height: 460 });
  await expect
    .poll(async () => {
      const box = await panel(page).boundingBox();
      return box.y + box.height;
    })
    .toBeLessThanOrEqual(460);
});

test("a document with no revision takes a form change", async ({ page }) => {
  const editor = await open(page);
  await page.evaluate(() => {
    localStorage.setItem(
      "taigi.settings",
      JSON.stringify({ values: { inputMode: "poj" } }),
    );
  });
  await page.reload();
  await expect(editor).toBeEnabled();
  await toggle(page).click();
  await inputMode(page).selectOption("tps");
  await expect(mode(page)).toHaveText("方");
  await page.reload();
  await expect(mode(page)).toHaveText("方");
});

test.describe(() => {
  // The reload aborts `ime.js`, and a page under a worker bypasses `page.route`.
  test.use({ serviceWorkers: "block" });

  test("a refused document is not painted from again", async ({ page }) => {
    await page.addInitScript(() => {
      if (!sessionStorage.getItem("seeded")) {
        sessionStorage.setItem("seeded", "1");
        localStorage.setItem(
          "taigi.settings",
          JSON.stringify({
            revision: "x",
            values: { candidateAppearanceMode: "dark" },
          }),
        );
      }
    });
    await open(page);
    await expect.poll(() => theme(page)).toBe("auto");
    await page.route("**/ime.js", (route) => route.abort());
    await page.reload();
    expect(await theme(page)).toBeUndefined();
  });
});

test("a state that appears shifts nothing", async ({ page }) => {
  const editor = await open(page, { ready: false });
  const before = await editor.boundingBox();
  await editor.dispatchEvent("compositionstart");
  await expect(page.locator("#notice")).toBeVisible();
  await editor.dispatchEvent("compositionend");
  await expectState(page, "ready");
  await editor.pressSequentially("taiuan");
  await expect(candidates(page).first()).toBeVisible();
  expect(await editor.boundingBox()).toEqual(before);
});
