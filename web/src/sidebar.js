// Wide screens dock the sidebar, closed unless a past visit opened it (`prepaint.js`
// restores that), and slide it in and out; narrow ones open it as a popover drawer.

const STORAGE_KEY = "taigi.sidebar";
// The breakpoint of the docked `@media` block in style.css.
const wideScreen = window.matchMedia("(min-width: 50rem)");

function remember(state) {
  try {
    localStorage.setItem(STORAGE_KEY, state);
  } catch {
    // Private windows may refuse. The sidebar stays as it is for this tab.
  }
}

export class Sidebar {
  #toggle;
  #root;

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

  toggle() {
    this.#setOpen(!this.#isOpen());
  }

  #isOpen() {
    return wideScreen.matches
      ? document.documentElement.dataset.sidebar === "open"
      : this.#root.matches(":popover-open");
  }

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

  #syncExpanded() {
    this.#toggle.setAttribute("aria-expanded", String(this.#isOpen()));
  }
}
