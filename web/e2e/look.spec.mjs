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
  TIER,
  toggle,
} from "./helpers.mjs";

/// The reset button of the section holding the `row` selector.
const resetSectionOf = (page, row) =>
  page
    .locator(".sidebar-section", { has: page.locator(row) })
    .getByRole("button", { name: "Reset to Defaults" });
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

test("the settings chord opens and closes the sidebar", async ({ page }) => {
  // trace: Ctrl+Alt+S = OpenLastSettingsPane, `Emit::OpenSettings`.
  const editor = await open(page);
  await expect(toggle(page)).toHaveAttribute("aria-expanded", "true");
  await editor.press(`${TIER}+s`);
  await expect(toggle(page)).toHaveAttribute("aria-expanded", "false");
  await editor.press(`${TIER}+s`);
  await expect(toggle(page)).toHaveAttribute("aria-expanded", "true");
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
  await editor.press(`${TIER}+c`);
  await expect(inputMode(page)).toHaveValue("poj");
});

test("a form change keeps what a chord wrote", async ({ page }) => {
  const editor = await open(page);
  await editor.press(`${TIER}+c`);
  await expect(inputMode(page)).toHaveValue("poj");
  await page.getByLabel("Appearance").selectOption("dark");
  await expect(inputMode(page)).toHaveValue("poj");
  await page.reload();
  await expect(inputMode(page)).toHaveValue("poj");
});

test("Appearance pins the theme before first paint", async ({ page }) => {
  await open(page);
  await page.getByLabel("Appearance").selectOption("dark");
  await expect.poll(() => pageColour(page)).toBe("rgb(17, 17, 17)");
  await page.getByLabel("Appearance").selectOption("light");
  await expect.poll(() => pageColour(page)).toBe("rgb(253, 253, 253)");
  await page.getByLabel("Appearance").selectOption("dark");
  // The script runs at parse time, ahead of the module that fills the form.
  await page.route("**/ime.js", (route) => route.abort());
  await page.reload();
  expect(await theme(page)).toBe("dark");
  await expect.poll(() => pageColour(page)).toBe("rgb(17, 17, 17)");
  // WebKit once lost the `body` rules from the first reload on.
  await page.reload();
  await expect.poll(() => pageColour(page)).toBe("rgb(17, 17, 17)");
});

test("a stored value no control offers shows the default", async ({ page }) => {
  await open(page, { settings: { inputMode: "bopomofo" } });
  await expect(inputMode(page)).toHaveValue("tl");
});

test("the panel stays inside the editor at its foot", async ({ page }) => {
  const editor = await open(page);
  await editor.fill("\n".repeat(60));
  await editor.evaluate((element) => {
    element.setSelectionRange(60, 60);
    element.scrollTop = element.scrollHeight;
  });
  await editor.pressSequentially("taiuan");
  await expect(candidates(page).first()).toBeVisible();
  const box = await panel(page).boundingBox();
  const area = await editor.boundingBox();
  expect(box.y + box.height).toBeLessThanOrEqual(area.y + area.height);
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
  await toggle(page).focus();
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

// ZenPen's page: the title and the text, nothing drawn around them once loaded.
test("around the textarea, nothing", async ({ page }) => {
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
        return isDrawn ? element.id : null;
      },
    ).filter((id) => id !== null),
  );
  expect(seen).toEqual([]);
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

test("the textarea takes the window's room left over", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  const editor = await open(page);
  const short = (await editor.boundingBox()).height;
  await page.setViewportSize({ width: 1280, height: 1200 });
  // Every pixel the window gains goes to the textarea.
  await expect
    .poll(async () => (await editor.boundingBox()).height)
    .toBeCloseTo(short + 400, 0);
});

test("the footer matches the converter's", async ({ page }) => {
  await open(page);
  const footer = page.locator("footer p");
  await expect(footer).toHaveText("© 2026 Bîn-hiân · 贊助支持 ❤️");
  await expect(footer.getByRole("link")).toHaveAttribute(
    "href",
    "https://p.ecpay.com.tw/AA663DE",
  );
});

test("the footer rests at the sidebar's foot, then below its sections", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 1200 });
  await open(page, { folded: true });
  const footer = page.locator("footer");
  // trace: `#sidebar` padding-bottom 1.5rem at the 16px root.
  const box = await footer.boundingBox();
  expect(box.y + box.height).toBeCloseTo(1200 - 24, 0);
  // Every section open: the footer follows the last one, out of view.
  for (const section of await page.locator(".sidebar-section").all()) {
    await section.evaluate((element) => {
      element.open = true;
    });
  }
  const last = await page.locator(".sidebar-section").last().boundingBox();
  expect((await footer.boundingBox()).y).toBeGreaterThanOrEqual(
    last.y + last.height,
  );
});

test("the footer closes with the sidebar", async ({ page }) => {
  await open(page);
  await expect(page.locator("footer")).toBeVisible();
  await toggle(page).click();
  await expect(page.locator("footer")).toBeHidden();
});

test("the footer's icons name their places", async ({ page }) => {
  await open(page);
  // Social networks, the community's rooms, then the code.
  await expect(page.locator(".social-links a")).toHaveText([
    "Facebook",
    "Instagram",
    "LINE",
    "Discord",
    "GitHub Issues",
  ]);
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
    // trace: web.desktopApp / web.iosApp / web.androidApp hanji (i18n/web.json).
    "台語齒盤電腦版",
    "台語齒盤 iOS",
    "台語齒盤 Android",
    "台語羅馬字轉換器",
    "教典備援網站",
    "教典鬥搜揣",
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

test("a section's reset takes back its own rows and keeps the language", async ({
  page,
}) => {
  const editor = await open(page, {
    settings: {
      inputMode: "poj",
      displayLanguage: "en",
      lastRomanizationMode: "poj",
      candidateAppearanceMode: "dark",
    },
  });
  await resetSectionOf(page, 'select[name="inputMode"]').click();
  await expect(inputMode(page)).toHaveValue("tl");
  await expect(page.locator('select[name="displayLanguage"]')).toHaveValue(
    "en",
  );
  await expect(
    page.locator('select[name="candidateAppearanceMode"]'),
  ).toHaveValue("dark");
  const stored = await page.evaluate(
    () => JSON.parse(localStorage.getItem("taigi.settings")).values,
  );
  expect(stored).toEqual({
    displayLanguage: "en",
    candidateAppearanceMode: "dark",
  });
  // The session took it too: TL's spelling, not POJ's.
  await editor.focus();
  await editor.pressSequentially("tsiah");
  await expect(preedit(page)).toContainText("tsiah");
});

test("the dictionary reset turns the sources back on", async ({ page }) => {
  const editor = await open(page, { settings: { moeDictEnabled: false } });
  const accent = page.locator('input[name="kautianAltReadingEnabled"]');
  await expect(accent).toBeDisabled();
  await resetSectionOf(page, 'input[name="moeDictEnabled"]').click();
  await expect(accent).toBeEnabled();
  await expect(page.locator('input[name="moeDictEnabled"]')).toBeChecked();
  await editor.focus();
  await editor.pressSequentially("taiuan");
  await expect(candidates(page).filter({ hasText: "台灣" })).not.toHaveCount(0);
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
    const lineHeight = 30;
    // Past that the textarea scrolls, which WebKit animates.
    if (line < 8) {
      // trace: `#text` padding-top 22px, line-height 30px.
      const expected = box.y + 22 + line * lineHeight;
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
