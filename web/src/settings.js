// The settings document and the sidebar form over it. One document, in desktop's
// shape and key spellings, is what the session reads: each form control is named
// after its key, and the markup's selected option is that key's default.

import { setDisplayLanguage } from "./i18n.js";

const STORAGE_KEY = "taigi.settings";

function emptyDocument() {
  return { revision: 0, values: {} };
}

/// What a past visit stored. Whether the session accepts it is the session's
/// call (`reset`).
function readStored() {
  try {
    const stored = JSON.parse(localStorage.getItem(STORAGE_KEY));
    if (stored?.values?.constructor === Object) {
      return { ...emptyDocument(), ...stored };
    }
  } catch {
    // Blocked storage or a damaged document.
  }
  return emptyDocument();
}

export class Settings {
  #form;
  #document = readStored();

  /// `onChange(json)`: the form changed the document.
  constructor(form, onChange) {
    this.#form = form;
    this.#show();
    form.addEventListener("change", ({ target }) => {
      this.#document.values[target.name] = target.value;
      this.#document.revision += 1;
      this.#show();
      this.#persist();
      onChange(this.json);
    });
  }

  get json() {
    return JSON.stringify(this.#document);
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
    for (const control of this.#form.elements) {
      const options = Array.from(control.options);
      const shown =
        options.find((option) => option.value === values[control.name]) ??
        options.find((option) => option.defaultSelected);
      shown.selected = true;
    }
    const { candidateAppearanceMode, fontType } = this.#form.elements;
    const root = document.documentElement;
    root.dataset.theme = candidateAppearanceMode.value;
    root.dataset.font = fontType.value;
    // The stored tag, not the control's: the session reads an unknown one too.
    setDisplayLanguage(values.displayLanguage ?? "system");
  }

  #persist() {
    try {
      localStorage.setItem(STORAGE_KEY, this.json);
    } catch {
      // Private windows may refuse. The session keeps the setting for this tab.
    }
  }
}
