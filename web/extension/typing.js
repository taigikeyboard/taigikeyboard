// A website's keys → the session → the site's own field. The page's `ime.js`
// is this file's twin over the page's editor; here the text belongs to the
// site, so the composition draws in an overlay and reaches the field only at
// commit, through the one write its undo history and its editor both follow
// (`field.js`). The engine runs in this tab, on its main thread, because
// `preventDefault` must run during `keydown`
// (docs/architecture/web-roadmap.md § Why the main thread).
//
// Loaded by `content.js` the first time typing is turned on in the tab.

import { createSession, installDictionaries } from "./engine.js";
import { classify, focusedElement, surfaceOf } from "./field.js";
import { chooseLanguage, translate } from "./i18n.js";
import { SharedLearning } from "./learning-client.js";
import { Overlay } from "./overlay.js";
import { emptyDocument, parseStored, STORAGE_KEY } from "./settings.js";

/// A call with no instance to answer it: the key stays the site's.
const NOTHING = { handled: false, emits: [] };

const DEFAULT_SETTINGS = JSON.stringify(emptyDocument());

/// How often a running composition checks that its field is still there: a
/// site can take a focused field out of the document, which fires no event.
const FIELD_WATCH_MILLISECONDS = 500;

/// `null` between instances.
let session = null;
/// The dictionaries are in.
let isReady = false;
let hasFailed = false;
/// Typing is on in this tab (`"taigi"` or `"english"` to the service worker).
let isOn = false;
/// The mode a reloaded tab comes back in, taken once by the first instance.
let isEnglishWanted = false;
/// The focused field, as `classify` answers, with its `surface` when the
/// extension types into it; `null` when nothing that takes text has the focus.
let field = null;
/// The keys the engine took, by `code`: their releases are not the site's.
const heldKeys = new Set();
/// The settings document the session holds.
let settingsJson = DEFAULT_SETTINGS;
let fieldWatch;
/// The last state told to the service worker.
let told = "";

let overlay;
let panel;

/// A message to the service worker; `undefined` when it does not answer. An
/// extension reloaded or removed leaves this script without one: it stops.
async function ask(message) {
  try {
    return await chrome.runtime.sendMessage(message);
  } catch (error) {
    if (!chrome.runtime?.id) {
      retire();
    } else {
      console.error(error);
    }
    return undefined;
  }
}

const learning = new SharedLearning({
  current: () => session,
  run,
  ask,
  isPrivate: chrome.extension.inIncognitoContext,
  // A composition ends under the instance it began with.
  onStale: restartWhenIdle,
});

function platform() {
  const name = navigator.userAgentData?.platform ?? navigator.platform;
  if (/mac|iphone|ipad/i.test(name)) {
    return "macos";
  }
  return /win/i.test(name) ? "windows" : "linux";
}

function revisionOf(json) {
  return parseStored(json).revision;
}

/// The settings the overlay draws by; the rest are the session's.
function drawSettings() {
  const { values } = parseStored(settingsJson);
  chooseLanguage(values.displayLanguage ?? "system");
  overlay.setLook(values.candidateAppearanceMode, values.candidateTextSize);
}

/// The session is the judge of the stored document: one it refuses gives way
/// to the defaults for good, as on the page.
function restoreSettings(target) {
  try {
    target.setSettings(settingsJson);
  } catch (error) {
    if (error instanceof WebAssembly.RuntimeError) {
      throw error;
    }
    console.error(error);
    saveSettings(DEFAULT_SETTINGS);
  }
}

function saveSettings(json) {
  settingsJson = json;
  drawSettings();
  chrome.storage.sync.set({ [STORAGE_KEY]: json }).catch(console.error);
}

function fail(error, starting) {
  console.error(error);
  if (session !== starting) {
    return;
  }
  // A trap here would only repeat on the next start.
  if (error instanceof WebAssembly.RuntimeError) {
    session = null;
    panel.clear();
  }
  hasFailed = true;
  redraw();
}

/// Starts an instance. Until the dictionaries are in, keys type romanization
/// (`runtime.rs` `install_lexicon`).
async function start() {
  let starting = null;
  isReady = false;
  hasFailed = false;
  redraw();
  try {
    starting = await createSession(platform(), navigator.language);
    session = starting;
    restoreSettings(starting);
    if (isEnglishWanted) {
      isEnglishWanted = false;
      starting.toggleLanguageMode();
    }
    // Off the first key: opening the stores takes tens of milliseconds.
    learning.attach(starting);
    redraw();
    if (await installDictionaries(starting, () => session === starting)) {
      isReady = true;
      redraw();
    }
  } catch (error) {
    fail(error, starting);
  }
}

/// One engine call, its answer as given; `undefined` with no instance. A trap
/// ends the instance: the composition goes with it, the field's text stays,
/// and a new instance starts.
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
    console.error(error);
    restart();
    return undefined;
  }
}

/// One engine call that answers a reply.
function call(action) {
  const reply = run(action);
  return reply === undefined ? NOTHING : JSON.parse(reply);
}

/// A new instance in the mode this one is in.
function restart() {
  isEnglishWanted = isEnglish();
  session = null;
  panel.clear();
  start();
}

let isRestartWanted = false;

function restartWhenIdle() {
  isRestartWanted = true;
  restartIfIdle();
}

function restartIfIdle() {
  if (isRestartWanted && !panel.isShown) {
    isRestartWanted = false;
    restart();
  }
}

/// Another tab may have learned while this one was behind it.
async function catchUp() {
  if (isOn && (await learning.isStale())) {
    restartWhenIdle();
  }
}

function modeLine() {
  const line = run((current) => current.modeLine());
  return line === undefined ? null : JSON.parse(line);
}

function isEnglish() {
  return modeLine()?.isEnglish ?? false;
}

/// The tab's state under `line`, the session's mode line, and the words for
/// it: `"off"`, `"failed"`, `"loading"`, `"unsupported"`, `"english"` or
/// `"taigi"`.
function stateOf(line) {
  if (!isOn) {
    return { state: "off", label: translate("web.extensionOff") };
  }
  if (hasFailed) {
    return { state: "failed", label: translate("web.loadFailed") };
  }
  if (line === null || !isReady) {
    return { state: "loading", label: translate("web.loading") };
  }
  if (field?.kind === "unsupported") {
    return {
      state: "unsupported",
      label: translate("web.extensionUnsupported"),
    };
  }
  return line.isEnglish
    ? { state: "english", label: line.mode }
    : { state: "taigi", label: `${line.mode} · ${line.script}` };
}

/// The chip beside the field and the toolbar button say the same thing.
function redraw() {
  const line = modeLine();
  const { state, label } = stateOf(line);
  if (isOn && field) {
    overlay.keep();
    overlay.showChip(field.element, state, label);
  } else {
    overlay.hideChip();
  }
  // The mode the tab returns in after a reload, whatever the field is.
  const mode = isOn ? (line?.isEnglish ? "english" : "taigi") : "off";
  const telling = JSON.stringify({ state, label, mode });
  if (telling !== told) {
    told = telling;
    ask({ type: "taigi.status", state, label, mode });
  }
}

function place() {
  if (panel.isShown && field?.surface) {
    overlay.setPreeditFont(caretElement());
    panel.place(field.surface.caretPoint());
  }
  overlay.placeChip();
}

/// The element whose font the composition takes.
function caretElement() {
  if (field.kind === "input") {
    return field.element;
  }
  const node = getSelection()?.focusNode;
  if (!node || !field.element.contains(node)) {
    return field.element;
  }
  return node.nodeType === Node.ELEMENT_NODE ? node : node.parentElement;
}

/// A write the field refused: its length is in the console, and the field's
/// keys are the site's from here on. Never the text: the console is the
/// site's, and the text is the user's.
function refuse(text) {
  console.error("taigi.commit.refused", [...text].length);
  field = { kind: "unsupported", element: field.element };
}

function applyEmit(emit) {
  switch (emit.type) {
    case "preedit":
      panel.showPreedit(emit.text, emit.caret);
      break;
    case "clearPreedit":
      panel.clearPreedit();
      break;
    case "commit":
      if (field?.surface && !field.surface.insert(emit.text)) {
        refuse(emit.text);
      }
      learning.scheduleSave();
      break;
    case "deleteSurrounding":
      field?.surface?.deleteBefore(emit.count);
      break;
    case "lookupTable":
      panel.showTable(emit);
      break;
    case "hideLookupTable":
      panel.hideTable();
      break;
    case "openSettings":
      ask({ type: "taigi.openSettings" });
      break;
    // "modeChanged" and "announceMode": `apply` redraws the chip. The TPS key
    // panel and the Telex guide are the page's; the extension draws neither.
  }
}

/// Joins commits in one reply that no `deleteSurrounding` separates, so one
/// reply executes one `insertText` (one undo step, and one `beforeinput` for
/// editors that reconcile the DOM in a microtask after `execCommand`).
function coalesceCommits(emits) {
  const joined = [];
  let lastCommit = null;
  for (const emit of emits) {
    if (emit.type === "commit") {
      if (lastCommit) {
        lastCommit.text += emit.text;
        continue;
      }
      lastCommit = { ...emit };
      joined.push(lastCommit);
      continue;
    }
    if (emit.type === "deleteSurrounding") {
      lastCommit = null;
    }
    joined.push(emit);
  }
  return joined;
}

function apply(reply) {
  if (reply.settings) {
    saveSettings(reply.settings);
  }
  if (reply.emits.length > 0) {
    const emits = coalesceCommits(reply.emits);
    if (field?.surface) {
      field.surface.batch(() => emits.forEach(applyEmit));
    } else {
      emits.forEach(applyEmit);
    }
    watchField();
    place();
    if (panel.isShown) {
      queueMicrotask(place);
    }
    redraw();
    restartIfIdle();
  }
}

/// A composition with no field to write to is dropped, never carried to the
/// next one.
function dropComposition() {
  if (panel.isShown) {
    call((current) => current.endSession());
    panel.clear();
  }
  clearInterval(fieldWatch);
  fieldWatch = undefined;
  restartIfIdle();
}

function watchField() {
  if (!panel.isShown) {
    clearInterval(fieldWatch);
    fieldWatch = undefined;
  } else if (fieldWatch === undefined) {
    fieldWatch = setInterval(syncField, FIELD_WATCH_MILLISECONDS);
  }
}

/// Follows the focus. The field that had it is done: its composition ends
/// there. Focus that left with the whole window (another tab, another
/// application) is still this field's.
function syncField() {
  const next = classify(focusedElement());
  if (next?.element === field?.element && next?.kind === field?.kind) {
    return;
  }
  // A refused write stays refused while the field keeps the focus.
  if (next?.element === field?.element && field?.kind === "unsupported") {
    return;
  }
  dropComposition();
  field = next;
  if (field && field.kind !== "unsupported") {
    field.surface = surfaceOf(field, overlay.box, overlay.mirror);
    if (isOn) {
      apply(call((current) => current.beginSession()));
    }
  }
  redraw();
}

function keyJson(event) {
  return JSON.stringify({
    key: event.key,
    code: event.code,
    shift: event.shiftKey,
    ctrl: event.ctrlKey,
    alt: event.altKey,
    meta: event.metaKey,
    repeat: event.repeat,
  });
}

/// A key another input method holds is its own.
function isForeignComposition(event) {
  return event.isComposing || event.keyCode === 229;
}

/// Whether the engine is asked about `event`: typing is on, the key is the
/// user's, and it goes to a field the extension types into.
function isOurs(event) {
  if (!isOn || !event.isTrusted || isForeignComposition(event)) {
    return false;
  }
  syncField();
  return Boolean(field?.surface);
}

/// The window's first `keydown` listener (`content.js` adds it at
/// `document_start`): a key the engine took never reaches the site.
export function onKeyDown(event) {
  if (!isOurs(event)) {
    return;
  }
  // The auto-space swap deletes the character before the caret unseen, so its
  // arm cannot outlive the caret it was set at.
  if (field.surface.takeForeignChange()) {
    call((current) => current.caretMoved());
  }
  const reply = call((current) => current.keyDown(keyJson(event)));
  if (reply.handled) {
    event.preventDefault();
    event.stopImmediatePropagation();
    heldKeys.add(event.code);
  }
  apply(reply);
}

export function onKeyUp(event) {
  // A site that acts on a release (a search on Enter) must not act on the key
  // that picked a candidate.
  if (heldKeys.delete(event.code)) {
    event.stopImmediatePropagation();
  }
  if (isOurs(event)) {
    apply(call((current) => current.keyUp(keyJson(event))));
  }
}

/// Aligns the session's language mode with `wantEnglish`.
function syncLanguageMode(wantEnglish) {
  if (session === null) {
    isEnglishWanted = wantEnglish;
    return;
  }
  if (isEnglish() !== wantEnglish) {
    apply(call((current) => current.toggleLanguageMode()));
  }
}

/// `mode`: `"off"`, `"taigi"` or `"english"`, from the toolbar, the shortcut
/// or the tab's own state after a reload or back-forward restore.
export function setMode(mode) {
  if (mode === "off") {
    if (!isOn) {
      return;
    }
    isOn = false;
    told = "";
    apply(call((current) => current.commitComposition()));
    dropComposition();
    redraw();
    return;
  }
  const wasOn = isOn;
  isOn = true;
  told = "";
  if (!wasOn) {
    field = null;
  }
  if (session === null) {
    isEnglishWanted = mode === "english";
    start();
  } else {
    syncLanguageMode(mode === "english");
  }
  syncField();
  redraw();
  catchUp();
}

/// The extension was reloaded or removed: this script can reach nothing of it.
function retire() {
  isOn = false;
  session = null;
  panel.clear();
  overlay.hideChip();
}

async function prepare() {
  const [styles, stored] = await Promise.all([
    fetch(chrome.runtime.getURL("overlay.css")).then((response) =>
      response.text(),
    ),
    chrome.storage.sync.get(STORAGE_KEY),
  ]);
  overlay = new Overlay(
    styles,
    (position) => {
      apply(call((current) => current.clickCandidate(position)));
    },
    (isForward) => {
      apply(call((current) => current.turnPage(isForward)));
    },
    (offset) => {
      call((current) => current.viewportScrolled(offset));
    },
  );
  panel = overlay.panel;
  // The session measures during a key: it needs the widths for that key's
  // labels. This world's global, not the site's.
  globalThis.taigiMeasureCells = (runs) => panel.measure(runs);
  if (typeof stored[STORAGE_KEY] === "string") {
    settingsJson = stored[STORAGE_KEY];
  }
  drawSettings();

  window.addEventListener("focusin", syncField, true);
  // Focus that leaves for nothing fires no `focusin`; by the next task the
  // document knows where it went.
  window.addEventListener("focusout", () => setTimeout(syncField), true);
  // Capture: this runs before the click moves the caret or takes the focus
  // `execCommand` needs. A click on a candidate belongs to the composition.
  window.addEventListener(
    "pointerdown",
    (event) => {
      if (!isOn) {
        return;
      }
      call((current) => current.caretMoved());
      if (!overlay.holds(event)) {
        apply(call((current) => current.commitComposition()));
      }
    },
    true,
  );
  // Nothing is written: the composition stays and continues on return.
  window.addEventListener("blur", () => {
    heldKeys.clear();
    call((current) => current.focusLost());
  });
  // Back in front: by the window, or by its tab.
  window.addEventListener("focus", catchUp);
  document.addEventListener("visibilitychange", () => {
    if (!document.hidden) {
      catchUp();
    }
  });
  window.addEventListener("scroll", place, { capture: true, passive: true });
  window.addEventListener("resize", place);

  // The options page or another tab's chord changed the document. The
  // composition is written first, as on the page (`ime.js` `changeSettings`):
  // a TPS buffer cannot carry over.
  chrome.storage.onChanged.addListener((changes, area) => {
    const json = changes[STORAGE_KEY]?.newValue;
    // Equal or older: this tab's own write coming back.
    if (
      area !== "sync" ||
      typeof json !== "string" ||
      revisionOf(json) <= revisionOf(settingsJson)
    ) {
      return;
    }
    apply(call((current) => current.commitComposition()));
    try {
      run((current) => current.setSettings(json));
      settingsJson = json;
      drawSettings();
      redraw();
    } catch (error) {
      console.error(error);
    }
  });
}

await prepare();
