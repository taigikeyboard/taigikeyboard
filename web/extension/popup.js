// The toolbar popup: what the extension is doing in the tab in front, and the
// controls for it. Opening it is the user's gesture the browser wants before
// it asks about a site.

import { setDisplayLanguage, translate } from "./i18n.js";
import { parseStored, STORAGE_KEY } from "./settings.js";

/** @typedef {import("./background.js").BadgeState} BadgeState */
/** @typedef {import("./background.js").TabMode} TabMode */
/**
 * @typedef {(
 *   | { site: null }
 *   | {
 *       site: string,
 *       pattern: string,
 *       hasAccess: boolean,
 *       mode: TabMode,
 *       state: BadgeState,
 *       label: string,
 *     }
 * )} PopupReadReply
 */

const site = /** @type {HTMLElement} */ (document.getElementById("site"));
const status = /** @type {HTMLElement} */ (document.getElementById("status"));
const buttons = {
  enable: /** @type {HTMLButtonElement} */ (document.getElementById("enable")),
  turnOn: /** @type {HTMLButtonElement} */ (document.getElementById("turn-on")),
  turnOff: /** @type {HTMLButtonElement} */ (
    document.getElementById("turn-off")
  ),
  remove: /** @type {HTMLButtonElement} */ (document.getElementById("remove")),
};

/**
 * Sends a message to the extension service worker and returns its response.
 * @param {unknown} message
 * @returns {Promise<unknown>}
 */
function ask(message) {
  return chrome.runtime.sendMessage(message);
}

/**
 * The tab the popup is about: the one in front, or the one `?tab=` names
 * (the tests open the popup as a page, which is then the tab in front).
 * @returns {Promise<number>}
 */
async function shownTab() {
  const named = new URLSearchParams(location.search).get("tab");
  if (named !== null) {
    return Number(named);
  }
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  if (tab?.id === undefined) {
    throw new Error("no active tab");
  }
  return tab.id;
}

const tabId = await shownTab();

/**
 * What the service worker last read, for the buttons' handlers.
 * @type {PopupReadReply | null}
 */
let shown = null;

/**
 * Fetches current tab status and updates the popup DOM controls.
 * @returns {Promise<void>}
 */
async function draw() {
  shown = /** @type {PopupReadReply} */ (
    await ask({ type: "taigi.popup.read", tabId })
  );
  if (shown.site === null) {
    site.textContent = "";
    status.textContent = translate("web.extensionUnavailable");
    status.dataset.state = "none";
    buttons.enable.hidden = true;
    buttons.turnOn.hidden = true;
    buttons.turnOff.hidden = true;
    buttons.remove.hidden = true;
    return;
  }
  const { hasAccess, mode, state, label } = shown;
  site.textContent = shown.site;
  if (!hasAccess) {
    status.textContent = translate("web.extensionNoAccess");
  } else {
    status.textContent = label || translate("web.extensionOff");
  }
  status.dataset.state = hasAccess ? state : "none";
  buttons.enable.hidden = hasAccess;
  buttons.turnOn.hidden = !hasAccess || mode !== "off";
  buttons.turnOff.hidden = !hasAccess || mode === "off";
  buttons.remove.hidden = !hasAccess;
}

/**
 * On or off in the tab in front. The popup closes: the next key is the page's.
 * @param {boolean} isOn
 * @returns {Promise<void>}
 */
async function setTyping(isOn) {
  await ask({ type: "taigi.popup.setTyping", tabId, isOn });
  window.close();
}

buttons.enable.addEventListener("click", async () => {
  if (!shown || shown.site === null) {
    return;
  }
  // The browser's own question. Enabling a site is asking to type there.
  if (await chrome.permissions.request({ origins: [shown.pattern] })) {
    await setTyping(true);
  }
});

buttons.turnOn.addEventListener("click", () => setTyping(true));
buttons.turnOff.addEventListener("click", () => setTyping(false));

buttons.remove.addEventListener("click", async () => {
  if (!shown || shown.site === null) {
    return;
  }
  if (shown.mode !== "off") {
    await ask({ type: "taigi.popup.setTyping", tabId, isOn: false });
  }
  await chrome.permissions.remove({ origins: [shown.pattern] });
  await draw();
});

document.getElementById("settings")?.addEventListener("click", () => {
  chrome.runtime.openOptionsPage();
});

/**
 * Displays the active `toggle-typing` keyboard shortcut inside the popup's
 * toggle buttons when the browser has one assigned.
 * @returns {Promise<void>}
 */
async function drawShortcut() {
  let shortcut = "";
  try {
    const commands = await chrome.commands.getAll();
    const toggle = commands.find((command) => command.name === "toggle-typing");
    shortcut = toggle?.shortcut?.trim() ?? "";
  } catch {
    shortcut = "";
  }
  for (const element of document.querySelectorAll(".popup-shortcut")) {
    if (element instanceof HTMLElement) {
      element.textContent = shortcut;
      element.hidden = shortcut === "";
    }
  }
}

// The tab says a new state while the popup is open (loading, then the mode).
chrome.storage.session.onChanged.addListener(() => {
  draw();
});

const { [STORAGE_KEY]: stored } = await chrome.storage.sync.get(STORAGE_KEY);
const displayLanguage = parseStored(stored).values.displayLanguage;
setDisplayLanguage(
  typeof displayLanguage === "string" ? displayLanguage : "system",
);
await Promise.all([draw(), drawShortcut()]);
