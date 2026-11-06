// The engine and its dictionaries. Fetched plainly; verifying and caching them
// is `docs/architecture/web-roadmap.md` § Phase 6.

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

/// `[name, bytes]` per dictionary file.
export function fetchDictionaries(names) {
  return Promise.all(
    names.map(async (name) => {
      const response = await fetch(`dictionaries/${name}`);
      if (!response.ok) {
        throw new Error(`${name}: HTTP ${response.status}`);
      }
      return [name, new Uint8Array(await response.arrayBuffer())];
    }),
  );
}
