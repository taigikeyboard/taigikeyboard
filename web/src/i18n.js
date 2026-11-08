// The page's strings, from the generated `i18n/*.json` maps. A translated element
// names its key: `data-i18n` for its text, `data-i18n-<attribute>` for an
// attribute. The markup's English text only holds the layout; style.css keeps it
// out of sight until `setDisplayLanguage` has run.

import { STRINGS } from "./generated/strings.js";

const ATTRIBUTES = ["aria-label", "title"];

/// BCP-47 per language: the browser picks CJK glyphs and hyphenation by it.
const LANGUAGE_TAGS = {
  hanji: "nan-Hant-TW",
  tailo: "nan-Latn-tailo",
  poj: "nan-Latn-pehoeji",
  ja: "ja",
  en: "en",
};

let language = "hanji";

/// The language of `tag`, the stored `displayLanguage`. Twin of `DisplayLanguage::
/// from_tag` + `effective` in `taigi-desktop-core`, which answer for the strings
/// the session sends: the page draws before the engine has loaded.
function effectiveLanguage(tag, locale) {
  if (tag === "system") {
    const subtag = locale.split(/[-_]/)[0].toLowerCase();
    return subtag === "ja" || subtag === "en" ? subtag : "hanji";
  }
  return Object.hasOwn(LANGUAGE_TAGS, tag) ? tag : "hanji";
}

/// Every key carries every production language (`validate_production_completeness`).
export function translate(key, placeholders = {}) {
  return STRINGS[language][key].replace(
    /\{(\w+)\}/g,
    (_, name) => placeholders[name],
  );
}

/// Draws the page in the language of `tag`.
export function setDisplayLanguage(tag) {
  language = effectiveLanguage(tag, navigator.language);
  const root = document.documentElement;
  root.lang = LANGUAGE_TAGS[language];
  for (const element of document.querySelectorAll("[data-i18n]")) {
    element.textContent = translate(element.dataset.i18n);
  }
  for (const attribute of ATTRIBUTES) {
    const source = `data-i18n-${attribute}`;
    for (const element of document.querySelectorAll(`[${source}]`)) {
      element.setAttribute(attribute, translate(element.getAttribute(source)));
    }
  }
  root.dataset.translated = "";
}
