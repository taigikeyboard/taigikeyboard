// The extension's content script, on every page of a site the user gave it,
// from `document_start`. It only asks the service worker which mode this tab
// is in; the engine and everything it draws with (`typing.js`) load the first
// time typing is turned on here.

(() => {
  // A page open when the site was enabled gets this script by hand, and the
  // registration may add it again.
  if (globalThis.taigiKeyboard) {
    return;
  }
  globalThis.taigiKeyboard = true;

  /** @typedef {import("./background.js").TabMode} TabMode */
  /** @typedef {typeof import("./typing.js")} TypingModule */

  /**
   * `typing.js`, once asked for.
   * @type {Promise<TypingModule> | null}
   */
  let typing = null;
  /**
   * Its key handlers, once it has loaded.
   * @type {TypingModule | null}
   */
  let handlers = null;

  // Now, so that these are the window's first key listeners: a key the engine
  // takes never reaches the site's own.
  window.addEventListener("keydown", (event) => handlers?.onKeyDown(event), {
    capture: true,
  });
  window.addEventListener("keyup", (event) => handlers?.onKeyUp(event), {
    capture: true,
  });

  /**
   * Sends a message to the extension service worker and returns `undefined` if unavailable.
   * @param {unknown} message
   * @returns {Promise<unknown>}
   */
  function tell(message) {
    return chrome.runtime.sendMessage(message).catch(() => undefined);
  }

  /**
   * Applies a mode transition, lazily importing `typing.js` on first activation.
   * @param {TabMode} mode
   * @returns {void}
   */
  function setMode(mode) {
    // A tab that never typed loads nothing to hear that it is off.
    if (mode === "off" && typing === null) {
      return;
    }
    typing ??= /** @type {Promise<TypingModule>} */ (
      import(chrome.runtime.getURL("typing.js"))
    );
    typing.then(
      (module) => {
        handlers = module;
        module.setMode(mode);
      },
      (error) => {
        typing = null;
        console.error(error);
        if (mode !== "off") {
          tell({ type: "taigi.status", state: "failed", label: "", mode });
        }
      },
    );
  }

  chrome.runtime.onMessage.addListener((rawMessage, _sender, respond) => {
    const message =
      /** @type {{ type?: string, mode?: TabMode } | undefined} */ (rawMessage);
    if (message?.type === "taigi.setMode" && message.mode) {
      setMode(message.mode);
      respond(true);
    }
  });

  /**
   * The permission and the mode are per site: the host, as the browser's
   * site access is.
   * @returns {void}
   */
  function hello() {
    tell({ type: "taigi.hello", site: location.hostname }).then((answer) => {
      const reply = /** @type {{ mode?: TabMode } | undefined} */ (answer);
      if (reply?.mode) {
        setMode(reply.mode);
      }
    });
  }

  // A page back from the browser's back-forward cache kept the mode it left
  // with, and its tab may have been to another site since.
  window.addEventListener("pageshow", (event) => {
    if (event.persisted) {
      hello();
    }
  });

  hello();
})();
