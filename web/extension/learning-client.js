// What the engine learns, shared by every tab of the profile. Each tab runs
// its own engine, whose stores are SQLite files in its memory (`learning.js`
// is the page's twin); the service worker keeps the one copy on disk
// (`learning-store.js`) and numbers it. A tab hands its stores in with the
// number it started from, and the worker takes them only while that number is
// still the latest: a tab that learned from an older copy starts a new
// instance over the latest one instead of writing over it. One keyboard types
// into one tab at a time, so a tab finds out when it comes back to the front.

/** @typedef {import("./learning-store.js").EncodedFiles} EncodedFiles */
/** @typedef {import("./learning-store.js").LearningCopy} LearningCopy */
/** @typedef {import("../types/site/pkg/taigi_web.js").Session} Session */
/** @typedef {import("../types/shared.d.ts").SessionRunner} SessionRunner */

/**
 * `bytes` for a message, which carries JSON.
 * @param {Uint8Array} bytes
 * @returns {string}
 */
function encode(bytes) {
  let text = "";
  for (let start = 0; start < bytes.length; start += 0x8000) {
    text += String.fromCharCode(...bytes.subarray(start, start + 0x8000));
  }
  return btoa(text);
}

/**
 * Decodes base64-encoded binary data received from the service worker.
 * @param {string} text
 * @returns {Uint8Array}
 */
function decode(text) {
  return Uint8Array.from(atob(text), (character) => character.charCodeAt(0));
}

export class SharedLearning {
  /**
   * The session in use, `null` between instances.
   * @type {() => Session | null}
   */
  #current;
  /**
   * The host's `run`: an engine call, `undefined` after a trap.
   * @type {SessionRunner}
   */
  #run;
  /**
   * The host's message to the service worker; `undefined` when it is gone.
   * @type {(message: unknown) => Promise<unknown>}
   */
  #ask;
  /**
   * A private window learns for the visit only: nothing read, nothing kept.
   * @type {boolean}
   */
  #isPrivate;
  /**
   * The latest copy is newer than this tab's: the host starts a new instance.
   * @type {() => void}
   */
  #onStale;
  /** @type {Session | null} */
  #attaching = null;
  /**
   * The session whose stores are open.
   * @type {Session | null}
   */
  #opened = null;
  /** @type {string[]} */
  #names = [];
  /** The number of the copy this tab's stores continue. */
  #base = 0;
  /**
   * Each file as last handed in or taken out, so a save sends only what
   * changed.
   * @type {EncodedFiles}
   */
  #sent = {};
  /** A trap while handing the copy in would repeat on the next instance. */
  #stopped = false;
  /** @type {ReturnType<typeof setTimeout> | undefined} */
  #saveTimer;
  /** @type {Promise<void> | null} */
  #saving = null;
  #isSaveWanted = false;

  /**
   * @param {{
   *   current: () => Session | null,
   *   run: SessionRunner,
   *   ask: (message: unknown) => Promise<unknown>,
   *   isPrivate: boolean,
   *   onStale: () => void,
   * }} options
   */
  constructor({ current, run, ask, isPrivate, onStale }) {
    this.#current = current;
    this.#run = run;
    this.#ask = ask;
    this.#isPrivate = isPrivate;
    this.#onStale = onStale;
  }

  /**
   * A new instance: it learns from the latest copy. Failures go to the
   * console; the tab still types.
   * @param {Session | null} session
   * @returns {Promise<void>}
   */
  async attach(session) {
    if (this.#stopped || session === null || this.#attaching === session) {
      return;
    }
    this.#attaching = session;
    const copy = this.#isPrivate
      ? undefined
      : /** @type {LearningCopy | undefined} */ (
          await this.#ask({ type: "taigi.learning.read" })
        );
    if (this.#current() !== session) {
      return;
    }
    const { generation = 0, files = {} } = copy ?? {};
    this.#names = this.#run((current) => current.userDataFiles()) ?? [];
    for (const name of this.#names) {
      if (!files[name]) {
        continue;
      }
      // A damaged copy is refused: that store starts empty.
      const imported = this.#runLogged((current) =>
        current.importUserData(name, decode(files[name])),
      );
      if (imported === undefined) {
        this.#stopped = true;
        return;
      }
    }
    if (this.#run((current) => current.openUserData()) === true) {
      this.#opened = session;
      this.#base = generation;
      this.#sent = files;
    }
  }

  /**
   * After a commit, which is what the engine learns from.
   * @returns {void}
   */
  scheduleSave() {
    if (this.#isPrivate) {
      return;
    }
    clearTimeout(this.#saveTimer);
    this.#saveTimer = setTimeout(() => this.#save(), 0);
  }

  /**
   * Whether another tab has learned since this one took its copy. Asked when
   * the tab comes back to the front.
   * @returns {Promise<boolean>}
   */
  async isStale() {
    if (this.#isPrivate || this.#opened === null) {
      return false;
    }
    const generation = /** @type {number | undefined} */ (
      await this.#ask({ type: "taigi.learning.generation" })
    );
    return generation !== undefined && generation !== this.#base;
  }

  /**
   * One save at a time: the next one's number is this one's answer.
   * @returns {void}
   */
  #save() {
    if (this.#saving) {
      this.#isSaveWanted = true;
      return;
    }
    this.#saving = this.#send().finally(() => {
      this.#saving = null;
      if (this.#isSaveWanted) {
        this.#isSaveWanted = false;
        this.#save();
      }
    });
  }

  /** @returns {Promise<void>} */
  async #send() {
    const session = this.#opened;
    if (session === null || this.#current() !== session) {
      return;
    }
    /** @type {EncodedFiles} */
    const files = {};
    for (const name of this.#names) {
      const bytes = this.#runLogged((current) => current.exportUserData(name));
      if (this.#current() !== session) {
        return;
      }
      const text = bytes ? encode(bytes) : undefined;
      if (text !== undefined && text !== this.#sent[name]) {
        files[name] = text;
      }
    }
    if (Object.keys(files).length === 0) {
      return;
    }
    const answer =
      /** @type {{ isTaken: boolean, generation: number } | undefined} */ (
        await this.#ask({
          type: "taigi.learning.write",
          base: this.#base,
          files,
        })
      );
    if (answer === undefined || this.#current() !== session) {
      return;
    }
    if (answer.isTaken) {
      this.#base = answer.generation;
      this.#sent = { ...this.#sent, ...files };
    } else {
      this.#opened = null;
      this.#onStale();
    }
  }

  /**
   * `#run`, with an error the engine raises on purpose (a damaged file)
   * logged and answered `null`. A trap still answers `undefined`.
   * @template T
   * @param {(session: Session) => T | null | undefined} action
   * @returns {T | null | undefined}
   */
  #runLogged(action) {
    return this.#run((current) => {
      try {
        return action(current) ?? null;
      } catch (error) {
        if (error instanceof WebAssembly.RuntimeError) {
          throw error;
        }
        console.error(error);
        return null;
      }
    });
  }
}
