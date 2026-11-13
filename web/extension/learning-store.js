// The profile's one copy of what the engine learned: the four store files
// every tab's engine starts from (`learning-client.js`), in the extension's
// IndexedDB. Only the service worker opens it. The copy is numbered, and a
// write names the number it continues: the check and the write are one
// transaction, so two tabs can never both write over the same copy.

const DATABASE = "taigi-extension-user-data";
const STORE = "files";
/// One record holding the number and all four files, so a save is one write.
const SNAPSHOT = "snapshot";

const EMPTY = { generation: 0, files: {} };

function request(idbRequest) {
  return new Promise((resolve, reject) => {
    idbRequest.onsuccess = () => resolve(idbRequest.result);
    idbRequest.onerror = () => reject(idbRequest.error);
  });
}

let opening = null;

function database() {
  opening ??= (() => {
    const open = indexedDB.open(DATABASE, 1);
    open.onupgradeneeded = () => open.result.createObjectStore(STORE);
    return request(open);
  })();
  return opening;
}

/// `{ generation, files }`: the copy's number and its files by name, as the
/// tabs sent them.
export async function readCopy() {
  const store = (await database()).transaction(STORE).objectStore(STORE);
  return (await request(store.get(SNAPSHOT))) ?? EMPTY;
}

/// Takes `files`, the ones that changed since copy `base`, when `base` is
/// still the latest. Answers `{ isTaken, generation }`, the latest number
/// either way.
export async function writeCopy(base, files) {
  const transaction = (await database()).transaction(STORE, "readwrite");
  const store = transaction.objectStore(STORE);
  const copy = (await request(store.get(SNAPSHOT))) ?? EMPTY;
  if (copy.generation !== base) {
    return { isTaken: false, generation: copy.generation };
  }
  const generation = base + 1;
  // In the transaction's own task: it would close across another await.
  const done = new Promise((resolve, reject) => {
    transaction.oncomplete = resolve;
    transaction.onabort = () => reject(transaction.error);
  });
  store.put({ generation, files: { ...copy.files, ...files } }, SNAPSHOT);
  await done;
  return { isTaken: true, generation };
}
