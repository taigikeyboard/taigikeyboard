// The options page of the ChromeOS input method and the browser extension: the
// web page's settings form over the settings document in chrome.storage.sync,
// which the worker reads too.

import init, { Session } from "./pkg/taigi_web.js";
import { hostPlatform } from "./platform.js";
import {
  emptyDocument,
  STORAGE_KEY as KEY,
  parseStored,
  Settings,
} from "./settings.js";
import { drawShortcuts } from "./shortcuts.js";

/** @typedef {import("../types/shared.d.ts").WireShortcuts} WireShortcuts */
/** @typedef {import("../types/shared.d.ts").WireShortcutRow} WireShortcutRow */

// The browser extension's own chord (its `manifest.json` `commands`), kept by
// the browser outside the settings document; ChromeOS declares none.
const TOGGLE_COMMAND = "toggle-typing";
const DEFAULT_SETTINGS = JSON.stringify(emptyDocument());

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
  followSettings(json).catch(shortcutsFailed);
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
    followSettings(change.newValue).catch(shortcutsFailed);
  }
});

/**
 * A session of this page's own, with no dictionaries, for the shortcut list:
 * it follows the document, so it never waits for the input method to run.
 * Keys are read as the visitor's machine, which on ChromeOS is "linux", as
 * the worker's `PLATFORM`. A document it refuses gives way to the defaults,
 * as on the page (`ime.js` `restoreSettings`).
 * @returns {Promise<Session>}
 */
async function startSession() {
  await init();
  if (typeof stored === "string") {
    try {
      return new Session(stored, hostPlatform(), navigator.language);
    } catch (error) {
      console.warn("settings.refused", errorMessage(error));
      settings.reset();
    }
  }
  return new Session(DEFAULT_SETTINGS, hostPlatform(), navigator.language);
}

// The form never waits for the engine: a failed load leaves only the
// Shortcuts section empty.
const starting = startSession();

/**
 * @param {unknown} error
 * @returns {void}
 */
function shortcutsFailed(error) {
  console.warn("shortcuts.draw.failed", errorMessage(error));
}

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
 * Draws the session's shortcut list, with no Telex guide or TPS key panel
 * rows (neither package draws those windows), the browser's own rows last in
 * its last block (Other, `shortcuts.rs`).
 * @returns {Promise<void>}
 */
async function showShortcuts() {
  if (shortcuts === null) {
    return;
  }
  const [session, commandRows] = await Promise.all([
    starting,
    browserCommandRows(),
  ]);
  const { groups } = /** @type {WireShortcuts} */ (
    JSON.parse(session.shortcuts(/* hasPageWindows */ false))
  );
  groups.at(-1)?.rows.push(...commandRows);
  drawShortcuts(shortcuts, groups);
}

/**
 * Tone Keys, the input script and the language change the list.
 * @param {string} json
 * @returns {Promise<void>}
 */
async function followSettings(json) {
  const session = await starting;
  try {
    session.setSettings(json);
  } catch (error) {
    console.warn("settings.refused", errorMessage(error));
    return;
  }
  await showShortcuts();
}

showShortcuts().catch(shortcutsFailed);
// The browser announces no rebound command: read it again on return from
// chrome://extensions/shortcuts.
document.addEventListener("visibilitychange", () => {
  if (!document.hidden) {
    showShortcuts().catch(shortcutsFailed);
  }
});

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
