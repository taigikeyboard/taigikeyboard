// The format bubble: over a selection in the editor, its buttons format it
// (`editor.js`). Absolutely placed over the text, with no transition, so
// nothing on the page moves when it shows.

import { normalizeUrl } from "./markdown.js";

/** @typedef {import("./editor.js").Editor} Editor */
/** @typedef {import("./surface.js").Surface} Surface */
/** @typedef {import("lexical").BaseSelection} BaseSelection */

/** Between the bubble and the line it sits over. */
const GAP = 6;

/**
 * A button's `data-markup` → what it does to the editor.
 *
 * @type {Record<string, (editor: Editor) => void>}
 */
const ACTIONS = {
  h1: (editor) => editor.toggleBlock("h1"),
  h2: (editor) => editor.toggleBlock("h2"),
  h3: (editor) => editor.toggleBlock("h3"),
  bold: (editor) => editor.toggleFormat("bold"),
  italic: (editor) => editor.toggleFormat("italic"),
  underline: (editor) => editor.toggleFormat("underline"),
  quote: (editor) => editor.toggleBlock("quote"),
  bulletList: (editor) => editor.toggleList("bullet"),
  numberedList: (editor) => editor.toggleList("number"),
};

export class FormatBubble {
  /** @type {HTMLElement} */
  #root;
  /** @type {HTMLInputElement} */
  #url;
  /** @type {Editor} */
  #editor;
  /** @type {Surface} */
  #surface;
  /** @type {() => boolean} */
  #isEngineComposing;
  /** Another input method holds a composition in the editor. */
  #isForeignComposing = false;
  /**
   * The selection the URL field links and the text it was taken from; `null`
   * while the buttons show.
   *
   * @type {{ selection: BaseSelection | null, markdown: string } | null}
   */
  #linkTarget = null;

  /**
   * `isEngineComposing` answers whether the engine's composition is on screen;
   * the bubble shows over no composition, its own or another input method's.
   *
   * @param {HTMLElement} root
   * @param {Editor} editor
   * @param {Surface} surface
   * @param {() => boolean} isEngineComposing
   */
  constructor(root, editor, surface, isEngineComposing) {
    this.#root = root;
    const urlInput = root.querySelector("input");
    if (!urlInput) {
      throw new Error("missing URL input in format bubble");
    }
    this.#url = urlInput;
    this.#editor = editor;
    this.#surface = surface;
    this.#isEngineComposing = isEngineComposing;
    const text = editor.root;

    // The editor keeps the focus, so a click on the bubble's padding is not lost
    // to its blur (`ime.js` covers every button already). The URL field takes
    // the focus.
    root.addEventListener("mousedown", (event) => {
      if (event.target !== this.#url) {
        event.preventDefault();
      }
    });
    root.addEventListener("click", (event) => {
      const target = /** @type {Element | null} */ (event.target);
      const button = /** @type {HTMLButtonElement | null} */ (
        target?.closest("button[data-markup]") ?? null
      );
      if (button?.dataset.markup) {
        this.format(button.dataset.markup);
      }
    });
    const update = () => {
      if (this.#linkTarget === null) {
        this.show();
      }
    };
    editor.onSelectionChange(update);
    // A key the engine composes with changes no selection, and the bubble goes.
    text.addEventListener("keyup", update);
    // A drag can end outside the editor.
    document.addEventListener("mouseup", update);
    const hide = () => this.hide();
    text.addEventListener("scroll", hide);
    window.addEventListener("resize", hide);
    new ResizeObserver(hide).observe(text);
    text.addEventListener("compositionstart", () => {
      this.#isForeignComposing = true;
      this.hide();
    });
    text.addEventListener("compositionend", () => {
      this.#isForeignComposing = false;
    });
    text.addEventListener("blur", (event) => {
      const related = /** @type {Node | null} */ (event.relatedTarget);
      if (!root.contains(related)) {
        this.hide();
      }
    });

    this.#url.addEventListener("keydown", (event) => {
      if (event.isComposing) {
        return;
      }
      if (event.key === "Enter") {
        event.preventDefault();
        this.#writeLink();
      } else if (event.key === "Escape") {
        event.preventDefault();
        this.#backToText();
      }
    });
    this.#url.addEventListener("blur", (event) => {
      const related = /** @type {Node | null} */ (event.relatedTarget);
      // Elsewhere on the page: the focus stays where the user put it.
      if (!root.contains(related) && this.#linkTarget !== null) {
        this.hide();
      }
    });
  }

  /**
   * Over the selection, when there is one to format.
   *
   * @returns {void}
   */
  show() {
    if (
      document.activeElement !== this.#editor.root ||
      !this.#editor.hasSelection() ||
      this.#isEngineComposing() ||
      this.#isForeignComposing
    ) {
      this.hide();
      return;
    }
    this.#root.hidden = false;
    this.#place();
  }

  /** @returns {void} */
  hide() {
    if (this.#root.hidden && this.#linkTarget === null) {
      return;
    }
    this.#leaveLink();
    this.#root.hidden = true;
  }

  /**
   * A button or its shortcut: a key of `ACTIONS`, or `link`. The link button
   * again closes the URL field.
   *
   * @param {string} kind
   * @returns {void}
   */
  format(kind) {
    if (kind !== "link") {
      ACTIONS[kind]?.(this.#editor);
      this.show();
    } else if (this.#linkTarget === null) {
      this.#openLink();
    } else {
      this.#backToText();
    }
  }

  /**
   * Above the selection's first line, below it when the editor's top is in the
   * way, inside the editor either way.
   *
   * @returns {void}
   */
  #place() {
    const root = this.#root;
    const box = root.offsetParent;
    const point = this.#surface.caretPoint(true);
    if (!box || !point) {
      return;
    }
    let top = point.top - root.offsetHeight - GAP;
    if (top < 0) {
      top = point.top + point.lineHeight + GAP;
    }
    const lowest = Math.max(0, box.clientHeight - root.offsetHeight);
    const rightmost = Math.max(0, box.clientWidth - root.offsetWidth);
    root.style.top = `${Math.min(Math.max(0, top), lowest)}px`;
    root.style.left = `${Math.min(Math.max(0, point.left), rightmost)}px`;
  }

  /**
   * The bubble turns into the URL field in place, holding the link's URL when
   * the selection is in one.
   *
   * @returns {void}
   */
  #openLink() {
    const selection = this.#editor.saveSelection();
    this.#linkTarget = { selection, markdown: this.#editor.markdown() };
    this.#url.value = this.#editor.linkAt(selection) ?? "";
    this.#root.dataset.mode = "link";
    this.#root.hidden = false;
    this.#place();
    this.#url.focus();
  }

  /** @returns {void} */
  #leaveLink() {
    this.#linkTarget = null;
    delete this.#root.dataset.mode;
  }

  /**
   * An empty field takes a link off. Nothing is written when the text changed
   * meanwhile, as another tab's save does.
   *
   * @returns {void}
   */
  #writeLink() {
    const target = this.#linkTarget;
    this.#leaveLink();
    if (target === null || target.markdown !== this.#editor.markdown()) {
      this.#editor.focus();
      return;
    }
    this.#editor.restoreSelection(
      target.selection,
      normalizeUrl(this.#url.value),
    );
    this.show();
  }

  /**
   * The editor again, its selection as the link field found it.
   *
   * @returns {void}
   */
  #backToText() {
    const target = this.#linkTarget;
    this.#leaveLink();
    if (target !== null && target.markdown === this.#editor.markdown()) {
      this.#editor.restoreSelection(target.selection);
    } else {
      this.#editor.focus();
    }
    this.show();
  }
}
