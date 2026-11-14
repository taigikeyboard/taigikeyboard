// The engine's view of the editor (`editor.js`): its writes, and where its caret
// is on screen. Every engine write goes through `Editor`, so Lexical's history
// and the page's storage see it.

/** @typedef {import("../types/shared.d.ts").CaretPoint} CaretPoint */
/** @typedef {import("../types/shared.d.ts").SurfaceEditor} SurfaceEditor */

export class Surface {
  /** @type {SurfaceEditor} */
  #editor;
  /** @type {HTMLElement} */
  #editorBox;
  /**
   * The editor state of the page's last write, and its snapshot.
   * @type {unknown}
   */
  #knownState = null;
  /** @type {unknown} */
  #known = null;

  /**
   * `editorBox` is `#editor`, the coordinates the panel is placed in.
   * @param {SurfaceEditor} editor
   * @param {HTMLElement} editorBox
   */
  constructor(editor, editorBox) {
    this.#editor = editor;
    this.#editorBox = editorBox;
    this.#remember();
  }

  /**
   * Everything one engine reply writes, as one undo step.
   * @param {() => void} writes
   * @returns {void}
   */
  batch(writes) {
    this.#editor.batch(writes);
    this.#remember();
  }

  /**
   * Replaces the selection, or inserts at the caret. Answers whether the
   * editor did.
   * @param {string} text
   * @returns {boolean}
   */
  insert(text) {
    const isDone = this.#editor.insertText(text);
    this.#remember();
    return isDone;
  }

  /**
   * Deletes `count` Unicode scalars before the caret.
   * @param {number} count
   * @returns {void}
   */
  deleteBefore(count) {
    this.#editor.deleteBefore(count);
    this.#remember();
  }

  /**
   * Whether anything but this page moved the caret or changed the text since
   * its last write: arrows, the mouse, paste, undo. Asked at the next key, the
   * one moment it matters.
   * @returns {boolean}
   */
  takeForeignChange() {
    const known = this.#known;
    this.#remember();
    return known !== this.#known;
  }

  /**
   * The caret's top left and its line height in `#editor`'s coordinates; with
   * `atStart`, those of the selection's start.
   * @param {boolean} [atStart]
   * @returns {CaretPoint}
   */
  caretPoint(atStart = false) {
    const selection = window.getSelection();
    const box = this.#editorBox.getBoundingClientRect();
    const fallback = { left: 0, top: 0, lineHeight: 0 };
    if (selection === null || selection.rangeCount === 0) {
      return fallback;
    }
    const range = selection.getRangeAt(0).cloneRange();
    range.collapse(atStart || selection.isCollapsed);
    /** @type {{ left: number, top: number, width?: number, height: number }} */
    let rect = range.getClientRects()[0] ?? range.getBoundingClientRect();
    const container = range.startContainer;
    const element =
      container instanceof Element ? container : container.parentElement;
    if (!element) {
      return fallback;
    }
    const style = getComputedStyle(element);
    const lineHeight = Number.parseFloat(style.lineHeight) || rect.height || 0;
    // An empty line or a block's start: Chromium and WebKit answer no box. The
    // node beside the caret (a `<br>` on an empty line) has one.
    if (rect.width === 0 && rect.height === 0 && rect.top === 0) {
      rect = this.#besideCaret(range, element, lineHeight);
    }
    // The glyph box sits inside the line, as the panel's `top` expects a line.
    const leading = (lineHeight - rect.height) / 2;
    return {
      left: rect.left - box.left,
      top: rect.top - box.top - leading,
      lineHeight,
    };
  }

  /**
   * The box of the node after the caret, or the right edge of the one before
   * it; the block's own top left when there is neither.
   * @param {Range} range
   * @param {Element} element
   * @param {number} lineHeight
   * @returns {{ left: number, top: number, height: number }}
   */
  #besideCaret(range, element, lineHeight) {
    const { startContainer: container, startOffset: offset } = range;
    // The character or node after the caret, then the one before it.
    for (const isAfter of [true, false]) {
      const probe = document.createRange();
      const at = isAfter ? offset : offset - 1;
      if (container instanceof Text) {
        if (at < 0 || at >= container.length) {
          continue;
        }
        probe.setStart(container, at);
        probe.setEnd(container, at + 1);
      } else {
        const node = container.childNodes[at];
        if (node === undefined) {
          continue;
        }
        probe.selectNode(node);
      }
      const box = probe.getClientRects()[0];
      if (box !== undefined && box.height > 0) {
        return {
          left: isAfter ? box.left : box.right,
          top: box.top,
          height: box.height,
        };
      }
    }
    const block = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return {
      left: block.left + Number.parseFloat(style.paddingLeft),
      top: block.top + Number.parseFloat(style.paddingTop),
      height: lineHeight,
    };
  }

  /**
   * An unchanged state object means nothing happened, at no cost; inside a
   * batch the committed state has not changed yet. Otherwise the snapshot, as
   * Lexical may make a new state for a selection it only normalizes.
   * @returns {void}
   */
  #remember() {
    const state = this.#editor.state();
    if (state !== this.#knownState) {
      this.#knownState = state;
      this.#known = this.#editor.snapshot();
    }
  }
}
