// Keys as the page sends them, and what the replies say: shared by the Node
// tests, which each drive their own session.

/** @typedef {import("../types/site/pkg/taigi_web.js").Session} Session */
/** @typedef {import("../types/shared.d.ts").BrowserKey} BrowserKey */
/** @typedef {import("../types/shared.d.ts").Reply} Reply */
/** @typedef {import("../types/shared.d.ts").WireEmit} WireEmit */
/** @typedef {Extract<WireEmit, { type: "lookupTable" }>} LookupTableEmit */

/**
 * @param {string} character
 * @returns {string}
 */
function codeOf(character) {
  return /\d/.test(character)
    ? `Digit${character}`
    : `Key${character.toUpperCase()}`;
}

/**
 * @param {Session} session
 * @param {string} key
 * @param {string} code
 * @param {Partial<BrowserKey>} [modifiers]
 * @returns {Reply}
 */
export function press(session, key, code, modifiers = {}) {
  const event = JSON.stringify({ key, code, ...modifiers });
  return JSON.parse(session.keyDown(event));
}

/**
 * @param {Session} session
 * @param {string} raw
 * @returns {Reply[]}
 */
export function type(session, raw) {
  return [...raw].map((character) =>
    press(session, character, codeOf(character)),
  );
}

/**
 * @template {WireEmit["type"]} T
 * @param {Pick<Reply, "emits">} reply
 * @param {T} type
 * @returns {Extract<WireEmit, { type: T }> | undefined}
 */
export function lastOf(reply, type) {
  return /** @type {Extract<WireEmit, { type: T }> | undefined} */ (
    reply.emits.findLast((emit) => emit.type === type)
  );
}

/**
 * The first cell a slot key picks, in its leading script: past the literal
 * roman row, which takes no key.
 * @param {LookupTableEmit} table
 * @returns {string}
 */
export function topCell(table) {
  const position = table.labels.findIndex((label) => label !== "");
  return table.cells[position].text;
}

/**
 * @param {Session} session
 * @returns {void}
 */
export function cancel(session) {
  press(session, "Escape", "Escape");
}
