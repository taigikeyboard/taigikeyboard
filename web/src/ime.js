// Browser events → the session → the screen. The engine stays on the main
// thread because `preventDefault` must run during `keydown`
// (docs/architecture/web-roadmap.md § Why the main thread).

import { Draft } from "./draft.js";
import { Editor } from "./editor.js";
import { FormatBubble } from "./format_bubble.js";
import { keepTitle } from "./header.js";
import { translate } from "./i18n.js";
import { Learning } from "./learning.js";
import { createSession, dictionaries, keepForOffline } from "./loader.js";
import { Panel } from "./panel.js";
import { wireSave } from "./save.js";
import { Settings } from "./settings.js";
import { drawShortcuts } from "./shortcuts.js";
import { Sidebar } from "./sidebar.js";
import { Surface } from "./surface.js";
import { TelexGuide } from "./telex_guide.js";
import { TpsKeyboard } from "./tps_keyboard.js";

/** @typedef {import("./pkg/taigi_web.js").Session} Session */
/** @typedef {import("../types/shared.d.ts").Platform} Platform */
/** @typedef {import("../types/shared.d.ts").Reply} Reply */
/** @typedef {import("../types/shared.d.ts").WireEmit} WireEmit */
/** @typedef {import("../types/shared.d.ts").WireModeLine} WireModeLine */
/** @typedef {import("../types/shared.d.ts").WireShortcuts} WireShortcuts */
/** @typedef {import("../types/shared.d.ts").WireTelexGuide} WireTelexGuide */
/** @typedef {import("../types/shared.d.ts").WireTpsRow} WireTpsRow */

/**
 * @template {HTMLElement} [T=HTMLElement]
 * @param {string} id
 * @returns {T}
 */
function requireElement(id) {
  const element = document.getElementById(id);
  if (!element) {
    throw new Error(`missing #${id}`);
  }
  return /** @type {T} */ (element);
}

/**
 * A call with no instance to answer it: the key is spent.
 *
 * @type {Reply}
 */
const NOTHING = { handled: true, emits: [] };

const editorBox = requireElement("editor");
const editor = new Editor(requireElement("text"));
const text = editor.root;
const notice = requireElement("notice");
const messageSlot = requireElement("message");
const modeFlash = requireElement("mode-flash");
const modeLine = requireElement("mode-line");
const modeName = requireElement("mode-name");
const modeDetail = requireElement("mode-detail");
const progress = /** @type {HTMLProgressElement} */ (
  requireElement("progress")
);
const retryButton = requireElement("retry");
const loadSlot = requireElement("load-slot");

const surface = new Surface(editor, editorBox);
const titleElement = requireElement("title");
wireSave(requireElement("save-formats"), () => editor.markdown(), titleElement);
// Before `Settings`, which translates the page.
keepTitle(titleElement, () => editor.focus());
const panel = new Panel(
  requireElement("panel"),
  requireElement("ruler"),
  requireElement("spoken"),
  (position) => {
    apply(call((session) => session.clickCandidate(position)));
  },
  (isForward) => {
    apply(call((session) => session.turnPage(isForward)));
  },
  (offset) => {
    run((session) => session.viewportScrolled(offset));
  },
);
// The session measures during a key: it needs the widths for that key's labels.
globalThis.taigiMeasureCells = (runs) => panel.measure(runs);
const tpsKeyboard = new TpsKeyboard(requireElement("tps-keyboard"), (glyph) => {
  // The next physical key belongs here too, not to a sidebar control.
  editor.focus();
  apply(call((session) => session.tpsKeyPress(glyph)));
});
const telexGuide = new TelexGuide(requireElement("telex-guide"));
const sidebar = new Sidebar(
  requireElement("sidebar-toggle"),
  requireElement("sidebar"),
);
const settings = new Settings(
  /** @type {HTMLFormElement} */ (requireElement("settings")),
  changeSettings,
);
const draft = new Draft(editor, translate("web.intro"));
const formatBubble = new FormatBubble(
  requireElement("format-bubble"),
  editor,
  surface,
  () => panel.isShown,
);
/**
 * Apple Notes' link chord, on Command on a Mac and Control elsewhere. Lexical
 * takes B, I and U itself.
 *
 * @type {Record<string, string>}
 */
const FORMAT_KEYS = { k: "link" };

/**
 * `null` between instances.
 *
 * @type {Session | null}
 */
let session = null;

const learning = new Learning(() => session, run);

/**
 * The session is the judge of the stored document: one it refuses leaves the
 * defaults.
 *
 * @param {Session} target
 * @returns {void}
 */
function restoreSettings(target) {
  try {
    target.setSettings(settings.json);
  } catch (error) {
    if (error instanceof WebAssembly.RuntimeError) {
      throw error;
    }
    console.error(error);
    settings.reset();
  }
}

/** @returns {Platform} */
function platform() {
  const name = navigator.userAgentData?.platform ?? navigator.platform;
  if (/mac|iphone|ipad/i.test(name)) {
    return "macos";
  }
  return /win/i.test(name) ? "windows" : "linux";
}

/**
 * `[key, placeholders]` of what the message slot says, so a language change can
 * redraw it. `null` for the session's own text, which arrives translated.
 *
 * @type {[string, Record<string, string> | undefined] | null}
 */
let said = null;

/**
 * @param {string} key
 * @param {string} state
 * @param {Record<string, string>} [placeholders]
 * @returns {void}
 */
function say(key, state, placeholders) {
  said = [key, placeholders];
  messageSlot.textContent = translate(key, placeholders);
  messageSlot.dataset.state = state;
}

/**
 * @param {string} label
 * @returns {void}
 */
function announce(label) {
  said = null;
  messageSlot.textContent = label;
  messageSlot.dataset.state = "mode";
  flashMode(label);
}

/** As long as the desktops hold theirs (`ModeFlashPanel.swift`). */
const MODE_FLASH_HOLD = 800;

/** @type {ReturnType<typeof setTimeout> | undefined} */
let modeFlashTimer;

/**
 * Replaces a flash still up: a quick double switch reads as two states.
 *
 * @param {string} label
 * @returns {void}
 */
function flashMode(label) {
  clearTimeout(modeFlashTimer);
  modeFlash.textContent = label;
  modeFlash.hidden = false;
  modeFlashTimer = setTimeout(() => {
    modeFlash.hidden = true;
  }, MODE_FLASH_HOLD);
}

/**
 * The session's text describes the settings just replaced, so it goes.
 *
 * @returns {void}
 */
function redrawMessage() {
  messageSlot.textContent = said === null ? "" : translate(...said);
}

/** @returns {void} */
function showLoading() {
  say("web.loading", "loading");
  loadSlot.dataset.show = "";
}

/**
 * @param {number} fraction
 * @returns {void}
 */
function showProgress(fraction) {
  progress.value = fraction;
  loadSlot.dataset.show = "progress";
}

/**
 * @param {unknown} error
 * @param {Session | null} starting
 * @returns {void}
 */
function failLoading(error, starting) {
  console.error(error);
  if (session !== starting) {
    return;
  }
  // A trap here would only repeat on the next start.
  if (error instanceof WebAssembly.RuntimeError) {
    session = null;
    panel.clear();
  }
  say("web.loadFailed", "failed");
  loadSlot.dataset.show = "retry";
  // With no instance a key would go nowhere; the text stays readable.
  editor.setEditable(session !== null);
}

/**
 * A trap may replace the instance during any of these awaits.
 *
 * @param {Session} starting
 * @returns {Promise<void>}
 */
async function loadDictionaries(starting) {
  try {
    const files = dictionaries(starting.dictionaryFiles(), (fraction) => {
      if (session === starting) {
        showProgress(fraction);
      }
    });
    for await (const [name, bytes] of files) {
      if (session !== starting) {
        return;
      }
      starting.addDictionary(name, bytes);
    }
    if (session !== starting) {
      return;
    }
    starting.install();
  } catch (error) {
    failLoading(error, starting);
    return;
  }
  loadSlot.dataset.show = "";
  say("web.ready", "ready");
  keepForOffline();
}

/**
 * Starts an instance. Until the dictionaries are in, keys type romanization
 * (`runtime.rs` `install_lexicon`).
 *
 * @returns {Promise<void>}
 */
async function start() {
  /** @type {Session | null} */
  let starting = null;
  showLoading();
  try {
    starting = await createSession(platform(), navigator.language);
    session = starting;
    restoreSettings(starting);
    // Off the first key: opening the stores takes tens of milliseconds.
    learning.attach(starting);
    redrawShortcuts();
    tpsKeyboard.draw(
      /** @type {WireTpsRow[]} */ (JSON.parse(starting.tpsKeyboardRows())),
    );
    redrawTelexGuide();
    redrawModeLine();
    editor.setEditable(true);
    editor.focus();
  } catch (error) {
    failLoading(error, starting);
    return;
  }
  await loadDictionaries(starting);
}

/**
 * One engine call, its answer as given; `undefined` with no instance. A trap
 * ends the instance: the composition goes with it, the text in the editor
 * stays, and the key that hit it is spent.
 *
 * @template T
 * @param {(session: Session) => T} action
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
    console.error(error);
    session = null;
    panel.clear();
    start();
    return undefined;
  }
}

/**
 * One engine call that answers a reply.
 *
 * @param {(session: Session) => string} action
 * @returns {Reply}
 */
function call(action) {
  const reply = run(action);
  return reply === undefined
    ? NOTHING
    : /** @type {Reply} */ (JSON.parse(reply));
}

/**
 * @param {WireEmit} emit
 * @returns {void}
 */
function applyEmit(emit) {
  switch (emit.type) {
    case "preedit":
      panel.showPreedit(emit.text, emit.caret);
      break;
    case "clearPreedit":
      panel.clearPreedit();
      break;
    case "commit":
      // The engine has let go of this text: it goes to the console and the
      // hidden live region, nothing is drawn (maintainer 2026-11-07).
      if (!surface.insert(emit.text)) {
        console.error("web.commit.refused", emit.text);
        say("web.writeFailed", "failed", { text: emit.text });
      }
      learning.scheduleSave();
      break;
    case "deleteSurrounding":
      surface.deleteBefore(emit.count);
      break;
    case "lookupTable":
      panel.showTable(emit);
      break;
    case "hideLookupTable":
      panel.hideTable();
      break;
    case "modeChanged":
      redrawModeLine();
      break;
    case "announceMode":
      announce(emit.label);
      break;
    case "openSettings":
      sidebar.toggle();
      break;
    case "toggleTpsKeyboard":
      settings.toggleTpsKeyboard();
      run((current) => current.setSettings(settings.json));
      break;
    case "tpsKeyboardCapTyped":
      tpsKeyboard.flash(emit.row, emit.cap);
      break;
  }
}

/**
 * The form's change. A running composition ends under the settings it began
 * with: TPS has another alphabet, so its raw buffer cannot carry over.
 *
 * @param {string} json
 * @returns {void}
 */
function changeSettings(json) {
  apply(call((current) => current.commitComposition()));
  run((current) => current.setSettings(json));
  redrawMessage();
  redrawShortcuts();
  redrawTelexGuide();
  redrawModeLine();
  draft.translateIntro(translate("web.intro"));
}

/**
 * Keys and labels follow Tone Keys, the input mode and the language.
 *
 * @returns {void}
 */
function redrawShortcuts() {
  const raw = run((current) => current.shortcuts());
  if (raw === undefined) {
    return;
  }
  const { groups } = /** @type {WireShortcuts} */ (JSON.parse(raw));
  if (groups) {
    drawShortcuts(requireElement("shortcuts"), groups);
  }
}

/**
 * The guide is spelled in the display language.
 *
 * @returns {void}
 */
function redrawTelexGuide() {
  const guide = run((current) => current.telexGuide());
  if (guide !== undefined) {
    telexGuide.draw(/** @type {WireTelexGuide} */ (JSON.parse(guide)));
  }
}

/**
 * Taigi: the mode and the input script. English: the mode and the Shift tap
 * back to Taigi.
 *
 * @returns {void}
 */
function redrawModeLine() {
  const line = run((current) => current.modeLine());
  if (line === undefined) {
    return;
  }
  const { isEnglish, mode, script, other } = /** @type {WireModeLine} */ (
    JSON.parse(line)
  );
  modeName.textContent = mode;
  if (isEnglish) {
    const shift = document.createElement("kbd");
    shift.textContent = "⇧";
    modeDetail.replaceChildren(shift, other);
  } else {
    modeDetail.textContent = script;
  }
  modeLine.toggleAttribute("data-english", isEnglish);
  modeLine.toggleAttribute("data-drawn", true);
}

/** @returns {void} */
function placePanel() {
  if (panel.isShown) {
    panel.place(surface.caretPoint());
  }
}

/**
 * @param {Reply} reply
 * @returns {void}
 */
function apply(reply) {
  if (reply.settings) {
    settings.adopt(reply.settings);
    redrawShortcuts();
  }
  if (reply.emits.length > 0) {
    surface.batch(() => reply.emits.forEach(applyEmit));
    placePanel();
  }
}

/**
 * @param {KeyboardEvent} event
 * @returns {string}
 */
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

/**
 * `link` for Command+K on a Mac (Control+K elsewhere), with no other
 * modifier; otherwise `undefined`.
 *
 * @param {KeyboardEvent} event
 * @returns {string | undefined}
 */
function formatKind(event) {
  const isMac = platform() === "macos";
  const command = isMac ? event.metaKey : event.ctrlKey;
  const other = isMac ? event.ctrlKey : event.metaKey;
  if (!command || other || event.shiftKey || event.altKey) {
    return undefined;
  }
  return FORMAT_KEYS[event.key.toLowerCase()];
}

/**
 * A key another input method holds is its own.
 *
 * @param {KeyboardEvent} event
 * @returns {boolean}
 */
function isForeignComposition(event) {
  return event.isComposing || event.keyCode === 229;
}

// Capture on `#editor`: the engine has the key before Lexical, whose handlers on
// the root skip a key the engine took (`Editor`, KEY_DOWN_COMMAND). The URL field in the
// format bubble is inside `#editor` too, and its keys are its own.
editorBox.addEventListener(
  "keydown",
  (event) => {
    const target = /** @type {Node | null} */ (event.target);
    if (!text.contains(target) || isForeignComposition(event)) {
      return;
    }
    onKeyDown(event);
  },
  { capture: true },
);

/**
 * @param {KeyboardEvent} event
 * @returns {void}
 */
function onKeyDown(event) {
  // The auto-space swap deletes the character before the caret unseen, so its
  // arm cannot outlive the caret it was set at.
  if (surface.takeForeignChange()) {
    run((current) => current.caretMoved());
  }
  const reply = call((current) => current.keyDown(keyJson(event)));
  if (reply.handled) {
    event.preventDefault();
  }
  apply(reply);
  // After the engine: a composition it commits replaces the selection, and the
  // key then goes on to the browser.
  const kind = !reply.handled && editor.hasSelection() && formatKind(event);
  if (kind) {
    event.preventDefault();
    formatBubble.format(kind);
  }
}

text.addEventListener("keyup", (event) => {
  if (!isForeignComposition(event)) {
    apply(call((current) => current.keyUp(keyJson(event))));
  }
});

text.addEventListener("compositionstart", () => {
  notice.toggleAttribute("data-active", true);
});

text.addEventListener("compositionend", () => {
  notice.toggleAttribute("data-active", false);
});

text.addEventListener("scroll", placePanel);
// The docked sidebar resizes the editor; the window can
// change the room below it and leave it as it is.
new ResizeObserver(placePanel).observe(text);
window.addEventListener("resize", placePanel);

// Capture: this runs before the click moves the caret or `blur` takes the
// focus `execCommand` needs. A click on a candidate or a key cap belongs to the
// composition.
document.addEventListener(
  "pointerdown",
  (event) => {
    const target = /** @type {Node | null} */ (event.target);
    run((current) => current.caretMoved());
    if (!panel.contains(target) && !tpsKeyboard.contains(target)) {
      apply(call((current) => current.commitComposition()));
    }
  },
  { capture: true },
);

// Nothing is written: the composition stays and continues on return.
window.addEventListener("blur", () => {
  run((current) => current.focusLost());
});

for (const button of document.querySelectorAll("button")) {
  button.addEventListener("mousedown", (event) => event.preventDefault());
}

// `/` jumps to the editor, as kautian-mirror's search does, from anywhere but the
// editor and any field that takes text: a select types ahead, and `/` is part
// of an address in the format bubble's URL field. A checkbox just clicked gives way. No
// select-all: the next key would replace the document.
document.addEventListener("keydown", (event) => {
  const { key, metaKey, ctrlKey, altKey, isComposing } = event;
  const target = /** @type {HTMLElement | null} */ (event.target);
  if (key !== "/" || metaKey || ctrlKey || altKey || isComposing || !target) {
    return;
  }
  if (
    target.isContentEditable ||
    target.closest("textarea, select, input:not([type=checkbox])")
  ) {
    return;
  }
  event.preventDefault();
  editor.focus();
});

retryButton.addEventListener("click", () => {
  if (session === null) {
    start();
    return;
  }
  showLoading();
  loadDictionaries(session);
});

start();
