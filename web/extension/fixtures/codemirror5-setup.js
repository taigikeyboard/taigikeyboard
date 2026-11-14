// Stock CodeMirror 5 (`inputStyle: "textarea"`), exposing `window.cm5Fixture`
// so tests read and seed CodeMirror's own Doc rather than scraping the DOM.

const cm = window.CodeMirror(document.getElementById("editor"), {
  value: "",
  mode: "javascript",
  lineNumbers: true,
});

window.cm5Fixture = {
  text() {
    return cm.getValue();
  },

  cursor() {
    const { line, ch } = cm.getCursor();
    return { line, ch };
  },

  setDocument(value, line, ch) {
    cm.setValue(value);
    cm.clearHistory();
    cm.setCursor({ line, ch });
    cm.focus();
  },

  setSelection(value, from, to) {
    cm.setValue(value);
    cm.clearHistory();
    cm.setSelection(from, to);
    cm.focus();
  },

  setReadOnly(value) {
    cm.setOption("readOnly", value);
    if (value !== "nocursor") {
      cm.focus();
    }
  },

  openDialogInput() {
    const bar = document.createElement("div");
    bar.className = "CodeMirror-dialog";
    const input = document.createElement("input");
    input.id = "cm-find";
    input.type = "text";
    bar.append(input);
    cm.getWrapperElement().prepend(bar);
    input.focus();
  },
};
