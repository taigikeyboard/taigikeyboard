// What the engine learns, shared by every tab of the profile. Each tab runs
// its own engine, whose stores are SQLite files in its memory (`learning.js`
// is the page's twin); the service worker keeps the one copy on disk
// (`learning-store.js`) and numbers it. A tab hands its stores in with the
// number it started from, and the worker takes them only while that number is
// still the latest: a tab that learned from an older copy starts a new
// instance over the latest one instead of writing over it. One keyboard types
// into one tab at a time, so a tab finds out when it comes back to the front.

/// `bytes` for a message, which carries JSON.
function encode(bytes) {
  let text = "";
  for (let start = 0; start < bytes.length; start += 0x8000) {
    text += String.fromCharCode(...bytes.subarray(start, start + 0x8000));
  }
  return btoa(text);
}

function decode(text) {
  return Uint8Array.from(atob(text), (character) => character.charCodeAt(0));
}

export class SharedLearning {
  /// The session in use, `null` between instances.
  #current;
  /// The host's `run`: an engine call, `undefined` after a trap.
  #run;
  /// The host's message to the service worker; `undefined` when it is gone.
  #ask;
  /// A private window learns for the visit only: nothing read, nothing kept.
  #isPrivate;
  /// The latest copy is newer than this tab's: the host starts a new instance.
  #onStale;
  #attaching = null;
  /// The session whose stores are open.
  #opened = null;
  #names = [];
  /// The number of the copy this tab's stores continue.
  #base = 0;
  /// Each file as last handed in or taken out, so a save sends only what
  /// changed.
  #sent = {};
  /// A trap while handing the copy in would repeat on the next instance.
  #stopped = false;
  #saveTimer;
  #saving = null;
  #isSaveWanted = false;

  constructor({ current, run, ask, isPrivate, onStale }) {
    this.#current = current;
    this.#run = run;
    this.#ask = ask;
    this.#isPrivate = isPrivate;
    this.#onStale = onStale;
  }

  /// A new instance: it learns from the latest copy. Failures go to the
  /// console; the tab still types.
  async attach(session) {
    if (this.#stopped || session === null || this.#attaching === session) {
      return;
    }
    this.#attaching = session;
    const copy = this.#isPrivate
      ? undefined
      : await this.#ask({ type: "taigi.learning.read" });
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

  /// After a commit, which is what the engine learns from.
  scheduleSave() {
    if (this.#isPrivate) {
      return;
    }
    clearTimeout(this.#saveTimer);
    this.#saveTimer = setTimeout(() => this.#save(), 0);
  }

  /// Whether another tab has learned since this one took its copy. Asked when
  /// the tab comes back to the front.
  async isStale() {
    if (this.#isPrivate || this.#opened === null) {
      return false;
    }
    const generation = await this.#ask({ type: "taigi.learning.generation" });
    return generation !== undefined && generation !== this.#base;
  }

  /// One save at a time: the next one's number is this one's answer.
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

  async #send() {
    const session = this.#opened;
    if (session === null || this.#current() !== session) {
      return;
    }
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
    const answer = await this.#ask({
      type: "taigi.learning.write",
      base: this.#base,
      files,
    });
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

  /// `#run`, with an error the engine raises on purpose (a damaged file)
  /// logged and answered `null`. A trap still answers `undefined`.
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
