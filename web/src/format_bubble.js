// The format bubble: over a selection in the textarea, its buttons wrap the
// selection in Markdown markers (`markdown.js`). Absolutely placed over the
// text, with no transition, so nothing on the page moves when it shows.

import {
  linkAt,
  normalizeUrl,
  toggleBold,
  toggleBulletList,
  toggleHeading,
  toggleItalic,
  toggleLink,
  toggleNumberedList,
  toggleQuote,
  toggleUnderline,
} from "./markdown.js";

const TOGGLES = {
  h1: (...range) => toggleHeading(1, ...range),
  h2: (...range) => toggleHeading(2, ...range),
  h3: (...range) => toggleHeading(3, ...range),
  bold: toggleBold,
  italic: toggleItalic,
  underline: toggleUnderline,
  quote: toggleQuote,
  bulletList: toggleBulletList,
  numberedList: toggleNumberedList,
};
/// Between the bubble and the line it sits over.
const GAP = 6;

export class FormatBubble {
  #root;
  #url;
  #textarea;
  #surface;
  #isEngineComposing;
  /// Another input method holds a composition in the textarea.
  #isForeignComposing = false;
  /// The selection the URL field writes to and the text it was taken from;
  /// `null` while the buttons show.
  #linkTarget = null;

  /// `isEngineComposing` answers whether the engine's composition is on screen;
  /// the bubble shows over no composition, its own or another input method's.
  constructor(root, textarea, surface, isEngineComposing) {
    this.#root = root;
    this.#url = root.querySelector("input");
    this.#textarea = textarea;
    this.#surface = surface;
    this.#isEngineComposing = isEngineComposing;

    // The textarea keeps the focus, so a click on the bubble's padding is not
    // lost to its blur (`ime.js` covers every button already). The URL field
    // takes the focus.
    root.addEventListener("mousedown", (event) => {
      if (event.target !== this.#url) {
        event.preventDefault();
      }
    });
    root.addEventListener("click", (event) => {
      const button = event.target.closest("button[data-markup]");
      if (button !== null) {
        this.format(button.dataset.markup);
      }
    });
    const update = () => {
      if (this.#linkTarget === null) {
        this.show();
      }
    };
    textarea.addEventListener("select", update);
    textarea.addEventListener("keyup", update);
    // A drag can end outside the textarea.
    document.addEventListener("mouseup", update);
    const hide = () => this.hide();
    textarea.addEventListener("input", hide);
    textarea.addEventListener("scroll", hide);
    textarea.addEventListener("compositionstart", () => {
      this.#isForeignComposing = true;
      this.hide();
    });
    textarea.addEventListener("compositionend", () => {
      this.#isForeignComposing = false;
    });
    window.addEventListener("resize", hide);
    new ResizeObserver(hide).observe(textarea);
    textarea.addEventListener("blur", (event) => {
      if (!root.contains(event.relatedTarget)) {
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
      // Elsewhere on the page: the focus stays where the user put it.
      if (!root.contains(event.relatedTarget) && this.#linkTarget !== null) {
        this.hide();
      }
    });
  }

  /// Over the selection, when there is one to format.
  show() {
    const { selectionStart, selectionEnd } = this.#textarea;
    if (
      document.activeElement !== this.#textarea ||
      selectionStart === selectionEnd ||
      this.#isEngineComposing() ||
      this.#isForeignComposing
    ) {
      this.hide();
      return;
    }
    this.#root.hidden = false;
    this.#place();
  }

  hide() {
    if (this.#root.hidden && this.#linkTarget === null) {
      return;
    }
    this.#leaveLink();
    this.#root.hidden = true;
  }

  /// A button or its shortcut: a key of `TOGGLES`, or `link`. The link button
  /// again closes the URL field.
  format(kind) {
    if (kind === "link" && this.#linkTarget !== null) {
      this.#backToText();
      return;
    }
    const { value, selectionStart, selectionEnd } = this.#textarea;
    if (kind === "link") {
      this.#openLink(value, selectionStart, selectionEnd);
      return;
    }
    this.#apply(TOGGLES[kind](value, selectionStart, selectionEnd));
    this.show();
  }

  #apply(change) {
    if (change !== null) {
      const { start, end, text, selectStart, selectEnd } = change;
      this.#surface.replaceRange(start, end, text, selectStart, selectEnd);
    }
  }

  /// Above the selection's first line, below it when the editor's top is in the
  /// way, inside the editor either way.
  #place() {
    const root = this.#root;
    const editor = root.offsetParent;
    const point = this.#surface.caretPoint(this.#textarea.selectionStart);
    let top = point.top - root.offsetHeight - GAP;
    if (top < 0) {
      top = point.top + point.lineHeight + GAP;
    }
    const lowest = Math.max(0, editor.clientHeight - root.offsetHeight);
    const rightmost = Math.max(0, editor.clientWidth - root.offsetWidth);
    root.style.top = `${Math.min(Math.max(0, top), lowest)}px`;
    root.style.left = `${Math.min(Math.max(0, point.left), rightmost)}px`;
  }

  /// The bubble turns into the URL field in place, holding the link's URL when
  /// the selection is in one.
  #openLink(value, start, end) {
    this.#linkTarget = { value, start, end };
    this.#url.value = linkAt(value, start, end)?.url ?? "";
    this.#root.dataset.mode = "link";
    this.#root.hidden = false;
    this.#place();
    this.#url.focus();
  }

  #leaveLink() {
    this.#linkTarget = null;
    delete this.#root.dataset.mode;
  }

  /// An empty field takes a link off. Nothing is written when the text changed
  /// meanwhile, as another tab's save does.
  #writeLink() {
    const target = this.#linkTarget;
    const url = normalizeUrl(this.#url.value);
    if (target.value !== this.#textarea.value) {
      this.#backToText();
      return;
    }
    const change = toggleLink(target.value, target.start, target.end, url);
    this.#leaveLink();
    if (change === null) {
      this.#backToText(target);
      return;
    }
    this.#apply(change);
    this.show();
  }

  /// The textarea again, its selection as the link field found it.
  #backToText(target = this.#linkTarget) {
    this.#leaveLink();
    this.#textarea.focus();
    if (target !== null && target.value === this.#textarea.value) {
      this.#textarea.setSelectionRange(target.start, target.end);
    }
    this.show();
  }
}
