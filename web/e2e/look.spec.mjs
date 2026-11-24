// The sidebar, the settings form over the session's document, and the fixed
// slots of docs/architecture/web-roadmap.md § Phase 5.

import { expect, test } from "@playwright/test";

import {
  candidates,
  commitWord,
  current,
  emptyBox,
  expectState,
  expectText,
  firstSection,
  inputMode,
  open,
  panel,
  preedit,
  setRootTextSize,
  settled,
  TIER,
  toggle,
} from "./helpers.mjs";

/** @typedef {import("@playwright/test").Page} Page */

/** A sentence typed as one word (a Discord report): wider than a phone. */
const SENTENCE = "inuilimatsaiiannlanlongsiuitiohpatlanglehuah";

/**
 * The reset button of the section holding the `row` selector.
 * @param {Page} page
 * @param {string} row
 */
const resetSectionOf = (page, row) =>
  page
    .locator(".sidebar-section", { has: page.locator(row) })
    .getByRole("button", { name: "Reset to Defaults" });
/** @param {Page} page */
const theme = (page) =>
  page.evaluate(() => document.documentElement.dataset.theme);
/** @param {Page} page */
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
  await expectText(editor, "台灣");
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

// A phone's drawer at a large text size: each value keeps room to be read,
// and no row runs past the drawer.
test("the drawer's values stay readable at a large text size", async ({
  page,
}) => {
  await page.setViewportSize({ width: 360, height: 800 });
  await open(page);
  await toggle(page).click();
  for (const size of ["100%", "125%", "200%"]) {
    await setRootTextSize(page, size);
    // Polled: the drawer may still be sliding in, and its rows relaying out.
    await expect
      .poll(
        () =>
          page.evaluate(() =>
            [...document.querySelectorAll(".setting:has(> select)")]
              .map((row) => {
                const box = row.getBoundingClientRect();
                const value = /** @type {HTMLSelectElement} */ (
                  row.querySelector("select")
                );
                const valueBox = value.getBoundingClientRect();
                return {
                  name: value.name,
                  squeezed: valueBox.width < 2 * value.offsetHeight,
                  pastRow: valueBox.right > box.right + 0.5,
                };
              })
              .filter((row) => row.squeezed || row.pastRow),
          ),
        { message: size },
      )
      .toEqual([]);
  }
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
  /** @param {string} name */
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
  await expectText(editor, "tâi-uân");
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
  await editor.press("ControlOrMeta+End");
  await editor.evaluate((element) => {
    element.scrollTop = element.scrollHeight;
  });
  await editor.pressSequentially("taiuan");
  await expect(candidates(page).first()).toBeVisible();
  const box = await panel(page).boundingBox();
  const area = await editor.boundingBox();
  expect(box).not.toBeNull();
  expect(area).not.toBeNull();
  const b = /** @type {NonNullable<typeof box>} */ (box);
  const a = /** @type {NonNullable<typeof area>} */ (area);
  expect(b.y + b.height).toBeLessThanOrEqual(a.y + a.height);
});

test("a short window keeps the panel inside the editor", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 600 });
  const editor = await open(page);
  await editor.fill("\n");
  await editor.pressSequentially("taiuan");
  await expect(candidates(page).first()).toBeVisible();
  const top = await panel(page).evaluate(
    (element) => /** @type {HTMLElement} */ (element).offsetTop,
  );
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
      return box ? box.y + box.height : Infinity;
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
  await expect(editor).toHaveAttribute("contenteditable", "true");
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

test("/ moves into the editor from anywhere but a text field", async ({
  page,
}) => {
  const editor = await open(page);
  await toggle(page).focus();
  await page.keyboard.press("/");
  await expect(editor).toBeFocused();
  await expectText(editor, "");
  await page.getByLabel("Auto-Space").focus();
  await page.keyboard.press("/");
  await expect(editor).toBeFocused();
  await inputMode(page).focus();
  await page.keyboard.press("/");
  await expect(inputMode(page)).toBeFocused();
});

// ZenPen's page: the title and the text, nothing drawn around them once loaded.
test("around the editor, nothing", async ({ page }) => {
  await open(page);
  const seen = await page.evaluate(() =>
    Array.from(
      document.querySelectorAll("main :is(p, button, progress):not(#text *)"),
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

// A phone at 200%: the mode chip takes the room the empty load slot gives up,
// and words it still cannot hold end in an ellipsis inside it, so the page
// never scrolls sideways.
test("the mode chip keeps inside a phone-wide window", async ({ page }) => {
  for (const width of [180, 360]) {
    await page.setViewportSize({ width, height: 600 });
    const editor = await open(page, { docked: false });
    const chip = page.locator("#mode-line");
    /**
     * `rowGrew`: the row under the editor passed its own height, which a
     * squeezed hidden child wrapping would do.
     * @returns {Promise<{ pageScrolls: boolean, pastChip: string[], rowGrew: boolean }>}
     */
    const fit = () =>
      chip.evaluate((line) => {
        const box = line.getBoundingClientRect();
        const row = /** @type {HTMLElement} */ (line.parentElement);
        return {
          rowGrew:
            row.getBoundingClientRect().height >
            Number.parseFloat(getComputedStyle(row).minHeight) + 0.5,
          pageScrolls:
            document.documentElement.scrollWidth >
            document.documentElement.clientWidth,
          pastChip: [...line.children]
            .filter((part) => part.getBoundingClientRect().right > box.right)
            .map((part) => part.id),
        };
      });
    const clean = { pageScrolls: false, pastChip: [], rowGrew: false };
    await expect(chip).toHaveAttribute("data-drawn");
    expect(await fit()).toEqual(clean);
    await editor.dispatchEvent("compositionstart");
    await expect(page.locator("#notice")).toHaveAttribute("data-active");
    expect(await fit()).toEqual(clean);
  }
});

test("the editor takes the window's room left over", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  const editor = await open(page);
  const initialBox = await editor.boundingBox();
  expect(initialBox).not.toBeNull();
  const short = /** @type {NonNullable<typeof initialBox>} */ (initialBox)
    .height;
  await page.setViewportSize({ width: 1280, height: 1200 });
  // Every pixel the window gains goes to the editor.
  await expect
    .poll(async () => (await editor.boundingBox())?.height)
    .toBeCloseTo(short + 400, 0);
});

test("the footer links to the website's sponsor section", async ({ page }) => {
  await open(page);
  const footer = page.locator("footer p");
  await expect(footer).toHaveText("© 2026 Bîn-hiân · 贊助支持");
  await expect(footer.getByRole("link")).toHaveAttribute(
    "href",
    "https://taigikeyboard.tw/#sponsor",
  );
});

test.describe("in Taipei", () => {
  test.use({ locale: "en-US", timezoneId: "Asia/Taipei" });

  test("the footer names the staged commit and time in the browser's zone", async ({
    page,
  }) => {
    await open(page);
    const commit = page.locator("#build-commit");
    await expect(commit).toHaveText(/^[0-9a-f]{7,}$/);
    const short = await commit.textContent();
    await expect(commit).toHaveAttribute(
      "href",
      new RegExp(
        `^https://github\\.com/taigikeyboard/taigikeyboard/commit/${short}[0-9a-f]*$`,
      ),
    );
    const time = page.locator("#build-time");
    await expect(time).toHaveAttribute(
      "datetime",
      /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{3}Z$/,
    );
    // trace: en-US, hourCycle h23, timeZoneName "short" in Asia/Taipei →
    // "11/23/2026, 19:20 GMT+8": no AM/PM, hour 00–23.
    await expect(time).toHaveText(
      /^\d\d\/\d\d\/\d{4},\s([01]\d|2[0-3]):\d\d\sGMT\+8$/,
    );
  });
});

test("the footer rests at the sidebar's foot, then below its sections", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 1200 });
  await open(page, { folded: true });
  const footer = page.locator("footer");
  // trace: `#sidebar` padding-bottom 1.5rem at the 16px root.
  const box = await footer.boundingBox();
  expect(box).not.toBeNull();
  const f = /** @type {NonNullable<typeof box>} */ (box);
  expect(f.y + f.height).toBeCloseTo(1200 - 24, 0);
  // Every section open: the footer follows the last one, out of view.
  for (const section of await page.locator(".sidebar-section").all()) {
    await section.evaluate((element) => {
      /** @type {HTMLDetailsElement} */ (element).open = true;
    });
  }
  const last = await page.locator(".sidebar-section").last().boundingBox();
  const footerBox = await footer.boundingBox();
  expect(last).not.toBeNull();
  expect(footerBox).not.toBeNull();
  const l = /** @type {NonNullable<typeof last>} */ (last);
  expect(
    /** @type {NonNullable<typeof footerBox>} */ (footerBox).y,
  ).toBeGreaterThanOrEqual(l.y + l.height);
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
  await expectText(editor, "tâi uân");
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
          JSON.parse(localStorage.getItem("taigi.settings") ?? "{}").values
            .isTranslateSwapped,
      ),
    )
    .toBe(false);
  await editor.focus();
  await commitWord(editor);
  await expectText(editor, "tâi-uân");
});

test("Output Script greys under Romanization Only", async ({ page }) => {
  await open(page, { settings: { candidateDisplayMode: "romanOnly" } });
  await expect(page.getByLabel("Output Script")).toBeDisabled();
  await expect(page.getByLabel("Annotate in Brackets")).toBeDisabled();
});

test("Annotate in Brackets writes the romanization after the pick", async ({
  page,
}) => {
  // trace: Hanji first, brackets on → `台語 (tâi-gí)`; Auto-Space is off, but a
  // bracket unit still takes its space (`spaces_candidate_commits`, D2).
  const editor = await open(page);
  await page.getByLabel("Annotate in Brackets").check();
  await editor.focus();
  await editor.pressSequentially("tai5gi2");
  await editor.press("Enter");
  await expectText(editor, "台語 (tâi-gí) ");
});

test("Annotate in Brackets greys under TPS", async ({ page }) => {
  // trace: TPS commits its Hanji as shown (`allows_output_both_scripts`, D4).
  await open(page, { settings: { inputMode: "tps", outputBothScripts: true } });
  await expect(page.getByLabel("Annotate in Brackets")).toBeDisabled();
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
    () => JSON.parse(localStorage.getItem("taigi.settings") ?? "{}").values,
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
  const before = (await first.textContent()) ?? "";
  await page.locator("#page-down").click();
  await expect(page.locator("#page-up")).toBeEnabled();
  await expect(first).not.toHaveText(before);
  await expect(editor).toBeFocused();
});

// A list that runs past one page keeps one window width from page to page and
// list to list, so the right edge does not move as the words change (macOS
// PR #894; `layout.rs` `horizontal_page_starts`).
for (const [candidateLayout, name] of [
  ["horizontal", "Horizontal"],
  ["expandable", "Horizontal Expandable"],
]) {
  test(`a ${name} list keeps one width across lists and pages`, async ({
    page,
  }) => {
    const editor = await open(page, { settings: { candidateLayout } });
    const widths = new Set();
    const firstCells = new Set();
    /** @returns {Promise<void>} */
    const record = async () => {
      // The arrow shows only for a list that runs past one page.
      const arrow = candidateLayout === "horizontal" ? "#page-down" : "#unfold";
      await expect(page.locator(arrow)).toBeVisible();
      const box = await panel(page).boundingBox();
      widths.add(Math.round(box?.width ?? 0));
      firstCells.add(await candidates(page).first().textContent());
    };
    for (const prefix of ["ti", "kong", "lang", "sim"]) {
      await editor.pressSequentially(prefix);
      await record();
      if (candidateLayout === "horizontal") {
        await page.locator("#page-down").click();
        await record();
      }
      await editor.press("Escape");
      await expect(candidates(page)).toHaveCount(0);
    }
    // Different words on screen, one width.
    expect(firstCells.size).toBeGreaterThan(1);
    expect([...widths]).toHaveLength(1);
  });
}

// A phone at Extra Large: the chevron column is whole pixels on both sides,
// so the paged row leaves it its room inside the window.
test("the page arrows stay inside a phone-wide window", async ({ page }) => {
  await page.setViewportSize({ width: 360, height: 800 });
  for (const layout of ["horizontal", "expandable"]) {
    const editor = await open(page, {
      settings: { candidateLayout: layout, candidateTextSize: "23" },
      docked: false,
    });
    await editor.pressSequentially("tsit");
    await expect(page.locator("#page-arrows")).toBeVisible();
    const fit = await panel(page).evaluate((root) => {
      const list = /** @type {HTMLElement} */ (root.querySelector("#list"));
      const arrows = /** @type {HTMLElement} */ (
        root.querySelector("#page-arrows")
      );
      return {
        pastWindow:
          arrows.getBoundingClientRect().right >
          root.getBoundingClientRect().right,
        listScrolls: list.scrollWidth > list.clientWidth,
      };
    });
    expect(fit, layout).toEqual({ pastWindow: false, listScrolls: false });
  }
});

test("a Horizontal list follows a window that narrowed at the next key", async ({
  page,
}) => {
  // Extra Large: the widest cells, so the fewest per page.
  const editor = await open(page, {
    settings: { candidateLayout: "horizontal", candidateTextSize: "23" },
  });
  await editor.pressSequentially("ti");
  await expect(page.locator("#page-down")).toBeVisible();
  const wide = (await panel(page).boundingBox())?.width ?? 0;
  await page.setViewportSize({ width: 420, height: 800 });
  await editor.press("ArrowRight");
  const fits = await page.evaluate(() => {
    /** @param {string} selector */
    const right = (selector) =>
      document.querySelector(selector)?.getBoundingClientRect().right ?? 0;
    return {
      width: document.querySelector("#panel")?.getBoundingClientRect().width,
      arrowsInPanel: right("#page-arrows") <= right("#panel") + 0.5,
      panelInEditor: right("#panel") <= right("#editor") + 0.5,
    };
  });
  expect(fits.width).toBeLessThan(wide);
  expect(fits.arrowsInPanel).toBe(true);
  expect(fits.panelInEditor).toBe(true);
});

test("the composition sits on the caret's line, clear of the list", async ({
  page,
}) => {
  const editor = await open(page);
  /**
   * @param {Element} element
   */
  const rect = (element) => {
    const { x, y, right, bottom } = element.getBoundingClientRect();
    return { x, y, right, bottom };
  };
  // From the first line to past the last: below, beside, above, then scrolled.
  for (let line = 0; line < 14; line += 1) {
    // Line breaks in one paragraph: every line 30px, no block margin between.
    await emptyBox(editor);
    for (let index = 0; index < line; index += 1) {
      await editor.press("Shift+Enter");
    }
    await editor.pressSequentially("tai");
    await expect(candidates(page).first()).toBeVisible();
    const composition = await preedit(page).evaluate(rect);
    const list = await panel(page).evaluate(rect);
    const box = await editor.evaluate(rect);
    const lineHeight = 30;
    // Past that the editor scrolls, which WebKit animates.
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

// A phone at 200% zoom: a column taller than the editor fits on no side of the
// composition, so it takes the taller side and scrolls there, never over it.
test("a column no side holds scrolls beside the composition, not over it", async ({
  page,
}) => {
  await page.setViewportSize({ width: 180, height: 400 });
  for (const word of ["tsiahpa", SENTENCE]) {
    const editor = await open(page, { docked: false });
    await editor.pressSequentially(word);
    await expect(candidates(page).first()).toBeVisible();
    const fit = await page.evaluate(() => {
      const box = (/** @type {string} */ id) =>
        /** @type {HTMLElement} */ (
          document.getElementById(id)
        ).getBoundingClientRect();
      const composition = box("preedit");
      const list = box("panel");
      const editorBox = box("editor");
      return {
        overlaps:
          list.right > composition.x &&
          list.x < composition.right &&
          list.bottom > composition.y + 1 &&
          list.y < composition.bottom - 1,
        pastEditor: list.bottom > editorBox.bottom + 1,
      };
    });
    expect(fit, word).toEqual({ overlaps: false, pastEditor: false });
    await expect(current(page)).toBeInViewport();
    // Walking the capped list scrolls the list, never the page under it.
    const before = await page.evaluate(() => window.scrollY);
    for (let step = 0; step < 8; step += 1) {
      await editor.press("ArrowDown");
    }
    await expect(current(page)).toBeInViewport();
    expect(await page.evaluate(() => window.scrollY), word).toBe(before);
  }
});

/// How far the selection sits inside the window, and every corner of the
/// selected cell's text that falls outside the window's rounded box or the
/// selection's.
/** @param {import("@playwright/test").Page} page */
const selectionFit = (page) =>
  panel(page).evaluate((root) => {
    const cell = /** @type {Element} */ (
      root.querySelector('li[aria-current="true"]')
    );
    const mark = getComputedStyle(cell, "::before");
    const window = root.getBoundingClientRect();
    const cellBox = cell.getBoundingClientRect();
    const inset = Number.parseFloat(mark.top);
    const shapes = {
      window: {
        box: window,
        radius: Number.parseFloat(getComputedStyle(root).borderTopLeftRadius),
      },
      selection: {
        box: new DOMRect(
          cellBox.x + inset,
          cellBox.y + inset,
          cellBox.width - 2 * inset,
          cellBox.height - 2 * inset,
        ),
        radius: Number.parseFloat(mark.borderTopLeftRadius),
      },
    };
    const outside = [];
    for (const part of cell.querySelectorAll(":scope > span")) {
      const text = part.getBoundingClientRect();
      for (const [name, { box, radius }] of Object.entries(shapes)) {
        for (const x of [text.left, text.right]) {
          for (const y of [text.top, text.bottom]) {
            const dx = Math.max(
              box.left + radius - x,
              x - (box.right - radius),
              0,
            );
            const dy = Math.max(
              box.top + radius - y,
              y - (box.bottom - radius),
              0,
            );
            const inside =
              x >= box.left - 0.5 &&
              x <= box.right + 0.5 &&
              y >= box.top - 0.5 &&
              y <= box.bottom + 0.5 &&
              Math.hypot(dx, dy) <= radius + 0.5;
            if (!inside) {
              outside.push(`${part.className} past the ${name} at ${x},${y}`);
            }
          }
        }
      }
    }
    return { inset: cellBox.top + inset - window.top, outside };
  });

test("the selection keeps one inset and the text stays inside the corners", async ({
  page,
}) => {
  const insets = new Set();
  for (const layout of [
    "vertical",
    "horizontal",
    "expandable",
    "verticalExpandable",
  ]) {
    for (const size of ["13", "17", "23"]) {
      const editor = await open(page, {
        settings: { candidateLayout: layout, candidateTextSize: size },
      });
      await editor.pressSequentially("tshuann");
      await expect(candidates(page).first()).toBeVisible();
      const { inset, outside } = await selectionFit(page);
      expect(outside, `${layout} ${size}`).toEqual([]);
      insets.add(inset);
    }
  }
  expect([...insets]).toHaveLength(1);
});

/**
 * The scripts whose text runs past their own box, by its range: a script that
 * keeps one line and is drawn narrower than its text ends in an ellipsis. The
 * range, not `scrollWidth`, which a box clipped on one axis does not report
 * alike in every engine.
 * @param {Page} page
 */
const cutScripts = (page) =>
  panel(page).evaluate((root) =>
    [...root.querySelectorAll("li > span:is(.text, .annotation)")]
      .filter((part) => {
        const range = document.createRange();
        range.selectNodeContents(part);
        const text = range.getBoundingClientRect();
        const box = part.getBoundingClientRect();
        return (
          range.getClientRects().length > 1 || text.width > box.width + 0.01
        );
      })
      .map((part) => part.textContent),
  );

/// Every script of every cell whole, collapsed and unfolded: a cell the model
/// sizes is as wide as the model measured, so any length drawn wider than the
/// model's cuts its widest script.
test("no script is cut at any size in any layout", async ({ page }) => {
  // Sixty compositions in five sizes: past the default under a loaded runner.
  test.setTimeout(90_000);
  /** @type {Array<[string, string | null]>} */
  const layouts = [
    ["vertical", null],
    ["horizontal", null],
    ["expandable", "ArrowDown"],
    ["verticalExpandable", "ArrowRight"],
  ];
  for (const [layout, unfoldKey] of layouts) {
    // `kong` and `tsit`: annotations a Gecko layout unit from their box.
    for (const size of ["13", "15", "17", "20", "23"]) {
      const editor = await open(page, {
        settings: { candidateLayout: layout, candidateTextSize: size },
      });
      for (const word of ["tsiahpa", "kong", "tsit"]) {
        // The last word's list goes with its composition: nothing is written.
        for (
          let step = 0;
          step < 3 && (await candidates(page).count());
          step++
        ) {
          await editor.press("Escape");
        }
        await expect(candidates(page)).toHaveCount(0);
        await editor.pressSequentially(word);
        await expect(candidates(page).first()).toBeVisible();
        const at = `${layout} ${size} ${word}`;
        expect(await cutScripts(page), at).toEqual([]);
        if (unfoldKey) {
          await editor.press(unfoldKey);
          await expect(page.locator("#candidates")).toHaveAttribute(
            "data-grid",
          );
          await settled(page);
          expect(await cutScripts(page), `${at} unfolded`).toEqual([]);
        }
      }
    }
  }
});

/// A cell wider than the window (a Discord report: a sentence typed as one
/// word): each cell keeps its height inside the panel, the candidate whole
/// and the annotation cut with an ellipsis, as macOS truncates it.
test("a cell wider than the window cuts its annotation, not its row", async ({
  page,
}) => {
  await page.setViewportSize({ width: 360, height: 800 });
  for (const layout of ["vertical", "verticalExpandable", "expandable"]) {
    const editor = await open(page, {
      settings: { candidateLayout: layout },
      docked: false,
    });
    await editor.pressSequentially(SENTENCE);
    await expect(candidates(page).first()).toBeVisible();
    const fit = await panel(page).evaluate((root) => {
      const window = root.getBoundingClientRect();
      const rows = [...root.querySelectorAll("#candidates li")];
      return {
        tallRows: rows
          .filter((row) => row.scrollHeight > row.clientHeight)
          .map((row) => row.textContent),
        pastWindow: [...root.querySelectorAll("#candidates li > span")]
          .filter((part) => part.getBoundingClientRect().right > window.right)
          .map((part) => part.textContent),
      };
    });
    expect(fit, layout).toEqual({ tallRows: [], pastWindow: [] });
    // The annotation gives way first: a candidate is cut only once its row's
    // annotation has nothing left to give (how much fits depends on the
    // face the machine draws Hanji in).
    const cut = await cutScripts(page);
    const first = candidates(page).first();
    expect(await first.locator(".text").textContent(), layout).toBe(
      "因為你嘛知影咱攏是為著別人咧活",
    );
    expect(cut, layout).toContain(
      await first.locator(".annotation").textContent(),
    );
    const candidateCutBeforeAnnotation = await panel(page).evaluate((root) =>
      [...root.querySelectorAll("#candidates li")]
        .filter((row) => {
          const text = row.querySelector(".text");
          const annotation = row.querySelector(".annotation");
          if (!text || !annotation) {
            return false;
          }
          const range = document.createRange();
          range.selectNodeContents(text);
          const isCut =
            range.getBoundingClientRect().width >
            text.getBoundingClientRect().width + 0.01;
          const annotationRoom =
            annotation.getBoundingClientRect().width -
            Number.parseFloat(getComputedStyle(annotation).paddingLeft);
          return isCut && annotationRoom > 0.5;
        })
        .map((row) => row.textContent),
    );
    expect(candidateCutBeforeAnnotation, layout).toEqual([]);
  }
});

/**
 * How far each drawn script of each cell runs into its cell's end padding.
 * @param {Page} page
 */
const paddingOverruns = (page) =>
  panel(page).evaluate((root) =>
    [...root.querySelectorAll("#candidates li")].flatMap((row) => {
      const end =
        row.getBoundingClientRect().right -
        Number.parseFloat(getComputedStyle(row).paddingRight);
      // A script cut to nothing draws nothing: its gap is padding alone.
      return [...row.querySelectorAll(".text, .annotation")]
        .filter(
          (part) =>
            part.getBoundingClientRect().width >
            Number.parseFloat(getComputedStyle(part).paddingLeft),
        )
        .map((part) => part.getBoundingClientRect())
        .filter((box) => box.right > end + 0.5)
        .map((box) => box.right - end);
    }),
  );

/// A candidate alone wider than the window: every script ends inside its
/// row's padding, collapsed and unfolded.
test("a candidate wider than the window stops at the row's padding", async ({
  page,
}) => {
  await page.setViewportSize({ width: 360, height: 800 });
  for (const layout of ["vertical", "verticalExpandable"]) {
    for (const size of ["17", "23"]) {
      const editor = await open(page, {
        settings: { candidateLayout: layout, candidateTextSize: size },
        docked: false,
      });
      await editor.pressSequentially(SENTENCE);
      await expect(candidates(page).first()).toBeVisible();
      expect(await paddingOverruns(page), `${layout} ${size}`).toEqual([]);
      if (layout === "verticalExpandable") {
        await editor.press("ArrowRight");
        await expect(page.locator("#candidates")).toHaveAttribute("data-grid");
        await settled(page);
        expect(
          await paddingOverruns(page),
          `${layout} ${size} unfolded`,
        ).toEqual([]);
      }
    }
  }
});
