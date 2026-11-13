// The segmentation and ranking page. Same session as the writing page, and the
// same docked sidebar. `explain` is drawn on every key (`inspector.js`).

import { Editor } from "./editor.js";
import { setDisplayLanguage, translate } from "./i18n.js";
import {
  defaultSortFields,
  Inspector,
  setSortFieldOrder,
} from "./inspector.js";
import { Learning } from "./learning.js";
import { createSession, dictionaries, keepForOffline } from "./loader.js";
import { emptyDocument, parseStored, STORAGE_KEY } from "./settings.js";
import { Sidebar } from "./sidebar.js";
import { Surface } from "./surface.js";
import { TpsKeyboard } from "./tps_keyboard.js";

const NOTHING = { handled: true, emits: [] };

const editorBox = document.getElementById("editor");
const editor = new Editor(document.getElementById("text"));
const text = editor.root;
const preedit = document.getElementById("preedit");
const candidates = document.getElementById("candidates");
const pageUp = document.getElementById("page-up");
const pageDown = document.getElementById("page-down");
const notice = document.getElementById("notice");
const messageSlot = document.getElementById("message");
const progress = document.getElementById("progress");
const retryButton = document.getElementById("retry");
const loadSlot = document.getElementById("load-slot");
const form = document.getElementById("settings");

const surface = new Surface(editor, editorBox);
const SORT_KEY = "taigi.devSortOrder";
const RANK_ONE_WORD_KEY = "taigi.devRankOneWord";
const PRESET_KEY = "taigi.devSortPreset";
/// Preference, then a partial prefix behind a full syllable or an
/// abbreviation, then selection weight, then abbreviation.
/// The rest stay in the engine's order.
const PROPOSAL = [
  "tier",
  "preference_class",
  "coverage_kind",
  "user_weight",
  "abbreviation",
  "context_rank",
  "walker_cost",
  "score",
  "frequency",
  "coverage",
  "source_rank",
  "stable_idx",
];
const rankOneWord = document.getElementById("rank-one-word-input");
const presetInputs = document.querySelectorAll("[name=sortPreset]");
let applyingPreset = false;

const inspector = new Inspector(
  document.getElementById("inspector-body"),
  document.getElementById("mechanism"),
  () => call((current) => current.explain()),
  (fields) => installSort(fields),
);

new Sidebar(
  document.getElementById("sidebar-toggle"),
  document.getElementById("sidebar"),
);

let state = readDocument();
let session = null;
let said = null;

let customOpen = false;
const learning = new Learning(
  () => session,
  run,
  () => {
    customOpen = true;
    setCustomFormEnabled(true);
    refreshCustomWords();
  },
);
const customForm = document.getElementById("custom-form");
const customRoman = document.getElementById("custom-roman");
const customHanji = document.getElementById("custom-hanji");
const customSave = document.getElementById("custom-save");
const customStatus = document.getElementById("custom-status");
const customList = document.getElementById("custom-list");
const tpsKeyboard = new TpsKeyboard(
  document.getElementById("tps-keyboard"),
  (glyph) => {
    editor.focus();
    apply(call((current) => current.tpsKeyPress(glyph)));
  },
);

showControls();
rememberedPreset();
inspector.sync();

function rememberedSort() {
  try {
    const parsed = JSON.parse(sessionStorage.getItem(SORT_KEY) ?? "null");
    const canonical = defaultSortFields();
    if (
      Array.isArray(parsed) &&
      parsed.length === canonical.length &&
      canonical.every((field) => parsed.includes(field))
    ) {
      return parsed;
    }
  } catch {
    // A damaged note is the default order.
  }
  return null;
}

function rememberedRankOneWord() {
  return sessionStorage.getItem(RANK_ONE_WORD_KEY) === "1";
}

function sameOrder(left, right) {
  return (
    left.length === right.length &&
    left.every((field, index) => field === right[index])
  );
}

function showPreset(name) {
  for (const input of presetInputs) {
    input.checked = input.value === name;
  }
  try {
    sessionStorage.setItem(PRESET_KEY, name);
  } catch {
    // A private window may refuse. The engine still keeps it for this tab.
  }
}

/// A saved choice, or one inferred from an older note that only stored the
/// order and the switch.
function rememberedPreset() {
  const stored = sessionStorage.getItem(PRESET_KEY);
  if (stored === "original" || stored === "proposal" || stored === "test") {
    if (stored === "original") {
      applyPreset("original");
    } else if (stored === "proposal") {
      applyPreset("proposal");
    } else {
      const order = rememberedSort();
      if (order) {
        setSortFieldOrder(order);
      }
      rankOneWord.checked = rememberedRankOneWord();
      showPreset("test");
    }
    return;
  }
  const order = rememberedSort();
  const rank = rememberedRankOneWord();
  if (order && sameOrder(order, PROPOSAL) && rank) {
    applyPreset("proposal");
  } else if ((!order || sameOrder(order, defaultSortFields())) && !rank) {
    applyPreset("original");
  } else {
    if (order) {
      setSortFieldOrder(order);
    }
    rankOneWord.checked = rank;
    showPreset("test");
  }
}

function applyPreset(name) {
  applyingPreset = true;
  if (name === "proposal") {
    installSort(PROPOSAL);
    installRankOneWord(true);
  } else if (name === "original") {
    installSort(defaultSortFields());
    installRankOneWord(false);
  }
  applyingPreset = false;
  showPreset(name);
}

function installRankOneWord(enabled) {
  rankOneWord.checked = enabled;
  try {
    sessionStorage.setItem(RANK_ONE_WORD_KEY, enabled ? "1" : "0");
  } catch {
    // A private window may refuse. The engine still keeps it for this tab.
  }
  if (!applyingPreset) {
    showPreset("test");
  }
  if (session === null) {
    return;
  }
  apply(call((current) => current.setRankOneWord(enabled)));
}

function installSort(fields) {
  setSortFieldOrder(fields);
  try {
    sessionStorage.setItem(SORT_KEY, JSON.stringify(fields));
  } catch {
    // A private window may refuse. The engine still keeps it for this tab.
  }
  if (!applyingPreset) {
    showPreset("test");
  }
  if (session === null) {
    inspector.sync();
    return;
  }
  apply(call((current) => current.setSortOrder(JSON.stringify(fields))));
}

function readDocument() {
  try {
    return parseStored(localStorage.getItem(STORAGE_KEY));
  } catch {
    return emptyDocument();
  }
}

function controlValue(control) {
  if (control.type === "checkbox") {
    return control.checked;
  }
  return "boolean" in control.dataset
    ? control.value === "true"
    : control.value;
}

/// The lab keeps Automatic, Hanji, English and Japanese. A Tâi-lô or
/// Pe̍h-ōe-jī tag from the writing page stays in the shared document and
/// draws here as Hanji.
function pageLanguage(tag) {
  return tag === "tailo" || tag === "poj" ? "hanji" : (tag ?? "system");
}

function showControls() {
  const { values } = state;
  for (const control of form.querySelectorAll("[name]")) {
    const value = values[control.name];
    if (control.type === "checkbox") {
      control.checked =
        typeof value === "boolean" ? value : control.defaultChecked;
      continue;
    }
    const shownValue =
      control.name === "displayLanguage" ? pageLanguage(value) : value;
    const options = Array.from(control.options);
    const shown =
      (shownValue == null
        ? undefined
        : options.find((option) => option.value === String(shownValue))) ??
      options.find((option) => option.defaultSelected);
    if (shown) {
      shown.selected = true;
    }
  }
  const mode = form.elements.inputMode.value;
  form.elements.tpsKeyboardShown.disabled = mode !== "tps";
  document.documentElement.toggleAttribute(
    "data-tps-keyboard",
    mode === "tps" && form.elements.tpsKeyboardShown.checked,
  );
  setDisplayLanguage(pageLanguage(values.displayLanguage));
  customRoman.placeholder = translate("dictionary.romanPlaceholder");
  customHanji.placeholder = translate("dictionary.hanziPlaceholder");
  redrawMessage();
  refreshCustomWords();
}

function writeSetting(name, value) {
  state.values[name] = value;
  state.revision += 1;
  const json = JSON.stringify(state);
  try {
    localStorage.setItem(STORAGE_KEY, json);
  } catch {
    // A private window may refuse. The session keeps it for this tab.
  }
  showControls();
  apply(call((current) => current.commitComposition()));
  call((current) => current.setSettings(json));
  inspector.sync();
}

form.addEventListener("change", ({ target }) => {
  if (target.name) {
    writeSetting(target.name, controlValue(target));
  }
});

function platform() {
  const name = navigator.userAgentData?.platform ?? navigator.platform;
  if (/mac|iphone|ipad/i.test(name)) {
    return "macos";
  }
  return /win/i.test(name) ? "windows" : "linux";
}

function say(key, status, placeholders) {
  said = [key, placeholders];
  messageSlot.textContent = translate(key, placeholders);
  messageSlot.dataset.state = status;
}

/// A mode announcement arrives already translated, so `said` stays empty and
/// the line is left as the session wrote it.
function redrawMessage() {
  if (said !== null) {
    messageSlot.textContent = translate(...said);
  }
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
  if (error instanceof WebAssembly.RuntimeError) {
    session = null;
    clearComposition();
  }
  say("web.loadFailed", "failed");
  loadSlot.dataset.show = "retry";
  editor.setEditable(session !== null);
}

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
  inspector.sync();
}

function restoreSettings(target) {
  try {
    target.setSettings(JSON.stringify(state));
  } catch (error) {
    if (error instanceof WebAssembly.RuntimeError) {
      throw error;
    }
    console.error(error);
    state = emptyDocument();
    showControls();
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
    } catch {
      // Blocked storage.
    }
  }
}

async function start() {
  let starting = null;
  showLoading();
  try {
    starting = await createSession(platform(), navigator.language);
    session = starting;
    const preset = sessionStorage.getItem(PRESET_KEY);
    if (preset === "proposal") {
      call((current) => current.setSortOrder(JSON.stringify(PROPOSAL)));
      call((current) => current.setRankOneWord(true));
    } else if (preset === "test") {
      const order = rememberedSort();
      if (order) {
        call((current) => current.setSortOrder(JSON.stringify(order)));
      }
      if (rememberedRankOneWord()) {
        call((current) => current.setRankOneWord(true));
      }
    } else {
      call((current) => current.setRankOneWord(false));
    }
    restoreSettings(starting);
    learning.attach(starting);
    tpsKeyboard.draw(JSON.parse(starting.tpsKeyboardRows()));
    editor.setEditable(true);
    editor.focus();
  } catch (error) {
    failLoading(error, starting);
    return;
  }
  await loadDictionaries(starting);
}

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
    clearComposition();
    start();
    return undefined;
  }
}

function call(action) {
  const reply = run(action);
  return reply === undefined ? NOTHING : JSON.parse(reply);
}

function showPreedit(text, caret) {
  const characters = Array.from(text);
  const mark = document.createElement("span");
  mark.textContent = "|";
  preedit.replaceChildren(
    characters.slice(0, caret).join(""),
    mark,
    characters.slice(caret).join(""),
  );
  preedit.hidden = text.length === 0;
}

function clearPreedit() {
  preedit.replaceChildren();
  preedit.hidden = true;
}

function showTable(table) {
  const { cells, labels, cursor, cursorVisible, pageSize } = table;
  const start = Math.floor(cursor / pageSize) * pageSize;
  const shown = cells.slice(start, start + pageSize);
  candidates.replaceChildren(
    ...shown.map((cell, index) => {
      const item = document.createElement("li");
      const button = document.createElement("button");
      button.type = "button";
      button.className = "choice";
      if (cursorVisible && start + index === cursor) {
        button.setAttribute("aria-current", "true");
      }
      if (labels[index]) {
        button.append(span(labels[index], "index"));
      }
      button.append(span(cell.text, "choice-text"));
      if (cell.annotation) {
        button.append(span(cell.annotation, "annotation"));
      }
      button.addEventListener("click", () => {
        apply(call((current) => current.clickCandidate(index)));
      });
      item.append(button);
      return item;
    }),
  );
  pageUp.hidden = start === 0;
  pageDown.hidden = start + pageSize >= cells.length;
}

function span(text, className) {
  const node = document.createElement("span");
  node.className = className;
  node.textContent = text;
  return node;
}

function clearComposition() {
  clearPreedit();
  candidates.replaceChildren();
  pageUp.hidden = true;
  pageDown.hidden = true;
}

function applyEmit(emit) {
  switch (emit.type) {
    case "preedit":
      showPreedit(emit.text, emit.caret);
      break;
    case "clearPreedit":
      clearPreedit();
      break;
    case "commit":
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
      showTable(emit);
      break;
    case "hideLookupTable":
      candidates.replaceChildren();
      pageUp.hidden = true;
      pageDown.hidden = true;
      break;
    case "announceMode":
      said = null;
      messageSlot.textContent = emit.label;
      messageSlot.dataset.state = "mode";
      break;
    default:
      break;
  }
}

function apply(reply) {
  if (typeof reply.settings === "string") {
    state = JSON.parse(reply.settings);
    try {
      localStorage.setItem(STORAGE_KEY, reply.settings);
    } catch {
      // Blocked storage.
    }
    showControls();
  }
  if (reply.emits.length > 0) {
    for (const emit of reply.emits) {
      applyEmit(emit);
    }
  }
  inspector.sync();
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

function isForeignComposition(event) {
  return event.isComposing || event.keyCode === 229;
}

// Capture on `#editor`: the engine has the key before Lexical, whose handlers
// on the root skip a key the engine took (`Editor`, KEY_DOWN_COMMAND).
editorBox.addEventListener(
  "keydown",
  (event) => {
    if (!text.contains(event.target) || isForeignComposition(event)) {
      return;
    }
    if (surface.takeForeignChange()) {
      call((current) => current.caretMoved());
    }
    const reply = call((current) => current.keyDown(keyJson(event)));
    if (reply.handled) {
      event.preventDefault();
    }
    apply(reply);
  },
  { capture: true },
);

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

pageUp.addEventListener("click", () => {
  apply(call((current) => current.turnPage(false)));
});

pageDown.addEventListener("click", () => {
  apply(call((current) => current.turnPage(true)));
});

document.addEventListener(
  "pointerdown",
  (event) => {
    const keeps = event.target.closest(
      "#text, #candidates, #tps-keyboard, #inspector-body, #page-up, #page-down, #sidebar, #sidebar-toggle, #rank-one-word, #clear-learning, #custom-words",
    );
    if (event.target.closest("#text")) {
      call((current) => current.caretMoved());
    }
    if (!keeps) {
      apply(call((current) => current.commitComposition()));
    }
  },
  { capture: true },
);

window.addEventListener("blur", () => {
  call((current) => current.focusLost());
});

for (const button of document.querySelectorAll("button")) {
  button.addEventListener("mousedown", (event) => event.preventDefault());
}

retryButton.addEventListener("click", () => {
  if (session === null) {
    start();
    return;
  }
  showLoading();
  loadDictionaries(session);
});

document.getElementById("clear-learning").addEventListener("click", () => {
  try {
    apply(call((current) => current.clearLearningRecords()));
    learning.scheduleSave();
  } catch (error) {
    console.error(error);
  }
  editor.focus();
});

function setCustomFormEnabled(enabled) {
  customRoman.disabled = !enabled;
  customHanji.disabled = !enabled;
  customSave.disabled = !enabled;
}

function refreshCustomWords() {
  if (!customOpen || session === null) {
    return;
  }
  let listing;
  try {
    listing = call((current) => current.listCustomEntries());
  } catch (error) {
    customStatus.textContent = String(error);
    return;
  }
  customList.replaceChildren(
    ...listing.entries.map((entry) => {
      const item = document.createElement("li");
      const roman = document.createElement("span");
      roman.textContent = entry.roman;
      const hanji = document.createElement("span");
      hanji.textContent = entry.hanji;
      const remove = document.createElement("button");
      remove.type = "button";
      remove.dataset.id = entry.id;
      remove.textContent = translate("common.delete");
      item.append(roman, hanji, remove);
      return item;
    }),
  );
}

customForm.addEventListener("submit", (event) => {
  event.preventDefault();
  customStatus.textContent = "";
  try {
    apply(
      call((current) =>
        current.saveCustomEntry(customRoman.value, customHanji.value),
      ),
    );
    customRoman.value = "";
    customHanji.value = "";
    learning.scheduleSave();
    refreshCustomWords();
  } catch (error) {
    customStatus.textContent = String(error);
  }
  customRoman.focus();
});

customList.addEventListener("click", (event) => {
  const button = event.target.closest("button");
  if (!button) {
    return;
  }
  customStatus.textContent = "";
  try {
    apply(call((current) => current.deleteCustomEntry(button.dataset.id)));
    learning.scheduleSave();
    refreshCustomWords();
  } catch (error) {
    customStatus.textContent = String(error);
  }
});

document
  .getElementById("custom-words")
  .addEventListener("mousedown", (event) => {
    if (event.target.closest("button")) {
      event.preventDefault();
    }
  });

rankOneWord.addEventListener("change", () => {
  installRankOneWord(rankOneWord.checked);
  editor.focus();
});

for (const input of presetInputs) {
  input.addEventListener("change", () => {
    if (!input.checked) {
      return;
    }
    if (input.value === "test") {
      showPreset("test");
    } else {
      applyPreset(input.value);
    }
    editor.focus();
  });
}

start();
