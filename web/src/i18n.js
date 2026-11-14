// The page's strings, from the generated `i18n/*.json` maps. A translated element
// names its key: `data-i18n` for its text, `data-i18n-<attribute>` for an
// attribute. The markup's English text only holds the layout; style.css keeps it
// out of sight until `setDisplayLanguage` has run.

import { STRINGS } from "./generated/strings.js";

/** @typedef {keyof typeof STRINGS} DisplayLanguage */
/** @typedef {keyof (typeof STRINGS)["hanji"]} StringKey */

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
