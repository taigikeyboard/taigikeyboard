// The profile's one copy of what the engine learned: the four store files
// every tab's engine starts from (`learning-client.js`), in the extension's
// IndexedDB. Only the service worker opens it. The copy is numbered, and a
// write names the number it continues: the check and the write are one
// transaction, so two tabs can never both write over the same copy.

/** @typedef {Record<string, string>} EncodedFiles */
/**
 * @typedef {{
 *   generation: number,
 *   files: EncodedFiles,
 * }} LearningCopy
 */

const DATABASE = "taigi-extension-user-data";
const STORE = "files";
/** One record holding the number and all four files, so a save is one write. */
const SNAPSHOT = "snapshot";

/** @type {LearningCopy} */
const EMPTY = { generation: 0, files: {} };

/**
 * Wraps an IndexedDB request as a Promise.
 * @template T
 * @param {IDBRequest<T>} idbRequest
 * @returns {Promise<T>}
 */
function request(idbRequest) {
  return new Promise((resolve, reject) => {
    idbRequest.onsuccess = () => resolve(idbRequest.result);
    idbRequest.onerror = () => reject(idbRequest.error);
  });
}

/** @type {Promise<IDBDatabase> | null} */
let opening = null;

/**
 * Opens and caches the extension's IndexedDB handle.
 * @returns {Promise<IDBDatabase>}
 */
function database() {
  opening ??= (() => {
    const open = indexedDB.open(DATABASE, 1);
    open.onupgradeneeded = () => open.result.createObjectStore(STORE);
    return request(open);
  })();
  return opening;
}

/**
 * `{ generation, files }`: the copy's number and its files by name, as the
 * tabs sent them.
 * @returns {Promise<LearningCopy>}
 */
export async function readCopy() {
  const store = (await database()).transaction(STORE).objectStore(STORE);
  return (await request(store.get(SNAPSHOT))) ?? EMPTY;
}

/**
 * Takes `files`, the ones that changed since copy `base`, when `base` is
 * still the latest. Answers `{ isTaken, generation }`, the latest number
 * either way.
 * @param {number} base
 * @param {EncodedFiles} files
 * @returns {Promise<{ isTaken: boolean, generation: number }>}
 */
export async function writeCopy(base, files) {
  const transaction = (await database()).transaction(STORE, "readwrite");
  const store = transaction.objectStore(STORE);
  const copy =
    /** @type {LearningCopy | undefined} */ (
      await request(store.get(SNAPSHOT))
    ) ?? EMPTY;
  if (copy.generation !== base) {
    return { isTaken: false, generation: copy.generation };
  }
  const generation = base + 1;
  // In the transaction's own task: it would close across another await.
  const done = new Promise((resolve, reject) => {
    transaction.oncomplete = () => resolve(undefined);
    transaction.onabort = () => reject(transaction.error);
  });
  store.put({ generation, files: { ...copy.files, ...files } }, SNAPSHOT);
  await done;
  return { isTaken: true, generation };
}
