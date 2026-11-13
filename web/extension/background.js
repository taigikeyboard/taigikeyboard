// The extension's service worker: which sites the content script runs on,
// each tab's typing mode, what the toolbar button shows, and the profile's
// learned data (`learning-store.js`). It holds no engine; each tab runs its
// own (`typing.js`).

import { readCopy, writeCopy } from "./learning-store.js";

/// The registered content script's id.
const SCRIPT_ID = "taigi";

/// The toolbar button per state the content script tells. A tab the script
/// does not run in has the manifest's title, no badge and the grey icon.
const BADGES = {
  off: { text: "" },
  loading: { text: "…", color: "#6e6e73" },
  taigi: { text: "台", color: "#0071e3" },
  english: { text: "A", color: "#1c1c1e" },
  unsupported: { text: "×", color: "#6e6e73" },
  failed: { text: "!", color: "#d70015" },
};

const ICON_SIZES = [16, 32];

const NAME = chrome.runtime.getManifest().short_name;

/// The pattern the browser's site access takes for `hostname`.
function patternOf(hostname) {
  return `*://${hostname}/*`;
}

/// Sites whose pages the browser lets no extension into, even though their
/// URLs are `https:`.
function isRestrictedSite(hostname, pathname) {
  return (
    hostname === "chromewebstore.google.com" ||
    (hostname === "chrome.google.com" && /^\/webstore(\/|$)/.test(pathname))
  );
}

/// The host of `url` when it is a website the extension may run on, else
/// `null`: a browser page, the web store, a file, or a URL the extension may
/// not see.
function siteOf(url) {
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

// A tab's state, `{ site, mode, state, label }`, in session storage: it lasts
// the browser session and outlives this worker. `mode` is what the tab types
// in (`"off"`, `"taigi"`, `"english"`); `state` and `label` are what the
// content script last said of it.
const tabKey = (tabId) => `tab.${tabId}`;

async function readTab(tabId) {
  const key = tabKey(tabId);
  return (await chrome.storage.session.get(key))[key] ?? null;
}

function writeTab(tabId, state) {
  return chrome.storage.session.set({ [tabKey(tabId)]: state });
}

function forgetTab(tabId) {
  return chrome.storage.session.remove(tabKey(tabId));
}

/// The icon in grey, for a tab the extension does not run in.
async function greyIcon(size) {
  const response = await fetch(chrome.runtime.getURL(`icons/${size}.png`));
  const bitmap = await createImageBitmap(await response.blob());
  const canvas = new OffscreenCanvas(size, size);
  const context = canvas.getContext("2d");
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

let greyIcons = null;

function defaultIconData() {
  greyIcons ??= Promise.all(ICON_SIZES.map(greyIcon)).then((images) =>
    Object.fromEntries(ICON_SIZES.map((size, index) => [size, images[index]])),
  );
  return greyIcons;
}

/// Every tab starts grey; a tab the content script answers from is drawn in
/// colour (`draw`). The browser forgets this with the worker, so each start
/// sets it again.
async function drawDefaultIcon() {
  await chrome.action.setIcon({ imageData: await defaultIconData() });
}

/// Returns `tabId`'s toolbar button to the state of a page the extension does
/// not run on.
async function resetToolbar(tabId) {
  try {
    const imageData = await defaultIconData();
    await Promise.all([
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

const COLOUR_ICON = Object.fromEntries(
  ICON_SIZES.map((size) => [size, `icons/${size}.png`]),
);

/// The toolbar button of `tabId` as `state` and `label` say. A tab closed
/// meanwhile has no button to draw.
async function draw(tabId, state, label) {
  const badge = BADGES[state] ?? BADGES.off;
  try {
    await Promise.all([
      chrome.action.setIcon({ tabId, path: COLOUR_ICON }),
      chrome.action.setBadgeText({ tabId, text: badge.text }),
      chrome.action.setTitle({
        tabId,
        // A tab that never typed has loaded no strings to name its state in.
        title: `${NAME}: ${label || chrome.i18n.getMessage(state)}`,
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

/// Turns typing on or off in `tab`. The content script answers when it is
/// there; a page loaded before its site was enabled gets it now. Throws on a
/// page the browser lets no extension into.
async function setTyping(tab, isOn) {
  const site = siteOf(tab.url);
  if (site === null) {
    throw new Error("no access to this tab");
  }
  if (!isOn) {
    await writeTab(tab.id, { site, mode: "off", state: "off", label: "" });
    await draw(tab.id, "off", "");
    await chrome.tabs
      .sendMessage(tab.id, { type: "taigi.setMode", mode: "off" })
      .catch(() => {});
    return;
  }
  const stored = await readTab(tab.id);
  // Back on in the mode it was turned off from is not kept: on is Taigi.
  await writeTab(tab.id, {
    site,
    mode: "taigi",
    state: stored?.site === site ? stored.state : "off",
    label: stored?.site === site ? stored.label : "",
  });
  try {
    await chrome.tabs.sendMessage(tab.id, {
      type: "taigi.setMode",
      mode: "taigi",
    });
  } catch {
    // No content script yet: its hello reads the mode just written.
    try {
      await chrome.scripting.executeScript({
        target: { tabId: tab.id },
        files: ["content.js"],
      });
    } catch (error) {
      await forgetTab(tab.id);
      throw error;
    }
  }
}

/// Stops typing and clears the toolbar button in any open tab whose site no
/// longer has access.
async function revokeRemovedTabs() {
  const all = await chrome.storage.session.get(null);
  for (const [key, stored] of Object.entries(all)) {
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

const answers = {
  /// A content script started: the mode its tab is in. A tab that came from
  /// another site, or whose site lost access while in bfcache, starts off.
  async "taigi.hello"({ site }, { tab }) {
    const hasAccess = await chrome.permissions.contains({
      origins: [patternOf(site)],
    });
    if (!hasAccess) {
      await forgetTab(tab.id);
      await resetToolbar(tab.id);
      return { mode: "off" };
    }
    const stored = await readTab(tab.id);
    const mode = stored?.site === site ? stored.mode : "off";
    await writeTab(tab.id, { site, mode, state: "off", label: "" });
    await draw(tab.id, "off", "");
    return { mode };
  },

  async "taigi.status"({ state, label, mode }, { tab }) {
    const stored = await readTab(tab.id);
    if (stored === null) {
      return;
    }
    await writeTab(tab.id, { ...stored, mode, state, label });
    await draw(tab.id, state, label);
  },

  async "taigi.openSettings"() {
    await chrome.runtime.openOptionsPage();
  },

  // A private window neither reads nor writes the profile's copy. Its content
  // script never asks; this is the second lock.
  async "taigi.learning.read"(_message, { tab }) {
    return tab.incognito ? undefined : readCopy();
  },

  async "taigi.learning.generation"(_message, { tab }) {
    return tab.incognito ? undefined : (await readCopy()).generation;
  },

  async "taigi.learning.write"({ base, files }, { tab }) {
    return tab.incognito ? undefined : writeCopy(base, files);
  },

  /// The popup: what it shows for `tabId`, the tab in front.
  async "taigi.popup.read"({ tabId }) {
    const tab = await chrome.tabs.get(tabId);
    const site = siteOf(tab.url);
    if (site === null) {
      return { site: null };
    }
    const hasAccess = await chrome.permissions.contains({
      origins: [patternOf(site)],
    });
    const stored = await readTab(tabId);
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
  },

  async "taigi.popup.setTyping"({ tabId, isOn }) {
    await setTyping(await chrome.tabs.get(tabId), isOn);
  },
};

chrome.runtime.onMessage.addListener((message, sender, respond) => {
  const answer = answers[message?.type];
  // Only this extension's own pages and content scripts.
  if (!answer || sender.id !== chrome.runtime.id) {
    return false;
  }
  answer(message, sender).then(respond, (error) => {
    console.error(message.type, error);
    respond(undefined);
  });
  return true;
});

chrome.permissions.onAdded.addListener(syncRegistration);
chrome.permissions.onRemoved.addListener(() => {
  syncRegistration();
  revokeRemovedTabs().catch((error) => console.error("revoke.failed", error));
});

chrome.tabs.onRemoved.addListener(forgetTab);

// A tab that leaves its site starts off wherever it lands, a site the
// extension cannot see (no `url`) included.
chrome.tabs.onUpdated.addListener(async (tabId, change, tab) => {
  if (change.status !== "loading" && change.url === undefined) {
    return;
  }
  const stored = await readTab(tabId);
  if (stored !== null && siteOf(tab.url) !== stored.site) {
    await forgetTab(tabId);
  }
});

// The shortcut: on or off in the tab in front. A site not yet enabled needs
// the user's yes, which the popup asks for.
chrome.commands.onCommand.addListener(async (command, tab) => {
  if (command !== "toggle-typing" || !tab) {
    return;
  }
  const site = siteOf(tab.url);
  const hasAccess =
    site !== null &&
    (await chrome.permissions.contains({ origins: [patternOf(site)] }));
  if (!hasAccess) {
    await chrome.action.openPopup().catch(console.error);
    return;
  }
  const stored = await readTab(tab.id);
  const isOn = stored?.site === site && stored.mode !== "off";
  await setTyping(tab, !isOn).catch(console.error);
});

chrome.runtime.onInstalled.addListener(syncRegistration);
chrome.runtime.onStartup.addListener(syncRegistration);

drawDefaultIcon().catch((error) => console.error("icon.failed", error));
