// What `make extension` cuts from the web page (web/scripts/stage-extension.mjs).

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { optionsMarkup, optionsPage } from "../chromeos/options-form.mjs";
import {
  EXTENSION_CUT,
  forShadowTree,
  panelMarkup,
} from "../scripts/stage-extension.mjs";
import { CAPTIONS } from "../src/generated/strings.js";

const index = readFileSync(
  new URL("../src/index.html", import.meta.url),
  "utf8",
);
const style = readFileSync(
  new URL("../src/style.css", import.meta.url),
  "utf8",
);

/**
 * @param {string} form
 * @param {string} name
 * @returns {string}
 */
function rowOf(form, name) {
  const lines = form.split("\n");
  const at = lines.findIndex((line) => line.includes(`name="${name}"`));
  return (
    lines
      .slice(0, at + 1)
      .findLast((line) => line.includes('<label class="setting"')) ?? ""
  );
}

test("the extension options form hides only the key panel and font rows", () => {
  const { form } = optionsMarkup(index, EXTENSION_CUT);
  for (const name of ["tpsKeyboardShown", "fontType"]) {
    assert.match(rowOf(form, name), /<label class="setting" hidden>/, name);
  }
  for (const name of [
    "inputMode",
    "candidateLayout",
    "candidateDisplayMode",
    "candidateAppearanceMode",
    "candidateTextSize",
    "displayLanguage",
  ]) {
    assert.match(rowOf(form, name), /<label class="setting">/, name);
  }
  assert.match(form, /value="expandable"/);
  const shell = readFileSync(
    new URL("../chromeos/options.html", import.meta.url),
    "utf8",
  );
  const page = optionsPage(shell, index, EXTENSION_CUT);
  assert.ok(page.includes(form));
  assert.match(page, /<title>台語齒盤<\/title>/);
});

test("the extension's side menu lists its Shortcuts section", () => {
  const { nav } = optionsMarkup(index, EXTENSION_CUT);
  // trace: General, Appearance, Shortcuts, Manage Dictionaries in page order
  assert.match(
    nav,
    /<a href="#section-3" data-i18n="desktop.shortcutsTab">Shortcuts<\/a>/,
  );
  assert.match(nav, /href="#section-4"/);
});

test("panelMarkup cuts the composition, panel, ruler and live region", () => {
  const markup = panelMarkup(index);
  assert.match(markup, /^<div id="preedit"><\/div>/);
  assert.match(markup, /id="panel"/);
  assert.match(markup, /id="candidates"/);
  assert.match(markup, /id="ruler"/);
  assert.match(markup, /id="spoken"/);
  assert.throws(
    () => panelMarkup(index.replace('id="preedit"', 'id="gone"')),
    /preedit/,
  );
});

test("forShadowTree moves :root rules onto :host", () => {
  const rewritten = forShadowTree(style);
  assert.doesNotMatch(rewritten, /:root/);
  assert.match(rewritten, /:host\s*\{/);
  assert.match(rewritten, /:host\(\[data-theme="dark"\]\)/);
  assert.match(rewritten, /:host\(:not\(\[data-translated\]\)\)/);
  assert.throws(() => forShadowTree("body { color: black; }"), /:root/);
});

test("the extension options form keeps all captioned General rows visible", () => {
  const { form } = optionsMarkup(index, EXTENSION_CUT);
  for (const baseKey of Object.keys(CAPTIONS)) {
    assert.ok(
      form.includes(`data-i18n="${baseKey}"`),
      `Extension form missing ${baseKey}`,
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
    assert.match(rowOf(form, name), /<label class="setting">/, name);
  }
});
