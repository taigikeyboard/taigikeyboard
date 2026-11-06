// The last step of `make site`: compresses the dictionaries into the staged site
// under content-hashed names, lists them for the page, writes the service
// worker's `assets-manifest.json`, and stamps `sw.js` with the build, because a
// browser looks for a new version by comparing the bytes of that one file.
//   node scripts/stage-assets.mjs <site> <dictionaries>

import { createHash } from "node:crypto";
import { mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { gzipSync } from "node:zlib";

const [site, dictionarySources] = process.argv.slice(2);

// The file of `data-font` on `<html>` in src/index.html.
const DEFAULT_FONT = "fonts/iansui_regular.woff2";

const sha256 = (bytes) => createHash("sha256").update(bytes).digest("base64");

/// Base64 without the characters a file name cannot carry.
const nameHash = (digest) =>
  digest.replaceAll("+", "-").replaceAll("/", "_").slice(0, 16);

function stageDictionary(name) {
  const compressed = gzipSync(readFileSync(join(dictionarySources, name)), {
    level: 9,
  });
  const digest = sha256(compressed);
  const file = `dictionaries/${name}.${nameHash(digest)}.gz`;
  writeFileSync(join(site, file), compressed);
  return { name, file, sha256: digest, size: compressed.length };
}

/// `sw.js` is the browser's own to fetch.
function isHashed(path) {
  return !/^dictionaries\/|^sw\.js$/.test(path);
}

/// The default font works offline from the first visit. The others stay off the
/// first load until selected, so the service worker takes each one when it is used.
function isShell({ path }) {
  return !path.endsWith(".woff2") || path === DEFAULT_FONT;
}

function stagedFiles() {
  return readdirSync(site, { recursive: true, withFileTypes: true })
    .filter((entry) => entry.isFile())
    .map((entry) => relative(site, join(entry.parentPath, entry.name)))
    .map((path) => path.split(sep).join("/"))
    .sort();
}

mkdirSync(join(site, "dictionaries"), { recursive: true });

const dictionaries = readdirSync(dictionarySources).sort().map(stageDictionary);
writeFileSync(
  join(site, "generated/dictionaries.js"),
  `export const DICTIONARIES = ${JSON.stringify(dictionaries)};\n`,
);

const staged = stagedFiles()
  .filter(isHashed)
  .map((path) => ({ path, sha256: sha256(readFileSync(join(site, path))) }));

const worker = readFileSync(join(site, "sw.js"), "utf8");
// Fonts count: their names carry no hash, and an old one stays in the shell cache
// until the build changes.
const build = nameHash(
  sha256(JSON.stringify({ dictionaries, staged, worker })),
);

const DEV_BUILD = 'const BUILD = "dev";';
if (!worker.includes(DEV_BUILD)) {
  throw new Error(`sw.js has no \`${DEV_BUILD}\` line to stamp`);
}
writeFileSync(
  join(site, "sw.js"),
  worker.replace(DEV_BUILD, `const BUILD = "${build}";`),
);
writeFileSync(
  join(site, "assets-manifest.json"),
  `${JSON.stringify({ build, shell: staged.filter(isShell), dictionaries }, null, 2)}\n`,
);
