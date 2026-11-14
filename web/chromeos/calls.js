// Translates between chrome.input.ime and the taigi-web session: a ChromeOS
// key as the session's browser key, a session reply as chrome.input.ime calls.
// Pure, so Node tests it (web/tests/chromeos-calls.test.mjs).

/** @typedef {import("../types/shared.d.ts").BrowserKey} BrowserKey */
/** @typedef {import("../types/shared.d.ts").Reply} Reply */
/** @typedef {import("../types/shared.d.ts").WireEmit} WireEmit */
/** @typedef {import("../types/shared.d.ts").WireModeLine} WireModeLine */
/** @typedef {Extract<WireEmit, { type: "lookupTable" }>} LookupTableEmit */
/**
 * @typedef {{
 *   engineID: string,
 *   contextID: number,
 *   listShown: boolean,
 * }} ImeTarget
 */
/**
 * @typedef {(
 *   | ["setComposition", { contextID: number, text: string, cursor: number }]
 *   | ["clearComposition", { contextID: number }]
 *   | ["commitText", { contextID: number, text: string }]
 *   | ["deleteSurroundingText", { engineID: string, contextID: number, offset: number, length: number }]
 *   | ["setCandidates", { contextID: number, candidates: Array<{ candidate: string, id: number, label: string, annotation: string | undefined }> }]
 *   | ["setCandidateWindowProperties", { engineID: string, properties: { visible?: boolean, vertical?: boolean, pageSize?: number, cursorVisible?: boolean, currentCandidateIndex?: number, totalCandidates?: number, auxiliaryText?: string, auxiliaryTextVisible: boolean } }]
 *   | ["setCursorPosition", { contextID: number, candidateID: number }]
 * )} ImeCall
 */

/**
 * ChromeOS names some keys its own way (`Down`, `Esc`, `Ctrl`, and `""` for
 * Delete; recorded in linux-chromeos r1738485), so these are read by
 * position and given the DOM name `key.rs` expects. Printable keys keep
 * `key`, which follows the layout.
 * @type {Record<string, string>}
 */
const DOM_KEY_BY_CODE = {
  ArrowDown: "ArrowDown",
  ArrowUp: "ArrowUp",
  ArrowLeft: "ArrowLeft",
  ArrowRight: "ArrowRight",
  Escape: "Escape",
  Delete: "Delete",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  Enter: "Enter",
  NumpadEnter: "Enter",
  Backspace: "Backspace",
  Tab: "Tab",
  ShiftLeft: "Shift",
  ShiftRight: "Shift",
  ControlLeft: "Control",
  ControlRight: "Control",
  AltLeft: "Alt",
  CapsLock: "CapsLock",
};

/**
 * `web/crates/taigi-web/src/key.rs` `BrowserKey`. A ChromeOS key event has
 * no Meta (Search) and no repeat flag. Caps Lock is ignored, as on the page.
 * @param {chrome.input.ime.KeyboardEvent} keyData
 * @returns {string}
 */
export function keyJson(keyData) {
  /** @type {BrowserKey} */
  const browserKey = {
    key: DOM_KEY_BY_CODE[keyData.code] ?? keyData.key,
    code: keyData.code,
    shift: Boolean(keyData.shiftKey),
    ctrl: Boolean(keyData.ctrlKey),
    alt: Boolean(keyData.altKey),
    meta: false,
    repeat: false,
  };
  return JSON.stringify(browserKey);
}

/**
 * The session counts the caret in characters, Chrome in UTF-16 units: a
 * supplementary-plane Hanji counts once there and twice here.
 * @param {string} text
 * @param {number} caret
 * @returns {number}
 */
export function utf16Caret(text, caret) {
  return Array.from(text).slice(0, caret).join("").length;
}

/**
 * `[method, details]` pairs for `chrome.input.ime[method](details)`, in
 * order. The reply carries the whole candidate list with an absolute cursor;
 * ChromeOS draws one page, and `onCandidateClicked` answers with an ID that
 * counts from that page, which is what `clickCandidate` takes. `listShown`
 * says whether the reply leaves a candidate list up (`announceMode`, which
 * reads it, is always a reply's last emit).
 * @param {Reply} reply
 * @param {ImeTarget} target
 * @returns {ImeCall[]}
 */
export function imeCalls(reply, { engineID, contextID, listShown }) {
  /** @type {ImeCall[]} */
  const calls = [];
  for (const emit of reply.emits) {
    switch (emit.type) {
      case "preedit":
        calls.push([
          "setComposition",
          {
            contextID,
            text: emit.text,
            cursor: utf16Caret(emit.text, emit.caret),
          },
        ]);
        break;
      case "clearPreedit":
        calls.push(["clearComposition", { contextID }]);
        break;
      case "commit":
        calls.push(["commitText", { contextID, text: emit.text }]);
        break;
      case "deleteSurrounding":
        calls.push([
          "deleteSurroundingText",
          { engineID, contextID, offset: emit.offset, length: emit.count },
        ]);
        break;
      case "lookupTable":
        calls.push(...candidateCalls(emit, { engineID, contextID }));
        break;
      case "hideLookupTable":
        calls.push(hideCandidateWindow(engineID));
        break;
      case "announceMode":
        // The page's mode flash (`ime.js` `flashMode`): the mode as the system
        // window's auxiliary text, which the worker hides after
        // `MODE_FLASH_HOLD`. A chord while composing keeps its list; with
        // none, the window shows the label alone.
        if (listShown) {
          calls.push([
            "setCandidateWindowProperties",
            {
              engineID,
              properties: {
                auxiliaryText: emit.label,
                auxiliaryTextVisible: true,
              },
            },
          ]);
          break;
        }
        calls.push(
          ["setCandidates", { contextID, candidates: [] }],
          [
            "setCandidateWindowProperties",
            {
              engineID,
              properties: {
                visible: true,
                // A vertical window keeps a column of empty rows.
                vertical: false,
                auxiliaryText: emit.label,
                auxiliaryTextVisible: true,
              },
            },
          ],
        );
        break;
      default:
        // modeChanged (the worker redraws the tray menu), openSettings (the
        // worker) and the TPS key panel (no ChromeOS surface).
        break;
    }
  }
  return calls;
}

/**
 * Converts a `lookupTable` emit into paged candidate window calls.
 * @param {LookupTableEmit} table
 * @param {Omit<ImeTarget, "listShown">} target
 * @returns {ImeCall[]}
 */
function candidateCalls(table, { engineID, contextID }) {
  const pageSize = Math.max(1, table.pageSize);
  const start = Math.floor(table.cursor / pageSize) * pageSize;
  const page = table.cells.slice(start, start + pageSize);
  /** @type {ImeCall[]} */
  const calls = [
    [
      "setCandidates",
      {
        contextID,
        candidates: page.map((cell, id) => ({
          candidate: cell.text,
          id,
          label: table.labels[id],
          annotation: cell.annotation,
        })),
      },
    ],
    [
      "setCandidateWindowProperties",
      {
        engineID,
        properties: {
          visible: true,
          vertical: table.vertical,
          pageSize,
          cursorVisible: table.cursorVisible,
          currentCandidateIndex: table.cursor,
          totalCandidates: table.cells.length,
          // A mode flash still up gives way to the list.
          auxiliaryTextVisible: false,
        },
      },
    ],
  ];
  if (table.cursorVisible) {
    calls.push([
      "setCursorPosition",
      { contextID, candidateID: table.cursor - start },
    ]);
  }
  return calls;
}

/**
 * Whether a candidate list is up after `reply`, given whether one was before.
 * @param {Reply} reply
 * @param {boolean} listShown
 * @returns {boolean}
 */
export function listShownAfter(reply, listShown) {
  return reply.emits.reduce((shown, emit) => {
    if (emit.type === "lookupTable") {
      return true;
    }
    return emit.type === "hideLookupTable" ? false : shown;
  }, listShown);
}

/**
 * Hides the system candidate window and any mode flash in it.
 * @param {string} engineID
 * @returns {ImeCall}
 */
function hideCandidateWindow(engineID) {
  return [
    "setCandidateWindowProperties",
    { engineID, properties: { visible: false, auxiliaryTextVisible: false } },
  ];
}

/**
 * Ends a mode flash: the label goes, and the window with it unless it holds
 * a candidate list.
 * @param {string} engineID
 * @param {boolean} listShown
 * @returns {ImeCall}
 */
export function endModeFlash(engineID, listShown) {
  return listShown
    ? [
        "setCandidateWindowProperties",
        { engineID, properties: { auxiliaryTextVisible: false } },
      ]
    : hideCandidateWindow(engineID);
}

/**
 * The tray menu's one row: the page's mode chip (`ime.js` `redrawModeLine`)
 * as text, the mode and the input script, or English and the Shift tap back.
 * @param {WireModeLine} line
 * @returns {chrome.input.ime.MenuItem}
 */
export function modeMenuItem({ isEnglish, mode, script, other }) {
  return {
    id: "mode",
    label: isEnglish ? `${mode} · ⇧ ${other}` : `${mode} · ${script}`,
  };
}

/**
 * Google Docs and X send `onReset` while the user types (McBopomofoWeb
 * `src/chromeos_ime.ts`, which skips its reset in these fields; not verified
 * here, both need a signed-in account). Recognised by the field's flags.
 * @param {chrome.input.ime.InputContext | null | undefined} context
 * @returns {boolean}
 */
export function isResetStorm(context) {
  if (context?.type !== "text") {
    return false;
  }
  const isGoogleDocs =
    !["characters", "words", "sentences"].includes(
      context.autoCapitalize ?? "",
    ) && context.spellCheck === false;
  const isX =
    context.autoCapitalize === "sentences" &&
    context.autoComplete === true &&
    context.autoCorrect === true &&
    context.shouldDoLearning === true &&
    context.spellCheck === true;
  return isGoogleDocs || isX;
}
