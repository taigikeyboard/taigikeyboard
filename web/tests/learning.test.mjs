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

/// One Learning Records call (`learning_records.rs`), its answer parsed.
function records(session, change = {}) {
  const call = {
    kind: 0,
    order: 0,
    filter: "",
    page: 0,
    utcOffsetSeconds: 0,
    ...change,
  };
  return JSON.parse(session.learningRecords(JSON.stringify(call)));
}

function exported(session) {
  return new Map(
    session.userDataFiles().map((name) => [name, session.exportUserData(name)]),
  );
}

test("Learning Records are refused before the stores open", async () => {
  instanceCount += 1;
  const glue = await import(
    `../target/pkg/taigi_web.js?instance=${instanceCount}`
  );
  glue.initSync({ module: wasm });
  const session = new glue.Session("{}", "linux", "en");
  assert.throws(() => records(session), /the stores are not open/);
});

test("Learning Records list the picks under Word Frequency", async () => {
  const session = await startSession(kept);
  // trace: three picks of 狗岫 for `kausiu` above; its reading as the
  // engine stores it, no learned phrase from single-word commits.
  const { rows, count, empty, wrote } = records(session);
  assert.deepEqual(
    rows.map(({ tl, text, count: picks }) => [tl, text, picks]),
    [["káu-siū", "狗岫", 3]],
  );
  assert.equal(count, "1");
  assert.equal(empty, undefined);
  assert.equal(wrote, false);
  assert.equal(records(session, { filter: "tai" }).empty, "No results");
  assert.equal(records(session, { kind: 1 }).empty, "Nothing Learned Yet");
});

test("a deleted record is gone from the files the page keeps", async () => {
  const session = await startSession(kept);
  const [{ id }] = records(session).rows;
  const answer = records(session, { write: { type: "delete", id } });
  assert.equal(answer.wrote, true);
  assert.deepEqual(answer.rows, []);
  const restored = await startSession(exported(session));
  assert.deepEqual(records(restored).rows, []);
  assert.equal(top(restored, RAW), "教授");
});

test("a delete of a record no longer listed says so and writes nothing", async () => {
  const session = await startSession(kept);
  const answer = records(session, { write: { type: "delete", id: 1 } });
  assert.equal(answer.wrote, false);
  assert.equal(answer.message.title, "This Record Is No Longer Stored");
  assert.equal(answer.rows.length, 1, "the reload still lists the pick");
});

test("Delete Learning Records empties every kind", async () => {
  const session = await startSession(kept);
  const answer = records(session, { write: { type: "clear" } });
  assert.equal(answer.wrote, true);
  assert.equal(answer.message, undefined, "no receipt: the empty list says it");
  assert.deepEqual(answer.rows, []);
  assert.deepEqual(records(session, { kind: 1 }).rows, []);
  assert.equal(top(session, RAW), "教授");
});
