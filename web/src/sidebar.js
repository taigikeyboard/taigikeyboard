// Wide screens dock the sidebar, closed unless a past visit opened it (`prepaint.js`
// restores that), and slide it in and out; narrow ones open it as a popover drawer.

import { STAGED } from "./generated/build.js";

const STORAGE_KEY = "taigi.sidebar";
const COMMIT_URL = "https://github.com/taigikeyboard/taigikeyboard/commit/";
// The breakpoint of the docked `@media` block in style.css.
const wideScreen = window.matchMedia("(min-width: 50rem)");

/**
 * @param {string} state
 * @returns {void}
 */
function remember(state) {
  try {
    localStorage.setItem(STORAGE_KEY, state);
  } catch {
    // Private windows may refuse. The sidebar stays as it is for this tab.
  }
}

/**
 * The footer's build line: the commit the site was staged from, and when, in
 * the browser's locale and time zone, on a 24-hour clock.
 *
 * @param {HTMLAnchorElement} commit
 * @param {HTMLTimeElement} time
 * @returns {void}
 */
export function drawBuild(commit, time) {
  commit.textContent = STAGED.short;
  commit.href = COMMIT_URL + STAGED.commit;
  time.dateTime = STAGED.time;
  time.textContent = new Intl.DateTimeFormat(undefined, {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
    timeZoneName: "short",
  }).format(new Date(STAGED.time));
}

export class Sidebar {
  /** @type {HTMLElement} */
  #toggle;
  /** @type {HTMLElement} */
  #root;

  /**
   * @param {HTMLElement} toggle
   * @param {HTMLElement} root
   */
  constructor(toggle, root) {
    this.#toggle = toggle;
    this.#root = root;
    toggle.addEventListener("click", () => this.toggle());
    root.addEventListener("toggle", () => this.#syncExpanded());
    // A drawer left open while the window widens would sit in the top layer,
    // over the docked sidebar.
    wideScreen.addEventListener("change", () => {
      if (root.matches(":popover-open")) {
        root.hidePopover();
      }
      this.#syncExpanded();
    });
    this.#syncExpanded();
    // The state `prepaint.js` restored is drawn as it is; only a toggle slides.
    requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        document.documentElement.dataset.animated = "";
      }),
    );
  }

  /** @returns {void} */
  toggle() {
    this.#setOpen(!this.#isOpen());
  }

  /** @returns {boolean} */
  #isOpen() {
    return wideScreen.matches
      ? document.documentElement.dataset.sidebar === "open"
      : this.#root.matches(":popover-open");
  }

  /**
   * @param {boolean} isOpen
   * @returns {void}
   */
  #setOpen(isOpen) {
    if (!wideScreen.matches) {
      this.#root.togglePopover(isOpen);
      return;
    }
    if (isOpen) {
      document.documentElement.dataset.sidebar = "open";
    } else {
      delete document.documentElement.dataset.sidebar;
    }
    remember(isOpen ? "open" : "closed");
    this.#syncExpanded();
  }

  /** @returns {void} */
  #syncExpanded() {
    this.#toggle.setAttribute("aria-expanded", String(this.#isOpen()));
  }
}
