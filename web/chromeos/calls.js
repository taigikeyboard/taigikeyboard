// Translates between chrome.input.ime and the taigi-web session: a ChromeOS
// key as the session's browser key, a session reply as chrome.input.ime calls.
// Pure, so Node tests it (web/tests/chromeos-calls.test.mjs).

/// ChromeOS names some keys its own way (`Down`, `Esc`, `Ctrl`, and `""` for
/// Delete; recorded in linux-chromeos r1738485), so these are read by
/// position and given the DOM name `key.rs` expects. Printable keys keep
/// `key`, which follows the layout.
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

/// `web/crates/taigi-web/src/key.rs` `BrowserKey`. A ChromeOS key event has
/// no Meta (Search) and no repeat flag. Caps Lock is ignored, as on the page.
export function keyJson(keyData) {
  return JSON.stringify({
    key: DOM_KEY_BY_CODE[keyData.code] ?? keyData.key,
    code: keyData.code,
    shift: keyData.shiftKey,
    ctrl: keyData.ctrlKey,
    alt: keyData.altKey,
    meta: false,
    repeat: false,
  });
}

/// The session counts the caret in characters, Chrome in UTF-16 units: a
/// supplementary-plane Hanji counts once there and twice here.
export function utf16Caret(text, caret) {
  return Array.from(text).slice(0, caret).join("").length;
}

/// `[method, details]` pairs for `chrome.input.ime[method](details)`, in
/// order. The reply carries the whole candidate list with an absolute cursor;
/// ChromeOS draws one page, and `onCandidateClicked` answers with an ID that
/// counts from that page, which is what `clickCandidate` takes.
export function imeCalls(reply, { engineID, contextID }) {
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
        calls.push([
          "setCandidateWindowProperties",
          { engineID, properties: { visible: false } },
        ]);
        break;
      default:
        // modeChanged, announceMode, openSettings and the TPS key panel have
        // no ChromeOS surface yet (chromeos-roadmap.md § Phases).
        break;
    }
  }
  return calls;
}

function candidateCalls(table, { engineID, contextID }) {
  const pageSize = Math.max(1, table.pageSize);
  const start = Math.floor(table.cursor / pageSize) * pageSize;
  const page = table.cells.slice(start, start + pageSize);
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

/// Google Docs and X send `onReset` while the user types (McBopomofoWeb
/// `src/chromeos_ime.ts`, which skips its reset in these fields; not verified
/// here, both need a signed-in account). Recognised by the field's flags.
export function isResetStorm(context) {
  if (context?.type !== "text") {
    return false;
  }
  const isGoogleDocs =
    !["characters", "words", "sentences"].includes(context.autoCapitalize) &&
    context.spellCheck === false;
  const isX =
    context.autoCapitalize === "sentences" &&
    context.autoComplete === true &&
    context.autoCorrect === true &&
    context.shouldDoLearning === true &&
    context.spellCheck === true;
  return isGoogleDocs || isX;
}
