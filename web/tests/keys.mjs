// Keys as the page sends them, and what the replies say: shared by the Node
// tests, which each drive their own session.

function codeOf(character) {
  return /\d/.test(character)
    ? `Digit${character}`
    : `Key${character.toUpperCase()}`;
}

export function press(session, key, code, modifiers = {}) {
  const event = JSON.stringify({ key, code, ...modifiers });
  return JSON.parse(session.keyDown(event));
}

export function type(session, raw) {
  return [...raw].map((character) =>
    press(session, character, codeOf(character)),
  );
}

export function lastOf(reply, type) {
  return reply.emits.findLast((emit) => emit.type === type);
}

/// The first cell a slot key picks, in its leading script: past the literal
/// roman row, which takes no key.
export function topCell(table) {
  const position = table.labels.findIndex((label) => label !== "");
  return table.cells[position].text;
}

export function cancel(session) {
  press(session, "Escape", "Escape");
}
