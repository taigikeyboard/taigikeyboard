// The engine and its dictionaries. `generated/dictionaries.js` is staged with the
// page (`scripts/stage-assets.mjs`), so the list always belongs to this engine.

import { DICTIONARIES } from "./generated/dictionaries.js";

const DICTIONARY_CACHE = "taigi-web-dictionaries"; // Also in `sw.js`.

let compiled;
let instanceCount = 0;

/// A session on a new instance. A trapped instance cannot be reused, and the
/// `wasm-bindgen` glue keeps the one it has, so each instance gets its own copy
/// of the glue module; the query string is what makes it a separate copy.
export async function createSession(platform, locale) {
  compiled ??= await WebAssembly.compileStreaming(
    fetch("pkg/taigi_web_bg.wasm"),
  );
  instanceCount += 1;
  const glue = await import(`./pkg/taigi_web.js?instance=${instanceCount}`);
  await glue.default({ module_or_path: compiled });
  return new glue.Session("{}", platform, locale);
}

/// `null` where storage is refused, a private window for one: the dictionaries
/// then come from the network each visit.
async function openCache() {
  try {
    return await caches.open(DICTIONARY_CACHE);
  } catch (error) {
    console.error(error);
    return null;
  }
}

/// `cache: "reload"`: bytes that failed the hash may be what the HTTP cache
/// holds, and a retry must not read them back. `onBytes(count)` per chunk.
async function download({ file, size }, onBytes) {
  const response = await fetch(file, { cache: "reload" });
  if (!response.ok) {
    throw new Error(`${file}: HTTP ${response.status}`);
  }
  const bytes = new Uint8Array(size);
  const reader = response.body.getReader();
  let length = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) {
      return bytes;
    }
    if (length + value.length > size) {
      throw new Error(`${file}: more than ${size} bytes`);
    }
    bytes.set(value, length);
    length += value.length;
    onBytes(value.length);
  }
}

async function sha256(bytes) {
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return btoa(String.fromCharCode(...new Uint8Array(digest)));
}

async function gunzip(bytes) {
  const stream = new Response(bytes).body.pipeThrough(
    new DecompressionStream("gzip"),
  );
  return new Uint8Array(await new Response(stream).arrayBuffer());
}

/// Yields `[name, bytes]` per dictionary file, one at a time, so that a file's
/// compressed and raw copies are the only ones held at once. Stored bytes pass
/// the same hash as downloaded ones: a damaged entry is dropped, and the retry
/// downloads it. `onProgress(fraction)` is called only while downloading.
export async function* dictionaries(names, onProgress) {
  const cache = await openCache();
  const entries = names.map((name) => {
    const entry = DICTIONARIES.find((candidate) => candidate.name === name);
    if (!entry) {
      throw new Error(`${name}: not staged`);
    }
    return entry;
  });
  const stored = await Promise.all(
    entries.map((entry) => cache?.match(entry.file)),
  );
  const total = entries.reduce(
    (sum, entry, index) => sum + (stored[index] ? 0 : entry.size),
    0,
  );
  let received = 0;
  const onBytes = (count) => {
    received += count;
    onProgress(received / total);
  };

  if (total > 0) {
    onProgress(0);
  }
  for (const [index, entry] of entries.entries()) {
    const found = stored[index];
    const bytes = found
      ? new Uint8Array(await found.arrayBuffer())
      : await download(entry, onBytes);
    if ((await sha256(bytes)) !== entry.sha256) {
      await cache?.delete(entry.file);
      throw new Error(`${entry.file}: hash mismatch`);
    }
    if (!found) {
      // A full disk costs the next visit a download, not this one its typing.
      await cache?.put(entry.file, new Response(bytes)).catch(console.error);
    }
    yield [entry.name, await gunzip(bytes)];
  }
}

/// Offline use is a bonus on top of a working page, so a refusal stays quiet.
export function keepForOffline() {
  navigator.serviceWorker?.register("sw.js").catch(console.error);
}
