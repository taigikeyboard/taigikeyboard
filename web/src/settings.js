// The settings document and the sidebar form over it. One document, in desktop's
// shape and key spellings, is what the session reads: each form control is named
// after its key, and the markup's selected option or `checked` is that key's
// default. `data-boolean` marks a select over a stored boolean, `data-requires` a
// checkbox that greys, keeping its value, while the one it names is off.

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
  #document = readStored();

  /// `onChange(json)`: the form changed the document.
  constructor(form, onChange) {
    this.#form = form;
    this.#show();
    form.addEventListener("change", ({ target }) => {
      this.#write(target.name, storedValue(target));
      onChange(this.json);
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
    for (const control of this.#form.elements) {
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
    this.#document.revision += 1;
    this.#show();
    this.#persist();
  }

  #persist() {
    try {
      localStorage.setItem(STORAGE_KEY, this.json);
    } catch {
      // Private windows may refuse. The session keeps the setting for this tab.
    }
  }
}
