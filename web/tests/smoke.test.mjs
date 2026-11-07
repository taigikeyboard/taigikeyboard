// The real engine and the real dictionaries in WASM, through the bindings the
// page will load, one key at a time as the page sends them. Cases:
// engine/composing/tests/walker_fixed_inputs_prod.rs, where each one's trace
// lives. Fixtures, not corpus/ (corpus/README.md).

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { initSync, Session } from "../target/pkg/taigi_web.js";
import { cancel, lastOf, press, topCell, type } from "./keys.mjs";

const dictionaries = new URL("../../assets/dictionaries/", import.meta.url);
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
  const [reply] = type(session, "a");
  assert.equal(reply.handled, true);
  assert.equal(lastOf(reply, "preedit").text, "a");
  assert.equal(lastOf(reply, "lookupTable"), undefined);
  cancel(session);
});

test("installs the dictionaries", () => {
  for (const name of session.dictionaryFiles()) {
    session.addDictionary(name, readFileSync(new URL(name, dictionaries)));
  }
  assert.ok(session.install() > 0n);
});

for (const [raw, hanji] of TOP_CANDIDATES) {
  test(`${raw} → ${hanji}`, () => {
    const replies = type(session, raw);
    assert.equal(topCell(lastOf(replies.at(-1), "lookupTable")), hanji);
    cancel(session);
  });
}

test("Enter commits the highlighted candidate", () => {
  type(session, "taiuan");
  const reply = press(session, "Enter", "Enter");
  assert.equal(lastOf(reply, "commit").text, "台灣");
});

test("a click selects and writes nothing", () => {
  const replies = type(session, "taiuan");
  const table = lastOf(replies.at(-1), "lookupTable");
  const position = table.cursor + 1;
  const clicked = JSON.parse(session.clickCandidate(position));
  assert.equal(lastOf(clicked, "lookupTable").cursor, position);
  assert.equal(lastOf(clicked, "commit"), undefined);
  cancel(session);
});

test("commitComposition writes the composition once", () => {
  type(session, "taiuan");
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
  assert.equal(type(session, "a")[0].handled, false);
  assert.equal(lastOf(tap(), "modeChanged").symbol, "台");
  assert.equal(type(session, "a")[0].handled, true);
  cancel(session);
});

test("the symbol picker chord opens the picker", () => {
  const chord = { ctrl: true, alt: true };
  const reply = press(session, ",", "Comma", chord);
  assert.ok(lastOf(reply, "lookupTable").cells.length > 0);
  session.keyUp(JSON.stringify({ key: ",", code: "Comma" }));
  cancel(session);
});

test("a switch that writes hands the settings back", () => {
  const chord = { ctrl: true, alt: true };
  assert.match(press(session, "c", "KeyC", chord).settings, /poj/);
  press(session, "c", "KeyC", chord);
});

// The list `body` typed "ti" gets, under Horizontal Expandable.
function underExpandable(body) {
  session.setSettings(
    JSON.stringify({ values: { candidateLayout: "expandable" } }),
  );
  try {
    body(
      lastOf(
        { emits: type(session, "ti").flatMap((r) => r.emits) },
        "lookupTable",
      ),
    );
    cancel(session);
  } finally {
    session.setSettings("{}");
  }
}

test("an expandable layout measures through the page and keys go by it", () => {
  // trace: 1 em for a character, the page's width is 600: the model's row
  // holds fewer cells than the list, so there is more to unfold.
  globalThis.taigiMeasureCells = (runs) =>
    Float32Array.of(
      600,
      ...JSON.parse(runs).map(([text, size]) => [...text].length * size),
    );
  underExpandable((table) => {
    assert.equal(table.layout.isExpanded, false);
    assert.equal(table.layout.hasOverflow, true);
    assert.ok(table.layout.collapsed.length < table.cells.length);
    const down = press(session, "ArrowDown", "ArrowDown");
    assert.equal(lastOf(down, "lookupTable").layout.isExpanded, true);
  });
});

test("a measure that throws leaves the list paged, the session alive", () => {
  globalThis.taigiMeasureCells = () => {
    throw new Error("no ruler");
  };
  underExpandable((table) => assert.equal(table.layout, undefined));
});
