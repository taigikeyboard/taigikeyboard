// Sets up `EditContext`, key logging, and Google Sheets' off-screen <-> cell
// transitions so `hidden.spec.mjs` can assert keys stay untouched and chips
// anchor cleanly.

const log = /** @type {HTMLElement} */ (document.getElementById("log"));
for (const type of /** @type {const} */ (["keydown", "keyup"])) {
  document.addEventListener(type, (event) => {
    log.textContent += `${type}:${event.key}\n`;
  });
}

const editContextTarget = /** @type {HTMLElement} */ (
  document.getElementById("editcontext-target")
);
editContextTarget.tabIndex = 0;
editContextTarget.setAttribute("role", "textbox");
let editContextText = "";
if (typeof window.EditContext === "function") {
  const context = new window.EditContext();
  editContextTarget.editContext = context;
  context.addEventListener("textupdate", (event) => {
    editContextText =
      editContextText.slice(0, event.updateRangeStart) +
      event.text +
      editContextText.slice(event.updateRangeEnd);
  });
}

const sheetsBox = /** @type {HTMLElement} */ (
  document.getElementById("sheets-input-box")
);
const sheetsEditor = /** @type {HTMLElement} */ (
  document.getElementById("waffle-rich-text-editor")
);
sheetsEditor.tabIndex = 0;
let sheetsCellText = "";

sheetsEditor.addEventListener("keydown", (event) => {
  if (sheetsEditor.getAttribute("contenteditable") === "false") {
    if (event.key.length === 1 && !event.ctrlKey && !event.metaKey) {
      event.preventDefault();
      sheetsBox.classList.add("editing");
      sheetsEditor.setAttribute("contenteditable", "true");
      sheetsEditor.textContent = event.key;
      getSelection()?.collapse(sheetsEditor, 1);
    }
    return;
  }
  if (event.key === "Enter") {
    event.preventDefault();
    sheetsCellText = sheetsEditor.textContent;
    sheetsEditor.textContent = "";
    sheetsEditor.setAttribute("contenteditable", "false");
    sheetsBox.classList.remove("editing");
  }
});

window.hiddenFixture = {
  clearLog() {
    log.textContent = "";
  },
  editContextText() {
    return editContextText;
  },
  /** @param {boolean} value */
  setEditContextReadOnly(value) {
    editContextTarget.setAttribute("aria-readonly", value ? "true" : "false");
  },
  focusDocsIframe() {
    const frame = /** @type {HTMLIFrameElement} */ (
      document.getElementById("docs-iframe")
    );
    frame.focus();
  },
  focusSheetsGrid() {
    sheetsBox.classList.remove("editing");
    sheetsEditor.setAttribute("contenteditable", "false");
    sheetsEditor.focus();
  },
  openSheetsCell() {
    sheetsBox.classList.add("editing");
    sheetsEditor.setAttribute("contenteditable", "true");
    sheetsEditor.focus();
  },
  sheetsCellText() {
    return sheetsCellText;
  },
};
