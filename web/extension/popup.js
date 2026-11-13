// The toolbar popup: what the extension is doing in the tab in front, and the
// controls for it. Opening it is the user's gesture the browser wants before
// it asks about a site.

import { setDisplayLanguage, translate } from "./i18n.js";
import { parseStored, STORAGE_KEY } from "./settings.js";

const site = document.getElementById("site");
const status = document.getElementById("status");
const buttons = {
  enable: document.getElementById("enable"),
  turnOn: document.getElementById("turn-on"),
  turnOff: document.getElementById("turn-off"),
  remove: document.getElementById("remove"),
};

function ask(message) {
  return chrome.runtime.sendMessage(message);
}

/// The tab the popup is about: the one in front, or the one `?tab=` names
/// (the tests open the popup as a page, which is then the tab in front).
async function shownTab() {
  const named = new URLSearchParams(location.search).get("tab");
  if (named !== null) {
    return Number(named);
  }
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  return tab.id;
}

const tabId = await shownTab();

/// What the service worker last read, for the buttons' handlers.
let shown = null;

async function draw() {
  shown = await ask({ type: "taigi.popup.read", tabId });
  const { hasAccess, mode, state, label } = shown;
  const isSite = shown.site !== null;
  site.textContent = shown.site ?? "";
  if (!isSite) {
    status.textContent = translate("web.extensionUnavailable");
  } else if (!hasAccess) {
    status.textContent = translate("web.extensionNoAccess");
  } else {
    status.textContent = label || translate("web.extensionOff");
  }
  status.dataset.state = isSite && hasAccess ? state : "none";
  buttons.enable.hidden = !isSite || hasAccess;
  buttons.turnOn.hidden = !isSite || !hasAccess || mode !== "off";
  buttons.turnOff.hidden = !isSite || !hasAccess || mode === "off";
  buttons.remove.hidden = !isSite || !hasAccess;
}

/// On or off in the tab in front. The popup closes: the next key is the page's.
async function setTyping(isOn) {
  await ask({ type: "taigi.popup.setTyping", tabId, isOn });
  window.close();
}

buttons.enable.addEventListener("click", async () => {
  // The browser's own question. Enabling a site is asking to type there.
  if (await chrome.permissions.request({ origins: [shown.pattern] })) {
    await setTyping(true);
  }
});

buttons.turnOn.addEventListener("click", () => setTyping(true));
buttons.turnOff.addEventListener("click", () => setTyping(false));

buttons.remove.addEventListener("click", async () => {
  if (shown.mode !== "off") {
    await ask({ type: "taigi.popup.setTyping", tabId, isOn: false });
  }
  await chrome.permissions.remove({ origins: [shown.pattern] });
  await draw();
});

document.getElementById("settings").addEventListener("click", () => {
  chrome.runtime.openOptionsPage();
});

// The tab says a new state while the popup is open (loading, then the mode).
chrome.storage.session.onChanged.addListener(draw);

const { [STORAGE_KEY]: stored } = await chrome.storage.sync.get(STORAGE_KEY);
setDisplayLanguage(parseStored(stored).values.displayLanguage ?? "system");
await draw();
