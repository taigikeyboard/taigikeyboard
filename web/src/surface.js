// The engine's view of the editor (`editor.js`): its writes, and where its caret
// is on screen. Every engine write goes through `Editor`, so Lexical's history
// and the page's storage see it.

export class Surface {
  #editor;
  #editorBox;
  /// The editor state of the page's last write, and its snapshot.
  #knownState = null;
  #known = null;

  /// `editorBox` is `#editor`, the coordinates the panel is placed in.
  constructor(editor, editorBox) {
    this.#editor = editor;
    this.#editorBox = editorBox;
    this.#remember();
  }

  /// Everything one engine reply writes, as one undo step.
  batch(writes) {
    this.#editor.batch(writes);
    this.#remember();
  }

  /// Replaces the selection, or inserts at the caret. Answers whether the
  /// editor did.
  insert(text) {
    const isDone = this.#editor.insertText(text);
    this.#remember();
    return isDone;
  }

  /// Deletes `count` Unicode scalars before the caret.
  deleteBefore(count) {
    this.#editor.deleteBefore(count);
    this.#remember();
  }

  /// Whether anything but this page moved the caret or changed the text since
  /// its last write: arrows, the mouse, paste, undo. Asked at the next key, the
  /// one moment it matters.
  takeForeignChange() {
    const known = this.#known;
    this.#remember();
    return known !== this.#known;
  }

  /// The caret's top left and its line height in `#editor`'s coordinates; with
  /// `atStart`, those of the selection's start.
  caretPoint(atStart = false) {
    const selection = window.getSelection();
    const box = this.#editorBox.getBoundingClientRect();
    const fallback = { left: 0, top: 0, lineHeight: 0 };
    if (selection === null || selection.rangeCount === 0) {
      return fallback;
    }
    const range = selection.getRangeAt(0).cloneRange();
    range.collapse(atStart || selection.isCollapsed);
    let rect = range.getClientRects()[0] ?? range.getBoundingClientRect();
    const container = range.startContainer;
    const element =
      container.nodeType === Node.ELEMENT_NODE
        ? container
        : container.parentElement;
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

  /// The box of the node after the caret, or the right edge of the one before
  /// it; the block's own top left when there is neither.
  #besideCaret(range, element, lineHeight) {
    const { startContainer: container, startOffset: offset } = range;
    const isText = container.nodeType === Node.TEXT_NODE;
    // The character or node after the caret, then the one before it.
    for (const isAfter of [true, false]) {
      const probe = document.createRange();
      const at = isAfter ? offset : offset - 1;
      if (isText) {
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

  /// An unchanged state object means nothing happened, at no cost; inside a
  /// batch the committed state has not changed yet. Otherwise the snapshot, as
  /// Lexical may make a new state for a selection it only normalizes.
  #remember() {
    const state = this.#editor.state();
    if (state !== this.#knownState) {
      this.#knownState = state;
      this.#known = this.#editor.snapshot();
    }
  }
}
