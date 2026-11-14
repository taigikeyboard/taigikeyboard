// The extension's service worker: which sites the content script runs on,
// each tab's typing mode, what the toolbar button shows, and the profile's
// learned data (`learning-store.js`). It holds no engine; each tab runs its
// own (`typing.js`).

import { readCopy, writeCopy } from "./learning-store.js";

/** @typedef {import("./learning-store.js").EncodedFiles} EncodedFiles */
/** @typedef {"off" | "taigi" | "english"} TabMode */
/** @typedef {"off" | "loading" | "taigi" | "english" | "unsupported" | "failed"} BadgeState */
/**
 * @typedef {{
 *   site: string,
 *   mode: TabMode,
 *   state: BadgeState,
 *   label: string,
 * }} TabState
 */

/** The registered content script's id. */
const SCRIPT_ID = "taigi";
const REMOVE_SITE_MENU_ID = "remove-site";

/**
 * The toolbar button per state the content script tells. A tab the script
 * does not run in has the manifest's title, no badge and the grey icon.
 * @type {Record<BadgeState, { text: string, color?: string }>}
 */
const BADGES = {
  off: { text: "" },
  loading: { text: "…", color: "#6e6e73" },
  taigi: { text: "台", color: "#0071e3" },
  english: { text: "A", color: "#1c1c1e" },
  unsupported: { text: "×", color: "#6e6e73" },
  failed: { text: "!", color: "#d70015" },
};

const ICON_SIZES = [16, 32];

const NAME = chrome.runtime.getManifest().short_name ?? "";

/**
 * Returns the active keyboard shortcut for `toggle-typing`, or `""` when unset.
 * @returns {Promise<string>}
 */
async function toggleShortcut() {
  try {
    const commands = await chrome.commands.getAll();
    const toggle = commands.find((command) => command.name === "toggle-typing");
    return toggle?.shortcut?.trim() ?? "";
  } catch {
    return "";
  }
}

/**
 * The pattern the browser's site access takes for `hostname`.
 * @param {string} hostname
 * @returns {string}
 */
function patternOf(hostname) {
  return `*://${hostname}/*`;
}

/**
 * Sites whose pages the browser lets no extension into, even though their
 * URLs are `https:`.
 * @param {string} hostname
 * @param {string} pathname
 * @returns {boolean}
 */
function isRestrictedSite(hostname, pathname) {
  return (
    hostname === "chromewebstore.google.com" ||
    (hostname === "chrome.google.com" && /^\/webstore(\/|$)/.test(pathname))
  );
}

/**
 * The host of `url` when it is a website the extension may run on, else
 * `null`: a browser page, the web store, a file, or a URL the extension may
 * not see.
 * @param {string | undefined} url
 * @returns {string | null}
 */
function siteOf(url) {
  if (!url) {
    return null;
  }
  try {
    const { protocol, hostname, pathname } = new URL(url);
    if (protocol !== "http:" && protocol !== "https:") {
      return null;
    }
    return isRestrictedSite(hostname, pathname) ? null : hostname;
  } catch {
    return null;
  }
}

/**
 * Checks whether `tab` currently has granted host permission for its site.
 * @param {chrome.tabs.Tab | undefined} tab
 * @returns {Promise<boolean>}
 */
async function tabHasAccess(tab) {
  const site = siteOf(tab?.url);
  if (site === null) {
    return false;
  }
  return chrome.permissions.contains({ origins: [patternOf(site)] });
}

// A tab's state, `{ site, mode, state, label }`, in session storage: it lasts
// the browser session and outlives this worker. `mode` is what the tab types
// in (`"off"`, `"taigi"`, `"english"`); `state` and `label` are what the
// content script last said of it.
/**
 * Builds the session-storage key for a tab ID.
 * @param {number} tabId
 * @returns {string}
 */
const tabKey = (tabId) => `tab.${tabId}`;

/**
 * Reads a tab's stored state from session storage, or `null` when absent.
 * @param {number} tabId
 * @returns {Promise<TabState | null>}
 */
async function readTab(tabId) {
  const key = tabKey(tabId);
  const result = await chrome.storage.session.get(key);
  return /** @type {TabState | undefined} */ (result[key]) ?? null;
}

/**
 * Writes a tab's state into session storage.
 * @param {number} tabId
 * @param {TabState} state
 * @returns {Promise<void>}
 */
function writeTab(tabId, state) {
  return chrome.storage.session.set({ [tabKey(tabId)]: state });
}

/**
 * Removes a tab's state from session storage.
 * @param {number} tabId
 * @returns {Promise<void>}
 */
function forgetTab(tabId) {
  return chrome.storage.session.remove(tabKey(tabId));
}

/**
 * The icon in grey, for a tab the extension does not run in.
 * @param {number} size
 * @returns {Promise<ImageData>}
 */
async function greyIcon(size) {
  const response = await fetch(chrome.runtime.getURL(`icons/${size}.png`));
  const bitmap = await createImageBitmap(await response.blob());
  const canvas = new OffscreenCanvas(size, size);
  const context = canvas.getContext("2d");
  if (!context) {
    throw new Error("2d canvas context unavailable");
  }
  context.drawImage(bitmap, 0, 0, size, size);
  const image = context.getImageData(0, 0, size, size);
  const { data } = image;
  for (let at = 0; at < data.length; at += 4) {
    const grey = 0.299 * data[at] + 0.587 * data[at + 1] + 0.114 * data[at + 2];
    data[at] = grey;
    data[at + 1] = grey;
    data[at + 2] = grey;
    data[at + 3] *= 0.6;
  }
  return image;
}

/** @type {Promise<{ [size: string]: ImageData }> | null} */
let greyIcons = null;

/**
 * Returns the cached map of greyscaled toolbar icons keyed by pixel size.
 * @returns {Promise<{ [size: string]: ImageData }>}
 */
function defaultIconData() {
  greyIcons ??= Promise.all(ICON_SIZES.map(greyIcon)).then((images) =>
    Object.fromEntries(ICON_SIZES.map((size, index) => [size, images[index]])),
  );
  return greyIcons;
}

/**
 * Every tab starts grey; a tab the content script answers from is drawn in
 * colour (`draw`). The browser forgets this with the worker, so each start
 * sets it again.
 * @returns {Promise<void>}
 */
async function drawDefaultIcon() {
  await chrome.action.setIcon({ imageData: await defaultIconData() });
}

/**
 * Returns `tabId`'s toolbar button to the state of a page the extension does
 * not run on.
 * @param {number} tabId
 * @returns {Promise<void>}
 */
async function resetToolbar(tabId) {
  try {
    const imageData = await defaultIconData();
    await Promise.all([
      chrome.action.setPopup({ tabId, popup: "popup.html" }),
      chrome.action.setIcon({ tabId, imageData }),
      chrome.action.setBadgeText({ tabId, text: "" }),
      chrome.action.setTitle({
        tabId,
        title: chrome.i18n.getMessage("title"),
      }),
    ]);
  } catch {
    // The tab is gone.
  }
}

/** @type {{ [size: string]: string }} */
const COLOUR_ICON = Object.fromEntries(
  ICON_SIZES.map((size) => [size, `icons/${size}.png`]),
);

/**
 * The toolbar button of `tabId` as `state` and `label` say. A tab closed
 * meanwhile has no button to draw.
 * @param {number} tabId
 * @param {BadgeState} state
 * @param {string} label
 * @returns {Promise<void>}
 */
async function draw(tabId, state, label) {
  const badge = BADGES[state] ?? BADGES.off;
  const shortcut = await toggleShortcut();
  const suffix = shortcut ? ` (${shortcut})` : "";
  try {
    await Promise.all([
      chrome.action.setPopup({ tabId, popup: "" }),
      chrome.action.setIcon({ tabId, path: COLOUR_ICON }),
      chrome.action.setBadgeText({ tabId, text: badge.text }),
      chrome.action.setTitle({
        tabId,
        // A tab that never typed has loaded no strings to name its state in.
        title: `${NAME}: ${label || chrome.i18n.getMessage(state)}${suffix}`,
      }),
      badge.color &&
        chrome.action.setBadgeBackgroundColor({ tabId, color: badge.color }),
    ]);
  } catch {
    // The tab is gone.
  }
}

// One registration, for every site the user gave: it follows the permission.
let registering = Promise.resolve();

/**
 * Synchronizes the registered content script match patterns with granted origins.
 * @returns {Promise<void>}
 */
function syncRegistration() {
  registering = registering
    .then(async () => {
      const { origins = [] } = await chrome.permissions.getAll();
      const matches = origins.filter((origin) => /^(\*|https?):/.test(origin));
      const registered = await chrome.scripting.getRegisteredContentScripts({
        ids: [SCRIPT_ID],
      });
      if (matches.length === 0) {
        if (registered.length > 0) {
          await chrome.scripting.unregisterContentScripts({
            ids: [SCRIPT_ID],
          });
        }
        return;
      }
      /** @type {chrome.scripting.RegisteredContentScript} */
      const script = {
        id: SCRIPT_ID,
        js: ["content.js"],
        matches,
        runAt: "document_start",
        allFrames: false,
        persistAcrossSessions: true,
      };
      if (registered.length > 0) {
        await chrome.scripting.updateContentScripts([script]);
      } else {
        await chrome.scripting.registerContentScripts([script]);
      }
    })
    .catch((error) => console.error("registration.failed", error));
  return registering;
}

/**
 * Turns typing on or off in `tab`. The content script answers when it is
 * there; a page loaded before its site was enabled gets it now. Throws on a
 * page the browser lets no extension into.
 * @param {chrome.tabs.Tab} tab
 * @param {boolean} isOn
 * @returns {Promise<void>}
 */
async function setTyping(tab, isOn) {
  const site = siteOf(tab.url);
  const tabId = tab.id;
  if (site === null || tabId === undefined) {
    throw new Error("no access to this tab");
  }
  if (!isOn) {
    await writeTab(tabId, { site, mode: "off", state: "off", label: "" });
    await draw(tabId, "off", "");
    await chrome.tabs
      .sendMessage(tabId, { type: "taigi.setMode", mode: "off" })
      .catch(() => {});
    return;
  }
  const stored = await readTab(tabId);
  // Back on in the mode it was turned off from is not kept: on is Taigi.
  await writeTab(tabId, {
    site,
    mode: "taigi",
    state: stored?.site === site ? stored.state : "off",
    label: stored?.site === site ? stored.label : "",
  });
  try {
    await chrome.tabs.sendMessage(tabId, {
      type: "taigi.setMode",
      mode: "taigi",
    });
  } catch {
    // No content script yet: its hello reads the mode just written.
    try {
      await chrome.scripting.executeScript({
        target: { tabId },
        files: ["content.js"],
      });
    } catch (error) {
      await forgetTab(tabId);
      throw error;
    }
  }
}

/**
 * Draws the enabled toolbar state (`popup: ""`, colour icon, shortcut title)
 * on every already-open tab whose site has granted host access.
 * @returns {Promise<void>}
 */
async function syncGrantedTabs() {
  const tabs = await chrome.tabs.query({});
  await Promise.all(
    tabs.map(async (tab) => {
      if (tab.id === undefined) {
        return;
      }
      const site = siteOf(tab.url);
      if (site === null) {
        return;
      }
      const hasAccess = await chrome.permissions.contains({
        origins: [patternOf(site)],
      });
      if (!hasAccess) {
        return;
      }
      const stored = await readTab(tab.id);
      const sameSite = stored?.site === site;
      await draw(
        tab.id,
        sameSite ? stored.state : "off",
        sameSite ? stored.label : "",
      );
    }),
  );
}

/**
 * Stops typing and clears the toolbar button in any open tab whose site no
 * longer has access.
 * @returns {Promise<void>}
 */
async function revokeRemovedTabs() {
  const all = await chrome.storage.session.get(null);
  for (const [key, value] of Object.entries(all)) {
    const stored = /** @type {TabState | undefined} */ (value);
    if (!key.startsWith("tab.") || !stored?.site) {
      continue;
    }
    const hasAccess = await chrome.permissions.contains({
      origins: [patternOf(stored.site)],
    });
    if (hasAccess) {
      continue;
    }
    const tabId = Number(key.slice("tab.".length));
    await forgetTab(tabId);
    await chrome.tabs
      .sendMessage(tabId, { type: "taigi.setMode", mode: "off" })
      .catch(() => {});
    await resetToolbar(tabId);
  }
}

/**
 * Wraps a callback-based `chrome.contextMenus` call in a Promise while reading
 * `chrome.runtime.lastError` so Chrome 120+ and `@types/chrome` agree.
 * @param {(done: () => void) => void} invoke
 * @returns {Promise<void>}
 */
function contextMenuCall(invoke) {
  return new Promise((resolve) => {
    invoke(() => {
      void chrome.runtime.lastError;
      resolve();
    });
  });
}

let menuQueue = Promise.resolve();
let menuCreated = false;
let removeSiteEnabled = false;

/**
 * Ensures the action context menu exists once per worker lifetime and sets its
 * `enabled` state through a serialized queue.
 * @param {boolean} enabled
 * @returns {Promise<void>}
 */
function enqueueRemoveSiteMenu(enabled) {
  menuQueue = menuQueue
    .then(async () => {
      if (!menuCreated) {
        await contextMenuCall((done) => chrome.contextMenus.removeAll(done));
        await contextMenuCall((done) => {
          chrome.contextMenus.create(
            {
              id: REMOVE_SITE_MENU_ID,
              title: chrome.i18n.getMessage("removeSite"),
              contexts: ["action"],
              enabled,
            },
            done,
          );
        });
        menuCreated = true;
        removeSiteEnabled = enabled;
        return;
      }
      await contextMenuCall((done) => {
        chrome.contextMenus.update(REMOVE_SITE_MENU_ID, { enabled }, done);
      });
      removeSiteEnabled = enabled;
    })
    .catch((error) => console.error("menu.failed", error));
  return menuQueue;
}

/**
 * Updates the "Stop Using on This Site" action context menu item to match
 * whether the active tab in `windowId` (or the focused window) has access.
 * @param {number} [windowId]
 * @returns {Promise<void>}
 */
async function syncRemoveSiteMenu(windowId) {
  const [tab] = await chrome.tabs.query(
    windowId === undefined
      ? { active: true, lastFocusedWindow: true }
      : { active: true, windowId },
  );
  await enqueueRemoveSiteMenu(await tabHasAccess(tab));
}

/**
 * Returns the current `enabled` state of the "Stop Using on This Site" action
 * context menu item after any queued update finishes.
 * @returns {Promise<boolean>}
 */
export async function removeSiteMenuEnabled() {
  await menuQueue;
  return removeSiteEnabled;
}

/**
 * Toggles typing on an enabled tab (preserving page focus) or opens the popup
 * on a tab without host access so the user can grant permission.
 * @param {chrome.tabs.Tab | undefined} tab
 * @returns {Promise<void>}
 */
export async function toggleTab(tab) {
  if (!tab || tab.id === undefined) {
    return;
  }
  const site = siteOf(tab.url);
  const hasAccess =
    site !== null &&
    (await chrome.permissions.contains({ origins: [patternOf(site)] }));
  if (!hasAccess) {
    await chrome.action
      .setPopup({ tabId: tab.id, popup: "popup.html" })
      .catch(() => {});
    await chrome.action.openPopup().catch(console.error);
    return;
  }
  const stored = await readTab(tab.id);
  const isOn = stored?.site === site && stored.mode !== "off";
  await setTyping(tab, !isOn);
}

/**
 * Stops typing on `tab` if active and revokes host permission for its site.
 * @param {chrome.tabs.Tab | undefined} tab
 * @returns {Promise<void>}
 */
export async function removeSiteAccess(tab) {
  const site = siteOf(tab?.url);
  if (site === null) {
    return;
  }
  if (tab?.id !== undefined) {
    const stored = await readTab(tab.id);
    if (stored?.site === site && stored.mode !== "off") {
      await setTyping(tab, false);
    }
  }
  await chrome.permissions.remove({ origins: [patternOf(site)] });
}

/**
 * @typedef {(
 *   | { type: "taigi.hello", site: string }
 *   | { type: "taigi.status", state: BadgeState, label: string, mode: TabMode }
 *   | { type: "taigi.openSettings" }
 *   | { type: "taigi.learning.read" }
 *   | { type: "taigi.learning.generation" }
 *   | { type: "taigi.learning.write", base: number, files: EncodedFiles }
 *   | { type: "taigi.popup.read", tabId: number }
 *   | { type: "taigi.popup.setTyping", tabId: number, isOn: boolean }
 * )} BackgroundRequest
 */

/**
 * Handles an incoming extension message from one of our pages or content scripts.
 * @param {BackgroundRequest} message
 * @param {chrome.runtime.MessageSender} sender
 * @returns {Promise<unknown>}
 */
async function dispatchMessage(message, sender) {
  switch (message.type) {
    // A content script started: the mode its tab is in. A tab that came from
    // another site, or whose site lost access while in bfcache, starts off.
    case "taigi.hello": {
      const tabId = sender.tab?.id;
      if (tabId === undefined) {
        return { mode: "off" };
      }
      const hasAccess = await chrome.permissions.contains({
        origins: [patternOf(message.site)],
      });
      if (!hasAccess) {
        await forgetTab(tabId);
        await resetToolbar(tabId);
        if (sender.tab?.active) {
          await syncRemoveSiteMenu();
        }
        return { mode: "off" };
      }
      const stored = await readTab(tabId);
      const mode = stored?.site === message.site ? stored.mode : "off";
      await writeTab(tabId, {
        site: message.site,
        mode,
        state: "off",
        label: "",
      });
      await draw(tabId, "off", "");
      if (sender.tab?.active) {
        await syncRemoveSiteMenu();
      }
      return { mode };
    }

    case "taigi.status": {
      const tabId = sender.tab?.id;
      if (tabId === undefined) {
        return undefined;
      }
      const stored = await readTab(tabId);
      if (stored === null) {
        return undefined;
      }
      await writeTab(tabId, {
        ...stored,
        mode: message.mode,
        state: message.state,
        label: message.label,
      });
      await draw(tabId, message.state, message.label);
      return undefined;
    }

    case "taigi.openSettings":
      await chrome.runtime.openOptionsPage();
      return undefined;

    // A private window neither reads nor writes the profile's copy. Its content
    // script never asks; this is the second lock.
    case "taigi.learning.read":
      return sender.tab?.incognito === false ? readCopy() : undefined;

    case "taigi.learning.generation":
      return sender.tab?.incognito === false
        ? (await readCopy()).generation
        : undefined;

    case "taigi.learning.write":
      return sender.tab?.incognito === false
        ? writeCopy(message.base, message.files)
        : undefined;

    // The popup: what it shows for `tabId`, the tab in front.
    case "taigi.popup.read": {
      const tab = await chrome.tabs.get(message.tabId);
      const site = siteOf(tab.url);
      if (site === null) {
        return { site: null };
      }
      const hasAccess = await chrome.permissions.contains({
        origins: [patternOf(site)],
      });
      const stored = await readTab(message.tabId);
      // A state from another site is a tab that has not loaded yet.
      const told = stored?.site === site ? stored : null;
      return {
        site,
        pattern: patternOf(site),
        hasAccess,
        mode: told?.mode ?? "off",
        state: told?.state ?? "off",
        label: told?.label ?? "",
      };
    }

    case "taigi.popup.setTyping":
      await setTyping(await chrome.tabs.get(message.tabId), message.isOn);
      return undefined;
  }
}

const KNOWN_MESSAGE_TYPES = new Set([
  "taigi.hello",
  "taigi.status",
  "taigi.openSettings",
  "taigi.learning.read",
  "taigi.learning.generation",
  "taigi.learning.write",
  "taigi.popup.read",
  "taigi.popup.setTyping",
]);

chrome.runtime.onMessage.addListener((rawMessage, sender, respond) => {
  const message = /** @type {BackgroundRequest | undefined} */ (rawMessage);
  // Only this extension's own pages and content scripts.
  if (
    !message ||
    !KNOWN_MESSAGE_TYPES.has(message.type) ||
    sender.id !== chrome.runtime.id
  ) {
    return false;
  }
  dispatchMessage(message, sender).then(respond, (error) => {
    console.error(message.type, error);
    respond(undefined);
  });
  return true;
});

chrome.permissions.onAdded.addListener(() => {
  syncRegistration();
  syncGrantedTabs().catch((error) => console.error("tabs.failed", error));
  syncRemoveSiteMenu().catch((error) => console.error("menu.failed", error));
});
chrome.permissions.onRemoved.addListener(() => {
  syncRegistration();
  revokeRemovedTabs().catch((error) => console.error("revoke.failed", error));
  syncRemoveSiteMenu().catch((error) => console.error("menu.failed", error));
});

chrome.tabs.onRemoved.addListener(forgetTab);
chrome.tabs.onActivated.addListener(() => {
  syncRemoveSiteMenu().catch((error) => console.error("menu.failed", error));
});

// A tab that leaves its site starts off wherever it lands, a site the
// extension cannot see (no `url`) included.
chrome.tabs.onUpdated.addListener(async (tabId, change, tab) => {
  if (change.status !== "loading" && change.url === undefined) {
    return;
  }
  const stored = await readTab(tabId);
  if (stored !== null && siteOf(tab.url) !== stored.site) {
    await forgetTab(tabId);
    if (!(await tabHasAccess(tab))) {
      await resetToolbar(tabId);
    }
  }
  if (tab.active) {
    await syncRemoveSiteMenu();
  }
});

chrome.windows.onFocusChanged.addListener((windowId) => {
  if (windowId === chrome.windows.WINDOW_ID_NONE) {
    return;
  }
  syncRemoveSiteMenu(windowId).catch((error) =>
    console.error("menu.failed", error),
  );
});

// Left-click on an enabled tab (whose per-tab popup is "") toggles typing
// directly without stealing focus from the field; ungranted tabs open popup.html.
chrome.action.onClicked.addListener((tab) => {
  toggleTab(tab).catch(console.error);
});

// The shortcut: on or off in the tab in front. A site not yet enabled needs
// the user's yes, which the popup asks for.
chrome.commands.onCommand.addListener((command, tab) => {
  if (command !== "toggle-typing") {
    return;
  }
  toggleTab(tab).catch(console.error);
});

chrome.contextMenus.onClicked.addListener((info, tab) => {
  if (info.menuItemId === REMOVE_SITE_MENU_ID && tab) {
    removeSiteAccess(tab).catch(console.error);
  }
});

/**
 * Synchronizes content script registration, already-open granted tabs, and the
 * toolbar action context menu when the worker installs or starts up.
 * @returns {void}
 */
function onWorkerStart() {
  syncRegistration();
  syncGrantedTabs().catch((error) => console.error("tabs.failed", error));
  syncRemoveSiteMenu().catch((error) => console.error("menu.failed", error));
}

chrome.runtime.onInstalled.addListener(onWorkerStart);
chrome.runtime.onStartup.addListener(onWorkerStart);

drawDefaultIcon().catch((error) => console.error("icon.failed", error));
syncRemoveSiteMenu().catch((error) => console.error("menu.failed", error));

Object.assign(globalThis, {
  toggleTab,
  removeSiteAccess,
  removeSiteMenuEnabled,
});
