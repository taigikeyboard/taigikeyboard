// Shared learning across tabs, and private-window isolation
// (web/extension/learning-client.js).

import assert from "node:assert/strict";
import { test } from "node:test";

import { SharedLearning } from "../extension/learning-client.js";

/** @typedef {import("../types/site/pkg/taigi_web.js").Session} Session */
/** @typedef {import("../extension/learning-store.js").EncodedFiles} EncodedFiles */
/** @typedef {{ type: "taigi.learning.read" }} ReadRequest */
/** @typedef {{ type: "taigi.learning.generation" }} GenerationRequest */
/** @typedef {{ type: "taigi.learning.write", base: number, files: EncodedFiles }} WriteRequest */
/** @typedef {ReadRequest | GenerationRequest | WriteRequest} LearningRequest */

/**
 * @param {Record<string, Uint8Array>} [files]
 */
function fakeSession(files = { "a.db": new Uint8Array([1, 2]) }) {
  /** @type {Record<string, Uint8Array>} */
  const imported = {};
  let opened = false;
  return {
    imported,
    get isOpen() {
      return opened;
    },
    userDataFiles: () => Object.keys(files),
    /**
     * @param {string} name
     * @param {Uint8Array} bytes
     */
    importUserData(name, bytes) {
      imported[name] = bytes;
    },
    openUserData() {
      opened = true;
      return true;
    },
    /**
     * @param {string} name
     * @returns {Uint8Array}
     */
    exportUserData(name) {
      return files[name];
    },
  };
}

test("a private tab opens its own stores and never asks the service worker", async () => {
  /** @type {unknown[]} */
  const asked = [];
  const fake = fakeSession();
  const session = /** @type {Session & typeof fake} */ (
    /** @type {unknown} */ (fake)
  );
  const learning = new SharedLearning({
    current: () => session,
    run: (action) => action(session),
    ask: async (message) => {
      asked.push(message);
      return undefined;
    },
    isPrivate: true,
    onStale: () => {
      throw new Error("a private tab cannot go stale");
    },
  });
  await learning.attach(session);
  assert.equal(session.isOpen, true);
  assert.deepEqual(session.imported, {});
  learning.scheduleSave();
  await new Promise((resolve) => setTimeout(resolve, 10));
  assert.equal(await learning.isStale(), false);
  assert.deepEqual(asked, []);
});

test("a normal tab sends only changed files and restarts on a conflict", async () => {
  /** @type {Record<string, Uint8Array>} */
  const files = {
    "a.db": new Uint8Array([1]),
    "b.db": new Uint8Array([2]),
  };
  const fake = fakeSession(files);
  const session = /** @type {Session & typeof fake} */ (
    /** @type {unknown} */ (fake)
  );
  /** @type {WriteRequest[]} */
  const writes = [];
  let generation = 3;
  let staleCount = 0;
  let acceptWrite = true;

  const learning = new SharedLearning({
    current: () => session,
    run: (action) => action(session),
    ask: async (rawMessage) => {
      const message = /** @type {LearningRequest} */ (rawMessage);
      if (message.type === "taigi.learning.read") {
        return {
          generation,
          files: { "a.db": btoa(String.fromCharCode(1)) },
        };
      }
      if (message.type === "taigi.learning.generation") {
        return generation;
      }
      if (message.type === "taigi.learning.write") {
        writes.push(message);
        if (!acceptWrite) {
          return { isTaken: false, generation: generation + 1 };
        }
        generation = message.base + 1;
        return { isTaken: true, generation };
      }
      throw new Error(
        `unexpected ${/** @type {{ type: string }} */ (message).type}`,
      );
    },
    isPrivate: false,
    onStale: () => {
      staleCount += 1;
    },
  });

  await learning.attach(session);
  assert.deepEqual(session.imported["a.db"], new Uint8Array([1]));
  assert.equal(await learning.isStale(), false);

  // Only `b.db` differs from what `read` handed in.
  learning.scheduleSave();
  await new Promise((resolve) => setTimeout(resolve, 10));
  assert.equal(writes.length, 1);
  assert.equal(writes[0].base, 3);
  assert.deepEqual(Object.keys(writes[0].files), ["b.db"]);
  assert.equal(staleCount, 0);

  // Another tab moved ahead before this tab's next write.
  files["a.db"] = new Uint8Array([9]);
  acceptWrite = false;
  learning.scheduleSave();
  await new Promise((resolve) => setTimeout(resolve, 10));
  assert.equal(writes.length, 2);
  assert.equal(writes[1].base, 4);
  assert.equal(staleCount, 1);
});
