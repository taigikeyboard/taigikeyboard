// Builds the browser extension's pure-JS fallback engine (`taigi_web_asm.js`
// and `taigi_web_js.js`) from `taigi_web.wasm` for sites whose Content Security
// Policy omits `'wasm-unsafe-eval'`, with no second `cargo build`.
//   node scripts/build-wasm2js.mjs <taigi_web.wasm> <out-dir>

import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const WASM_HEADER = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
const DISABLED_FEATURES = new Set(["multivalue", "reference-types"]);

function readUleb128(bytes, start) {
  let value = 0;
  let factor = 1;
  let offset = start;
  while (offset < bytes.length) {
    const byte = bytes[offset];
    offset += 1;
    value += (byte & 0x7f) * factor;
    if ((byte & 0x80) === 0) {
      return [value, offset];
    }
    factor *= 128;
  }
  throw new Error("wasm target_features: truncated LEB128");
}

function replaceOnce(source, needle, replacement, label) {
  const index = source.indexOf(needle);
  if (index < 0 || source.indexOf(needle, index + needle.length) >= 0) {
    throw new Error(`wasm2js rewrite: expected one ${label}`);
  }
  return (
    source.slice(0, index) + replacement + source.slice(index + needle.length)
  );
}

/// Marks `multivalue` and `reference-types` disabled in the module's
/// `target_features` custom section so `wasm-bindgen` emits one function table
/// and stack out-pointers, the subset `wasm2js` accepts.
export function disableWasm2jsUnsupportedFeatures(wasmBytes) {
  if (
    wasmBytes.length < WASM_HEADER.length ||
    !WASM_HEADER.every((byte, index) => wasmBytes[index] === byte)
  ) {
    throw new Error("wasm target_features: not a WebAssembly binary");
  }
  const patched = new Uint8Array(wasmBytes);
  const decoder = new TextDecoder();
  const seen = new Set();
  let foundSection = false;
  let offset = WASM_HEADER.length;

  while (offset < patched.length) {
    const sectionId = patched[offset];
    const [sectionSize, bodyStart] = readUleb128(patched, offset + 1);
    const sectionEnd = bodyStart + sectionSize;
    if (sectionEnd > patched.length) {
      throw new Error("wasm target_features: section past end of binary");
    }
    if (sectionId === 0) {
      const [nameLength, nameStart] = readUleb128(patched, bodyStart);
      const nameEnd = nameStart + nameLength;
      const name = decoder.decode(patched.subarray(nameStart, nameEnd));
      if (name === "target_features") {
        foundSection = true;
        let [count, cursor] = readUleb128(patched, nameEnd);
        for (; count > 0; count -= 1) {
          const prefixAt = cursor;
          const [featureLength, featureStart] = readUleb128(
            patched,
            cursor + 1,
          );
          const featureEnd = featureStart + featureLength;
          const feature = decoder.decode(
            patched.subarray(featureStart, featureEnd),
          );
          if (DISABLED_FEATURES.has(feature)) {
            patched[prefixAt] = 0x2d; // '-'
            seen.add(feature);
          }
          cursor = featureEnd;
        }
      }
    }
    offset = sectionEnd;
  }

  if (!foundSection) {
    throw new Error("wasm target_features: missing target_features section");
  }
  for (const feature of DISABLED_FEATURES) {
    if (!seen.has(feature)) {
      throw new Error(`wasm target_features: missing ${feature} entry`);
    }
  }
  return patched;
}

/// Turns `wasm2js` output into a reusable module exporting `asmFunc` with no
/// eager instantiation, `WebAssembly.RuntimeError` traps, and no retained
/// reference to a grown `ArrayBuffer` across instances.
export function rewriteAsmModule(source) {
  if (!source.startsWith("import * as ")) {
    throw new Error("wasm2js rewrite: expected leading import lines");
  }
  let rewritten = source.replace(/^(?:import \* as [^\n]+\n)+/, "");
  rewritten = replaceOnce(
    rewritten,
    "function wasm2js_trap() { throw new Error('abort'); }",
    "function wasm2js_trap() { throw new WebAssembly.RuntimeError('abort'); }",
    "wasm2js_trap",
  );
  rewritten = replaceOnce(
    rewritten,
    " bufferView = HEAPU8;\n initActiveSegments(imports);",
    " bufferView = HEAPU8;\n initActiveSegments(imports);\n bufferView = null;",
    "initActiveSegments bufferView reset",
  );
  rewritten = replaceOnce(
    rewritten,
    "   buffer = newBuffer;\n   bufferView = HEAPU8;",
    "   buffer = newBuffer;",
    "__wasm_memory_grow bufferView drop",
  );
  const footer = rewritten.indexOf("\nvar retasmFunc = asmFunc(");
  if (footer < 0) {
    throw new Error("wasm2js rewrite: missing retasmFunc instantiation");
  }
  return `${rewritten.slice(0, footer)}\nexport { asmFunc };\n`;
}

/// Adapts `wasm-bindgen`'s classic glue to instantiate through `asmFunc` and
/// refresh cached typed views when `__wasm_memory_grow` swaps `ArrayBuffer`s
/// without detaching the old one.
export function rewriteGlueModule(source) {
  const boundary = source.match(
    /function dropObject\(idx\) \{\s*if \(idx < (\d+)\) return;/,
  );
  if (!boundary) {
    throw new Error("wasm2js rewrite: missing dropObject static boundary");
  }
  let rewritten = replaceOnce(
    source,
    'import * as import1 from "__wbindgen_externref_xform__"',
    `const import1 = { __wbindgen_externref_statics_end: () => ${boundary[1]} };`,
    "externref xform import",
  );
  rewritten = replaceOnce(
    rewritten,
    "cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)",
    "cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer !== wasm.memory.buffer",
    "DataView memory guard",
  );
  rewritten = replaceOnce(
    rewritten,
    "cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.byteLength === 0",
    "cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.buffer !== wasm.memory.buffer",
    "Float32Array memory guard",
  );
  rewritten = replaceOnce(
    rewritten,
    "cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0",
    "cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.buffer !== wasm.memory.buffer",
    "Uint8Array memory guard",
  );
  const loader = rewritten.indexOf("\nasync function __wbg_load(");
  if (loader < 0) {
    throw new Error("wasm2js rewrite: missing __wbg_load");
  }
  return `${rewritten.slice(0, loader)}

export function initAsm(asmFunc) {
  if (wasm !== undefined) return wasm;
  const imports = __wbg_get_imports();
  const exports = asmFunc(imports);
  return __wbg_finalize_init({ exports }, null);
}
`;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const [wasmInput, outArg] = process.argv.slice(2);
  const out = resolve(outArg);
  const work = join(out, ".work");
  rmSync(out, { recursive: true, force: true });
  mkdirSync(work, { recursive: true });

  const compatWasm = join(work, "compat_in.wasm");
  writeFileSync(
    compatWasm,
    disableWasm2jsUnsupportedFeatures(readFileSync(wasmInput)),
  );

  execFileSync("wasm-bindgen", [
    "--target",
    "web",
    "--out-dir",
    work,
    "--out-name",
    "compat",
    compatWasm,
  ]);

  const optimizedWasm = join(work, "compat_opt.wasm");
  execFileSync("wasm-opt", [
    "-O3",
    "--strip-debug",
    "--strip-producers",
    "--llvm-memory-copy-fill-lowering",
    "--enable-bulk-memory",
    "--enable-sign-ext",
    "--enable-mutable-globals",
    "--enable-nontrapping-float-to-int",
    "--enable-extended-const",
    join(work, "compat_bg.wasm"),
    "-o",
    optimizedWasm,
  ]);

  const rawAsmPath = join(work, "raw_asm.js");
  execFileSync("wasm2js", [
    "--enable-sign-ext",
    "--enable-mutable-globals",
    "--enable-nontrapping-float-to-int",
    "--enable-extended-const",
    optimizedWasm,
    "-o",
    rawAsmPath,
  ]);

  const unminifiedAsmPath = join(work, "taigi_web_asm.raw.js");
  writeFileSync(
    unminifiedAsmPath,
    rewriteAsmModule(readFileSync(rawAsmPath, "utf8")),
  );

  const asmPath = join(out, "taigi_web_asm.js");
  const gluePath = join(out, "taigi_web_js.js");
  const { build } = await import("esbuild");
  await build({
    entryPoints: [unminifiedAsmPath],
    outfile: asmPath,
    format: "esm",
    minify: true,
    legalComments: "none",
  });
  writeFileSync(
    gluePath,
    rewriteGlueModule(readFileSync(join(work, "compat.js"), "utf8")),
  );
  rmSync(work, { recursive: true, force: true });

  // Smoke-check the emitted pair, including a linear-memory grow before a key.
  const { asmFunc } = await import(pathToFileURL(asmPath).href);
  const { initAsm, Session } = await import(pathToFileURL(gluePath).href);
  initAsm(asmFunc);
  const session = new Session("{}", "linux", "en");
  session.addDictionary(
    session.dictionaryFiles()[0],
    new Uint8Array(2_000_000),
  );
  const reply = JSON.parse(
    session.keyDown(JSON.stringify({ key: "a", code: "KeyA" })),
  );
  if (reply.handled !== true) {
    throw new Error("wasm2js smoke: Session did not handle a key");
  }
  session.free();
}
