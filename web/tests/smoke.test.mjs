// The real engine and the real dictionaries in WASM, through the bindings the
// page will load. Cases: engine/composing/tests/walker_fixed_inputs_prod.rs,
// where each one's trace lives. Fixtures, not corpus/ (corpus/README.md).

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
const session = new Session();

test("an install with a dictionary missing is refused", () => {
  assert.throws(() => session.install());
});

test("installs the dictionaries", () => {
  for (const name of FILE_NAMES) {
    session.addDictionary(name, readFileSync(new URL(name, dictionaries)));
  }
  assert.ok(session.install() > 0n);
});

for (const [raw, hanji] of TOP_CANDIDATES) {
  test(`${raw} → ${hanji}`, () => {
    assert.equal(session.topCandidate(raw), hanji);
  });
}
