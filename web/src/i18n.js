// The page's strings, from the generated `i18n/*.json` maps. A translated element
// names its key: `data-i18n` for its text, `data-i18n-<attribute>` for an
// attribute. The markup's English text only holds the layout; style.css keeps it
// out of sight until `setDisplayLanguage` has run.

import { CAPTIONS, STRINGS } from "./generated/strings.js";

/** @typedef {keyof typeof STRINGS} DisplayLanguage */
/** @typedef {keyof (typeof STRINGS)["hanji"]} StringKey */

/** @type {Record<string, string>} */
const CAPTION_KEYS = CAPTIONS;

const ATTRIBUTES = ["aria-label", "title"];

/** BCP-47 per language: the browser picks CJK glyphs and hyphenation by it. */
/** @type {Record<DisplayLanguage, string>} */
const LANGUAGE_TAGS = {
  hanji: "nan-Hant-TW",
  tailo: "nan-Latn-tailo",
  poj: "nan-Latn-pehoeji",
  ja: "ja",
  en: "en",
};

/** @type {DisplayLanguage} */
let language = "hanji";

/**
 * The language of `tag`, the stored `displayLanguage`. Twin of `DisplayLanguage::
 * from_tag` + `effective` in `taigi-desktop-core`, which answer for the strings
 * the session sends: the page draws before the engine has loaded.
 * @param {string} tag
 * @param {string} locale
 * @returns {DisplayLanguage}
 */
function effectiveLanguage(tag, locale) {
  if (tag === "system") {
    const subtag = locale.split(/[-_]/)[0].toLowerCase();
    return subtag === "ja" || subtag === "en" ? subtag : "hanji";
  }
  return Object.hasOwn(LANGUAGE_TAGS, tag)
    ? /** @type {DisplayLanguage} */ (tag)
    : "hanji";
}

/**
 * Every key carries every production language (`validate_production_completeness`).
 * @param {StringKey | string} key
 * @param {Record<string, string>} [placeholders]
 * @returns {string}
 */
export function translate(key, placeholders = {}) {
  const table = /** @type {Record<string, string>} */ (STRINGS[language]);
  return table[key].replace(/\{(\w+)\}/g, (_, name) => placeholders[name]);
}

/**
 * `translate` answers in the language of `tag` from here on. For a caller
 * with no page of its own to draw (the browser extension on a website).
 * @param {string} tag
 * @returns {void}
 */
export function chooseLanguage(tag) {
  language = effectiveLanguage(tag, navigator.language);
}

/**
 * The caption text for `key` in the current display language, or `null`
 * when `key` has no paired caption in `CAPTIONS`.
 * @param {string | undefined} key
 * @returns {string | null}
 */
export function caption(key) {
  if (!key || !Object.hasOwn(CAPTION_KEYS, key)) {
    return null;
  }
  return translate(CAPTION_KEYS[key]);
}

/**
 * Renders or updates inline `<small>` captions beneath setting labels inside
 * `#settings`, preferring a `<select>`'s selected `<option>` key over the row's
 * own key so scheme-dependent rows (such as Tone Keys) switch live.
 * @param {ParentNode} [root]
 * @returns {void}
 */
export function refreshSettingCaptions(root = document) {
  for (const span of root.querySelectorAll(
    "#settings .setting > span[data-i18n]",
  )) {
    if (!(span instanceof HTMLElement) || !span.dataset.i18n) {
      continue;
    }
    const select = span.parentElement?.querySelector("select") ?? null;
    const selectedOption =
      select && select.selectedIndex >= 0
        ? select.options[select.selectedIndex]
        : null;
    const text =
      caption(selectedOption?.dataset.i18n) ?? caption(span.dataset.i18n);
    const existing = span.querySelector(":scope > small");
    if (text === null) {
      existing?.remove();
    } else if (existing) {
      existing.textContent = text;
    } else {
      const note = document.createElement("small");
      note.textContent = text;
      span.append(note);
    }
  }
}

/**
 * Draws the page in the language of `tag`.
 * @param {string} tag
 * @returns {void}
 */
export function setDisplayLanguage(tag) {
  chooseLanguage(tag);
  const root = document.documentElement;
  root.lang = LANGUAGE_TAGS[language];
  for (const element of document.querySelectorAll("[data-i18n]")) {
    if (element instanceof HTMLElement && element.dataset.i18n) {
      element.textContent = translate(element.dataset.i18n);
    }
  }
  refreshSettingCaptions();
  for (const attribute of ATTRIBUTES) {
    const source = `data-i18n-${attribute}`;
    for (const element of document.querySelectorAll(`[${source}]`)) {
      const key = element.getAttribute(source);
      if (key) {
        element.setAttribute(attribute, translate(key));
      }
    }
  }
  root.dataset.translated = "";
}
