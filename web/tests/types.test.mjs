// Type-checking guards: keeps `types/site/pkg/taigi_web.d.ts` aligned with
// `wasm-bindgen`'s output and verifies every handwritten `.js` and `.mjs` file
// under `web/` belongs to at least one `tsconfig.json` project reference.

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readdirSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const WEB_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const TSC_BIN = resolve(WEB_ROOT, "node_modules/typescript/bin/tsc");

const SKIP_DIRS = new Set([
  "node_modules",
  "playwright-report",
  "target",
  "test-results",
]);

/**
 * Strips whitespace and trailing commas before closing delimiters so Biome
 * formatting differences do not obscure signature drift in `.d.ts` files.
 * @param {string} source
 * @returns {string}
 */
function normalizeDeclaration(source) {
  return source
    .replace(/,(\s*[)\]}])/g, "$1")
    .replace(/\s+/g, " ")
    .replace(/ ?([(),:;?{}<>|=]) ?/g, "$1")
    .replace(/([=:])\|/g, "$1")
    .trim();
}

/**
 * Collects every handwritten `.js` and `.mjs` file under `dir`.
 * @param {string} dir
 * @param {string[]} [found]
 * @returns {string[]}
 */
function collectHandwrittenJs(dir, found = []) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = resolve(dir, entry.name);
    if (entry.isDirectory()) {
      if (SKIP_DIRS.has(entry.name)) {
        continue;
      }
      if (full === resolve(WEB_ROOT, "src/generated")) {
        continue;
      }
      collectHandwrittenJs(full, found);
    } else if (
      entry.isFile() &&
      (entry.name.endsWith(".js") || entry.name.endsWith(".mjs"))
    ) {
      found.push(full);
    }
  }
  return found;
}

test("types/site/pkg/taigi_web.d.ts matches wasm-bindgen output", () => {
  const generated = readFileSync(
    resolve(WEB_ROOT, "target/pkg/taigi_web.d.ts"),
    "utf8",
  );
  const checkedIn = readFileSync(
    resolve(WEB_ROOT, "types/site/pkg/taigi_web.d.ts"),
    "utf8",
  );
  assert.equal(
    normalizeDeclaration(checkedIn),
    normalizeDeclaration(generated),
    "Run `make -C web build`, copy target/pkg/taigi_web.d.ts to types/site/pkg/taigi_web.d.ts, and format with Biome",
  );
});

test("every handwritten .js and .mjs file in web/ is covered by a tsconfig", () => {
  const rootConfig = /** @type {{ references?: Array<{ path: string }> }} */ (
    JSON.parse(readFileSync(resolve(WEB_ROOT, "tsconfig.json"), "utf8"))
  );
  const references = rootConfig.references ?? [];
  assert.ok(references.length > 0, "tsconfig.json has no project references");

  const makefile = readFileSync(resolve(WEB_ROOT, "Makefile"), "utf8");
  const makefileConfigs = makefile
    .match(/^TSCONFIGS\s*:=\s*(.+)$/m)?.[1]
    .trim()
    .split(/\s+/);
  assert.deepEqual(
    makefileConfigs,
    references.map((ref) => ref.path.replace(/^\.\//, "")),
    "web/Makefile TSCONFIGS must match tsconfig.json references",
  );

  /** @type {Set<string>} */
  const covered = new Set();
  for (const ref of references) {
    const configPath = resolve(WEB_ROOT, ref.path);
    const output = execFileSync(
      process.execPath,
      [TSC_BIN, "--listFilesOnly", "-p", configPath],
      { cwd: WEB_ROOT, encoding: "utf8" },
    );
    for (const line of output.split(/\r?\n/)) {
      const trimmed = line.trim();
      if (trimmed) {
        covered.add(resolve(trimmed));
      }
    }
  }

  const missing = collectHandwrittenJs(WEB_ROOT)
    .filter((file) => !covered.has(file))
    .map((file) => file.slice(WEB_ROOT.length + 1))
    .sort();
  assert.deepEqual(
    missing,
    [],
    `Unchecked JS files in web/: ${missing.join(", ")}`,
  );
});
