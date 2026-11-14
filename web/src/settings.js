// The settings document and the sidebar form over it. One document, in desktop's
// shape and key spellings, is what the session reads: each form control is named
// after its key, and the markup's selected option or `checked` is that key's
// default. `data-boolean` marks a select over a stored boolean, `data-requires` a
// checkbox that greys, keeping its value, while the one it names is off. A
// section's `.setting-reset` drops its rows' keys, and the one in `data-also`, which
// no row shows. `data-keep` marks a row it leaves: as on the desktops, a reset does
// not switch the language under the user.

import { setDisplayLanguage } from "./i18n.js";

/** @typedef {import("../types/shared.d.ts").SettingsDocument} SettingsDocument */
/** @typedef {import("../types/shared.d.ts").SettingsStore} SettingsStore */
/** @typedef {import("../types/shared.d.ts").SettingsValue} SettingsValue */

/** Also the ChromeOS input method's key in chrome.storage.sync. */
export const STORAGE_KEY = "taigi.settings";

/**
 * Returns a fresh empty settings document at revision 0.
 * @returns {SettingsDocument & { revision: number }}
 */
export function emptyDocument() {
  return { revision: 0, values: {} };
}

/**
 * A stored document, or the empty one when `text` is missing or damaged.
 * Whether the session accepts it is the session's call (`reset`).
 * @param {unknown} text
 * @returns {SettingsDocument}
 */
export function parseStored(text) {
  if (typeof text !== "string") {
    return emptyDocument();
  }
  try {
    const stored = JSON.parse(text);
    if (stored?.values?.constructor === Object) {
      return { ...emptyDocument(), ...stored };
    }
  } catch {
    // A damaged document.
  }
  return emptyDocument();
}

/**
 * The page's store. The ChromeOS options page passes its own
 * (`web/chromeos/options.js`).
 * @type {SettingsStore}
 */
const LOCAL_STORE = {
  read() {
    try {
      return parseStored(localStorage.getItem(STORAGE_KEY));
    } catch {
      // Blocked storage.
      return emptyDocument();
    }
  },
  write(json) {
    try {
      localStorage.setItem(STORAGE_KEY, json);
    } catch {
      // Private windows may refuse. The session keeps the setting for this tab.
    }
  },
};

/**
 * Reads the persisted value represented by a form control.
 * @param {HTMLInputElement | HTMLSelectElement} control
 * @returns {SettingsValue}
 */
function storedValue(control) {
  if (control instanceof HTMLInputElement && control.type === "checkbox") {
    return control.checked;
  }
  return "boolean" in control.dataset
    ? control.value === "true"
    : control.value;
}

export class Settings {
  /** @type {HTMLFormElement} */
  #form;
  /** @type {SettingsStore} */
  #store;
  /** @type {SettingsDocument} */
  #document;

  /**
   * `onChange(json)`: the form changed the document. `store`: `read()` the
   * stored document, `write(json)` a new one.
   * @param {HTMLFormElement} form
   * @param {(json: string) => void} onChange
   * @param {SettingsStore} [store]
   */
  constructor(form, onChange, store = LOCAL_STORE) {
    this.#form = form;
    this.#store = store;
    this.#document = store.read();
    this.#show();
    form.addEventListener("change", ({ target }) => {
      if (
        target instanceof HTMLInputElement ||
        target instanceof HTMLSelectElement
      ) {
        this.#write(target.name, storedValue(target));
        onChange(this.json);
      }
    });
    form.addEventListener("click", ({ target }) => {
      if (target instanceof HTMLElement && target.matches(".setting-reset")) {
        this.#resetSection(target);
        onChange(this.json);
      }
    });
  }

  /** @returns {string} */
  get json() {
    return JSON.stringify(this.#document);
  }

  /**
   * The key panel's chord. Not a form change: the panel is looked at, so the
   * composition stays.
   * @returns {void}
   */
  toggleTpsKeyboard() {
    const tpsKeyboardShown = /** @type {HTMLInputElement} */ (
      this.#form.elements.namedItem("tpsKeyboardShown")
    );
    this.#write("tpsKeyboardShown", !tpsKeyboardShown.checked);
  }

  /**
   * The document as the session wrote it.
   * @param {string} json
   * @returns {void}
   */
  adopt(json) {
    this.#document = /** @type {SettingsDocument} */ (JSON.parse(json));
    this.#show();
    this.#persist();
  }

  /**
   * The session refused the document: the defaults, for good, so that
   * `prepaint.js` does not paint from it again.
   * @returns {void}
   */
  reset() {
    this.#document = emptyDocument();
    this.#show();
    this.#persist();
  }

  /** @returns {void} */
  #show() {
    const { values } = this.#document;
    const controls =
      /** @type {NodeListOf<HTMLInputElement | HTMLSelectElement>} */ (
        this.#form.querySelectorAll("[name]")
      );
    for (const control of controls) {
      const value = values[control.name];
      if (control instanceof HTMLInputElement && control.type === "checkbox") {
        control.checked =
          typeof value === "boolean" ? value : control.defaultChecked;
        continue;
      }
      if (control instanceof HTMLSelectElement) {
        const options = Array.from(control.options);
        const shown =
          options.find((option) => option.value === String(value)) ??
          options.find((option) => option.defaultSelected);
        if (shown) {
          shown.selected = true;
        }
      }
    }
    const elements =
      /** @type {Record<string, HTMLInputElement | HTMLSelectElement>} */ (
        /** @type {unknown} */ (this.#form.elements)
      );
    const {
      candidateAppearanceMode,
      candidateDisplayMode,
      candidateTextSize,
      fontType,
      inputMode,
      isTranslateSwapped,
      tpsKeyboardShown,
    } = elements;
    const dependentControls =
      /** @type {NodeListOf<HTMLInputElement | HTMLSelectElement>} */ (
        this.#form.querySelectorAll("[data-requires]")
      );
    for (const control of dependentControls) {
      const requiredKey = control.dataset.requires;
      if (requiredKey) {
        const required = /** @type {HTMLInputElement} */ (
          elements[requiredKey]
        );
        control.disabled = !required.checked;
      }
    }
    // The row stays in its place outside TPS: the choice waits for the way back.
    const isTyping = inputMode.value === "tps";
    tpsKeyboardShown.disabled = !isTyping;
    // No Hanji to lead with: `allows_swap_toggle`.
    isTranslateSwapped.disabled = candidateDisplayMode.value === "romanOnly";
    const root = document.documentElement;
    root.dataset.theme = candidateAppearanceMode.value;
    root.dataset.font = fontType.value;
    root.style.setProperty("--candidate-size", `${candidateTextSize.value}px`);
    root.toggleAttribute(
      "data-tps-keyboard",
      isTyping && /** @type {HTMLInputElement} */ (tpsKeyboardShown).checked,
    );
    // No form row: only its chord flips it, and TPS types no Telex keys
    // (`is_telex_guide_wanted`).
    root.toggleAttribute(
      "data-telex-guide",
      !isTyping && values.telexGuideShown === true,
    );
    // The stored tag, not the control's: the session reads an unknown one too.
    setDisplayLanguage(
      typeof values.displayLanguage === "string"
        ? values.displayLanguage
        : "system",
    );
  }

  /**
   * @param {string} name
   * @param {SettingsValue} value
   * @returns {void}
   */
  #write(name, value) {
    this.#document.values[name] = value;
    this.#changed();
  }

  /**
   * @param {HTMLElement} button
   * @returns {void}
   */
  #resetSection(button) {
    const { values } = this.#document;
    const section = button.closest("details");
    if (section) {
      const resettable =
        /** @type {NodeListOf<HTMLInputElement | HTMLSelectElement>} */ (
          section.querySelectorAll("[name]:not([data-keep])")
        );
      for (const { name } of resettable) {
        delete values[name];
      }
    }
    if (button.dataset.also) {
      delete values[button.dataset.also];
    }
    this.#changed();
  }

  /** @returns {void} */
  #changed() {
    this.#document.revision = (this.#document.revision ?? 0) + 1;
    this.#show();
    this.#persist();
  }

  /** @returns {void} */
  #persist() {
    this.#store.write(this.json);
  }
}
