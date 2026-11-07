// What the engine learns from picks, kept across visits
// (docs/architecture/web-roadmap.md § Phase 8). The engine's four SQLite files
// live in its memory VFS; the page keeps their bytes in IndexedDB. One tab at a
// time learns, the one holding the Web Lock (D9): the others type without
// learning until it closes, then take the latest copy. An instance opens its
// stores once and an open file cannot be handed in again, so a tab never saves
// over a newer copy with what it learned from an older one.

const LOCK = "taigi-user-data";
const DATABASE = "taigi-web-user-data";
const STORE = "files";
/// One record holding all four files, so a save is one write.
const SNAPSHOT = "snapshot";

function request(idbRequest) {
  return new Promise((resolve, reject) => {
    idbRequest.onsuccess = () => resolve(idbRequest.result);
    idbRequest.onerror = () => reject(idbRequest.error);
  });
}

function openDatabase() {
  const opening = indexedDB.open(DATABASE, 1);
  opening.onupgradeneeded = () => opening.result.createObjectStore(STORE);
  return request(opening);
}

/// The four files as last saved, `{ [file name]: Uint8Array }`, or `undefined`
/// before the first save. For tests; a learning tab keeps its connection.
export async function readSnapshot() {
  const database = await openDatabase();
  try {
    const store = database.transaction(STORE).objectStore(STORE);
    return await request(store.get(SNAPSHOT));
  } finally {
    database.close();
  }
}

export class Learning {
  /// The session `ime.js` calls now, `null` between instances.
  #current;
  /// The host's `run` (`ime.js`, the ChromeOS worker): an engine call,
  /// `undefined` after a trap, which the host recovers from in its own way.
  #run;
  #holdsLock = false;
  /// The session being opened, so a second `attach` does not hand files in
  /// twice.
  #attaching = null;
  /// The session whose stores are open.
  #opened = null;
  /// Kept open while this tab learns, so a save starts its write at once.
  #database = null;
  /// `userDataFiles`, which no instance changes.
  #names = [];
  /// The copy last read or written, by file name. A file that cannot be taken
  /// out keeps its bytes here, so the others still save.
  #files = {};
  /// A trap while handing the copy in: it would trap again on the next
  /// instance, so this tab stops learning and the copy stays as it is.
  #stopped = false;
  #saveTimer;

  constructor(current, run) {
    this.#current = current;
    this.#run = run;
    // Without Web Locks the tabs could not agree on one learner, so none
    // learns. Every browser the page supports has them.
    navigator.locks
      ?.request(LOCK, () => {
        this.#holdsLock = true;
        this.attach(this.#current());
        // Held until the tab closes.
        return new Promise(() => {});
      })
      .catch(console.error);
  }

  /// A new instance: it learns once this tab holds the lock, from the latest
  /// copy. Failures go to the console; the tab still types.
  async attach(session) {
    if (
      !this.#holdsLock ||
      this.#stopped ||
      session === null ||
      this.#attaching === session
    ) {
      return;
    }
    this.#attaching = session;
    try {
      this.#database ??= await openDatabase();
      const transaction = this.#database.transaction(STORE);
      this.#files =
        (await request(transaction.objectStore(STORE).get(SNAPSHOT))) ??
        this.#files;
    } catch (error) {
      // No storage, a private window for one: learning lasts the visit.
      console.error(error);
    }
    if (this.#current() !== session) {
      return;
    }
    this.#names = this.#run((current) => current.userDataFiles()) ?? [];
    for (const name of this.#names) {
      if (!this.#files[name]) {
        continue;
      }
      // A damaged copy is refused: that store starts empty.
      const imported = this.#runLogged((current) =>
        current.importUserData(name, this.#files[name]),
      );
      if (imported === undefined) {
        this.#stopped = true;
        return;
      }
    }
    if (this.#run((current) => current.openUserData()) === true) {
      this.#opened = session;
    }
  }

  /// After a commit, which is what the engine learns from: at once, once per
  /// key, after the key's handler. Not on the way out: every engine tested
  /// aborts an IndexedDB write still running when the page unloads, Firefox
  /// with a warning, so a save on `pagehide` or a hidden tab never landed.
  scheduleSave() {
    clearTimeout(this.#saveTimer);
    this.#saveTimer = setTimeout(() => this.#save(), 0);
  }

  #save() {
    const session = this.#opened;
    if (session === null || this.#current() !== session || !this.#database) {
      return;
    }
    for (const name of this.#names) {
      const bytes = this.#runLogged((current) => current.exportUserData(name));
      if (this.#current() !== session) {
        return;
      }
      if (bytes) {
        this.#files[name] = bytes;
      }
    }
    try {
      const transaction = this.#database.transaction(STORE, "readwrite");
      transaction.objectStore(STORE).put(this.#files, SNAPSHOT);
      transaction.onerror = () => console.error(transaction.error);
    } catch (error) {
      console.error(error);
    }
  }

  /// `#run`, with an error the engine raises on purpose (a damaged file, a
  /// file that is not there) logged and answered `null`. A trap still answers
  /// `undefined`.
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
