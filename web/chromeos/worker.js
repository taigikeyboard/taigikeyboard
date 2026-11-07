// The ChromeOS input method's service worker: chrome.input.ime events in, the
// taigi-web session's replies out (docs/architecture/chromeos-roadmap.md).

import { imeCalls, isResetStorm, keyJson } from "./calls.js";
import { Learning } from "./learning.js";
import init, { Session } from "./pkg/taigi_web.js";
import { emptyDocument, STORAGE_KEY as SETTINGS_KEY } from "./settings.js";

// The expandable layouts measure through the page's DOM, which a worker has
// none of; the session's `catch` import then keeps the list paged.
globalThis.taigiMeasureCells = () => {
  throw new Error("no DOM to measure candidate cells");
};

// `input_components[0].id` in manifest.json: known before any event names it,
// so a worker started by onFocus can still address the candidate window.
const ENGINE_ID = "taigi";

const DEFAULT_SETTINGS = JSON.stringify(emptyDocument());

let session = null;
let context = null;
// A trapped session is never kept: this worker waits to be stopped.
let trapped = false;
// The settings document the session holds.
let settingsJson = DEFAULT_SETTINGS;

/// A document's revision, which only grows: -1 for one that cannot say.
function revisionOf(json) {
  try {
    return JSON.parse(json).revision ?? -1;
  } catch {
    return -1;
  }
}

function saveSettings(json) {
  settingsJson = json;
  chrome.storage.sync
    .set({ [SETTINGS_KEY]: json })
    .catch((error) => console.warn("settings.save.failed", error.message));
}

// ChromeOS keyboards are PC keyboards: "linux" and "windows" read keys the
// same, and "linux" also matches the framework (the field confirms the
// preedit on blur, as IBus does).
const PLATFORM = "linux";

/// A stored document the session refuses gives way to the defaults for good,
/// as on the page (`ime.js` `restoreSettings`), so the options page does not
/// show it either.
function newSession(stored) {
  if (stored !== undefined) {
    try {
      const restored = new Session(stored, PLATFORM, navigator.language);
      settingsJson = stored;
      return restored;
    } catch (error) {
      console.warn("settings.refused", error.message);
      saveSettings(DEFAULT_SETTINGS);
    }
  }
  return new Session(DEFAULT_SETTINGS, PLATFORM, navigator.language);
}

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
}

/// One engine call, its answer as given; `undefined` with no session. A trap
/// ends the session for this worker: the glue keeps the instance it has and a
/// service worker cannot import a fresh copy, so the worker is let go and the
/// next focus starts a new one with a new session.
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

/// One engine call that answers a reply, applied; whether it took the key.
function call(action) {
  const reply = run(action);
  return reply === undefined ? false : apply(reply);
}

function apply(replyJson) {
  const reply = JSON.parse(replyJson);
  for (const [method, details] of imeCalls(reply, {
    engineID: ENGINE_ID,
    contextID: context?.contextID,
  })) {
    chrome.input.ime[method](details).catch((error) =>
      console.warn("ime.call.failed", method, error.message),
    );
  }
  if (reply.emits.some((emit) => emit.type === "openSettings")) {
    chrome.runtime
      .openOptionsPage()
      .catch((error) => console.warn("settings.open.failed", error.message));
  }
  // A chord wrote the document: the options page and the next worker read it.
  if (reply.settings) {
    saveSettings(reply.settings);
  }
  // A commit is what the engine learns from.
  if (reply.emits.some((emit) => emit.type === "commit")) {
    learning.scheduleSave();
  }
  return reply.handled;
}

/// The field lost the keyboard: the composition it confirmed ends with it.
function leaveField() {
  call((current) => current.endSession());
  run((current) => current.focusLost());
  context = null;
}

chrome.input.ime.onActivate.addListener(() => keepAlive());

// Another input method took over: let the worker, the engine and its
// dictionaries go. onActivate wakes a stopped worker, which keeps it again.
chrome.input.ime.onDeactivated.addListener(() => {
  leaveField();
  // A trap already let it go.
  if (!trapped) {
    letGo();
  }
});

chrome.input.ime.onFocus.addListener((focused) => {
  context = focused;
  keepAlive();
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
      }
    } catch (error) {
      console.warn("settings.refused", error.message);
    }
  });
});

// The offscreen document's messages are what keep this worker running; a key
// alone never wakes a stopped one (chromeos-roadmap.md § Spike result).
chrome.runtime.onMessage.addListener(() => {});

// onActivate and onFocus arrive together; one creation at a time.
let keeping = null;

function keepAlive() {
  if (trapped) {
    return;
  }
  keeping ??= openKeepAlive().finally(() => {
    keeping = null;
  });
}

/// Stops keeping the worker: about 30 s later ChromeOS stops it, and the next
/// event starts a new one.
function letGo() {
  chrome.offscreen
    .closeDocument()
    .catch((error) => console.warn("keepAlive.close.failed", error.message));
}

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
    console.warn("keepAlive.create.failed", error.message);
  }
}
