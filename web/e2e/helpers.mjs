import { expect } from "@playwright/test";

/** @typedef {import("@playwright/test").Locator} Locator */
/** @typedef {import("@playwright/test").Page} Page */
/** @typedef {import("../types/shared.d.ts").SettingsValue} SettingsValue */

/**
 * The global tier on the machine running the test: a Mac's page takes the
 * Mac app's chords (`key.rs`).
 */
export const TIER =
  process.platform === "darwin" ? "Control+Meta" : "Control+Alt";

/**
 * `settings` as if a past visit had stored them.
 * @param {Page} page
 * @param {Record<string, SettingsValue>} settings
 * @returns {ReturnType<Page["addInitScript"]>}
 */
export function seedSettings(page, settings) {
  return page.addInitScript(
    (document) => {
      localStorage.setItem("taigi.settings", document);
    },
    JSON.stringify({ values: settings }),
  );
}

/**
 * `text` as if a past visit had stored it, on the first load only: a stored text
 * comes in through `setMarkdown`, so the undo history starts empty.
 * @param {Page} page
 * @param {string} text
 * @returns {ReturnType<Page["addInitScript"]>}
 */
export function seedText(page, text) {
  return page.addInitScript((text) => {
    if (sessionStorage.getItem("test.seeded") === null) {
      sessionStorage.setItem("test.seeded", "");
      localStorage.setItem("taigi.document", text);
    }
  }, text);
}

/**
 * Clicks Save's `format` button: the file's name and its text.
 * @param {Page} page
 * @param {string} format
 * @returns {Promise<{ name: string, text: string }>}
 */
export async function saveAs(page, format) {
  const [download] = await Promise.all([
    page.waitForEvent("download"),
    page.locator(`#save-formats [data-format="${format}"]`).click(),
  ]);
  /** @type {Buffer[]} */
  const chunks = [];
  for await (const chunk of await download.createReadStream()) {
    chunks.push(/** @type {Buffer} */ (chunk));
  }
  return {
    name: download.suggestedFilename(),
    text: Buffer.concat(chunks).toString("utf8"),
  };
}

/**
 * The page with `settings` as if a past visit had stored them. `ready`: wait
 * for the dictionaries. `folded`: leave the sidebar's sections as a visit finds
 * them, rather than open for a test that works on their rows. `docked`: the
 * sidebar open, as a past visit that opened it left it (it starts closed).
 * `intro`: keep the intro text a first visit finds, rather than an empty box
 * for a test that asserts what it typed.
 * @param {Page} page
 * @param {{
 *   settings?: Record<string, SettingsValue>,
 *   ready?: boolean,
 *   folded?: boolean,
 *   docked?: boolean,
 *   intro?: boolean,
 * }} [options]
 * @returns {Promise<Locator>}
 */
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
  await expect(editor).toHaveAttribute("contenteditable", "true");
  await expect(editor).toBeFocused();
  if (!intro) {
    await emptyBox(editor);
  }
  if (ready) {
    await expectState(page, "ready");
  }
  return editor;
}

/**
 * Takes the intro an empty box opens on out of `editor`, for a test that
 * asserts what it types. No `input`: nothing is stored, as for a box the user
 * has not touched, so a reload brings the intro back.
 * @param {Locator} editor
 * @returns {Promise<void>}
 */
export function emptyBox(editor) {
  return editor.evaluate((element) => {
    /** @type {HTMLElement & { taigiEditor: { setMarkdown(text: string): void } }} */ (
      element
    ).taigiEditor.setMarkdown("");
  });
}

/**
 * The editor's text as it is stored: Markdown, which is the text itself when
 * nothing is formatted.
 * @param {Locator} editor
 * @returns {Promise<string>}
 */
export const textOf = (editor) =>
  editor.evaluate((element) =>
    /** @type {HTMLElement & { taigiEditor: { markdown(): string } }} */ (
      element
    ).taigiEditor.markdown(),
  );

/**
 * The editor holds `expected`, a string or a pattern, now or once its update
 * lands.
 * @param {Locator} editor
 * @param {string | RegExp} expected
 * @returns {Promise<void>}
 */
export function expectText(editor, expected) {
  const poll = expect.poll(() => textOf(editor));
  return expected instanceof RegExp
    ? poll.toMatch(expected)
    : poll.toBe(expected);
}

/**
 * As `expectText`, for a text the editor must not hold.
 * @param {Locator} editor
 * @param {string | RegExp} expected
 * @returns {Promise<void>}
 */
export function expectNotText(editor, expected) {
  return expect.poll(() => textOf(editor)).not.toBe(expected);
}

/**
 * On every load, so a reload keeps them open.
 * @param {Page} page
 * @returns {ReturnType<Page["addInitScript"]>}
 */
const unfoldSections = (page) =>
  page.addInitScript(() => {
    document.addEventListener("DOMContentLoaded", () => {
      for (const section of document.querySelectorAll(".sidebar-section")) {
        /** @type {HTMLDetailsElement} */ (section).open = true;
      }
    });
  });

/** @param {Page} page */
export const firstSection = (page) => page.locator("summary").first();
/** @param {Page} page */
export const toggle = (page) => page.locator("#sidebar-toggle");
/** @param {Page} page */
export const preedit = (page) => page.locator("#preedit");
/** @param {Page} page */
export const candidates = (page) => page.locator("#candidates li");
/** @param {Page} page */
export const inputMode = (page) => page.locator('select[name="inputMode"]');
/** @param {Page} page */
export const panel = (page) => page.locator("#panel");

/**
 * @param {Page} page
 * @param {string} state
 * @returns {Promise<void>}
 */
export function expectState(page, state) {
  return expect(page.locator("#message")).toHaveAttribute("data-state", state);
}

/**
 * @param {Locator} editor
 * @returns {Promise<void>}
 */
export async function commitWord(editor) {
  await editor.pressSequentially("taiuan");
  await editor.press("Enter");
}

/**
 * Selects `part`, the first text node holding it, as a drag would; or puts the
 * caret at its start with `collapse`. Waits for Lexical to read the selection.
 * @param {Locator} editor
 * @param {string} part
 * @param {{ collapse?: boolean }} [options]
 * @returns {Promise<void>}
 */
export function select(editor, part, { collapse = false } = {}) {
  return editor.evaluate(
    (element, [part, collapse]) => {
      const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT);
      for (let node = walker.nextNode(); node; node = walker.nextNode()) {
        const text = /** @type {Text} */ (node);
        const start = text.data.indexOf(part);
        if (start >= 0) {
          /** @type {HTMLElement} */ (element).focus();
          const end = collapse ? start : start + part.length;
          getSelection()?.setBaseAndExtent(text, start, text, end);
          return new Promise((resolve) =>
            requestAnimationFrame(() => setTimeout(resolve)),
          );
        }
      }
      throw new Error(`no text node holds ${part}`);
    },
    /** @type {const} */ ([part, collapse]),
  );
}

/**
 * The selected text.
 * @param {Locator} editor
 * @returns {Promise<string>}
 */
export const selected = (editor) =>
  editor.evaluate(() => getSelection()?.toString() ?? "");

/**
 * The text between the caret and the editor's end; `""` with the caret at the
 * end, `null` with no caret yet.
 * @param {Locator} editor
 * @returns {Promise<string | null>}
 */
export const textAfterCaret = (editor) =>
  editor.evaluate((element) => {
    const caret = getSelection();
    if (!caret || caret.focusNode === null) {
      return null;
    }
    const range = document.createRange();
    range.setStart(caret.focusNode, caret.focusOffset);
    range.setEnd(element, element.childNodes.length);
    return range.toString();
  });

/**
 * Puts the caret `offset` characters into the editor's text, a line break one
 * character, as a script would, and waits for Lexical to read it.
 * @param {Locator} editor
 * @param {number} offset
 * @returns {Promise<void>}
 */
export function caretTo(editor, offset) {
  return editor.evaluate((element, offset) => {
    const walker = document.createTreeWalker(
      element,
      NodeFilter.SHOW_TEXT | NodeFilter.SHOW_ELEMENT,
    );
    let left = offset;
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      if (node.nodeName === "BR") {
        if (left === 0) {
          const parent = node.parentNode;
          if (parent) {
            getSelection()?.collapse(
              parent,
              [...parent.childNodes].indexOf(/** @type {ChildNode} */ (node)),
            );
          }
          return new Promise((resolve) =>
            requestAnimationFrame(() => setTimeout(resolve)),
          );
        }
        left -= 1;
        continue;
      }
      if (node.nodeType !== Node.TEXT_NODE) {
        continue;
      }
      const text = /** @type {Text} */ (node);
      if (left <= text.length) {
        getSelection()?.collapse(text, left);
        return new Promise((resolve) =>
          requestAnimationFrame(() => setTimeout(resolve)),
        );
      }
      left -= text.length;
    }
    throw new Error(`the text is shorter than ${offset}`);
  }, offset);
}
