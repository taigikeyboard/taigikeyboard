// The browser extension's wasm2js build transforms (`scripts/build-wasm2js.mjs`).

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import {
  disableWasm2jsUnsupportedFeatures,
  rewriteAsmModule,
  rewriteGlueModule,
} from "../scripts/build-wasm2js.mjs";

const wasm = readFileSync(
  new URL(
    "../target/wasm32-unknown-unknown/release/taigi_web.wasm",
    import.meta.url,
  ),
);

test("disableWasm2jsUnsupportedFeatures flips multivalue and reference-types only", () => {
  const patched = disableWasm2jsUnsupportedFeatures(wasm);
  assert.equal(patched.length, wasm.length);
  /** @type {Array<[number, number, number]>} */
  const changed = [];
  for (let i = 0; i < wasm.length; i += 1) {
    if (patched[i] !== wasm[i]) {
      changed.push([i, wasm[i], patched[i]]);
    }
  }
  assert.equal(changed.length, 2);
  for (const [, before, after] of changed) {
    assert.equal(before, 0x2b); // '+'
    assert.equal(after, 0x2d); // '-'
  }
  // Idempotent on an already-patched binary.
  assert.deepEqual(disableWasm2jsUnsupportedFeatures(patched), patched);
  assert.throws(
    () => disableWasm2jsUnsupportedFeatures(new Uint8Array([1, 2, 3])),
    /not a WebAssembly binary/,
  );
});

test("rewriteAsmModule exports asmFunc, throws RuntimeError on trap, and drops bufferView", () => {
  const sample = [
    "import * as $_compat_bg_js from './compat_bg.js';",
    "import * as __wbindgen_externref_xform__ from '__wbindgen_externref_xform__';",
    "var bufferView;",
    "function wasm2js_trap() { throw new Error('abort'); }",
    "function asmFunc(imports) {",
    " var HEAPU8 = new Uint8Array(16);",
    " bufferView = HEAPU8;",
    " initActiveSegments(imports);",
    " function grow() {",
    "   var newBuffer = new ArrayBuffer(32);",
    "   buffer = newBuffer;",
    "   bufferView = HEAPU8;",
    " }",
    " return { hit: wasm2js_trap };",
    "}",
    "var retasmFunc = asmFunc({});",
    "export var memory = retasmFunc.memory;",
  ].join("\n");

  const rewritten = rewriteAsmModule(sample);
  assert.doesNotMatch(rewritten, /^import /);
  assert.doesNotMatch(rewritten, /retasmFunc/);
  assert.match(rewritten, /WebAssembly\.RuntimeError\('abort'\)/);
  assert.match(
    rewritten,
    /initActiveSegments\(imports\);\n bufferView = null;/,
  );
  assert.doesNotMatch(
    rewritten,
    /buffer = newBuffer;\n {3}bufferView = HEAPU8;/,
  );
  assert.match(rewritten, /\nexport \{ asmFunc \};\n$/);

  assert.throws(
    () => rewriteAsmModule(sample.replace("wasm2js_trap()", "other()")),
    /wasm2js_trap/,
  );
});

test("rewriteGlueModule wires initAsm and refreshes memory views on buffer swap", () => {
  const sample = `/* @ts-self-types="./compat.d.ts" */
import * as import1 from "__wbindgen_externref_xform__"

function dropObject(idx) {
    if (idx < 1028) return;
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

let cachedFloat32ArrayMemory0 = null;
function getFloat32ArrayMemory0() {
    if (cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.byteLength === 0) {
        cachedFloat32ArrayMemory0 = new Float32Array(wasm.memory.buffer);
    }
    return cachedFloat32ArrayMemory0;
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    return wasm;
}

async function __wbg_load(module, imports) {}
export { initSync };
`;

  const rewritten = rewriteGlueModule(sample);
  assert.match(
    rewritten,
    /const import1 = \{ __wbindgen_externref_statics_end: \(\) => 1028 \};/,
  );
  assert.doesNotMatch(rewritten, /\.detached|\.byteLength === 0|__wbg_load/);
  assert.match(rewritten, /export function initAsm\(asmFunc\)/);

  assert.throws(
    () => rewriteGlueModule(sample.replace("idx < 1028", "idx < nope")),
    /dropObject/,
  );
});
