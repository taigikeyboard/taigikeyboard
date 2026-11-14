// The ChromeOS options page: the web page's settings form over the settings
// document in chrome.storage.sync, which the worker reads too.

import { STORAGE_KEY as KEY, parseStored, Settings } from "./settings.js";

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
