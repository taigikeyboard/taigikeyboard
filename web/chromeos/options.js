// The ChromeOS options page: the web page's settings form over the settings
// document in chrome.storage.sync, which the worker reads too.

import { STORAGE_KEY as KEY, parseStored, Settings } from "./settings.js";
import { drawShortcuts } from "./shortcuts.js";

/** @typedef {import("../types/shared.d.ts").WireShortcuts} WireShortcuts */

// The ChromeOS worker's session writes its shortcut list here (the same key
// in `worker.js`); the browser extension's cut hides the section.
const SHORTCUTS_KEY = "taigi.shortcuts";

const { [KEY]: stored } = await chrome.storage.sync.get(KEY);
// The last document this page wrote or adopted: an equal one is no change.
/** @type {unknown} */
let last = stored;

/**
 * Formats an unknown caught error into a loggable message string.
 * @param {unknown} error
 * @returns {string}
 */
function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

/**
 * Persists a serialized settings document if it differs from the last written one.
 * @param {string} json
 * @returns {void}
 */
function save(json) {
  if (json === last) {
    return;
  }
  last = json;
  chrome.storage.sync
    .set({ [KEY]: json })
    .catch((/** @type {unknown} */ error) =>
      console.warn("settings.save.failed", errorMessage(error)),
    );
}

// Every form change goes through the store's write.
const form = /** @type {HTMLFormElement} */ (
  document.getElementById("settings")
);
const settings = new Settings(form, () => {}, {
  read: () => parseStored(stored),
  write: save,
});

// A chord wrote the document while this page was open (the worker saves it).
chrome.storage.onChanged.addListener((changes, area) => {
  const change = changes[KEY];
  if (area !== "sync" || typeof change?.newValue !== "string") {
    return;
  }
  if (change.newValue !== last) {
    last = change.newValue;
    settings.adopt(change.newValue);
  }
});

const shortcuts = document.getElementById("shortcuts");

/**
 * Draws the worker's shortcut list; none yet (the input method has not run)
 * leaves the section empty.
 * @param {HTMLElement} root
 * @param {unknown} json
 * @returns {void}
 */
function showShortcuts(root, json) {
  if (typeof json !== "string") {
    return;
  }
  const { groups } = /** @type {WireShortcuts} */ (JSON.parse(json));
  if (groups) {
    drawShortcuts(root, groups);
  }
}

if (shortcuts !== null && shortcuts.closest("[hidden]") === null) {
  const { [SHORTCUTS_KEY]: published } =
    await chrome.storage.local.get(SHORTCUTS_KEY);
  showShortcuts(shortcuts, published);
  // Tone Keys, the input script and the language change the list.
  chrome.storage.onChanged.addListener((changes, area) => {
    if (area === "local") {
      showShortcuts(shortcuts, changes[SHORTCUTS_KEY]?.newValue);
    }
  });
}
