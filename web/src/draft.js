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
  /// What the textarea holds for `#intro`: its signature padded right.
  #introShown = null;

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
    // A new width moves the right edge the signature sits on.
    new ResizeObserver(() => this.translateIntro(this.#intro)).observe(
      textarea,
    );
  }

  /// Display Language changed, or the width: an untouched intro follows it.
  translateIntro(intro) {
    if (this.#intro !== null && this.#textarea.value === this.#introShown) {
      this.#showIntro(intro);
    }
  }

  #showIntro(intro) {
    this.#intro = intro;
    this.#introShown = this.#signedRight(intro);
    this.#surface.replaceAll(this.#introShown);
  }

  /// The intro's last line, the maintainer's signature, pushed to the right
  /// edge with spaces: a textarea aligns no single line. The spaces are what
  /// fits the current width, one short so the line never wraps.
  #signedRight(intro) {
    const textarea = this.#textarea;
    const style = getComputedStyle(textarea);
    const room =
      textarea.clientWidth -
      Number.parseFloat(style.paddingLeft) -
      Number.parseFloat(style.paddingRight);
    const breakAt = intro.lastIndexOf("\n") + 1;
    const signature = intro.slice(breakAt);
    const spaceWidth = this.#width(" ".repeat(100), style) / 100;
    const free = room - this.#width(signature, style);
    const count = Math.max(0, Math.floor(free / spaceWidth) - 1);
    return intro.slice(0, breakAt) + " ".repeat(count) + signature;
  }

  /// `text` laid out in the textarea's font. A canvas resolves `-apple-system`
  /// to another face in WebKit, so the page measures itself.
  #width(text, style) {
    const probe = document.createElement("span");
    probe.style.cssText =
      "position:absolute;visibility:hidden;white-space:pre;top:0;left:0";
    for (const name of ["fontFamily", "fontSize", "fontStyle", "fontWeight"]) {
      probe.style[name] = style[name];
    }
    probe.textContent = text;
    document.body.append(probe);
    const { width } = probe.getBoundingClientRect();
    probe.remove();
    return width;
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
