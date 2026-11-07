// The ChromeOS options form, cut from the real web page (web/chromeos/options-form.mjs).

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { optionsForm, optionsPage } from "../chromeos/options-form.mjs";

const index = readFileSync(
  new URL("../src/index.html", import.meta.url),
  "utf8",
);
const form = optionsForm(index);

function rowOf(name) {
  const lines = form.split("\n");
  const at = lines.findIndex((line) => line.includes(`name="${name}"`));
  return lines
    .slice(0, at + 1)
    .findLast((line) => line.includes('<label class="setting"'));
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

test("the Shortcuts section is hidden", () => {
  const lines = form.split("\n");
  const at = lines.findIndex((line) => line.includes('<div id="shortcuts">'));
  const section = lines
    .slice(0, at)
    .findLast((line) => line.includes("<details"));
  assert.match(section, /<details class="sidebar-section" hidden>/);
});

test("the options page puts the form where its shell marks", () => {
  const shell = readFileSync(
    new URL("../chromeos/options.html", import.meta.url),
    "utf8",
  );
  const page = optionsPage(shell, index);
  assert.doesNotMatch(page, /<!-- form:/);
  assert.ok(page.includes(form));
  assert.throws(() => optionsPage("<body></body>", index), /form:/);
});

test("the candidate layout offers Horizontal and Vertical only", () => {
  assert.doesNotMatch(form, /value="expandable"|value="verticalExpandable"/);
  assert.match(form, /<option value="horizontal"/);
  assert.match(form, /<option value="vertical" selected/);
});

test("markup drift fails the build", () => {
  assert.throws(
    () => optionsForm(index.replace('name="fontType"', 'name="font"')),
    /fontType/,
  );
  assert.throws(() => optionsForm("<p>no form</p>"), /form id="settings"/);
});
