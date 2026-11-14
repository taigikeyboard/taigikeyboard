// The Telex key table as a card of its own, the macOS one (`TelexGuidePanel.swift`):
// the scheme's name over two columns, key and meaning. It stays up while the
// setting `telexGuideShown` does and TPS is not typed (`Settings`), beside the
// candidate list rather than in its place. The rows are the core's
// (`keys/telex_guide_rows.rs`), spelled in the display language.

/** @typedef {import("../types/shared.d.ts").WireTelexGuide} WireTelexGuide */

/**
 * @param {keyof HTMLElementTagNameMap} tag
 * @param {string} text
 * @returns {HTMLElement}
 */
function cell(tag, text) {
  const element = document.createElement(tag);
  element.textContent = text;
  return element;
}

export class TelexGuide {
  /** @type {HTMLElement} */
  #root;

  /** @param {HTMLElement} root */
  constructor(root) {
    this.#root = root;
  }

  /**
   * `guide`: the session's `telexGuide()`. Again on a language change.
   *
   * @param {WireTelexGuide} guide
   * @returns {void}
   */
  draw({ title, rows }) {
    const table = document.createElement("dl");
    table.append(
      ...rows.flatMap(({ key, meaning }) => [
        cell("dt", key),
        cell("dd", meaning),
      ]),
    );
    this.#root.replaceChildren(cell("h2", title), table);
    this.#root.toggleAttribute("data-drawn", true);
  }
}
