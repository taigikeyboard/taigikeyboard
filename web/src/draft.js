// The text outlives the tab: its Markdown is kept in `localStorage` as it
// changes, put back at load, and shared by every open tab. An empty text opens
// on the intro (`web.intro`), as ZenPen opens on its own: the user deletes it
// and writes. Its last line, the maintainer's signature, sits against the right
// edge.

import { LOAD_TAG } from "./editor.js";
import { fromPlainText } from "./markdown.js";

/// The Markdown. `taigi.text`, the textarea's plain text before it, is read once
/// when this is missing and left as it was.
const STORAGE_KEY = "taigi.document";
const PLAIN_TEXT_KEY = "taigi.text";
// A burst of keys is one write.
const SAVE_DELAY = 300;

export class Draft {
  #editor;
  /// The pending save; `null` when storage holds this tab's text.
  #timer = null;
  /// The intro's Markdown while it is untouched; then `null`.
  #intro = null;

  /// Runs after `Settings` has set the display language `intro` is in.
  constructor(editor, intro) {
    this.#editor = editor;
    let stored = null;
    try {
      stored = localStorage.getItem(STORAGE_KEY);
      // Not the empty string: a text the user emptied stays empty.
      if (stored === null) {
        const plain = localStorage.getItem(PLAIN_TEXT_KEY);
        stored = plain === null ? null : fromPlainText(plain);
      }
    } catch {
      // Blocked storage: the intro shows.
    }
    // ZenPen's `loadState`: an empty text, stored or not, opens on the intro.
    if (stored) {
      editor.setMarkdown(stored);
    } else {
      this.#showIntro(intro);
    }
    editor.onTextChange((tags) => {
      if (tags.has(LOAD_TAG)) {
        return;
      }
      this.#intro = null;
      clearTimeout(this.#timer);
      this.#timer = setTimeout(() => this.#flush(), SAVE_DELAY);
    });
    // `localStorage` is synchronous, so a write on the way out completes. A
    // hidden tab may be discarded without `pagehide`.
    window.addEventListener("pagehide", () => this.#flush());
    document.addEventListener("visibilitychange", () => {
      if (document.visibilityState === "hidden") {
        this.#flush();
      }
    });
    // Another tab saved: this one takes its text, so its next key does not write
    // an older copy over it.
    window.addEventListener("storage", (event) => {
      if (event.key !== STORAGE_KEY || event.newValue === null) {
        return;
      }
      if (event.newValue !== editor.markdown()) {
        clearTimeout(this.#timer);
        this.#timer = null;
        this.#intro = null;
        editor.setMarkdown(event.newValue);
      }
    });
  }

  /// Display Language changed: an untouched intro follows it.
  translateIntro(intro) {
    if (this.#intro !== null && this.#editor.markdown() === this.#intro) {
      this.#showIntro(intro);
    }
  }

  #showIntro(intro) {
    this.#editor.setMarkdown(intro, { alignLastRight: true });
    this.#intro = this.#editor.markdown();
  }

  #flush() {
    if (this.#timer === null) {
      return;
    }
    clearTimeout(this.#timer);
    this.#timer = null;
    try {
      localStorage.setItem(STORAGE_KEY, this.#editor.markdown());
    } catch {
      // Private windows and a full quota refuse. The text stays in this tab.
    }
  }
}
