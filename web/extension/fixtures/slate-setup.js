// Stock Slate editor shaped like a chat message box (Discord's is Slate):
// Enter sends, Shift+Enter breaks the line. Bundled by `make
// extension-fixtures` from the pinned `slate` / `slate-react` packages and
// exposing `window.slateFixture`, so tests read Slate's own model rather than
// the DOM. Slate edits its model from `beforeinput` alone; text that reaches
// only the DOM is not in the message.

import { createElement, useMemo } from "react";
import { createRoot } from "react-dom/client";
import { createEditor, Editor, Node, Transforms } from "slate";
import { withHistory } from "slate-history";
import { Editable, Slate, withReact } from "slate-react";

/** @typedef {import("slate").Descendant} Descendant */
/** @typedef {import("slate").Point} Point */

/** @returns {Descendant[]} */
const emptyValue = () => [{ children: [{ text: "" }] }];

const editor = withHistory(withReact(createEditor()));

/** @type {string[]} */
const sent = [];

/** @returns {string} */
const textOf = () =>
  editor.children.map((node) => Node.string(node)).join("\n");

/**
 * The offset of `point` in `textOf()`.
 * @param {Point} point
 * @returns {number}
 */
function offsetOf(point) {
  let offset = 0;
  for (let line = 0; line < point.path[0]; line += 1) {
    offset += Node.string(editor.children[line]).length + 1;
  }
  return offset + point.offset;
}

/**
 * The editor's own keys, as a chat box has them.
 * @param {import("react").KeyboardEvent} event
 * @returns {void}
 */
function onKeyDown(event) {
  if (event.key !== "Enter") {
    return;
  }
  event.preventDefault();
  if (event.shiftKey) {
    Transforms.splitNodes(editor, { always: true });
    return;
  }
  sent.push(textOf());
  Transforms.delete(editor, {
    at: { anchor: Editor.start(editor, []), focus: Editor.end(editor, []) },
  });
}

function App() {
  const initialValue = useMemo(emptyValue, []);
  return createElement(Slate, {
    editor,
    initialValue,
    // biome-ignore lint/correctness/noChildrenProp: `@types/react` types `Slate`'s `children` as a required prop, so tsc refuses it as a third argument.
    children: createElement(Editable, {
      id: "editor",
      placeholder: "Message #general",
      onKeyDown,
    }),
  });
}

createRoot(/** @type {HTMLElement} */ (document.getElementById("root"))).render(
  createElement(App),
);

/**
 * Waits one animation frame and macrotask so Slate's DOM selection settles.
 * @returns {Promise<void>}
 */
const settle = () =>
  new Promise((resolve) => requestAnimationFrame(() => setTimeout(resolve)));

window.slateFixture = {
  text: textOf,

  sent() {
    return [...sent];
  },

  selection() {
    const { selection } = editor;
    return selection
      ? { anchor: offsetOf(selection.anchor), focus: offsetOf(selection.focus) }
      : null;
  },

  /**
   * One line of `value`, with `start`…`end` selected.
   * @param {string} value
   * @param {number} start
   * @param {number} end
   */
  setSelectedText(value, start, end) {
    Transforms.delete(editor, {
      at: { anchor: Editor.start(editor, []), focus: Editor.end(editor, []) },
    });
    Transforms.insertText(editor, value, { at: Editor.start(editor, []) });
    Transforms.select(editor, {
      anchor: { path: [0, 0], offset: start },
      focus: { path: [0, 0], offset: end },
    });
    return settle();
  },
};
