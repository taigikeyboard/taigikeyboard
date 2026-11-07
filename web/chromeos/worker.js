// The ChromeOS input method's service worker: chrome.input.ime events in, the
// taigi-web session's replies out (docs/architecture/chromeos-roadmap.md).

import { imeCalls, isResetStorm, keyJson } from "./calls.js";
import init, { Session } from "./pkg/taigi_web.js";

// The expandable layouts measure through the page's DOM, which a worker has
// none of; the session's `catch` import then keeps the list paged.
globalThis.taigiMeasureCells = () => {
  throw new Error("no DOM to measure candidate cells");
};

// `input_components[0].id` in manifest.json: known before any event names it,
// so a worker started by onFocus can still address the candidate window.
const ENGINE_ID = "taigi";

let session = null;
let context = null;

async function start() {
  const module = await WebAssembly.compileStreaming(
    fetch(chrome.runtime.getURL("pkg/taigi_web_bg.wasm")),
  );
  await init({ module_or_path: module });
  // ChromeOS keyboards are PC keyboards: "linux" and "windows" read keys the
  // same, and "linux" also matches the framework (the field confirms the
  // preedit on blur, as IBus does).
  const starting = new Session("{}", "linux", navigator.language);
  for (const name of starting.dictionaryFiles()) {
    const response = await fetch(chrome.runtime.getURL(`dictionaries/${name}`));
    starting.addDictionary(name, new Uint8Array(await response.arrayBuffer()));
  }
  starting.install();
  session = starting;
}

const started = start().catch((error) =>
  console.error("session.start.failed", error),
);

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
  return reply.handled;
}

/// Ends the composition the field has just confirmed. With no session yet
/// there is nothing to end.
function endComposition() {
  if (session) {
    apply(session.endSession());
  }
}

/// The field lost the keyboard: its composition ends with it.
function leaveField() {
  endComposition();
  session?.focusLost();
  context = null;
}

chrome.input.ime.onActivate.addListener(() => keepAlive());

// Another input method took over: let the worker, the engine and its
// dictionaries go. onActivate wakes a stopped worker, which keeps it again.
chrome.input.ime.onDeactivated.addListener(() => {
  leaveField();
  chrome.offscreen
    .closeDocument()
    .catch((error) => console.warn("keepAlive.close.failed", error.message));
});

chrome.input.ime.onFocus.addListener((focused) => {
  context = focused;
  keepAlive();
  started.then(() => {
    if (session) {
      apply(session.beginSession());
    }
  });
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
  endComposition();
  if (session) {
    apply(session.beginSession());
  }
});

// A key with no focused field (a worker restarted since the last onFocus) or
// before the session starts stays the field's. Every request is answered
// exactly once, a failing engine call included, or ChromeOS holds the key.
chrome.input.ime.onKeyEvent.addListener((_id, keyData, requestId) => {
  started.then(() => {
    let handled = false;
    try {
      if (session && context) {
        const json = keyJson(keyData);
        handled = apply(
          keyData.type === "keydown"
            ? session.keyDown(json)
            : session.keyUp(json),
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
  if (session && context) {
    apply(session.clickCandidate(candidateID));
  }
});

// The offscreen document's messages are what keep this worker running; a key
// alone never wakes a stopped one (chromeos-roadmap.md § Spike result).
chrome.runtime.onMessage.addListener(() => {});

// onActivate and onFocus arrive together; one creation at a time.
let keeping = null;

function keepAlive() {
  keeping ??= openKeepAlive().finally(() => {
    keeping = null;
  });
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
