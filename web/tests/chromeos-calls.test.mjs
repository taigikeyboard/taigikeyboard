// The ChromeOS glue's translations (web/chromeos/calls.js), without ChromeOS.

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  imeCalls,
  isResetStorm,
  keyJson,
  utf16Caret,
} from "../chromeos/calls.js";

const ids = { engineID: "taigi", contextID: 7 };

test("a ChromeOS key becomes the session's browser key", () => {
  const json = keyJson({
    type: "keydown",
    key: "A",
    code: "KeyA",
    shiftKey: true,
    ctrlKey: false,
    altKey: false,
    capsLock: true,
  });
  assert.deepEqual(JSON.parse(json), {
    key: "A",
    code: "KeyA",
    shift: true,
    ctrl: false,
    alt: false,
    meta: false,
    repeat: false,
  });
});

test("ChromeOS key names become the DOM's", () => {
  // Pairs as linux-chromeos r1738485 reported them.
  const reported = [
    ["Down", "ArrowDown", "ArrowDown"],
    ["Esc", "Escape", "Escape"],
    ["", "Delete", "Delete"],
    ["Ctrl", "ControlLeft", "Control"],
    ["Enter", "NumpadEnter", "Enter"],
    [";", "Semicolon", ";"],
    ["1", "Numpad1", "1"],
  ];
  for (const [key, code, expected] of reported) {
    assert.equal(JSON.parse(keyJson({ key, code })).key, expected, code);
  }
});

test("the preedit caret is counted in UTF-16 units", () => {
  // 𧉟 is U+2725F, two UTF-16 units: caret 2 (after 𧉟, a) is 3 there.
  assert.equal(utf16Caret("𧉟ab", 2), 3);
  assert.deepEqual(
    imeCalls({ emits: [{ type: "preedit", text: "𧉟ab", caret: 2 }] }, ids),
    [["setComposition", { contextID: 7, text: "𧉟ab", cursor: 3 }]],
  );
});

test("commit, clear and the auto-space swap map one to one", () => {
  const reply = {
    emits: [
      { type: "clearPreedit" },
      { type: "commit", text: "台語" },
      { type: "deleteSurrounding", offset: -1, count: 1 },
      { type: "hideLookupTable" },
    ],
  };
  assert.deepEqual(imeCalls(reply, ids), [
    ["clearComposition", { contextID: 7 }],
    ["commitText", { contextID: 7, text: "台語" }],
    [
      "deleteSurroundingText",
      { engineID: "taigi", contextID: 7, offset: -1, length: 1 },
    ],
    [
      "setCandidateWindowProperties",
      { engineID: "taigi", properties: { visible: false } },
    ],
  ]);
});

test("only the cursor's page of candidates is sent, with page-relative IDs", () => {
  const cells = ["一", "二", "三", "四", "五"].map((text) => ({
    text,
    annotation: "x",
  }));
  const table = {
    type: "lookupTable",
    cells,
    labels: ["1", "2"],
    cursor: 3,
    cursorVisible: true,
    pageSize: 2,
    vertical: true,
  };
  const [candidates, properties, cursor] = imeCalls({ emits: [table] }, ids);
  assert.deepEqual(candidates, [
    "setCandidates",
    {
      contextID: 7,
      candidates: [
        { candidate: "三", id: 0, label: "1", annotation: "x" },
        { candidate: "四", id: 1, label: "2", annotation: "x" },
      ],
    },
  ]);
  assert.equal(properties[0], "setCandidateWindowProperties");
  assert.equal(properties[1].properties.pageSize, 2);
  assert.equal(properties[1].properties.totalCandidates, 5);
  assert.deepEqual(cursor, [
    "setCursorPosition",
    { contextID: 7, candidateID: 1 },
  ]);
});

test("a hidden cursor sets no candidate position", () => {
  const table = {
    type: "lookupTable",
    cells: [{ text: "一" }],
    labels: ["1"],
    cursor: 0,
    cursorVisible: false,
    pageSize: 9,
    vertical: false,
  };
  const calls = imeCalls({ emits: [table] }, ids);
  assert.equal(calls.length, 2);
});

test("emits with no ChromeOS surface make no call", () => {
  const reply = {
    emits: [
      { type: "modeChanged", symbol: "A" },
      { type: "announceMode", label: "x" },
    ],
  };
  assert.deepEqual(imeCalls(reply, ids), []);
});

test("Google Docs and X fields are told apart from a plain field", () => {
  const plain = { type: "text", autoCapitalize: "sentences", spellCheck: true };
  const docs = { type: "text", autoCapitalize: "off", spellCheck: false };
  const x = {
    type: "text",
    autoCapitalize: "sentences",
    autoComplete: true,
    autoCorrect: true,
    shouldDoLearning: true,
    spellCheck: true,
  };
  assert.equal(isResetStorm(plain), false);
  assert.equal(isResetStorm(docs), true);
  assert.equal(isResetStorm(x), true);
  assert.equal(isResetStorm({ type: "url", spellCheck: false }), false);
  assert.equal(isResetStorm(null), false);
});
