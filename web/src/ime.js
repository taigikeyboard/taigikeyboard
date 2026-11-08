// Browser events → the session → the screen. The engine stays on the main
// thread because `preventDefault` must run during `keydown`
// (docs/architecture/web-roadmap.md § Why the main thread).

import { Draft } from "./draft.js";
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
import { TpsKeyboard } from "./tps_keyboard.js";

/// A call with no instance to answer it: the key is spent.
const NOTHING = { handled: true, emits: [] };

const textarea = document.getElementById("text");
const notice = document.getElementById("notice");
const messageSlot = document.getElementById("message");
const modeFlash = document.getElementById("mode-flash");
const progress = document.getElementById("progress");
const retryButton = document.getElementById("retry");
const loadSlot = document.getElementById("load-slot");

const surface = new Surface(textarea);
wireSave(
  document.getElementById("save-formats"),
  textarea,
  document.getElementById("title"),
);
// Before `Settings`, which translates the page.
keepTitle(document.getElementById("title"), textarea);
const panel = new Panel(
  document.getElementById("panel"),
  document.getElementById("ruler"),
  document.getElementById("spoken"),
  (position) => {
    apply(call((session) => session.clickCandidate(position)));
  },
  (isForward) => {
    apply(call((session) => session.turnPage(isForward)));
  },
  (offset) => {
    call((session) => session.viewportScrolled(offset));
  },
);
// The session measures during a key: it needs the widths for that key's labels.
globalThis.taigiMeasureCells = (runs) => panel.measure(runs);
const tpsKeyboard = new TpsKeyboard(
  document.getElementById("tps-keyboard"),
  (glyph) => {
    // The next physical key belongs here too, not to a sidebar control.
    textarea.focus();
    apply(call((session) => session.tpsKeyPress(glyph)));
  },
);
const sidebar = new Sidebar(
  document.getElementById("sidebar-toggle"),
  document.getElementById("sidebar"),
);
const settings = new Settings(
  document.getElementById("settings"),
  changeSettings,
);
const draft = new Draft(textarea, surface, translate("web.intro"));

/// `null` between instances.
let session = null;

const learning = new Learning(() => session, run);

/// The session is the judge of the stored document: one it refuses leaves the
/// defaults.
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

function platform() {
  const name = navigator.userAgentData?.platform ?? navigator.platform;
  if (/mac|iphone|ipad/i.test(name)) {
    return "macos";
  }
  return /win/i.test(name) ? "windows" : "linux";
}

/// `[key, placeholders]` of what the message slot says, so a language change can
/// redraw it. `null` for the session's own text, which arrives translated.
let said = null;

function say(key, state, placeholders) {
  said = [key, placeholders];
  messageSlot.textContent = translate(key, placeholders);
  messageSlot.dataset.state = state;
}

function announce(label) {
  said = null;
  messageSlot.textContent = label;
  messageSlot.dataset.state = "mode";
  flashMode(label);
}

/// As long as the desktops hold theirs (`ModeFlashPanel.swift`).
const MODE_FLASH_HOLD = 800;

let modeFlashTimer;

/// Replaces a flash still up: a quick double switch reads as two states.
function flashMode(label) {
  clearTimeout(modeFlashTimer);
  modeFlash.textContent = label;
  modeFlash.hidden = false;
  modeFlashTimer = setTimeout(() => {
    modeFlash.hidden = true;
  }, MODE_FLASH_HOLD);
}

/// The session's text describes the settings just replaced, so it goes.
function redrawMessage() {
  messageSlot.textContent = said === null ? "" : translate(...said);
}

function showLoading() {
  say("web.loading", "loading");
  loadSlot.dataset.show = "";
}

function showProgress(fraction) {
  progress.value = fraction;
  loadSlot.dataset.show = "progress";
}

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
  textarea.disabled = session === null;
}

/// A trap may replace the instance during any of these awaits.
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

/// Starts an instance. Until the dictionaries are in, keys type romanization
/// (`runtime.rs` `install_lexicon`).
async function start() {
  let starting = null;
  showLoading();
  try {
    starting = await createSession(platform(), navigator.language);
    session = starting;
    restoreSettings(starting);
    // Off the first key: opening the stores takes tens of milliseconds.
    learning.attach(starting);
    redrawShortcuts();
    tpsKeyboard.draw(JSON.parse(starting.tpsKeyboardRows()));
    textarea.disabled = false;
    textarea.focus();
  } catch (error) {
    failLoading(error, starting);
    return;
  }
  await loadDictionaries(starting);
}

/// One engine call, its answer as given; `undefined` with no instance. A trap
/// ends the instance: the composition goes with it, the text in the textarea
/// stays, and the key that hit it is spent.
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

/// One engine call that answers a reply.
function call(action) {
  const reply = run(action);
  return reply === undefined ? NOTHING : JSON.parse(reply);
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
    case "announceMode":
      announce(emit.label);
      break;
    case "openSettings":
      sidebar.toggle();
      break;
    case "toggleTpsKeyboard":
      settings.toggleTpsKeyboard();
      call((current) => current.setSettings(settings.json));
      break;
    case "tpsKeyboardCapTyped":
      tpsKeyboard.flash(emit.row, emit.cap);
      break;
  }
}

/// The form's change. A running composition ends under the settings it began
/// with: TPS has another alphabet, so its raw buffer cannot carry over.
function changeSettings(json) {
  apply(call((current) => current.commitComposition()));
  call((current) => current.setSettings(json));
  redrawMessage();
  redrawShortcuts();
  draft.translateIntro(translate("web.intro"));
}

/// Keys and labels follow Tone Keys, the input mode and the language.
function redrawShortcuts() {
  const { groups } = call((current) => current.shortcuts());
  if (groups) {
    drawShortcuts(document.getElementById("shortcuts"), groups);
  }
}

function placePanel() {
  if (panel.isShown) {
    panel.place(surface.caretPoint());
  }
}

function apply(reply) {
  if (reply.settings) {
    settings.adopt(reply.settings);
    redrawShortcuts();
  }
  if (reply.emits.length > 0) {
    reply.emits.forEach(applyEmit);
    placePanel();
  }
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

textarea.addEventListener("keydown", (event) => {
  if (isForeignComposition(event)) {
    return;
  }
  // The auto-space swap deletes the character before the caret unseen, so its
  // arm cannot outlive the caret it was set at.
  if (surface.takeForeignChange()) {
    call((current) => current.caretMoved());
  }
  const reply = call((current) => current.keyDown(keyJson(event)));
  if (reply.handled) {
    event.preventDefault();
  }
  apply(reply);
});

textarea.addEventListener("keyup", (event) => {
  if (!isForeignComposition(event)) {
    apply(call((current) => current.keyUp(keyJson(event))));
  }
});

textarea.addEventListener("compositionstart", () => {
  notice.toggleAttribute("data-active", true);
});

textarea.addEventListener("compositionend", () => {
  notice.toggleAttribute("data-active", false);
});

textarea.addEventListener("scroll", placePanel);
// The docked sidebar and the textarea's own grip resize it; the window can
// change the room below it and leave it as it is.
new ResizeObserver(placePanel).observe(textarea);
window.addEventListener("resize", placePanel);

// Capture: this runs before the click moves the caret or `blur` takes the
// focus `execCommand` needs. A click on a candidate or a key cap belongs to the
// composition.
document.addEventListener(
  "pointerdown",
  (event) => {
    call((current) => current.caretMoved());
    if (!panel.contains(event.target) && !tpsKeyboard.contains(event.target)) {
      apply(call((current) => current.commitComposition()));
    }
  },
  { capture: true },
);

// Nothing is written: the composition stays and continues on return.
window.addEventListener("blur", () => {
  call((current) => current.focusLost());
});

for (const button of document.querySelectorAll("button")) {
  button.addEventListener("mousedown", (event) => event.preventDefault());
}

// `/` jumps to the textarea, as kautian-mirror's search does, from anywhere but the
// textarea and a select, which types ahead. A checkbox just clicked gives way. No
// select-all: the next key would replace the document.
document.addEventListener("keydown", (event) => {
  const { key, metaKey, ctrlKey, altKey, isComposing, target } = event;
  if (key !== "/" || metaKey || ctrlKey || altKey || isComposing) {
    return;
  }
  if (target.isContentEditable || target.closest("textarea, select")) {
    return;
  }
  event.preventDefault();
  textarea.focus();
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
