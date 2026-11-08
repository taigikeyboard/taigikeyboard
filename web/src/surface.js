// The textarea: every write the page makes, and where its caret is on screen.
//
// `execCommand` is deprecated and is the only call that keeps the native undo
// history (docs/architecture/web-roadmap.md § Text surface). It works only with
// the textarea focused, which `#write` makes sure of.

// What makes the mirror wrap exactly as the textarea does.
const MIRRORED_STYLES = [
  "fontFamily",
  "fontSize",
  "fontStyle",
  "fontWeight",
  "letterSpacing",
  "lineHeight",
  "overflowWrap",
  "paddingBottom",
  "paddingLeft",
  "paddingRight",
  "paddingTop",
  "tabSize",
  "textIndent",
  "textTransform",
  "wordBreak",
  "wordSpacing",
];

export class Surface {
  #textarea;
  #mirror;
  #caretMark;
  #known;

  constructor(textarea) {
    this.#textarea = textarea;
    this.#mirror = document.createElement("div");
    this.#mirror.setAttribute("aria-hidden", "true");
    this.#mirror.style.position = "absolute";
    this.#mirror.style.visibility = "hidden";
    this.#mirror.style.top = "0";
    // Laid out, but it must not lengthen the page.
    this.#mirror.style.height = "0";
    this.#mirror.style.overflow = "hidden";
    this.#mirror.style.left = "0";
    this.#caretMark = document.createElement("span");
    this.#caretMark.textContent = "​";
    textarea.after(this.#mirror);
    this.#remember();
  }

  /// Replaces the selection, or inserts at the caret. Answers whether the
  /// browser did.
  insert(text) {
    return this.#write("insertText", text);
  }

  /// Deletes `count` Unicode scalars before the caret.
  deleteBefore(count) {
    const end = this.#textarea.selectionStart;
    // At most two code units a scalar.
    const tail = this.#textarea.value.slice(Math.max(0, end - count * 2), end);
    const removed = Array.from(tail).slice(-count).join("");
    this.#textarea.setSelectionRange(end - removed.length, end);
    this.#write("delete");
  }

  /// The whole text, from outside this tab's editing: the stored draft at load,
  /// another tab's save. No `execCommand`, so the undo history starts over, and
  /// the caret goes to the end. Not remembered: the next key sees it as a
  /// foreign change.
  replaceAll(text) {
    this.#textarea.value = text;
    this.#textarea.setSelectionRange(text.length, text.length);
  }

  /// Whether anything but this page moved the caret or changed the text since
  /// its last write: arrows, the mouse, paste, undo. Asked at the next key, the
  /// one moment it matters, which `selectionchange` would answer too late.
  takeForeignChange() {
    const textarea = this.#textarea;
    const known = this.#known;
    const isSame =
      textarea.selectionStart === known.start &&
      textarea.selectionEnd === known.end &&
      textarea.value === known.value;
    this.#remember();
    return !isSame;
  }

  /// The caret's top left and its line height, in the editor's coordinates.
  caretPoint() {
    const textarea = this.#textarea;
    const style = getComputedStyle(textarea);
    const mirror = this.#mirror;
    for (const name of MIRRORED_STYLES) {
      mirror.style[name] = style[name];
    }
    mirror.style.boxSizing = "border-box";
    mirror.style.width = `${textarea.clientWidth}px`;
    mirror.style.whiteSpace = "pre-wrap";
    const { value, selectionStart } = textarea;
    // The rest of the word, which decides whether the line wraps before it.
    const [wordRest] = value.slice(selectionStart).match(/^\S*/);
    const before = value.slice(0, selectionStart);
    mirror.replaceChildren(before, this.#caretMark, wordRest);
    const mark = this.#caretMark;
    // The mark's box is the font's, inside the line's.
    const lineHeight = Number.parseFloat(style.lineHeight);
    const leading = (lineHeight - mark.offsetHeight) / 2;
    return {
      left:
        textarea.offsetLeft +
        textarea.clientLeft +
        mark.offsetLeft -
        textarea.scrollLeft,
      top:
        textarea.offsetTop +
        textarea.clientTop +
        mark.offsetTop -
        leading -
        textarea.scrollTop,
      lineHeight,
    };
  }

  #write(command, value) {
    this.#textarea.focus();
    const isDone = document.execCommand(command, false, value);
    this.#remember();
    return isDone;
  }

  #remember() {
    this.#known = {
      start: this.#textarea.selectionStart,
      end: this.#textarea.selectionEnd,
      value: this.#textarea.value,
    };
  }
}
