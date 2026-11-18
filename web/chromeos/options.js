// The options page of the ChromeOS input method and the browser extension: the
// web page's settings form over the settings document in chrome.storage.sync,
// which the worker reads too.

import {
  STORAGE_KEY as KEY,
  parseStored,
  Settings,
  SHORTCUTS_KEY,
} from "./settings.js";
import { drawShortcuts } from "./shortcuts.js";

/** @typedef {import("../types/shared.d.ts").WireShortcuts} WireShortcuts */
/** @typedef {import("../types/shared.d.ts").WireShortcutRow} WireShortcutRow */

// The browser extension's own chord (its `manifest.json` `commands`), kept by
// the browser outside the settings document; ChromeOS declares none.
const TOGGLE_COMMAND = "toggle-typing";

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
 * The browser extension's toggle as the browser's shortcuts page shows it,
 * in the browser's language; none on ChromeOS.
 * @returns {Promise<WireShortcutRow[]>}
 */
async function browserCommandRows() {
  const commands = (await chrome.commands?.getAll()) ?? [];
  return commands
    .filter((command) => command.name === TOGGLE_COMMAND)
    .map((command) => ({
      label: command.description ?? "",
      keys: command.shortcut?.trim() ?? "",
    }));
}

/**
 * Draws the session's shortcut list, the browser's own rows last in its last
 * block (Other, `shortcuts.rs`); none yet (the input method has not run)
 * leaves the section empty.
 * @param {HTMLElement} root
 * @returns {Promise<void>}
 */
async function showShortcuts(root) {
  const [{ [SHORTCUTS_KEY]: published }, commandRows] = await Promise.all([
    chrome.storage.local.get(SHORTCUTS_KEY),
    browserCommandRows(),
  ]);
  if (typeof published !== "string") {
    return;
  }
  const { groups } = /** @type {WireShortcuts} */ (JSON.parse(published));
  if (groups) {
    groups.at(-1)?.rows.push(...commandRows);
    drawShortcuts(root, groups);
  }
}

if (shortcuts !== null) {
  await showShortcuts(shortcuts);
  // Tone Keys, the input script and the language change the list.
  chrome.storage.onChanged.addListener((changes, area) => {
    if (area === "local" && SHORTCUTS_KEY in changes) {
      showShortcuts(shortcuts);
    }
  });
  // The browser announces no rebound command: read it again on return from
  // chrome://extensions/shortcuts.
  document.addEventListener("visibilitychange", () => {
    if (!document.hidden) {
      showShortcuts(shortcuts);
    }
  });
}

// The side menu marks the section in view, as the settings page marks its
// page: the last one whose heading has reached the window's top. A menu
// link scrolls its section to 16px (`scroll-margin-top` in options.css),
// inside this reach, and the last section is long enough to get there.
const SECTION_REACH = 80;
const menu = /** @type {HTMLAnchorElement[]} */ ([
  ...document.querySelectorAll(".options-nav a"),
]).map((link) => ({
  link,
  section: /** @type {HTMLElement} */ (
    document.getElementById(link.hash.slice(1))
  ),
}));
/** @type {HTMLAnchorElement | null} */
let marked = null;

/** @returns {void} */
function markSection() {
  let current = menu[0].link;
  for (const { link, section } of menu) {
    if (section.getBoundingClientRect().top <= SECTION_REACH) {
      current = link;
    }
  }
  if (current === marked) {
    return;
  }
  marked = current;
  for (const { link } of menu) {
    link.setAttribute("aria-current", String(link === current));
  }
}

if (menu.length > 0) {
  window.addEventListener("scroll", markSection, { passive: true });
  markSection();
}
