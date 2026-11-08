// The text outlives the tab: it is kept in `localStorage` as it changes, put
// back at load, and shared by every open tab. Every page write goes through
// `execCommand`, which fires `input`, so one listener sees typing, commits,
// paste and undo. An empty text opens on the intro (`web.intro`), as ZenPen
// opens on its own: the user deletes it and writes.

const STORAGE_KEY = "taigi.text";
// A burst of keys is one write.
const SAVE_DELAY = 300;

export class Draft {
  #textarea;
  #surface;
  /// The pending save; `null` when storage holds this tab's text.
  #timer = null;
  /// The intro on show, until the first edit; then `null`.
  #intro = null;

  /// Runs after `Settings` has set the display language `intro` is in.
  constructor(textarea, surface, intro) {
    this.#textarea = textarea;
    this.#surface = surface;
    let stored = null;
    try {
      stored = localStorage.getItem(STORAGE_KEY);
    } catch {
      // Blocked storage: the intro shows.
    }
    // ZenPen's `loadState`: an empty text, stored or not, opens on the intro.
    if (stored) {
      surface.replaceAll(stored);
    } else {
      this.#showIntro(intro);
    }
    textarea.addEventListener("input", () => {
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
      if (event.newValue !== textarea.value) {
        clearTimeout(this.#timer);
        this.#timer = null;
        this.#intro = null;
        surface.replaceAll(event.newValue);
      }
    });
  }

  /// Display Language changed: an untouched intro follows it.
  translateIntro(intro) {
    if (this.#intro !== null && this.#textarea.value === this.#intro) {
      this.#showIntro(intro);
    }
  }

  #showIntro(intro) {
    this.#intro = intro;
    this.#surface.replaceAll(intro);
  }

  #flush() {
    if (this.#timer === null) {
      return;
    }
    clearTimeout(this.#timer);
    this.#timer = null;
    try {
      localStorage.setItem(STORAGE_KEY, this.#textarea.value);
    } catch {
      // Private windows and a full quota refuse. The text stays in this tab.
    }
  }
}
