// The ChromeOS glue's translations (web/chromeos/calls.js), without ChromeOS.

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  endModeFlash,
  imeCalls,
  isResetStorm,
  keyJson,
  listShownAfter,
  modeMenuItem,
  utf16Caret,
} from "../chromeos/calls.js";

/** @typedef {import("../types/shared.d.ts").Reply} Reply */
/** @typedef {import("../types/shared.d.ts").WireEmit} WireEmit */
/** @typedef {Extract<WireEmit, { type: "lookupTable" }>} LookupTableEmit */

const ids = { engineID: "taigi", contextID: 7, listShown: false };

test("a ChromeOS key becomes the session's browser key", () => {
  const json = keyJson(
    /** @type {chrome.input.ime.KeyboardEvent} */ ({
      type: "keydown",
      key: "A",
      code: "KeyA",
      shiftKey: true,
      ctrlKey: false,
      altKey: false,
      capsLock: true,
    }),
  );
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
  /** @type {Array<[string, string, string]>} */
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
    assert.equal(
      JSON.parse(
        keyJson(/** @type {chrome.input.ime.KeyboardEvent} */ ({ key, code })),
      ).key,
      expected,
      code,
    );
  }
});

test("the preedit caret is counted in UTF-16 units", () => {
  // 𧉟 is U+2725F, two UTF-16 units: caret 2 (after 𧉟, a) is 3 there.
  assert.equal(utf16Caret("𧉟ab", 2), 3);
  assert.deepEqual(
    imeCalls(
      /** @type {Reply} */ ({
        emits: [{ type: "preedit", text: "𧉟ab", caret: 2 }],
      }),
      ids,
    ),
    [["setComposition", { contextID: 7, text: "𧉟ab", cursor: 3 }]],
  );
});

test("commit, clear and the auto-space swap map one to one", () => {
  /** @type {Reply} */
  const reply = {
    handled: true,
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
      {
        engineID: "taigi",
        properties: { visible: false, auxiliaryTextVisible: false },
      },
    ],
  ]);
});

test("only the cursor's page of candidates is sent, with page-relative IDs", () => {
  const cells = ["一", "二", "三", "四", "五"].map((text) => ({
    text,
    annotation: "x",
  }));
  /** @type {LookupTableEmit} */
  const table = {
    type: "lookupTable",
    cells,
    labels: ["1", "2"],
    cursor: 3,
    cursorVisible: true,
    pageSize: 2,
    vertical: true,
  };
  const [candidates, properties, cursor] = imeCalls(
    { handled: true, emits: [table] },
    ids,
  );
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
  const propsCall =
    /** @type {Extract<import("../chromeos/calls.js").ImeCall, ["setCandidateWindowProperties", unknown]>} */ (
      properties
    );
  assert.equal(propsCall[1].properties.pageSize, 2);
  assert.equal(propsCall[1].properties.totalCandidates, 5);
  assert.equal(propsCall[1].properties.auxiliaryTextVisible, false);
  assert.deepEqual(cursor, [
    "setCursorPosition",
    { contextID: 7, candidateID: 1 },
  ]);
});

test("a hidden cursor sets no candidate position", () => {
  /** @type {LookupTableEmit} */
  const table = {
    type: "lookupTable",
    cells: [{ text: "一" }],
    labels: ["1"],
    cursor: 0,
    cursorVisible: false,
    pageSize: 9,
    vertical: false,
  };
  const calls = imeCalls({ handled: true, emits: [table] }, ids);
  assert.equal(calls.length, 2);
});

test("modeChanged makes no call: the worker redraws the tray menu", () => {
  assert.deepEqual(
    imeCalls(
      { handled: true, emits: [{ type: "modeChanged", symbol: "A" }] },
      ids,
    ),
    [],
  );
});

test("announceMode flashes the mode as the empty window's auxiliary text", () => {
  assert.deepEqual(
    imeCalls(
      { handled: true, emits: [{ type: "announceMode", label: "English" }] },
      ids,
    ),
    [
      ["setCandidates", { contextID: 7, candidates: [] }],
      [
        "setCandidateWindowProperties",
        {
          engineID: "taigi",
          properties: {
            visible: true,
            vertical: false,
            auxiliaryText: "English",
            auxiliaryTextVisible: true,
          },
        },
      ],
    ],
  );
});

test("a chord while composing flashes over the list it keeps", () => {
  /** @type {LookupTableEmit} */
  const table = {
    type: "lookupTable",
    cells: [{ text: "一" }],
    labels: ["1"],
    cursor: 0,
    cursorVisible: false,
    pageSize: 9,
    vertical: true,
  };
  const calls = imeCalls(
    {
      handled: true,
      emits: [
        table,
        { type: "modeChanged", symbol: "A" },
        { type: "announceMode", label: "漢字" },
      ],
    },
    { ...ids, listShown: true },
  );
  assert.equal(calls.length, 3);
  assert.deepEqual(calls[2], [
    "setCandidateWindowProperties",
    {
      engineID: "taigi",
      properties: { auxiliaryText: "漢字", auxiliaryTextVisible: true },
    },
  ]);
  // The list's own candidates only: no empty set over them.
  assert.equal(
    calls.filter(([method]) => method === "setCandidates").length,
    1,
  );
});

test("a flash over a list ends with its label; alone, with the window", () => {
  assert.deepEqual(endModeFlash("taigi", true), [
    "setCandidateWindowProperties",
    { engineID: "taigi", properties: { auxiliaryTextVisible: false } },
  ]);
  assert.deepEqual(endModeFlash("taigi", false), [
    "setCandidateWindowProperties",
    {
      engineID: "taigi",
      properties: { visible: false, auxiliaryTextVisible: false },
    },
  ]);
});

test("the tray row reads as the page's mode chip", () => {
  assert.deepEqual(
    modeMenuItem({
      isEnglish: false,
      mode: "台語",
      script: "Tâi-lô",
      other: "",
    }),
    { id: "mode", label: "台語 · Tâi-lô" },
  );
  assert.deepEqual(
    modeMenuItem({
      isEnglish: true,
      mode: "English",
      script: "",
      other: "台語",
    }),
    { id: "mode", label: "English · ⇧ 台語" },
  );
});

test("Google Docs and X fields are told apart from a plain field", () => {
  const plain = /** @type {chrome.input.ime.InputContext} */ (
    /** @type {unknown} */ ({
      type: "text",
      autoCapitalize: "sentences",
      spellCheck: true,
    })
  );
  const docs = /** @type {chrome.input.ime.InputContext} */ (
    /** @type {unknown} */ ({
      type: "text",
      autoCapitalize: "off",
      spellCheck: false,
    })
  );
  const x = /** @type {chrome.input.ime.InputContext} */ (
    /** @type {unknown} */ ({
      type: "text",
      autoCapitalize: "sentences",
      autoComplete: true,
      autoCorrect: true,
      shouldDoLearning: true,
      spellCheck: true,
    })
  );
  assert.equal(isResetStorm(plain), false);
  assert.equal(isResetStorm(docs), true);
  assert.equal(isResetStorm(x), true);
  assert.equal(
    isResetStorm(
      /** @type {chrome.input.ime.InputContext} */ (
        /** @type {unknown} */ ({
          type: "url",
          spellCheck: false,
        })
      ),
    ),
    false,
  );
  assert.equal(isResetStorm(null), false);
});

test("a reply's last table emit decides whether a list is up", () => {
  /** @type {LookupTableEmit} */
  const table = {
    type: "lookupTable",
    cells: [{ text: "一" }],
    labels: ["1"],
    cursor: 0,
    cursorVisible: false,
    pageSize: 9,
    vertical: true,
  };
  assert.equal(listShownAfter({ handled: true, emits: [table] }, false), true);
  assert.equal(
    listShownAfter(
      { handled: true, emits: [table, { type: "hideLookupTable" }] },
      false,
    ),
    false,
  );
  assert.equal(
    listShownAfter(
      { handled: true, emits: [{ type: "commit", text: "x" }] },
      true,
    ),
    true,
  );
});
