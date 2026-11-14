// Keeps the app shell and the dictionaries for offline use. `dictionaries` in
// `loader.js` writes the dictionary cache on a first visit, and this worker is
// registered only once that is done, so the 15 MB is never fetched twice.
//
// A new version waits until every tab has closed: an open tab can still start an
// instance from its own glue (`loader.js`), which must match the wasm it holds.
// For the same reason there is no `clients.claim()`. The origin is shared with
// other sites, so only caches named here are ever deleted.

/** @typedef {import("../types/shared.d.ts").AssetsManifest} AssetsManifest */

const BUILD = "dev"; // `scripts/stage-assets.mjs` writes the real one.
const SHELL_PREFIX = "taigi-web-shell-";
const SHELL_CACHE = `${SHELL_PREFIX}${BUILD}`;
const DICTIONARY_CACHE = "taigi-web-dictionaries"; // Also in `loader.js`.
const MANIFEST = "assets-manifest.json";
const sw = /** @type {ServiceWorkerGlobalScope} */ (
  /** @type {unknown} */ (self)
);

/**
 * Fetches an asset with SRI verification and rejects non-OK HTTP responses.
 * @param {string} path
 * @param {string} sha256
 * @param {RequestCache} cacheMode
 * @returns {Promise<Response>}
 */
async function fetchVerified(path, sha256, cacheMode) {
  const response = await fetch(path, {
    cache: cacheMode,
    integrity: `sha256-${sha256}`,
  });
  if (!response.ok) {
    throw new Error(`${path}: HTTP ${response.status}`);
  }
  return response;
}

/**
 * All or nothing: a failure rejects the install, the page keeps what it has, and
 * the browser tries again on the next visit.
 * @returns {Promise<void>}
 */
async function install() {
  const response = await fetch(MANIFEST, { cache: "no-cache" });
  const manifest = /** @type {AssetsManifest} */ (
    await response.clone().json()
  );
  // An edge still holding the last deploy.
  if (manifest.build !== BUILD) {
    throw new Error(`manifest ${manifest.build}, worker ${BUILD}`);
  }
  const shell = await caches.open(SHELL_CACHE);
  await Promise.all(
    manifest.shell.map(async ({ path, sha256 }) => {
      await shell.put(path, await fetchVerified(path, sha256, "no-cache"));
    }),
  );
  await shell.put(MANIFEST, response);

  const dictionaries = await caches.open(DICTIONARY_CACHE);
  for (const { file, sha256 } of manifest.dictionaries) {
    if (!(await dictionaries.match(file))) {
      await dictionaries.put(file, await fetchVerified(file, sha256, "reload"));
    }
  }
}

/**
 * Removes superseded shell caches and dictionary entries no longer listed in the
 * active manifest.
 * @returns {Promise<void>}
 */
async function activate() {
  const names = await caches.keys();
  await Promise.all(
    names
      .filter((name) => name.startsWith(SHELL_PREFIX) && name !== SHELL_CACHE)
      .map((name) => caches.delete(name)),
  );

  const shell = await caches.open(SHELL_CACHE);
  const manifestResponse = await shell.match(MANIFEST);
  if (!manifestResponse) {
    throw new Error(`${MANIFEST}: missing from ${SHELL_CACHE}`);
  }
  const manifest = /** @type {AssetsManifest} */ (
    await manifestResponse.json()
  );
  const current = new Set(
    manifest.dictionaries.map(
      ({ file }) => new URL(file, sw.location.href).href,
    ),
  );
  const dictionaries = await caches.open(DICTIONARY_CACHE);
  for (const request of await dictionaries.keys()) {
    if (!current.has(request.url)) {
      await dictionaries.delete(request);
    }
  }
}

/**
 * Serves precached shell entries and caches runtime-selected woff2 fonts.
 * @param {Request} request
 * @returns {Promise<Response>}
 */
async function respond(request) {
  const url = new URL(request.url);
  const key = url.pathname.endsWith("/")
    ? new URL("index.html", url).href
    : request;
  const shell = await caches.open(SHELL_CACHE);
  // The glue is imported as `taigi_web.js?instance=N`.
  const found = await shell.match(key, { ignoreSearch: true });
  if (found) {
    return found;
  }
  const response = await fetch(request);
  // A chosen font keeps working offline, and goes with this version's cache.
  if (response.ok && url.pathname.endsWith(".woff2")) {
    await shell.put(request, response.clone()).catch(console.error);
  }
  return response;
}

sw.addEventListener("install", (event) => {
  event.waitUntil(install());
});

sw.addEventListener("activate", (event) => {
  event.waitUntil(activate());
});

sw.addEventListener("fetch", (event) => {
  event.respondWith(respond(event.request));
});
