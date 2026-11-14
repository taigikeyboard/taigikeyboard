// Stock CodeMirror 6 editor, bundled by `make extension-fixtures` from the
// pinned `@codemirror/*` packages and exposing `window.cm6Fixture` so tests
// read and seed `EditorState` directly rather than scraping the DOM.

import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { Compartment, EditorState } from "@codemirror/state";
import { EditorView, keymap, lineNumbers } from "@codemirror/view";

const editableCompartment = new Compartment();

/**
 * Creates a CodeMirror 6 state with standard editing extensions and optional selection.
 * @param {string} [doc]
 * @param {{ anchor: number, head?: number }} [selection]
 * @returns {EditorState}
 */
function makeState(doc = "", selection = undefined) {
  return EditorState.create({
    doc,
    selection,
    extensions: [
      lineNumbers(),
      history(),
      keymap.of([...defaultKeymap, ...historyKeymap]),
      editableCompartment.of(EditorView.editable.of(true)),
    ],
  });
}

const view = new EditorView({
  state: makeState(),
  parent: /** @type {HTMLElement} */ (document.getElementById("editor")),
});

/**
 * Waits one animation frame and macrotask so DOM selection updates settle.
 * @returns {Promise<void>}
 */
const settle = () =>
  new Promise((resolve) => requestAnimationFrame(() => setTimeout(resolve)));

window.cm6Fixture = {
  text() {
    return view.state.doc.toString();
  },

  selection() {
    const { anchor, head } = view.state.selection.main;
    return { anchor, head };
  },

  /**
   * @param {string} doc
   * @param {number} [pos]
   */
  setDocument(doc, pos = doc.length) {
    view.setState(makeState(doc, { anchor: pos }));
    view.focus();
    return settle();
  },

  /**
   * @param {string} doc
   * @param {number} anchor
   * @param {number} head
   */
  setSelectedText(doc, anchor, head) {
    view.setState(makeState(doc, { anchor, head }));
    view.focus();
    return settle();
  },
};
