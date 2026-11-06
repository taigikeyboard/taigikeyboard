// Hashes a deployed site against its own `assets-manifest.json`. The service
// worker's install fails quietly on one mismatch, and a host can cause one: a CDN
// rewriting HTML, or `Content-Encoding: gzip` on a `.gz`, which `fetch` undoes
// before the page reads it. Also prints how long the download took from here.
//   node scripts/check-live.mjs <site URL> [expected build]

import { createHash } from "node:crypto";

const [site, expectedBuild] = process.argv.slice(2);
const base = new URL(site.replace(/\/?$/, "/"));
const problems = [];

const sha256 = (bytes) => createHash("sha256").update(bytes).digest("base64");

async function check(path, expected) {
  const response = await fetch(new URL(path, base));
  const bytes = new Uint8Array(await response.arrayBuffer());
  if (!response.ok) {
    problems.push(`${path}: HTTP ${response.status}`);
    return;
  }
  if (sha256(bytes) !== expected) {
    problems.push(`${path}: hash mismatch`);
  }
  const encoding = response.headers.get("content-encoding");
  if (path.endsWith(".gz") && encoding) {
    problems.push(`${path}: Content-Encoding ${encoding}`);
  }
  const type = response.headers.get("content-type");
  if (path.endsWith(".wasm") && type !== "application/wasm") {
    problems.push(`${path}: Content-Type ${type}`);
  }
}

const started = performance.now();
const response = await fetch(new URL("assets-manifest.json", base));
if (!response.ok) {
  console.error(`assets-manifest.json: HTTP ${response.status}`);
  process.exit(1);
}
const { build, shell, dictionaries } = await response.json();

if (expectedBuild !== undefined && build !== expectedBuild) {
  problems.push(`assets-manifest.json: build ${build}, not ${expectedBuild}`);
}

const worker = await (await fetch(new URL("sw.js", base))).text();
if (!worker.includes(`const BUILD = "${build}";`)) {
  problems.push(`sw.js: not build ${build}`);
}

for (const { path, sha256: expected } of shell) {
  await check(path, expected);
}
for (const { file, sha256: expected } of dictionaries) {
  await check(file, expected);
}

const seconds = ((performance.now() - started) / 1000).toFixed(1);
const count = shell.length + dictionaries.length;
console.log(`build ${build}: ${count} files in ${seconds} s`);
if (problems.length > 0) {
  console.error(problems.join("\n"));
  process.exit(1);
}
