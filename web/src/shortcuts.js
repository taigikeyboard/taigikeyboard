// The sidebar's Shortcuts section, drawn from `Session.shortcuts` (`shortcuts.rs`):
// blocks of rows that read, with no control to change a key.

/** @typedef {import("../types/shared.d.ts").WireShortcutGroup} WireShortcutGroup */
/** @typedef {import("../types/shared.d.ts").WireShortcutRow} WireShortcutRow */

/**
 * @param {keyof HTMLElementTagNameMap} tag
 * @param {string} className
 * @param {string} [text]
 * @returns {HTMLElement}
 */
function element(tag, className, text) {
  const node = document.createElement(tag);
  node.className = className;
  if (text !== undefined) {
    node.textContent = text;
  }
  return node;
}

/**
 * @param {WireShortcutRow} entry
 * @returns {HTMLElement}
 */
function row({ label, keys, fixed_keys: fixedKeys }) {
  const name = element("span", "", label);
  if (fixedKeys) {
    name.append(element("small", "", fixedKeys));
  }
  const line = element("div", "setting");
  line.append(name, element("span", "setting-keys", keys));
  return line;
}

/**
 * @param {HTMLElement} root
 * @param {WireShortcutGroup[]} groups
 * @returns {void}
 */
export function drawShortcuts(root, groups) {
  root.replaceChildren(
    ...groups.flatMap(({ title, rows }) => [
      element("h3", "setting-group", title),
      ...rows.map(row),
    ]),
  );
}
