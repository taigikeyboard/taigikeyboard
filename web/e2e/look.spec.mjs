// The sidebar, the settings form over the session's document, and the fixed
// slots of docs/architecture/web-roadmap.md § Phase 5.

import { expect, test } from "@playwright/test";

import {
  candidates,
  commitWord,
  expectState,
  firstSection,
  inputMode,
  open,
  panel,
  preedit,
  toggle,
} from "./helpers.mjs";

const theme = (page) =>
  page.evaluate(() => document.documentElement.dataset.theme);
const pageColour = (page) =>
  page.evaluate(() => getComputedStyle(document.body).backgroundColor);

test("the sidebar starts open beside the page and keeps the keyboard", async ({
  page,
}) => {
  const editor = await open(page, { folded: true });
  await expect(firstSection(page)).toBeVisible();
  await expect(toggle(page)).toHaveAttribute("aria-expanded", "true");
  await expect(editor).toBeFocused();
  await commitWord(editor);
  await expect(editor).toHaveValue("台灣");
});

test("the sidebar remembers being closed", async ({ page }) => {
  await open(page);
  const section = firstSection(page);
  await toggle(page).click();
  await expect(section).toBeHidden();
  await page.reload();
  await expect(section).toBeHidden();
  await toggle(page).click();
  await page.reload();
  await expect(section).toBeVisible();
});

test("a narrow window opens the sidebar as a drawer", async ({ page }) => {
  await page.setViewportSize({ width: 500, height: 700 });
  const editor = await open(page);
  await expect(inputMode(page)).toBeHidden();
  await toggle(page).click();
  await expect(inputMode(page)).toBeVisible();
  await expect(toggle(page)).toHaveAttribute("aria-expanded", "true");
  await page.getByRole("button", { name: "Close" }).click();
  await expect(inputMode(page)).toBeHidden();
  await expect(toggle(page)).toHaveAttribute("aria-expanded", "false");
  await expect(editor).toBeVisible();
});

test("every section starts folded and unfolds on a click", async ({ page }) => {
  await open(page, { folded: true });
  const moe = page.locator('input[name="moeDictEnabled"]');
  await expect(inputMode(page)).toBeHidden();
  await expect(page.getByLabel("Appearance")).toBeHidden();
  await expect(moe).toBeHidden();
  await page.locator("summary").nth(3).click();
  await expect(moe).toBeVisible();
  await page.locator("summary").nth(3).click();
  await expect(moe).toBeHidden();
});

test("Shortcuts lists the keys and follows the settings", async ({ page }) => {
  await open(page);
  const rows = page.locator("#shortcuts .setting");
  const row = (name) => rows.filter({ hasText: name });
  // trace: `shipped_chord` ConfirmHighlighted = Enter + Space, `display`.
  await expect(row("Confirm Key").locator(".setting-keys")).toHaveText(
    /Enter|↩/,
  );
  await expect(row("Output the Other Script")).toContainText("QWDFZXVY;");
  await page.locator('[name="toneInputScheme"]').selectOption("telex");
  // trace: the Telex slot set, behind one Shift (`shifted_slot_keys_label`).
  await expect(row("Output the Other Script")).toContainText("123456789");
  await page.locator('[name="displayLanguage"]').selectOption("hanji");
  await expect(row("確定齒")).toBeAttached();
});

test("Input Mode reaches the session and survives a reload", async ({
  page,
}) => {
  // trace: tps_layout.rs KEYS '8' → ㄚ.
  const editor = await open(page);
  await inputMode(page).selectOption("tps");
  await editor.focus();
  await editor.press("8");
  await expect(preedit(page)).toContainText("ㄚ");
  await page.reload();
  await expect(inputMode(page)).toHaveValue("tps");
});

test("Candidate Display reaches the session", async ({ page }) => {
  // trace: `RomanOnly` shows no Hanji, so the top candidate is the romanization.
  const editor = await open(page);
  await page.getByLabel("Candidate Display").selectOption("romanOnly");
  await editor.focus();
  await commitWord(editor);
  await expect(editor).toHaveValue("tâi-uân");
});

test("a chord's write shows in the form", async ({ page }) => {
  // trace: Ctrl+Alt+C = ToggleRomanization, TL → POJ.
  const editor = await open(page);
  await editor.press("Control+Alt+c");
  await expect(inputMode(page)).toHaveValue("poj");
});

test("a form change keeps what a chord wrote", async ({ page }) => {
  const editor = await open(page);
  await editor.press("Control+Alt+c");
  await expect(inputMode(page)).toHaveValue("poj");
  await page.getByLabel("Appearance").selectOption("dark");
  await expect(inputMode(page)).toHaveValue("poj");
  await page.reload();
  await expect(inputMode(page)).toHaveValue("poj");
});

test("Appearance pins the theme before first paint", async ({ page }) => {
  await open(page);
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
  await expect(inputMode(page)).toHaveValue("tl");
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
  await inputMode(page).selectOption("tps");
  await page.reload();
  await expect(inputMode(page)).toHaveValue("tps");
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
  await expect(page.locator("#notice")).toHaveAttribute("data-active");
  await editor.dispatchEvent("compositionend");
  await expectState(page, "ready");
  await editor.pressSequentially("taiuan");
  await expect(candidates(page).first()).toBeVisible();
  expect(await editor.boundingBox()).toEqual(before);
});

test("/ moves into the textarea from anywhere but a text field", async ({
  page,
}) => {
  const editor = await open(page);
  await page.getByRole("button", { name: "Copy" }).focus();
  await page.keyboard.press("/");
  await expect(editor).toBeFocused();
  await expect(editor).toHaveValue("");
  await page.getByLabel("Auto-Space").focus();
  await page.keyboard.press("/");
  await expect(editor).toBeFocused();
  await inputMode(page).focus();
  await page.keyboard.press("/");
  await expect(inputMode(page)).toBeFocused();
});

test("the / hint shows only over an empty textarea", async ({ page }) => {
  const editor = await open(page);
  const hint = page.locator("#text-hint");
  await expect(hint.locator("kbd")).toHaveText("/");
  await expect(hint).toBeVisible();
  await editor.pressSequentially("tai");
  await expect(hint).toBeHidden();
  await editor.press("Escape");
  await expect(hint).toBeVisible();
  await editor.fill("台灣");
  await expect(hint).toBeHidden();
});

test("around the textarea, only Copy and Clear", async ({ page }) => {
  await open(page);
  const seen = await page.evaluate(() =>
    Array.from(
      document.querySelectorAll("main :is(p, button, progress)"),
      (element) => {
        const { width, height } = element.getBoundingClientRect();
        const isDrawn =
          width > 1 &&
          height > 1 &&
          getComputedStyle(element).visibility === "visible";
        return isDrawn ? element.getAttribute("aria-label") : null;
      },
    ).filter((text) => text !== null),
  );
  expect(seen).toEqual(["Copy", "Clear"]);
});

test("a window that fits the page does not scroll", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await open(page);
  const { scrollHeight, clientHeight } = await page.evaluate(() => {
    const { scrollHeight, clientHeight } = document.documentElement;
    return { scrollHeight, clientHeight };
  });
  expect(scrollHeight).toBeLessThanOrEqual(clientHeight);
});

test("the textarea starts short however tall the window", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 1200 });
  const editor = await open(page);
  // trace: `#text` height 22rem at the 16px root.
  expect((await editor.boundingBox()).height).toBeCloseTo(352, 0);
});

test("the footer names its author", async ({ page }) => {
  await open(page);
  await expect(page.locator("footer p")).toHaveText("© 2026 Bîn-hiân");
});

test("the footer rests at the window's foot, then below the page", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 1200 });
  await open(page);
  const footer = page.locator("footer");
  // trace: `body` padding-bottom 2rem at the 16px root.
  const bottom = async () => {
    const box = await footer.boundingBox();
    return box.y + box.height;
  };
  expect(await bottom()).toBeCloseTo(1200 - 32, 0);

  await page.setViewportSize({ width: 1280, height: 400 });
  const editor = await page.locator("#editor").boundingBox();
  expect((await footer.boundingBox()).y).toBeGreaterThan(
    editor.y + editor.height,
  );
});

test("the footer's icons name their places", async ({ page }) => {
  await open(page);
  // Social networks, the community's rooms, the code, then mail.
  await expect(page.locator(".social-links a")).toHaveText([
    "Facebook",
    "Instagram",
    "LINE",
    "Discord",
    "GitHub Issues",
    "Email",
  ]);
  await expect(page.locator(".social-links a").last()).toHaveAttribute(
    "href",
    "mailto:info@taigikeyboard.tw",
  );
});

test("the sidebar links fold under their own section", async ({ page }) => {
  await open(page, {
    settings: { displayLanguage: "hanji" },
    folded: true,
  });
  const links = page.locator(".sidebar-links a");
  await expect(links.first()).toBeHidden();
  await page.getByText("其他資源", { exact: true }).click();
  await expect(links).toHaveText([
    "台語齒盤官方網站",
    "台語羅馬字轉換器",
    "教典備援網站",
  ]);
});

test("General rows reach the session", async ({ page }) => {
  // trace: `RomanOnly` commits romanization; `SyllableSeparator::Space` joins its
  // syllables with a space (behavioral-invariants.md §49).
  const editor = await open(page, {
    settings: { candidateDisplayMode: "romanOnly" },
  });
  await page.getByLabel("Syllable Separator").selectOption("space");
  await editor.focus();
  await commitWord(editor);
  await expect(editor).toHaveValue("tâi uân");
});

test("Output Script stores a boolean and writes romanization", async ({
  page,
}) => {
  // trace: isTranslateSwapped false, Pairing: the row leads with romanization.
  const editor = await open(page);
  await page.getByLabel("Output Script").selectOption("false");
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          JSON.parse(localStorage.getItem("taigi.settings")).values
            .isTranslateSwapped,
      ),
    )
    .toBe(false);
  await editor.focus();
  await commitWord(editor);
  await expect(editor).toHaveValue("tâi-uân");
});

test("Output Script greys under Romanization Only", async ({ page }) => {
  await open(page, { settings: { candidateDisplayMode: "romanOnly" } });
  await expect(page.getByLabel("Output Script")).toBeDisabled();
});

test("a dictionary row keeps its default and its value", async ({ page }) => {
  await open(page);
  const moe = page.locator('input[name="moeDictEnabled"]');
  const accent = page.locator('input[name="kautianAccentLukangEnabled"]');
  await expect(moe).toBeChecked();
  await expect(
    page.locator('input[name="iTaigiDictEnabled"]'),
  ).not.toBeChecked();
  await moe.uncheck();
  await expect(accent).toBeDisabled();
  await expect(accent).toBeChecked();
  await moe.check();
  await expect(accent).toBeEnabled();
});

test("turning a source off takes its words from the list", async ({ page }) => {
  // trace: every source off leaves no entry for the word, so no Hanji is offered.
  const editor = await open(page);
  await editor.pressSequentially("taiuan");
  await expect(candidates(page).filter({ hasText: "台灣" })).not.toHaveCount(0);
  await editor.press("Escape");
  for (const row of await page
    .locator(".sidebar-section:nth-of-type(4) input:not([data-requires])")
    .all()) {
    await row.uncheck();
  }
  await editor.focus();
  await editor.pressSequentially("taiuan");
  await expect(preedit(page)).toContainText("taiuan");
  await expect(candidates(page).filter({ hasText: "台灣" })).toHaveCount(0);
});

test("Layout sets the list's shape and Size its text", async ({ page }) => {
  const editor = await open(page, { settings: { candidateTextSize: "23" } });
  await editor.pressSequentially("taiuan");
  const list = page.locator("#candidates");
  await expect(list).toHaveAttribute("data-vertical");
  await expect(panel(page)).toHaveCSS("font-size", "23px");
  await page.getByLabel("Candidate Window Layout").selectOption("horizontal");
  await editor.focus();
  await editor.pressSequentially("taiuan");
  await expect(candidates(page).first()).toBeVisible();
  await expect(list).not.toHaveAttribute("data-vertical");
});

test("a horizontal list keeps its scripts apart and turns pages", async ({
  page,
}) => {
  const editor = await open(page, {
    settings: { candidateLayout: "horizontal" },
  });
  await editor.pressSequentially("ta");
  const first = page.locator("#candidates li").first();
  await expect(first).toBeVisible();
  await expect(page.locator("#page-up")).toBeDisabled();
  const before = await first.textContent();
  await page.locator("#page-down").click();
  await expect(page.locator("#page-up")).toBeEnabled();
  await expect(first).not.toHaveText(before);
  await expect(editor).toBeFocused();
});

test("the composition sits on the caret's line, clear of the list", async ({
  page,
}) => {
  const editor = await open(page);
  const rect = (element) => {
    const { x, y, right, bottom } = element.getBoundingClientRect();
    return { x, y, right, bottom };
  };
  // From the first line to past the last: below, beside, above, then scrolled.
  for (let line = 0; line < 14; line += 1) {
    await editor.fill("\n".repeat(line));
    await editor.press("End");
    await editor.pressSequentially("tai");
    await expect(candidates(page).first()).toBeVisible();
    const composition = await preedit(page).evaluate(rect);
    const list = await panel(page).evaluate(rect);
    const box = await editor.evaluate(rect);
    const lineHeight = 1.65 * 18;
    // Past that the textarea scrolls, which WebKit animates.
    if (line < 10) {
      // trace: `#text` padding 1.5rem, line-height 1.65 at 1.125rem.
      const expected = box.y + 24 + line * lineHeight;
      expect(Math.abs(composition.y - expected)).toBeLessThanOrEqual(2);
    }
    // `offsetHeight` rounds, so up to a pixel either way.
    const isClear =
      list.right <= composition.x ||
      list.x >= composition.right ||
      list.bottom <= composition.y + 1 ||
      list.y >= composition.bottom - 1;
    expect(isClear, `line ${line}`).toBe(true);
    expect(list.bottom).toBeLessThanOrEqual(box.bottom + 1);
    await editor.press("Escape");
  }
});
