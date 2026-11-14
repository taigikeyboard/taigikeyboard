// What the extension draws on a website: the page's candidate panel
// (`panel.js`, its markup cut from `index.html` at build time) and a chip
// naming the mode beside the focused field. All of it lives in one shadow tree
// over the viewport, so the site's styles and the panel's never meet.

import { chipAnchor, visibleBox } from "./field.js";
import { PANEL_MARKUP } from "./generated/panel-markup.js";
import { Panel } from "./panel.js";

/** @typedef {import("./background.js").BadgeState} BadgeState */

/** Between the field and its chip. */
const CHIP_GAP = 4;

export class Overlay {
  /** @type {HTMLElement} */
  #host;
  /** @type {HTMLElement} */
  #box;
  /** @type {HTMLElement} */
  #preedit;
  /** @type {HTMLElement} */
  #chip;
  /**
   * The field or editor box the chip sits beside, `null` when only the
   * viewport is left or while the chip is hidden.
   * @type {Element | null}
   */
  #chipAnchor = null;
  /** @type {Panel} */
  panel;
  /**
   * Out of sight but laid out: `InputSurface` measures a caret on it.
   * @type {HTMLElement}
   */
  mirror;

  /**
   * `styles`: the text of `overlay.css`. `onPick`, `onTurnPage`,
   * `onScrolled`: as `Panel` takes them.
   * @param {string} styles
   * @param {(position: number) => void} onPick
   * @param {(isForward: boolean) => void} onTurnPage
   * @param {(offset: number) => void} onScrolled
   */
  constructor(styles, onPick, onTurnPage, onScrolled) {
    const host = document.createElement("taigi-keyboard");
    // Inline and important: a site's rule for every element must not move or
    // hide the overlay.
    for (const [property, value] of [
      ["position", "fixed"],
      ["inset", "0"],
      ["z-index", "2147483647"],
      ["display", "block"],
      ["margin", "0"],
      ["padding", "0"],
      ["border", "0"],
      ["pointer-events", "none"],
    ]) {
      host.style.setProperty(property, value, "important");
    }
    const root = host.attachShadow({ mode: "open" });
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(styles);
    root.adoptedStyleSheets = [sheet];
    const parsed = new DOMParser().parseFromString(PANEL_MARKUP, "text/html");
    const box = document.createElement("div");
    box.id = "box";
    box.append(...parsed.body.childNodes);
    const chip = document.createElement("div");
    chip.id = "chip";
    chip.setAttribute("role", "status");
    chip.hidden = true;
    const mirror = document.createElement("div");
    mirror.id = "mirror";
    mirror.setAttribute("aria-hidden", "true");
    box.append(chip, mirror);
    root.append(box);
    this.#host = host;
    this.#box = box;
    this.#preedit = /** @type {HTMLElement} */ (box.querySelector("#preedit"));
    this.#chip = chip;
    this.mirror = mirror;
    // Before `Panel`, which looks its parts up in the document.
    document.documentElement.append(host);
    this.panel = new Panel(
      /** @type {HTMLElement} */ (box.querySelector("#panel")),
      /** @type {HTMLElement} */ (box.querySelector("#ruler")),
      /** @type {HTMLElement} */ (box.querySelector("#spoken")),
      onPick,
      onTurnPage,
      onScrolled,
    );
  }

  /**
   * The viewport, in whose coordinates the panel is placed.
   * @returns {HTMLElement}
   */
  get box() {
    return this.#box;
  }

  /**
   * Whether `event` happened on the panel or the composition.
   * @param {Event} event
   * @returns {boolean}
   */
  holds(event) {
    return event.composedPath().includes(this.#host);
  }

  /**
   * A site can empty its document (`document.write`, a framework's first
   * render): the overlay goes back in.
   * @returns {void}
   */
  keep() {
    if (!this.#host.isConnected) {
      document.documentElement.append(this.#host);
    }
  }

  /**
   * `theme`: `"light"`, `"dark"` or anything else for the system's.
   * `candidateSize`: CSS pixels, `undefined` for the stylesheet's.
   * @param {unknown} theme
   * @param {unknown} candidateSize
   * @returns {void}
   */
  setLook(theme, candidateSize) {
    if (theme === "light" || theme === "dark") {
      this.#host.dataset.theme = theme;
    } else {
      delete this.#host.dataset.theme;
    }
    if (candidateSize === undefined) {
      this.#host.style.removeProperty("--candidate-size");
    } else {
      this.#host.style.setProperty("--candidate-size", `${candidateSize}px`);
    }
  }

  /**
   * The composition reads as the line it sits on: the font of `element`, the
   * field or the element holding the caret.
   * @param {Element} element
   * @returns {void}
   */
  setPreeditFont(element) {
    const { fontFamily, fontSize, fontStyle, fontWeight, lineHeight } =
      getComputedStyle(element);
    Object.assign(this.#preedit.style, {
      fontFamily,
      fontSize,
      fontStyle,
      fontWeight,
      lineHeight,
    });
  }

  /**
   * The chip beside `anchor` (or in the viewport's corner when `anchor` is
   * `null`): `text`, drawn as `state` (`"taigi"`, `"english"`, `"loading"`,
   * `"unsupported"` or `"failed"`).
   * @param {Element | null} anchor
   * @param {BadgeState} state
   * @param {string} text
   * @returns {void}
   */
  showChip(anchor, state, text) {
    this.#chipAnchor = anchor;
    this.#chip.dataset.state = state;
    this.#chip.textContent = text;
    this.#chip.hidden = false;
    this.placeChip();
  }

  /** @returns {void} */
  hideChip() {
    this.#chipAnchor = null;
    this.#chip.hidden = true;
  }

  /**
   * Under the field's right end; above it where the window ends first, and
   * inside its corner where neither has the room.
   * @returns {void}
   */
  placeChip() {
    const chip = this.#chip;
    if (chip.hidden) {
      return;
    }
    const anchor = this.#chipAnchor;
    /** @type {{ left: number, top: number, right: number, bottom: number } | null} */
    let field = anchor ? visibleBox(anchor) : null;
    if (field === null) {
      if (chip.dataset.state === "unsupported") {
        const fallback = anchor ? chipAnchor(anchor) : null;
        field = fallback
          ? visibleBox(fallback)
          : this.#box.getBoundingClientRect();
      } else if (anchor) {
        field = anchor.getBoundingClientRect();
      } else {
        return;
      }
    }
    if (field === null) {
      return;
    }
    const { clientWidth, clientHeight } = this.#box;
    const width = chip.offsetWidth;
    const height = chip.offsetHeight;
    let left = Math.min(field.right, clientWidth) - width;
    let top = field.bottom + CHIP_GAP;
    if (top + height > clientHeight) {
      top = field.top - height - CHIP_GAP;
      if (top < 0) {
        top = Math.min(field.bottom, clientHeight) - height - CHIP_GAP;
        left = Math.min(field.right, clientWidth) - width - CHIP_GAP;
      }
    }
    chip.style.left = `${Math.max(0, left)}px`;
    chip.style.top = `${Math.max(0, top)}px`;
  }
}
