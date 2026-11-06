// The on-screen TPS key panel: the rows the core builds from the layout table, so
// it cannot drift from what the keys type (`keys/tps_keyboard_rows.rs`). Each cap
// is a button out of the tab order: whoever can reach it by Tab has the key itself.

/// How long a typed key lights its cap (desktop TPS roadmap § Flash).
const FLASH_MS = 150;

function cell(text) {
  const span = document.createElement("span");
  span.textContent = text;
  return span;
}

export class TpsKeyboard {
  #root;
  #lit = null;
  #timer;

  /// `onPress(glyph)`: a cap was clicked. A click is the release over the cap
  /// the press began on, as on both desktop panels.
  constructor(root, onPress) {
    this.#root = root;
    // Focus stays in the textarea, which a commit needs.
    root.addEventListener("mousedown", (event) => event.preventDefault());
    root.addEventListener("click", (event) => {
      const cap = event.target.closest("button");
      if (!cap) {
        return;
      }
      const { glyph, shiftGlyph } = cap.dataset;
      const { top, height } = cap.getBoundingClientRect();
      // `detail` 0: a screen reader's activation, which has no position.
      const isTopHalf = event.detail !== 0 && event.clientY < top + height / 2;
      const isShiftLayer = event.shiftKey || isTopHalf;
      onPress(isShiftLayer && shiftGlyph ? shiftGlyph : glyph);
    });
  }

  contains(node) {
    return this.#root.contains(node);
  }

  draw(rows) {
    this.#root.replaceChildren(
      ...rows.map(({ indent, caps }) => {
        const row = document.createElement("div");
        row.className = "tps-row";
        row.style.setProperty("--indent", indent);
        row.append(
          ...caps.map(({ label, glyph, shiftGlyph }) => {
            const cap = document.createElement("button");
            cap.type = "button";
            cap.className = "tps-cap";
            cap.tabIndex = -1;
            cap.dataset.glyph = glyph;
            if (shiftGlyph) {
              cap.dataset.shiftGlyph = shiftGlyph;
            }
            cap.append(cell(label), cell(shiftGlyph ?? ""), cell(glyph));
            return cap;
          }),
        );
        return row;
      }),
    );
  }

  /// One cap at a time: a new key moves the light.
  flash(row, cap) {
    this.#lit?.removeAttribute("data-lit");
    clearTimeout(this.#timer);
    this.#lit = this.#root.children[row].children[cap];
    this.#lit.toggleAttribute("data-lit", true);
    this.#timer = setTimeout(() => {
      this.#lit.removeAttribute("data-lit");
    }, FLASH_MS);
  }
}
