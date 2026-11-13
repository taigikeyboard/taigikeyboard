// A website's text field as the engine sees it: which focused elements the
// extension types into, and the page's `Surface` (`surface.js`) over each kind.
// Every write is `execCommand`, the one call that keeps a field's own undo
// history and tells the page's editor (`input`, `beforeinput`) what happened.

import { Surface } from "./surface.js";

/// `<input>` types that take plain text.
const TEXT_TYPES = new Set(["text", "search"]);
/// Types that take typing the extension leaves alone. A password is never
/// read; the others refuse the text an input method writes, or have no caret
/// to ask for.
const OTHER_TYPING_TYPES = new Set([
  "password",
  "email",
  "number",
  "tel",
  "url",
]);

/// The element holding the keyboard, through open shadow roots.
export function focusedElement() {
  let element = document.activeElement;
  while (element?.shadowRoot?.activeElement) {
    element = element.shadowRoot.activeElement;
  }
  return element;
}

/// What `element`, the focused one, is to the extension: `{ kind, element }`,
/// or `null` for one that takes no text. `kind`: `"input"` (a text input or a
/// textarea), `"editable"` (`element` is the editing host) or `"unsupported"`,
/// whose keys stay the page's.
export function classify(element) {
  if (!element) {
    return null;
  }
  if (element instanceof HTMLTextAreaElement) {
    return element.readOnly || element.disabled
      ? null
      : { kind: "input", element };
  }
  if (element instanceof HTMLInputElement) {
    if (element.readOnly || element.disabled) {
      return null;
    }
    if (TEXT_TYPES.has(element.type)) {
      return { kind: "input", element };
    }
    return OTHER_TYPING_TYPES.has(element.type)
      ? { kind: "unsupported", element }
      : null;
  }
  // A frame's keys never reach this document.
  if (element instanceof HTMLIFrameElement) {
    return { kind: "unsupported", element };
  }
  if (element.isContentEditable) {
    let host = element;
    while (host.parentElement?.isContentEditable) {
      host = host.parentElement;
    }
    // `getSelection` does not see into a shadow tree.
    const kind = host.getRootNode() === document ? "editable" : "unsupported";
    return { kind, element: host };
  }
  return null;
}

/// How many UTF-16 units the last `count` Unicode scalars of `text` take.
function unitsOfLastScalars(text, count) {
  // A scalar is at most two units.
  const tail = Array.from(text.slice(-2 * count)).slice(-count);
  return tail.join("").length;
}

/// The writes on an `<input>` or a `<textarea>`, as `Surface` calls them.
class InputWrites {
  #element;
  /// `deleteBefore` selected what it deletes: the commit that follows replaces
  /// it, so one reply is one undo step.
  #isDeletePending = false;

  constructor(element) {
    this.#element = element;
  }

  batch(writes) {
    writes();
    if (this.#isDeletePending) {
      this.#isDeletePending = false;
      document.execCommand("delete");
    }
  }

  insertText(text) {
    const element = this.#element;
    this.#isDeletePending = false;
    if (focusedElement() !== element) {
      return false;
    }
    const before = element.value;
    const selected = before.slice(element.selectionStart, element.selectionEnd);
    const isDone = document.execCommand("insertText", false, text);
    return isDone && (element.value !== before || selected === text);
  }

  deleteBefore(count) {
    const element = this.#element;
    const caret = element.selectionStart;
    const units = unitsOfLastScalars(element.value.slice(0, caret), count);
    element.setSelectionRange(caret - units, element.selectionEnd);
    this.#isDeletePending = true;
  }

  /// Never the same twice: `Surface` then compares snapshots.
  state() {
    return {};
  }

  snapshot() {
    const { selectionStart, selectionEnd, value } = this.#element;
    return `${selectionStart},${selectionEnd}\n${value}`;
  }
}

/// The text node that ends where `(container, offset)` points, and its length;
/// `null` at the host's start.
function textBefore(walker, container, offset) {
  if (container.nodeType === Node.TEXT_NODE) {
    return [container, offset];
  }
  const next = container.childNodes[offset];
  walker.currentNode = next ?? container;
  const node = next
    ? walker.previousNode()
    : (walker.lastChild() ?? walker.previousNode());
  return node ? [node, node.length] : null;
}

/// The point `count` Unicode scalars before `(container, offset)` in `host`'s
/// text, or the host's first character when it has fewer; `null` with no text
/// before the point.
function pointBefore(host, container, offset, count) {
  const walker = document.createTreeWalker(host, NodeFilter.SHOW_TEXT);
  let found = textBefore(walker, container, offset);
  let point = null;
  let left = count;
  while (found && left > 0) {
    const [node, end] = found;
    const scalars = Array.from(node.data.slice(0, end));
    const taken = Math.min(left, scalars.length);
    const units = scalars.slice(scalars.length - taken).join("").length;
    point = [node, end - units];
    left -= taken;
    walker.currentNode = node;
    const previous = walker.previousNode();
    found = previous ? [previous, previous.length] : null;
  }
  return point;
}

/// The writes on a `contenteditable` host, as `Surface` calls them.
class EditableWrites {
  #host;
  #isDeletePending = false;
  #didWrite = false;
  /// The caret and its text as last seen, and the token that stands for them.
  #seen = [];
  #token = 0;

  constructor(host) {
    this.#host = host;
  }

  batch(writes) {
    this.#didWrite = false;
    writes();
    if (this.#isDeletePending) {
      this.#isDeletePending = false;
      this.#didWrite = document.execCommand("delete") || this.#didWrite;
    }
    if (this.#didWrite) {
      // Editors like Lexical cancel `beforeinput` and reconcile the DOM in a
      // microtask; adopt the selection their own reconciliation lands on.
      queueMicrotask(() => {
        this.#seen = this.#currentSeen();
      });
    }
  }

  /// The caret, when it is in this host.
  #range() {
    const selection = getSelection();
    if (selection === null || selection.rangeCount === 0) {
      return null;
    }
    const range = selection.getRangeAt(0);
    return this.#host.contains(range.commonAncestorContainer) ? range : null;
  }

  insertText(text) {
    this.#isDeletePending = false;
    if (this.#range() === null) {
      return false;
    }
    const isDone = document.execCommand("insertText", false, text);
    this.#didWrite ||= isDone;
    return isDone;
  }

  deleteBefore(count) {
    const range = this.#range();
    if (range === null) {
      return;
    }
    const start = pointBefore(
      this.#host,
      range.startContainer,
      range.startOffset,
      count,
    );
    if (start === null) {
      return;
    }
    getSelection().setBaseAndExtent(
      ...start,
      range.endContainer,
      range.endOffset,
    );
    this.#isDeletePending = true;
  }

  state() {
    return {};
  }

  #currentSeen() {
    const selection = getSelection();
    if (selection === null) {
      return [null, 0, null, 0, undefined];
    }
    const { anchorNode, anchorOffset, focusNode, focusOffset } = selection;
    return [
      anchorNode,
      anchorOffset,
      focusNode,
      focusOffset,
      anchorNode?.nodeValue,
    ];
  }

  /// The caret's points and the text it stands in. A change elsewhere in the
  /// document is no concern of the engine's, and reading a whole document on
  /// every key would be.
  snapshot() {
    const seen = this.#currentSeen();
    if (seen.some((part, index) => part !== this.#seen[index])) {
      this.#seen = seen;
      this.#token += 1;
    }
    return this.#token;
  }
}

/// What a caret's place in an `<input>` or `<textarea>` depends on.
const MIRRORED = [
  "boxSizing",
  "width",
  "borderTopWidth",
  "borderRightWidth",
  "borderBottomWidth",
  "borderLeftWidth",
  "paddingTop",
  "paddingRight",
  "paddingBottom",
  "paddingLeft",
  "direction",
  "fontFamily",
  "fontSize",
  "fontStyle",
  "fontVariant",
  "fontWeight",
  "fontStretch",
  "letterSpacing",
  "lineHeight",
  "tabSize",
  "textAlign",
  "textIndent",
  "textTransform",
  "wordSpacing",
];

/// A text input or a textarea. The selection API does not reach inside one,
/// so the caret is measured on a copy of its text laid out as it is.
class InputSurface extends Surface {
  #element;
  #mirror;

  /// `mirror`: an element of the overlay, out of sight but laid out.
  constructor(element, box, mirror) {
    super(new InputWrites(element), box);
    this.#element = element;
    this.#mirror = mirror;
  }

  caretPoint(atStart = false) {
    const element = this.#element;
    const mirror = this.#mirror;
    const style = getComputedStyle(element);
    const isLine = element instanceof HTMLInputElement;
    for (const property of MIRRORED) {
      mirror.style[property] = style[property];
    }
    mirror.style.borderStyle = "solid";
    mirror.style.whiteSpace = isLine ? "pre" : "pre-wrap";
    mirror.style.overflowWrap = isLine ? "normal" : "break-word";
    const at = atStart ? element.selectionStart : element.selectionEnd;
    const mark = document.createElement("span");
    // A box even at the text's end.
    mark.textContent = element.value.slice(at) || ".";
    mirror.replaceChildren(element.value.slice(0, at), mark);
    const origin = mirror.getBoundingClientRect();
    const caret = mark.getClientRects()[0] ?? mark.getBoundingClientRect();
    mirror.replaceChildren();
    const field = element.getBoundingClientRect();
    const lineHeight = Number.parseFloat(style.lineHeight) || caret.height;
    const leading = (lineHeight - caret.height) / 2;
    // An input centres its one line; a textarea starts at its padding.
    const top = isLine
      ? field.top + (field.height - lineHeight) / 2
      : field.top + caret.top - origin.top - leading - element.scrollTop;
    const left = field.left + caret.left - origin.left - element.scrollLeft;
    // A caret scrolled out of the field stays at the field's edge.
    return {
      left: Math.max(field.left, Math.min(left, field.right)),
      top: Math.max(field.top, Math.min(top, field.bottom - lineHeight)),
      lineHeight,
    };
  }
}

/// The surface of what `classify` answered `"input"` or `"editable"`. `box`:
/// the overlay's box, which covers the viewport, so a caret's point in it is
/// its point on screen.
export function surfaceOf({ kind, element }, box, mirror) {
  return kind === "input"
    ? new InputSurface(element, box, mirror)
    : new Surface(new EditableWrites(element), box);
}
