// The settings document and the sidebar form over it. One document, in desktop's
// shape and key spellings, is what the session reads: each form control is named
// after its key, and the markup's selected option or `checked` is that key's
// default. `data-boolean` marks a select over a stored boolean, `data-requires` a
// checkbox that greys, keeping its value, while the one it names is off. A
// section's `.setting-reset` drops its rows' keys, and the one in `data-also`, which
// no row shows. `data-keep` marks a row it leaves: as on the desktops, a reset does
// not switch the language under the user.

import { setDisplayLanguage } from "./i18n.js";

/// Also the ChromeOS input method's key in chrome.storage.sync.
export const STORAGE_KEY = "taigi.settings";

export function emptyDocument() {
  return { revision: 0, values: {} };
}

/// A stored document, or the empty one when `text` is missing or damaged.
/// Whether the session accepts it is the session's call (`reset`).
export function parseStored(text) {
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

/// The page's store. The ChromeOS options page passes its own
/// (`web/chromeos/options.js`).
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

function storedValue(control) {
  if (control.type === "checkbox") {
    return control.checked;
  }
  return "boolean" in control.dataset
    ? control.value === "true"
    : control.value;
}

export class Settings {
  #form;
  #store;
  #document;

  /// `onChange(json)`: the form changed the document. `store`: `read()` the
  /// stored document, `write(json)` a new one.
  constructor(form, onChange, store = LOCAL_STORE) {
    this.#form = form;
    this.#store = store;
    this.#document = store.read();
    this.#show();
    form.addEventListener("change", ({ target }) => {
      this.#write(target.name, storedValue(target));
      onChange(this.json);
    });
    form.addEventListener("click", ({ target }) => {
      if (target.matches(".setting-reset")) {
        this.#resetSection(target);
        onChange(this.json);
      }
    });
  }

  get json() {
    return JSON.stringify(this.#document);
  }

  /// The key panel's chord. Not a form change: the panel is looked at, so the
  /// composition stays.
  toggleTpsKeyboard() {
    const { tpsKeyboardShown } = this.#form.elements;
    this.#write("tpsKeyboardShown", !tpsKeyboardShown.checked);
  }

  /// The document as the session wrote it.
  adopt(json) {
    this.#document = JSON.parse(json);
    this.#show();
    this.#persist();
  }

  /// The session refused the document: the defaults, for good, so that
  /// `prepaint.js` does not paint from it again.
  reset() {
    this.#document = emptyDocument();
    this.#show();
    this.#persist();
  }

  #show() {
    const { values } = this.#document;
    for (const control of this.#form.querySelectorAll("[name]")) {
      const value = values[control.name];
      if (control.type === "checkbox") {
        control.checked =
          typeof value === "boolean" ? value : control.defaultChecked;
        continue;
      }
      const options = Array.from(control.options);
      const shown =
        options.find((option) => option.value === String(value)) ??
        options.find((option) => option.defaultSelected);
      shown.selected = true;
    }
    const {
      candidateAppearanceMode,
      candidateDisplayMode,
      candidateTextSize,
      fontType,
      inputMode,
      isTranslateSwapped,
      tpsKeyboardShown,
    } = this.#form.elements;
    for (const control of this.#form.querySelectorAll("[data-requires]")) {
      control.disabled = !this.#form.elements[control.dataset.requires].checked;
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
      isTyping && tpsKeyboardShown.checked,
    );
    // The stored tag, not the control's: the session reads an unknown one too.
    setDisplayLanguage(values.displayLanguage ?? "system");
  }

  #write(name, value) {
    this.#document.values[name] = value;
    this.#changed();
  }

  #resetSection(button) {
    const { values } = this.#document;
    const section = button.closest("details");
    for (const { name } of section.querySelectorAll(
      "[name]:not([data-keep])",
    )) {
      delete values[name];
    }
    delete values[button.dataset.also];
    this.#changed();
  }

  #changed() {
    this.#document.revision += 1;
    this.#show();
    this.#persist();
  }

  #persist() {
    this.#store.write(this.json);
  }
}
