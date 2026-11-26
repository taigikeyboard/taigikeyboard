// The ChromeOS options page, cut from the real web page (web/chromeos/options-form.mjs).

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { optionsMarkup, optionsPage } from "../chromeos/options-form.mjs";
import { CAPTIONS, STRINGS } from "../src/generated/strings.js";

const index = readFileSync(
  new URL("../src/index.html", import.meta.url),
  "utf8",
);
const { form, nav } = optionsMarkup(index);

/**
 * @param {string} name
 * @returns {string}
 */
function rowOf(name) {
  const lines = form.split("\n");
  const at = lines.findIndex((line) => line.includes(`name="${name}"`));
  return (
    lines
      .slice(0, at + 1)
      .findLast((line) => line.includes('<label class="setting"')) ?? ""
  );
}

test("the form is the page's whole settings form", () => {
  assert.ok(form.trimStart().startsWith('<form id="settings">'));
  assert.ok(form.trimEnd().endsWith("</form>"));
});

test("rows ChromeOS cannot honour are hidden, kept for Settings to read", () => {
  for (const name of [
    "tpsKeyboardShown",
    "candidateAppearanceMode",
    "fontType",
    "candidateTextSize",
  ]) {
    assert.match(rowOf(name), /<label class="setting" hidden>/, name);
  }
  for (const name of [
    "inputMode",
    "candidateLayout",
    "candidateDisplayMode",
    "displayLanguage",
  ]) {
    assert.match(rowOf(name), /<label class="setting">/, name);
  }
});

test("the Shortcuts section is shown, for the worker's list", () => {
  const lines = form.split("\n");
  const at = lines.findIndex((line) => line.includes('<div id="shortcuts">'));
  const section =
    lines.slice(0, at).findLast((line) => line.includes("<details")) ?? "";
  assert.match(
    section,
    /<details class="sidebar-section" id="section-3" open>/,
  );
});

test("every section is unfolded, a heading over a card", () => {
  assert.doesNotMatch(form, /<details class="sidebar-section">/);
  // trace: General, Appearance, Shortcuts, Manage Dictionaries in index.html
  for (const id of [1, 2, 3, 4]) {
    assert.match(
      form,
      new RegExp(`<details class="sidebar-section" id="section-${id}" open>`),
    );
  }
  assert.equal(form.match(/<div class="options-card">/g)?.length, 4);
  assert.match(form, /<\/summary>\n\s*<div class="options-card">/);
  assert.match(form, /<\/div>\n\s*<\/details>/);
  assert.equal(
    form.match(
      /<summary role="heading" aria-level="2" tabindex="-1" data-i18n=/g,
    )?.length,
    4,
  );
});

test("the side menu links every shown section by its summary's key", () => {
  assert.equal(
    nav,
    [
      '<nav class="options-nav">',
      '<a href="#section-1" data-i18n="desktop.generalTab">General</a>',
      '<a href="#section-2" data-i18n="desktop.appearanceTab">Appearance</a>',
      '<a href="#section-3" data-i18n="desktop.shortcutsTab">Shortcuts</a>',
      '<a href="#section-4" data-i18n="desktop.dictionarySourcesLink">Manage Dictionaries</a>',
      "</nav>",
    ].join("\n"),
  );
});

test("the options page fills its shell's title, menu and form", () => {
  const shell = readFileSync(
    new URL("../chromeos/options.html", import.meta.url),
    "utf8",
  );
  const page = optionsPage(shell, index);
  assert.doesNotMatch(page, /<!-- (nav|form):/);
  assert.match(page, /<title>台語齒盤 ChromeOS<\/title>/);
  assert.ok(page.includes(form));
  assert.ok(page.includes(nav));
  assert.throws(() => optionsPage("<body></body>", index), /title/);
  assert.throws(
    () => optionsPage("<title></title><body></body>", index),
    /nav:/,
  );
});

test("the candidate layout offers Horizontal and Vertical only", () => {
  assert.doesNotMatch(form, /value="expandable"|value="verticalExpandable"/);
  assert.match(form, /<option value="horizontal"/);
  assert.match(form, /<option value="vertical" selected/);
});

test("markup drift fails the build", () => {
  assert.throws(
    () => optionsMarkup(index.replace('name="fontType"', 'name="font"')),
    /fontType/,
  );
  assert.throws(() => optionsMarkup("<p>no form</p>"), /form id="settings"/);
});

test("all 8 inline captions are scoped for web and kept in the ChromeOS form", () => {
  assert.deepEqual(CAPTIONS, {
    "settings.toneSchemeStandard": "settings.toneSchemeStandardInfo",
    "settings.toneSchemeTelex": "settings.toneSchemeTelexInfo",
    "settings.hanLoMixed": "settings.hanLoMixedInfo",
    "settings.literalRomanCandidate": "desktop.literalRomanCandidateInfo",
    "settings.outputBothScripts": "settings.outputBothScriptsInfo",
    "settings.khinsiannInHanji": "settings.khinsiannInHanjiInfo",
    "settings.syllableSeparator": "settings.syllableSeparatorInfo",
    "settings.autoSpace": "settings.autoSpaceInfo",
  });
  for (const [lang, table] of Object.entries(STRINGS)) {
    const strings = /** @type {Record<string, string>} */ (table);
    for (const captionKey of Object.values(CAPTIONS)) {
      assert.ok(
        Object.hasOwn(strings, captionKey) && strings[captionKey].length > 0,
        `${lang} missing caption ${captionKey}`,
      );
    }
  }
  for (const baseKey of Object.keys(CAPTIONS)) {
    assert.ok(
      form.includes(`data-i18n="${baseKey}"`),
      `ChromeOS form missing ${baseKey}`,
    );
  }
  for (const name of [
    "toneInputScheme",
    "hanLoMixedEnabled",
    "literalRomanCandidateEnabled",
    "outputBothScripts",
    "khinsiannInHanjiEnabled",
    "syllableSeparator",
    "autoSpaceEnabled",
  ]) {
    assert.match(rowOf(name), /<label class="setting">/, name);
  }
});
