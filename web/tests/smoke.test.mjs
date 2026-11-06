// The real engine and the real dictionaries in WASM, through the bindings the
// page will load, one key at a time as the page sends them. Cases:
// engine/composing/tests/walker_fixed_inputs_prod.rs, where each one's trace
// lives. Fixtures, not corpus/ (corpus/README.md).

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { initSync, Session } from "../target/pkg/taigi_web.js";

const dictionaries = new URL("../../assets/dictionaries/", import.meta.url);
const FILE_NAMES = [
  "dictionary.fst",
  "dictionary.bin",
  "association.bin",
  "syllables.fst",
];

const TOP_CANDIDATES = [
  ["kau3siu7", "教授"],
  ["kausiu", "教授"],
  ["tai5gi2", "台語"],
  ["hoogua", "予我"],
  ["taiuan", "台灣"],
  ["ginalangtsiahpngbesai", "囡仔人食飯袂使"],
];

initSync({
  module: readFileSync(
    new URL("../target/pkg/taigi_web_bg.wasm", import.meta.url),
  ),
});
const session = new Session("{}", "linux", "en");

function codeOf(character) {
  return /\d/.test(character)
    ? `Digit${character}`
    : `Key${character.toUpperCase()}`;
}

function press(key, code, modifiers = {}) {
  const event = JSON.stringify({ key, code, ...modifiers });
  return JSON.parse(session.keyDown(event));
}

function type(raw) {
  return [...raw].map((character) => press(character, codeOf(character)));
}

function lastOf(reply, type) {
  return reply.emits.findLast((emit) => emit.type === type);
}

// The first cell a slot key picks, in its leading script: past the literal
// roman row, which takes no key, and before the other script after the space.
function topCell(table) {
  const position = table.labels.findIndex((label) => label !== "");
  return table.candidates[position].split(" ")[0];
}

function cancel() {
  press("Escape", "Escape");
}

test("an unknown platform is refused", () => {
  assert.throws(() => new Session("{}", "beos", "en"));
});

test("malformed settings are refused", () => {
  assert.throws(() => new Session("{", "linux", "en"));
});

test("an install with a dictionary missing is refused", () => {
  assert.throws(() => session.install());
});

test("keys before the install type romanization", () => {
  const [reply] = type("a");
  assert.equal(reply.handled, true);
  assert.equal(lastOf(reply, "preedit").text, "a");
  assert.equal(lastOf(reply, "lookupTable"), undefined);
  cancel();
});

test("installs the dictionaries", () => {
  for (const name of FILE_NAMES) {
    session.addDictionary(name, readFileSync(new URL(name, dictionaries)));
  }
  assert.ok(session.install() > 0n);
});

for (const [raw, hanji] of TOP_CANDIDATES) {
  test(`${raw} → ${hanji}`, () => {
    const replies = type(raw);
    assert.equal(topCell(lastOf(replies.at(-1), "lookupTable")), hanji);
    cancel();
  });
}

test("Enter commits the highlighted candidate", () => {
  type("taiuan");
  const reply = press("Enter", "Enter");
  assert.equal(lastOf(reply, "commit").text, "台灣");
});

test("a click selects and writes nothing", () => {
  const replies = type("taiuan");
  const table = lastOf(replies.at(-1), "lookupTable");
  const position = table.cursor + 1;
  const clicked = JSON.parse(session.clickCandidate(position));
  assert.equal(lastOf(clicked, "lookupTable").cursor, position);
  assert.equal(lastOf(clicked, "commit"), undefined);
  cancel();
});

test("commitComposition writes the composition once", () => {
  type("taiuan");
  const reply = JSON.parse(session.commitComposition());
  assert.ok(lastOf(reply, "commit").text.length > 0);
  assert.notEqual(lastOf(reply, "hideLookupTable"), undefined);
  assert.deepEqual(JSON.parse(session.commitComposition()).emits, []);
});

test("a Shift tap hands the keys to the page and back", () => {
  const tap = () => {
    const down = { key: "Shift", code: "ShiftLeft", shift: true };
    session.keyDown(JSON.stringify(down));
    const up = { key: "Shift", code: "ShiftLeft" };
    return JSON.parse(session.keyUp(JSON.stringify(up)));
  };
  assert.equal(lastOf(tap(), "modeChanged").symbol, "英");
  assert.equal(type("a")[0].handled, false);
  assert.equal(lastOf(tap(), "modeChanged").symbol, "台");
  assert.equal(type("a")[0].handled, true);
  cancel();
});

test("the symbol picker chord opens the picker", () => {
  const chord = { ctrl: true, alt: true };
  const reply = press(",", "Comma", chord);
  assert.ok(lastOf(reply, "lookupTable").candidates.length > 0);
  session.keyUp(JSON.stringify({ key: ",", code: "Comma" }));
  cancel();
});

test("a switch that writes hands the settings back", () => {
  const chord = { ctrl: true, alt: true };
  assert.match(press("c", "KeyC", chord).settings, /poj/);
  press("c", "KeyC", chord);
});
