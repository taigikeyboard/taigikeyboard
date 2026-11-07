// Learning on the web (docs/architecture/web-roadmap.md § Phase 8): the engine
// keeps a pick in SQLite's memory VFS, the page takes the files out, and a new
// instance that is handed them before it opens ranks the same way. Each
// instance is its own copy of the glue, as `loader.js` makes them.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { cancel, lastOf, press, topCell, type } from "./keys.mjs";

const dictionaries = new URL("../../assets/dictionaries/", import.meta.url);
const wasm = readFileSync(
  new URL("../target/pkg/taigi_web_bg.wasm", import.meta.url),
);

let instanceCount = 0;

/// A session on a new instance with the dictionaries in, its own copy of the
/// glue as `loader.js` makes one; `files` are handed
/// in before the stores open.
async function startSession(files = new Map()) {
  instanceCount += 1;
  const glue = await import(
    `../target/pkg/taigi_web.js?instance=${instanceCount}`
  );
  glue.initSync({ module: wasm });
  const session = new glue.Session("{}", "linux", "en");
  for (const [name, bytes] of files) {
    session.importUserData(name, bytes);
  }
  assert.equal(session.openUserData(), true);
  for (const name of session.dictionaryFiles()) {
    session.addDictionary(name, readFileSync(new URL(name, dictionaries)));
  }
  session.install();
  return session;
}

function lastTable(replies) {
  return lastOf(
    { emits: replies.flatMap((reply) => reply.emits) },
    "lookupTable",
  );
}

/// Types `raw`, clicks the cell showing `hanji` and commits it with Enter.
function pick(session, raw, hanji) {
  const table = lastTable(type(session, raw));
  const position = table.cells.findIndex((cell) => cell.text === hanji);
  assert.ok(position >= 0, `${hanji} is on the first page for ${raw}`);
  session.clickCandidate(position);
  assert.equal(lastOf(press(session, "Enter", "Enter"), "commit").text, hanji);
}

function top(session, raw) {
  const cell = topCell(lastTable(type(session, raw)));
  cancel(session);
  return cell;
}

// trace: engine/composing/tests/walker_fixed_inputs_prod.rs ranks 教授 first
// for `kausiu`; 狗岫 is a lower cell on the same page (web-roadmap § Spike
// 2026-11-07: three picks put it first).
const RAW = "kausiu";
const PICKED = "狗岫";

let kept;

test("picks are learned and come out as the four files", async () => {
  const session = await startSession();
  assert.equal(top(session, RAW), "教授");
  for (let round = 0; round < 3; round += 1) {
    pick(session, RAW, PICKED);
  }
  assert.equal(top(session, RAW), PICKED);
  kept = new Map(
    session.userDataFiles().map((name) => [name, session.exportUserData(name)]),
  );
  assert.equal(kept.size, 4);
  for (const [name, bytes] of kept) {
    assert.equal(
      new TextDecoder().decode(bytes.subarray(0, 15)),
      "SQLite format 3",
      `${name} is a database file`,
    );
  }
});

test("a new instance handed the files ranks the pick first", async () => {
  const session = await startSession(kept);
  assert.equal(top(session, RAW), PICKED);
});

test("an instance with nothing handed in starts from the dictionaries", async () => {
  const session = await startSession();
  assert.equal(top(session, RAW), "教授");
});

test("an open file and an unknown name are refused", async () => {
  const session = await startSession();
  const [name] = session.userDataFiles();
  assert.throws(() => session.importUserData(name, kept.get(name)));
  assert.throws(() => session.exportUserData("settings.json"));
});
