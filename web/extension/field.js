// A website's text field as the engine sees it: which focused elements the
// extension types into, and the page's `Surface` (`surface.js`) over each kind.
// Every write is `execCommand`, the one call that keeps a field's own undo
// history and tells the page's editor (`input`, `beforeinput`) what happened.

import { Surface } from "./surface.js";

/** @typedef {import("../types/shared.d.ts").CaretPoint} CaretPoint */
/** @typedef {import("../types/shared.d.ts").SurfaceEditor} SurfaceEditor */
/**
 * @typedef {{
 *   left: number,
 *   top: number,
 *   right: number,
 *   bottom: number,
 *   width: number,
 *   height: number,
 * }} BoxRect
 */
/**
 * @typedef {(
 *   | { kind: "input", element: HTMLInputElement | HTMLTextAreaElement, anchor: Element, fontElement: Element }
 *   | { kind: "codemirror5", element: HTMLTextAreaElement, anchor: Element, fontElement: Element }
 *   | { kind: "editable", element: HTMLElement, anchor: Element, fontElement: Element }
 *   | { kind: "unsupported", element: Element, anchor: Element | null }
 * )} ClassifiedField
 */

/** `<input>` types that take plain text. */
const TEXT_TYPES = new Set(["text", "search"]);
/**
 * Types that take typing the extension leaves alone. A password is never
 * read; the others refuse the text an input method writes, or have no caret
 * to ask for.
 */
const OTHER_TYPING_TYPES = new Set([
  "password",
  "email",
  "number",
  "tel",
  "url",
]);

/**
 * The element holding the keyboard, through open shadow roots.
 * @returns {Element | null}
 */
export function focusedElement() {
  let element = document.activeElement;
  while (element?.shadowRoot?.activeElement) {
    element = element.shadowRoot.activeElement;
  }
  return element;
}

/**
 * Checks whether a CSS color string resolves to full transparency.
 * @param {string} color
 * @returns {boolean}
 */
function isTransparent(color) {
  return (
    color === "transparent" ||
    /^rgba\([^)]*,\s*0(?:\.0+)?\)$/.test(color) ||
    /\/\s*0(?:\.0+)?\)$/.test(color)
  );
}

/**
 * Reports whether an ancestor's style establishes a containing block for `position`.
 * @param {CSSStyleDeclaration} style
 * @param {string} position
 * @returns {boolean}
 */
function createsContainingBlock(style, position) {
  if (position !== "fixed" && position !== "absolute") {
    return true;
  }
  if (position === "absolute" && style.position !== "static") {
    return true;
  }
  return (
    style.transform !== "none" ||
    style.perspective !== "none" ||
    style.filter !== "none" ||
    /layout|paint|strict|content/.test(style.contain)
  );
}

/**
 * Returns the DOM parent element, crossing open shadow-root host boundaries.
 * @param {Element} element
 * @returns {Element | null}
 */
function parentOf(element) {
  if (element.parentElement) {
    return element.parentElement;
  }
  const root = element.getRootNode();
  return root instanceof ShadowRoot ? root.host : null;
}

/**
 * The part of `element` that is visible on screen, clipped by its scrolling
 * and overflow containers; `null` when it is zero-size, clipped away,
 * transparent or outside the viewport.
 * @param {Element | null | undefined} element
 * @returns {BoxRect | null}
 */
export function visibleBox(element) {
  if (!(element instanceof Element) || !element.isConnected) {
    return null;
  }
  const style = getComputedStyle(element);
  if (
    style.visibility !== "visible" ||
    Number.parseFloat(style.opacity) <= 0 ||
    style.clip === "rect(0px, 0px, 0px, 0px)" ||
    /^inset\((?:50|[5-9]\d|\d{3,})%/.test(style.clipPath) ||
    /^circle\(0(?:px|%)?[\s)]/.test(style.clipPath)
  ) {
    return null;
  }
  if (
    (element instanceof HTMLInputElement ||
      element instanceof HTMLTextAreaElement) &&
    isTransparent(style.color) &&
    isTransparent(style.caretColor)
  ) {
    return null;
  }
  const rect = element.getBoundingClientRect();
  let { left, top, right, bottom } = rect;
  if (right - left <= 1 || bottom - top <= 1) {
    return null;
  }
  let position = style.position;
  for (let parent = parentOf(element); parent; parent = parentOf(parent)) {
    const parentStyle = getComputedStyle(parent);
    if (Number.parseFloat(parentStyle.opacity) <= 0) {
      return null;
    }
    if (
      parent !== document.body &&
      parent !== document.documentElement &&
      parentStyle.display !== "contents" &&
      createsContainingBlock(parentStyle, position)
    ) {
      if (parentStyle.overflowX !== "visible") {
        const clip = parent.getBoundingClientRect();
        left = Math.max(left, clip.left);
        right = Math.min(right, clip.right);
      }
      if (parentStyle.overflowY !== "visible") {
        const clip = parent.getBoundingClientRect();
        top = Math.max(top, clip.top);
        bottom = Math.min(bottom, clip.bottom);
      }
      if (right - left <= 1 || bottom - top <= 1) {
        return null;
      }
      position = parentStyle.position;
    }
  }
  const vw = document.documentElement.clientWidth;
  const vh = document.documentElement.clientHeight;
  if (
    vw > 0 &&
    vh > 0 &&
    (right <= 0 || bottom <= 0 || left >= vw || top >= vh)
  ) {
    return null;
  }
  return {
    left,
    top,
    right,
    bottom,
    width: right - left,
    height: bottom - top,
  };
}

/**
 * The visible element a chip can sit beside for `element`, walking out to a
 * surrounding editor when `element` itself is hidden; `null` when only the
 * viewport is left.
 * @param {Element} element
 * @returns {Element | null}
 */
export function chipAnchor(element) {
  /** @type {Element | null} */
  let current = element;
  while (
    current &&
    current !== document.body &&
    current !== document.documentElement
  ) {
    if (visibleBox(current)) {
      return current;
    }
    current = parentOf(current);
  }
  return null;
}

/**
 * Builds an unsupported-field descriptor anchored to the nearest visible wrapper.
 * @param {Element} element
 * @returns {Extract<ClassifiedField, { kind: "unsupported" }>}
 */
function unsupported(element) {
  return { kind: "unsupported", element, anchor: chipAnchor(element) };
}

/**
 * Checks whether ARIA attributes mark an element as read-only or disabled.
 * @param {Element} element
 * @returns {boolean}
 */
function isAriaReadOnly(element) {
  return (
    element.getAttribute("aria-readonly") === "true" ||
    element.getAttribute("aria-disabled") === "true"
  );
}

/**
 * CodeMirror 5's `TextareaInput` parks its `<textarea>` inside a zero-height
 * wrapper directly under `.CodeMirror`; a visible `<textarea>` in a line
 * widget stays an ordinary `"input"`.
 * @param {HTMLTextAreaElement} textarea
 * @returns {Element | null}
 */
function codeMirror5Of(textarea) {
  const root = textarea.parentElement?.parentElement;
  if (
    !root?.classList.contains("CodeMirror") ||
    visibleBox(textarea) !== null ||
    !visibleBox(root)
  ) {
    return null;
  }
  return root;
}

/**
 * What `element`, the focused one, is to the extension:
 * `{ kind, element, anchor, fontElement }`, or `null` for one that takes no
 * text. `kind`: `"input"`, `"codemirror5"`, `"editable"` (`element` is the
 * editing host) or `"unsupported"`, whose keys stay the page's.
 * @param {Element | null} element
 * @returns {ClassifiedField | null}
 */
export function classify(element) {
  if (!element) {
    return null;
  }
  // A frame's keys never reach this document.
  if (element instanceof HTMLIFrameElement) {
    return unsupported(element);
  }
  // `EditContext` bypasses DOM text mutations and `execCommand` (Monaco in
  // Chromium, canvas editors).
  if (element instanceof HTMLElement && element.editContext) {
    return isAriaReadOnly(element) ? null : unsupported(element);
  }
  // Google Sheets keeps focus on a singleton cell / formula editor driven by
  // Closure key handlers (`contenteditable="false"` while navigating cells).
  const sheetsEditor = element.closest?.(
    "#waffle-rich-text-editor, [docs-unhandledkeys]",
  );
  if (sheetsEditor) {
    return isAriaReadOnly(sheetsEditor) ? null : unsupported(sheetsEditor);
  }
  if (element instanceof HTMLTextAreaElement) {
    if (element.readOnly || element.disabled) {
      return null;
    }
    const cm5 = codeMirror5Of(element);
    if (cm5) {
      return {
        kind: "codemirror5",
        element,
        anchor: cm5,
        fontElement: cm5.querySelector(".CodeMirror-code") ?? cm5,
      };
    }
    return visibleBox(element)
      ? { kind: "input", element, anchor: element, fontElement: element }
      : unsupported(element);
  }
  if (element instanceof HTMLInputElement) {
    if (element.readOnly || element.disabled) {
      return null;
    }
    if (TEXT_TYPES.has(element.type)) {
      return visibleBox(element)
        ? { kind: "input", element, anchor: element, fontElement: element }
        : unsupported(element);
    }
    return OTHER_TYPING_TYPES.has(element.type) ? unsupported(element) : null;
  }
  if (element instanceof HTMLElement && element.isContentEditable) {
    let host = element;
    while (host.parentElement?.isContentEditable) {
      host = host.parentElement;
    }
    // `getSelection` does not see into a shadow tree, and an off-screen or
    // zero-size `contenteditable` draws its text elsewhere.
    if (host.getRootNode() !== document || !visibleBox(host)) {
      return unsupported(host);
    }
    const anchor = host.closest(".cm-editor, .CodeMirror") ?? host;
    return { kind: "editable", element: host, anchor, fontElement: host };
  }
  return null;
}

/**
 * How many UTF-16 units the last `count` Unicode scalars of `text` take.
 * @param {string} text
 * @param {number} count
 * @returns {number}
 */
function unitsOfLastScalars(text, count) {
  // A scalar is at most two units.
  const tail = Array.from(text.slice(-2 * count)).slice(-count);
  return tail.join("").length;
}

/**
 * The writes on an `<input>` or a `<textarea>`, as `Surface` calls them.
 * @implements {SurfaceEditor}
 */
class InputWrites {
  /** @type {HTMLInputElement | HTMLTextAreaElement} */
  #element;
  /**
   * `deleteBefore` selected what it deletes: the commit that follows replaces
   * it, so one reply is one undo step.
   */
  #isDeletePending = false;

  /** @param {HTMLInputElement | HTMLTextAreaElement} element */
  constructor(element) {
    this.#element = element;
  }

  /**
   * Executes batched writes and commits any pending deletion.
   * @param {() => void} writes
   * @returns {void}
   */
  batch(writes) {
    writes();
    if (this.#isDeletePending) {
      this.#isDeletePending = false;
      document.execCommand("delete");
    }
  }

  /**
   * Inserts `text` at the current selection using `execCommand`.
   * @param {string} text
   * @returns {boolean}
   */
  insertText(text) {
    const element = this.#element;
    this.#isDeletePending = false;
    if (focusedElement() !== element) {
      return false;
    }
    const before = element.value;
    const start = element.selectionStart ?? 0;
    const end = element.selectionEnd ?? start;
    const selected = before.slice(start, end);
    const isDone = document.execCommand("insertText", false, text);
    return isDone && (element.value !== before || selected === text);
  }

  /**
   * Selects `count` Unicode scalars preceding the caret for pending deletion.
   * @param {number} count
   * @returns {void}
   */
  deleteBefore(count) {
    const element = this.#element;
    const caret = element.selectionStart ?? 0;
    const end = element.selectionEnd ?? caret;
    const units = unitsOfLastScalars(element.value.slice(0, caret), count);
    element.setSelectionRange(caret - units, end);
    this.#isDeletePending = true;
  }

  /**
   * Never the same twice: `Surface` then compares snapshots.
   * @returns {object}
   */
  state() {
    return {};
  }

  /** @returns {string} */
  snapshot() {
    const { selectionStart, selectionEnd, value } = this.#element;
    return `${selectionStart},${selectionEnd}\n${value}`;
  }
}

/**
 * The text node that ends where `(container, offset)` points, and its length;
 * `null` at the host's start.
 * @param {TreeWalker} walker
 * @param {Node} container
 * @param {number} offset
 * @returns {[Text, number] | null}
 */
function textBefore(walker, container, offset) {
  if (container instanceof Text) {
    return [container, offset];
  }
  const next = container.childNodes[offset];
  walker.currentNode = next ?? container;
  const node = /** @type {Text | null} */ (
    next ? walker.previousNode() : (walker.lastChild() ?? walker.previousNode())
  );
  return node ? [node, node.length] : null;
}

/**
 * The point `count` Unicode scalars before `(container, offset)` in `host`'s
 * text, or the host's first character when it has fewer; `null` with no text
 * before the point.
 * @param {HTMLElement} host
 * @param {Node} container
 * @param {number} offset
 * @param {number} count
 * @returns {[Text, number] | null}
 */
function pointBefore(host, container, offset, count) {
  const walker = document.createTreeWalker(host, NodeFilter.SHOW_TEXT);
  let found = textBefore(walker, container, offset);
  /** @type {[Text, number] | null} */
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
    const previous = /** @type {Text | null} */ (walker.previousNode());
    found = previous ? [previous, previous.length] : null;
  }
  return point;
}

/**
 * The writes on a `contenteditable` host, as `Surface` calls them.
 * @implements {SurfaceEditor}
 */
class EditableWrites {
  /** @type {HTMLElement} */
  #host;
  #isDeletePending = false;
  #didWrite = false;
  /**
   * The caret and its text as last seen, and the token that stands for them.
   * @type {[Node | null, number, Node | null, number, string | null | undefined]}
   */
  #seen = [null, 0, null, 0, undefined];
  #token = 0;

  /** @param {HTMLElement} host */
  constructor(host) {
    this.#host = host;
  }

  /**
   * Executes batched edits and reconciles selection afterwards.
   * @param {() => void} writes
   * @returns {void}
   */
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

  /**
   * The caret, when it is in this host.
   * @returns {Range | null}
   */
  #range() {
    const selection = getSelection();
    if (selection === null || selection.rangeCount === 0) {
      return null;
    }
    const range = selection.getRangeAt(0);
    return this.#host.contains(range.commonAncestorContainer) ? range : null;
  }

  /**
   * Inserts `text` into the editable host if the selection lies within it.
   * @param {string} text
   * @returns {boolean}
   */
  insertText(text) {
    this.#isDeletePending = false;
    if (this.#range() === null) {
      return false;
    }
    const isDone = document.execCommand("insertText", false, text);
    this.#didWrite ||= isDone;
    return isDone;
  }

  /**
   * Selects `count` Unicode scalars preceding the caret for deletion.
   * @param {number} count
   * @returns {void}
   */
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
    getSelection()?.setBaseAndExtent(
      ...start,
      range.endContainer,
      range.endOffset,
    );
    // `selectionchange` is otherwise queued as a task after `execCommand`:
    // editors that sync their model selection from it (CodeMirror 6's
    // `DOMObserver`, which otherwise mistakes a Mac `. ` swap for double-space
    // autocorrect) need to see the backward selection first.
    document.dispatchEvent(new Event("selectionchange"));
    this.#isDeletePending = true;
  }

  /** @returns {object} */
  state() {
    return {};
  }

  /** @returns {[Node | null, number, Node | null, number, string | null | undefined]} */
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

  /**
   * The caret's points and the text it stands in. A change elsewhere in the
   * document is no concern of the engine's, and reading a whole document on
   * every key would be.
   * @returns {number}
   */
  snapshot() {
    const seen = this.#currentSeen();
    if (seen.some((part, index) => part !== this.#seen[index])) {
      this.#seen = seen;
      this.#token += 1;
    }
    return this.#token;
  }
}

/** What a caret's place in an `<input>` or `<textarea>` depends on. */
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

/**
 * A text input or a textarea. The selection API does not reach inside one,
 * so the caret is measured on a copy of its text laid out as it is.
 */
class InputSurface extends Surface {
  /** @type {HTMLInputElement | HTMLTextAreaElement} */
  #element;
  /** @type {HTMLElement} */
  #mirror;

  /**
   * `mirror`: an element of the overlay, out of sight but laid out.
   * @param {HTMLInputElement | HTMLTextAreaElement} element
   * @param {HTMLElement} box
   * @param {HTMLElement} mirror
   */
  constructor(element, box, mirror) {
    super(new InputWrites(element), box);
    this.#element = element;
    this.#mirror = mirror;
  }

  /**
   * Measures the caret coordinates in overlay coordinates via the mirror element.
   * @override
   * @param {boolean} [atStart]
   * @returns {CaretPoint}
   */
  caretPoint(atStart = false) {
    const element = this.#element;
    const mirror = this.#mirror;
    const style = getComputedStyle(element);
    const mirrorStyle = /** @type {Record<string, string>} */ (
      /** @type {unknown} */ (mirror.style)
    );
    const computedStyle = /** @type {Record<string, string>} */ (
      /** @type {unknown} */ (style)
    );
    const isLine = element instanceof HTMLInputElement;
    for (const property of MIRRORED) {
      mirrorStyle[property] = computedStyle[property];
    }
    mirror.style.borderStyle = "solid";
    mirror.style.whiteSpace = isLine ? "pre" : "pre-wrap";
    mirror.style.overflowWrap = isLine ? "normal" : "break-word";
    const at =
      (atStart ? element.selectionStart : element.selectionEnd) ??
      element.value.length;
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

class CodeMirror5Writes extends InputWrites {
  /** @type {Element} */
  #root;

  /**
   * @param {HTMLTextAreaElement} element
   * @param {Element} root
   */
  constructor(element, root) {
    super(element);
    this.#root = root;
  }

  /**
   * @override
   * @returns {string}
   */
  snapshot() {
    const cursor = /** @type {HTMLElement | null} */ (
      this.#root.querySelector(
        ".CodeMirror-cursors > .CodeMirror-cursor:not(.CodeMirror-secondarycursor)",
      )
    );
    return `${super.snapshot()}\n${cursor?.style.cssText ?? ""}`;
  }
}

/**
 * CodeMirror 5 (`inputStyle: "textarea"`): `TextareaInput` diffs the hidden
 * `<textarea>`'s value against its `prevInput` on `input`, while the visible
 * caret is `.CodeMirror-cursor` inside `.CodeMirror`.
 */
class CodeMirror5Surface extends Surface {
  /** @type {HTMLTextAreaElement} */
  #textarea;
  /** @type {Element} */
  #root;
  /** @type {HTMLElement} */
  #box;

  /**
   * @param {HTMLTextAreaElement} textarea
   * @param {Element} root
   * @param {HTMLElement} box
   */
  constructor(textarea, root, box) {
    super(new CodeMirror5Writes(textarea, root), box);
    this.#textarea = textarea;
    this.#root = root;
    this.#box = box;
  }

  /**
   * Reads the CodeMirror 5 visual cursor or selection box relative to the overlay.
   * @override
   * @param {boolean} [atStart]
   * @returns {CaretPoint}
   */
  caretPoint(atStart = false) {
    const root = this.#root;
    const code = root.querySelector(".CodeMirror-code") ?? root;
    const scroll = root.querySelector(".CodeMirror-scroll") ?? root;
    const cursor = root.querySelector(
      ".CodeMirror-cursors > .CodeMirror-cursor:not(.CodeMirror-secondarycursor)",
    );
    const selected = cursor
      ? []
      : root.querySelectorAll(".CodeMirror-selected");
    let x;
    let y;
    let height = 0;
    if (cursor) {
      const rect = cursor.getBoundingClientRect();
      x = rect.left;
      y = rect.top;
      height = rect.height;
    } else if (selected.length > 0) {
      const pick = atStart ? selected[0] : selected[selected.length - 1];
      const rect = pick.getBoundingClientRect();
      x = atStart ? rect.left : rect.right;
      y = rect.top;
      height = rect.height;
    } else {
      const rect = (
        this.#textarea.parentElement ?? this.#textarea
      ).getBoundingClientRect();
      x = rect.left;
      y = rect.top;
    }
    const lineHeight =
      Number.parseFloat(getComputedStyle(code).lineHeight) || height || 0;
    const leading = height > 0 ? (lineHeight - height) / 2 : 0;
    const box = this.#box.getBoundingClientRect();
    const field = visibleBox(scroll) ?? scroll.getBoundingClientRect();
    return {
      left: Math.max(field.left, Math.min(x, field.right)) - box.left,
      top:
        Math.max(field.top, Math.min(y - leading, field.bottom - lineHeight)) -
        box.top,
      lineHeight,
    };
  }
}

/**
 * The surface of what `classify` answered `"input"`, `"codemirror5"` or
 * `"editable"`. `box`: the overlay's box, which covers the viewport, so a
 * caret's point in it is its point on screen.
 * @param {Exclude<ClassifiedField, { kind: "unsupported" }>} classified
 * @param {HTMLElement} box
 * @param {HTMLElement} mirror
 * @returns {Surface}
 */
export function surfaceOf(classified, box, mirror) {
  if (classified.kind === "codemirror5") {
    return new CodeMirror5Surface(classified.element, classified.anchor, box);
  }
  return classified.kind === "input"
    ? new InputSurface(classified.element, box, mirror)
    : new Surface(new EditableWrites(classified.element), box);
}
