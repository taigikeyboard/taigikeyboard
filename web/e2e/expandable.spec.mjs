// Horizontal Expandable and Vertical Expandable: the keys go by
// `ExpandableListModel`, so what is under test is that the page draws what it
// says and feeds it what the page really measured.

import { expect, test } from "@playwright/test";

import { candidates, open, panel, TIER } from "./helpers.mjs";

const ROW = { candidateLayout: "expandable" };
const COLUMN = { candidateLayout: "verticalExpandable" };
/// A prefix with a long list. Its top cell comes from no fixture: tests compare
/// the page with itself.
const LONG_LIST = "ti";

const list = (page) => page.locator("#candidates");
const unfold = (page) => page.locator("#unfold");
const current = (page) => page.locator('#candidates li[aria-current="true"]');

async function expectExpanded(page) {
  await expect(list(page)).toHaveAttribute("data-grid");
}

async function expectCollapsed(page) {
  await expect(list(page)).not.toHaveAttribute("data-grid");
}

const position = (locator) => locator.getAttribute("data-position");

/// The unfold has finished, so boxes read their final size.
const settled = (page) =>
  expect
    .poll(() => panel(page).evaluate((root) => root.getAnimations().length))
    .toBe(0);

test("the first key of a word packs from real widths", async ({ page }) => {
  const editor = await open(page, { settings: ROW });
  await editor.press("t");
  await expect(candidates(page).first()).toBeVisible();
  const box = await candidates(page).first().boundingBox();
  // Zero widths would floor every cell at the same slot and pack nine.
  expect(box.width).toBeGreaterThan(20);
  await editor.press("Escape");
  await editor.pressSequentially("tsiahpng");
  const widths = await candidates(page).evaluateAll((cells) =>
    cells.map((cell) => cell.getBoundingClientRect().width),
  );
  expect(new Set(widths).size).toBeGreaterThan(1);
});

test("a row walks along itself and unfolds with Down", async ({ page }) => {
  const editor = await open(page, { settings: ROW });
  await editor.pressSequentially(LONG_LIST);
  await expect(list(page)).toHaveAttribute("data-row");
  await expect(unfold(page)).toBeVisible();
  await editor.press("ArrowRight");
  expect(await position(current(page))).toBe("1");
  await editor.press("ArrowDown");
  await expectExpanded(page);
  await expect(unfold(page)).toBeHidden();
  await editor.press("ArrowUp");
  await editor.press("ArrowUp");
  await expectCollapsed(page);
});

test("walking off a row's end unfolds onto the next cell", async ({ page }) => {
  const editor = await open(page, { settings: ROW });
  await editor.pressSequentially(LONG_LIST);
  const shown = await candidates(page).count();
  for (let step = 0; step < shown; step += 1) {
    await editor.press("ArrowRight");
  }
  await expectExpanded(page);
  expect(await position(current(page))).toBe(String(shown));
});

test("a column unfolds with Right at its own height and folds with Left", async ({
  page,
}) => {
  const editor = await open(page, { settings: COLUMN });
  await editor.pressSequentially(LONG_LIST);
  await expect(list(page)).toHaveAttribute("data-vertical");
  await expect(candidates(page)).toHaveCount(9);
  await settled(page);
  const { height } = await list(page).boundingBox();
  await editor.press("ArrowRight");
  await expectExpanded(page);
  await expect(list(page)).toHaveAttribute("data-inline");
  await settled(page);
  expect((await list(page).boundingBox()).height).toBeCloseTo(height, 0);
  await editor.press("ArrowDown");
  await editor.press("ArrowLeft");
  await expectCollapsed(page);
  await expect(list(page)).toHaveAttribute("data-vertical");
});

test("the keys number the row the highlight is on", async ({ page }) => {
  const editor = await open(page, { settings: ROW });
  await editor.pressSequentially(LONG_LIST);
  await editor.press("ArrowDown");
  await editor.press("ArrowDown");
  await expectExpanded(page);
  const row = await current(page).evaluate((cell) => {
    const top = cell.getBoundingClientRect().top;
    return [...cell.parentElement.children]
      .filter((other) => other.getBoundingClientRect().top === top)
      .map((other) => other.querySelector(".index").textContent);
  });
  expect(row.every(Boolean)).toBe(true);
  const labelled = page.locator("#candidates li:has(.index:not(:empty))");
  await expect(labelled).toHaveCount(row.length);
  const picked = await labelled.nth(1).locator(".text").textContent();
  await editor.press(row[1]);
  await expect(editor).toHaveValue(picked);
});

test("the chevron and a click reach the same model", async ({ page }) => {
  const editor = await open(page, { settings: ROW });
  await editor.pressSequentially(LONG_LIST);
  await unfold(page).click();
  await expectExpanded(page);
  await expect(editor).toBeFocused();
  const target = candidates(page).nth(10);
  const index = await position(target);
  await target.click();
  await expect(editor).toBeFocused();
  expect(await position(current(page))).toBe(index);
  await expect(editor).toHaveValue("");
});

test("Page Down after the wheel starts from what the wheel left", async ({
  page,
}) => {
  const editor = await open(page, { settings: ROW });
  await editor.pressSequentially(LONG_LIST);
  await editor.press("ArrowDown");
  await expectExpanded(page);
  await list(page).evaluate((grid) => {
    grid.scrollTop = grid.scrollHeight;
  });
  await expect
    .poll(() => list(page).evaluate((grid) => grid.scrollTop))
    .toBeGreaterThan(0);
  await page.waitForTimeout(50);
  await editor.press("PageDown");
  const visible = await current(page).evaluate((cell) => {
    const grid = cell.parentElement.getBoundingClientRect();
    const box = cell.getBoundingClientRect();
    return box.top >= grid.top - 1 && box.bottom <= grid.bottom + 1;
  });
  expect(visible).toBe(true);
});

test("the symbol picker lays out as a list does", async ({ page }) => {
  const editor = await open(page, { settings: ROW });
  await editor.press(`${TIER}+Comma`);
  await expect(list(page)).toHaveAttribute("data-row");
  await editor.press("ArrowDown");
  await expectExpanded(page);
  await editor.press("Escape");
  await expect(panel(page)).toBeHidden();
});

test("a guide and a list in one reply draw the list", async ({ page }) => {
  const errors = [];
  page.on("pageerror", (error) => errors.push(error));
  const editor = await open(page, {
    settings: { ...ROW, inputMode: "taiLo", telexGuideShown: true },
  });
  await editor.pressSequentially(LONG_LIST);
  await expect(list(page)).toHaveAttribute("data-row");
  await editor.press("Escape");
  await editor.press("Escape");
  await expect(list(page)).not.toHaveAttribute("data-row");
  expect(errors).toEqual([]);
});

test("the grid stays clear of Copy and Clear", async ({ page }) => {
  const editor = await open(page, { settings: ROW });
  await editor.pressSequentially(LONG_LIST);
  await editor.press("ArrowDown");
  await expectExpanded(page);
  await settled(page);
  const grid = await panel(page).boundingBox();
  const copy = await page.locator("#copy").boundingBox();
  expect(grid.y + grid.height).toBeLessThanOrEqual(copy.y);
});

test("a key during the unfold is not lost", async ({ page }) => {
  const editor = await open(page, { settings: ROW });
  await editor.pressSequentially(LONG_LIST);
  await editor.press("ArrowDown");
  await editor.press("ArrowDown");
  await expectExpanded(page);
  expect(Number(await position(current(page)))).toBeGreaterThan(0);
});

test("the unfold animates, unless motion is reduced", async ({ page }) => {
  const editor = await open(page, { settings: ROW });
  const running = () =>
    panel(page).evaluate((root) => root.getAnimations().length);
  await editor.pressSequentially(LONG_LIST);
  await editor.press("ArrowDown");
  expect(await running()).toBe(1);
  await editor.press("Escape");
  await page.emulateMedia({ reducedMotion: "reduce" });
  await editor.pressSequentially(LONG_LIST);
  await editor.press("ArrowDown");
  await expectExpanded(page);
  expect(await running()).toBe(0);
});

test("a short editor keeps the grid off Copy and Clear", async ({ page }) => {
  await page.setViewportSize({ width: 1000, height: 480 });
  const editor = await open(page, {
    settings: { ...ROW, candidateTextSize: 23 },
  });
  await editor.pressSequentially(LONG_LIST);
  await editor.press("ArrowDown");
  await expectExpanded(page);
  await settled(page);
  const box = await editor.boundingBox();
  const grid = await panel(page).boundingBox();
  expect(grid.y + grid.height).toBeLessThanOrEqual(box.y + box.height);
  await editor.press("ArrowDown");
  await editor.press("ArrowDown");
  await editor.press("ArrowDown");
  await editor.press("ArrowDown");
  const inView = await current(page).evaluate((cell) => {
    const frame = cell.parentElement.getBoundingClientRect();
    const box = cell.getBoundingClientRect();
    return box.top >= frame.top - 1 && box.bottom <= frame.bottom + 1;
  });
  expect(inView).toBe(true);
});

test("a table taken down mid-word leaves no empty grid", async ({ page }) => {
  const editor = await open(page, { settings: ROW });
  await editor.pressSequentially(LONG_LIST);
  await editor.press("ArrowDown");
  await expectExpanded(page);
  await editor.press(`${TIER}+Comma`);
  await editor.press(`${TIER}+Comma`);
  await expect(panel(page)).toBeHidden();
  await expect(list(page)).not.toHaveAttribute("data-grid");
});

test("typing in the grid starts a new list with no fold", async ({ page }) => {
  const editor = await open(page, { settings: ROW });
  await editor.pressSequentially(LONG_LIST);
  await editor.press("ArrowDown");
  await expectExpanded(page);
  await settled(page);
  await editor.press("a");
  await expectCollapsed(page);
  expect(
    await panel(page).evaluate((root) => root.getAnimations().length),
  ).toBe(0);
});
