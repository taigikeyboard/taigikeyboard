// The ChromeOS input method's service worker: chrome.input.ime events in, the
// taigi-web session's replies out (docs/architecture/chromeos-roadmap.md).

import {
  endModeFlash,
  imeCalls,
  isResetStorm,
  keyJson,
  listShownAfter,
  modeMenuItem,
} from "./calls.js";
import { Learning } from "./learning.js";
import init, { Session } from "./pkg/taigi_web.js";
import { emptyDocument, STORAGE_KEY as SETTINGS_KEY } from "./settings.js";

/** @typedef {import("../types/shared.d.ts").Reply} Reply */
/** @typedef {import("../types/shared.d.ts").WireModeLine} WireModeLine */
/** @typedef {import("./calls.js").ImeCall} ImeCall */

// The expandable layouts measure through the page's DOM, which a worker has
// none of; the session's `catch` import then keeps the list paged.
globalThis.taigiMeasureCells = () => {
  throw new Error("no DOM to measure candidate cells");
};

// `input_components[0].id` in manifest.json: known before any event names it,
// so a worker started by onFocus can still address the candidate window.
const ENGINE_ID = "taigi";

const DEFAULT_SETTINGS = JSON.stringify(emptyDocument());

// The session's shortcut list, in chrome.storage.local for the options page to
// draw (the same key in `options.js`).
const SHORTCUTS_KEY = "taigi.shortcuts";

/** As long as the page holds its mode flash (`ime.js` `MODE_FLASH_HOLD`). */
const MODE_FLASH_HOLD = 800;

/** @type {Session | null} */
let session = null;
/** @type {chrome.input.ime.InputContext | null} */
let context = null;
// A trapped session is never kept: this worker waits to be stopped.
let trapped = false;
// The settings document the session holds.
let settingsJson = DEFAULT_SETTINGS;
// Whether this input method is the active one: the tray menu is its own.
let active = false;
// The shortcut list last written for the options page.
let publishedShortcuts = "";
// Whether the system window holds a candidate list: a mode flash leaves it.
let listShown = false;
/** @type {ReturnType<typeof setTimeout> | undefined} */
let modeFlashTimer;

/**
 * Formats an unknown caught value for warning logs.
 * @param {unknown} error
 * @returns {string}
 */
function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

/**
 * A document's revision, which only grows: -1 for one that cannot say.
 * @param {string} json
 * @returns {number}
 */
function revisionOf(json) {
  try {
    const parsed = /** @type {{ revision?: unknown }} */ (JSON.parse(json));
    return typeof parsed.revision === "number" ? parsed.revision : -1;
  } catch {
    return -1;
  }
}

/**
 * Saves the serialized settings document to `chrome.storage.sync`.
 * @param {string} json
 * @returns {void}
 */
function saveSettings(json) {
  settingsJson = json;
  chrome.storage.sync
    .set({ [SETTINGS_KEY]: json })
    .catch((/** @type {unknown} */ error) =>
      console.warn("settings.save.failed", errorMessage(error)),
    );
}

// ChromeOS keyboards are PC keyboards: "linux" and "windows" read keys the
// same, and "linux" also matches the framework (the field confirms the
// preedit on blur, as IBus does).
const PLATFORM = "linux";

/**
 * A stored document the session refuses gives way to the defaults for good,
 * as on the page (`ime.js` `restoreSettings`), so the options page does not
 * show it either.
 * @param {unknown} stored
 * @returns {Session}
 */
function newSession(stored) {
  if (typeof stored === "string") {
    try {
      const restored = new Session(stored, PLATFORM, navigator.language);
      settingsJson = stored;
      return restored;
    } catch (error) {
      console.warn("settings.refused", errorMessage(error));
      saveSettings(DEFAULT_SETTINGS);
    }
  }
  return new Session(DEFAULT_SETTINGS, PLATFORM, navigator.language);
}

/**
 * Initializes the WASM engine, loads bundled dictionaries, and attaches learning.
 * @returns {Promise<void>}
 */
async function start() {
  const reading = chrome.storage.sync.get(SETTINGS_KEY);
  const module = await WebAssembly.compileStreaming(
    fetch(chrome.runtime.getURL("pkg/taigi_web_bg.wasm")),
  );
  await init({ module_or_path: module });
  const { [SETTINGS_KEY]: stored } = await reading;
  const starting = newSession(stored);
  for (const name of starting.dictionaryFiles()) {
    const response = await fetch(chrome.runtime.getURL(`dictionaries/${name}`));
    starting.addDictionary(name, new Uint8Array(await response.arrayBuffer()));
  }
  starting.install();
  session = starting;
  // Off the first key: opening the stores takes tens of milliseconds.
  learning.attach(starting);
  publishState();
}

/**
 * What follows the session's settings outside the field: the options page's
 * shortcut list and the tray menu's mode row. The page redraws the same on a
 * new instance and a settings change; `modeChanged` redraws only the mode.
 * @returns {void}
 */
function publishState() {
  const shortcuts = run((current) => current.shortcuts());
  if (shortcuts !== undefined && shortcuts !== publishedShortcuts) {
    publishedShortcuts = shortcuts;
    chrome.storage.local
      .set({ [SHORTCUTS_KEY]: shortcuts })
      .catch((/** @type {unknown} */ error) =>
        console.warn("shortcuts.save.failed", errorMessage(error)),
      );
  }
  showModeInTray();
}

/**
 * The tray's input method menu names the mode, as the page's chip does.
 * Only the active input method has a menu to set.
 * @returns {void}
 */
function showModeInTray() {
  if (!active) {
    return;
  }
  const line = run((current) => current.modeLine());
  if (line === undefined) {
    return;
  }
  chrome.input.ime
    .setMenuItems({
      engineID: ENGINE_ID,
      items: [modeMenuItem(/** @type {WireModeLine} */ (JSON.parse(line)))],
    })
    .catch((/** @type {unknown} */ error) =>
      console.warn("tray.menu.failed", errorMessage(error)),
    );
}

/**
 * A flash replaces one still up, as the page's `flashMode` does.
 * @param {Reply} reply
 * @returns {void}
 */
function timeModeFlash(reply) {
  if (!reply.emits.some((emit) => emit.type === "announceMode")) {
    return;
  }
  clearTimeout(modeFlashTimer);
  modeFlashTimer = setTimeout(() => {
    invokeImeCall(endModeFlash(ENGINE_ID, listShown)).catch(
      (/** @type {unknown} */ error) =>
        console.warn("modeFlash.hide.failed", errorMessage(error)),
    );
  }, MODE_FLASH_HOLD);
}

/**
 * One engine call, its answer as given; `undefined` with no session. A trap
 * ends the session for this worker: the glue keeps the instance it has and a
 * service worker cannot import a fresh copy, so the worker is let go and the
 * next focus starts a new one with a new session.
 * @template T
 * @param {(current: Session) => T} action
 * @returns {T | undefined}
 */
function run(action) {
  if (session === null) {
    return undefined;
  }
  try {
    return action(session);
  } catch (error) {
    if (!(error instanceof WebAssembly.RuntimeError)) {
      throw error;
    }
    console.error("session.trapped", error);
    session = null;
    trapped = true;
    letGo();
    return undefined;
  }
}

// What the engine learns from picks, kept in IndexedDB (`learning.js`, web
// Phase 8a). This worker is the extension's only learner.
const learning = new Learning(() => session, run);

const started = start().catch((error) =>
  console.error("session.start.failed", error),
);

/**
 * Dispatches one translated call against `chrome.input.ime`.
 * @param {ImeCall} imeCall
 * @returns {Promise<unknown>}
 */
function invokeImeCall(imeCall) {
  switch (imeCall[0]) {
    case "setComposition":
      return chrome.input.ime.setComposition(imeCall[1]);
    case "clearComposition":
      return chrome.input.ime.clearComposition(imeCall[1]);
    case "commitText":
      return chrome.input.ime.commitText(imeCall[1]);
    case "deleteSurroundingText":
      return chrome.input.ime.deleteSurroundingText(imeCall[1]);
    case "setCandidates":
      return chrome.input.ime.setCandidates(imeCall[1]);
    case "setCandidateWindowProperties":
      return chrome.input.ime.setCandidateWindowProperties(imeCall[1]);
    case "setCursorPosition":
      return chrome.input.ime.setCursorPosition(imeCall[1]);
  }
}

/**
 * One engine call that answers a reply, applied; whether it took the key.
 * @param {(current: Session) => string} action
 * @returns {boolean}
 */
function call(action) {
  const reply = run(action);
  return reply === undefined ? false : apply(reply);
}

/**
 * Applies a serialized session reply to ChromeOS IME APIs and storage.
 * @param {string} replyJson
 * @returns {boolean}
 */
function apply(replyJson) {
  const reply = /** @type {Reply} */ (JSON.parse(replyJson));
  listShown = listShownAfter(reply, listShown);
  for (const imeCall of imeCalls(reply, {
    engineID: ENGINE_ID,
    contextID: context?.contextID ?? 0,
    listShown,
  })) {
    invokeImeCall(imeCall).catch((/** @type {unknown} */ error) =>
      console.warn("ime.call.failed", imeCall[0], errorMessage(error)),
    );
  }
  timeModeFlash(reply);
  if (reply.emits.some((emit) => emit.type === "openSettings")) {
    chrome.runtime
      .openOptionsPage()
      .catch((/** @type {unknown} */ error) =>
        console.warn("settings.open.failed", errorMessage(error)),
      );
  }
  // A chord wrote the document: the options page and the next worker read it.
  if (reply.settings) {
    saveSettings(reply.settings);
  }
  if (reply.settings) {
    publishState();
  } else if (reply.emits.some((emit) => emit.type === "modeChanged")) {
    showModeInTray();
  }
  // A commit is what the engine learns from.
  if (reply.emits.some((emit) => emit.type === "commit")) {
    learning.scheduleSave();
  }
  return reply.handled;
}

/**
 * The field lost the keyboard: the composition it confirmed ends with it.
 * @returns {void}
 */
function leaveField() {
  call((current) => current.endSession());
  run((current) => current.focusLost());
  context = null;
  listShown = false;
  clearTimeout(modeFlashTimer);
}

/**
 * This input method is the active one (onActivate, or onFocus for a worker
 * started since): the tray menu shows its mode.
 * @returns {void}
 */
function activate() {
  keepAlive();
  if (!active) {
    active = true;
    started.then(showModeInTray);
  }
}

chrome.input.ime.onActivate.addListener(activate);

// Another input method took over: let the worker, the engine and its
// dictionaries go. onActivate wakes a stopped worker, which keeps it again.
chrome.input.ime.onDeactivated.addListener(() => {
  active = false;
  leaveField();
  // A trap already let it go.
  if (!trapped) {
    letGo();
  }
});

chrome.input.ime.onFocus.addListener((focused) => {
  context = focused;
  activate();
  started.then(() => call((current) => current.beginSession()));
});

// A blur can arrive after the next field's focus; it ends only its own field.
chrome.input.ime.onBlur.addListener((contextID) => {
  if (context?.contextID === contextID) {
    leaveField();
  }
});

chrome.input.ime.onReset.addListener(() => {
  if (isResetStorm(context)) {
    return;
  }
  call((current) => current.endSession());
  call((current) => current.beginSession());
});

// A key with no focused field (a worker restarted since the last onFocus) or
// before the session starts stays the field's. Every request is answered
// exactly once, a failing engine call included, or ChromeOS holds the key.
chrome.input.ime.onKeyEvent.addListener((_id, keyData, requestId) => {
  started.then(() => {
    let handled = false;
    try {
      if (context) {
        const json = keyJson(keyData);
        handled = call((current) =>
          keyData.type === "keydown"
            ? current.keyDown(json)
            : current.keyUp(json),
        );
      }
    } catch (error) {
      console.error("key.failed", error);
    } finally {
      chrome.input.ime.keyEventHandled(requestId, handled);
    }
  });
  return undefined;
});

chrome.input.ime.onCandidateClicked.addListener((_id, candidateID) => {
  if (context) {
    call((current) => current.clickCandidate(candidateID));
  }
});

// The options page changed the document. The composition is written first, as
// on the page (`ime.js` `changeSettings`): a TPS buffer cannot carry over.
chrome.storage.onChanged.addListener((changes, area) => {
  const json = changes[SETTINGS_KEY]?.newValue;
  if (area !== "sync" || typeof json !== "string") {
    return;
  }
  started.then(() => {
    // Equal or older: this worker's own write coming back, or one already
    // passed by a newer chord, which must not roll the session back.
    if (!session || revisionOf(json) <= revisionOf(settingsJson)) {
      return;
    }
    // A document the session then refuses still ends the composition, as on
    // the page.
    try {
      call((current) => current.commitComposition());
      run((current) => current.setSettings(json));
      if (!trapped) {
        settingsJson = json;
        publishState();
      }
    } catch (error) {
      console.warn("settings.refused", errorMessage(error));
    }
  });
});

// The offscreen document's messages are what keep this worker running; a key
// alone never wakes a stopped one (chromeos-roadmap.md § Spike result).
chrome.runtime.onMessage.addListener(() => {});

// onActivate and onFocus arrive together; one creation at a time.
/** @type {Promise<void> | null} */
let keeping = null;

/**
 * Ensures the keep-alive offscreen document is open while the IME is active.
 * @returns {void}
 */
function keepAlive() {
  if (trapped) {
    return;
  }
  keeping ??= openKeepAlive().finally(() => {
    keeping = null;
  });
}

/**
 * Stops keeping the worker: about 30 s later ChromeOS stops it, and the next
 * event starts a new one.
 * @returns {void}
 */
function letGo() {
  chrome.offscreen
    .closeDocument()
    .catch((/** @type {unknown} */ error) =>
      console.warn("keepAlive.close.failed", errorMessage(error)),
    );
}

/**
 * Creates the keep-alive offscreen document if it is not already present.
 * @returns {Promise<void>}
 */
async function openKeepAlive() {
  try {
    if (await chrome.offscreen.hasDocument()) {
      return;
    }
    await chrome.offscreen.createDocument({
      url: "offscreen.html",
      reasons: ["WORKERS"],
      justification: "Keeps the input method running between keystrokes.",
    });
  } catch (error) {
    console.warn("keepAlive.create.failed", errorMessage(error));
  }
}
