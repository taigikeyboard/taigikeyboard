// The engine and its dictionaries, from the extension's own package: nothing
// is downloaded, so there is no hash to check and no cache to keep
// (`loader.js` is the page's).
//
// Chromium applies the page's Content Security Policy to `WebAssembly.compile`
// in a content script. When a site's `script-src` omits `'wasm-unsafe-eval'`,
// `createSession` falls back to the `wasm2js` translation in `pkg/taigi_web_asm.js`.

let compiled;
let asmModule;
let useJsFallback = false;
let instanceCount = 0;

/// Whether `error` is the browser refusing to compile WebAssembly under the
/// page's Content Security Policy.
function isWasmBlocked(error) {
  return error instanceof WebAssembly.CompileError;
}

/// A session on a new instance. As on the page, each instance gets its own
/// copy of the glue module, which keeps the instance it has.
export async function createSession(platform, locale) {
  if (!useJsFallback && compiled === undefined) {
    try {
      compiled = await WebAssembly.compileStreaming(
        fetch(chrome.runtime.getURL("pkg/taigi_web_bg.wasm")),
      );
    } catch (error) {
      if (!isWasmBlocked(error)) {
        throw error;
      }
      useJsFallback = true;
    }
  }
  instanceCount += 1;
  if (useJsFallback) {
    asmModule ??= await import(chrome.runtime.getURL("pkg/taigi_web_asm.js"));
    const glue = await import(
      chrome.runtime.getURL(`pkg/taigi_web_js.js?instance=${instanceCount}`)
    );
    glue.initAsm(asmModule.asmFunc);
    return new glue.Session("{}", platform, locale);
  }
  const glue = await import(
    chrome.runtime.getURL(`pkg/taigi_web.js?instance=${instanceCount}`)
  );
  await glue.default({ module_or_path: compiled });
  return new glue.Session("{}", platform, locale);
}

/// Hands `session` its dictionaries and installs them. `isCurrent()`: whether
/// `session` is still the one in use; a replaced one is left as it is.
export async function installDictionaries(session, isCurrent) {
  for (const name of session.dictionaryFiles()) {
    const response = await fetch(chrome.runtime.getURL(`dictionaries/${name}`));
    const bytes = new Uint8Array(await response.arrayBuffer());
    if (!isCurrent()) {
      return false;
    }
    session.addDictionary(name, bytes);
  }
  session.install();
  return true;
}
