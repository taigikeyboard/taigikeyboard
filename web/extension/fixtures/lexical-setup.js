// Stock Lexical rich-text editor, exposing `window.lexicalFixture` so tests
// inspect and seed Lexical's own EditorState rather than touching the DOM.

import {
  $createHeadingNode,
  $createParagraphNode,
  $createTextNode,
  $getRoot,
  $getSelection,
  $isElementNode,
  $isHeadingNode,
  $isRangeSelection,
  $isTextNode,
  createEditor,
  createEmptyHistoryState,
  FORMAT_TEXT_COMMAND,
  HeadingNode,
  LinkNode,
  ListItemNode,
  ListNode,
  QuoteNode,
  registerHistory,
  registerList,
  registerRichText,
} from "./lexical.js";

const editor = createEditor({
  namespace: "fixture",
  nodes: [HeadingNode, QuoteNode, ListNode, ListItemNode, LinkNode],
  onError(error) {
    throw error;
  },
});

registerRichText(editor);
registerList(editor);
registerHistory(editor, createEmptyHistoryState(), 300);
editor.setRootElement(document.getElementById("editor"));

// Toolbar buttons keep the focus inside the editor, as rich-text toolbars do.
const boldButton = document.getElementById("bold");
boldButton.addEventListener("mousedown", (event) => event.preventDefault());
boldButton.addEventListener("click", () => {
  editor.dispatchCommand(FORMAT_TEXT_COMMAND, "bold");
});

const settleSelection = () =>
  new Promise((resolve) => requestAnimationFrame(() => setTimeout(resolve)));

window.lexicalFixture = {
  text() {
    return editor.getEditorState().read(() => $getRoot().getTextContent());
  },

  blocks() {
    return editor.getEditorState().read(() =>
      $getRoot()
        .getChildren()
        .map((block) => ({
          type: $isHeadingNode(block) ? block.getTag() : block.getType(),
          runs: $isElementNode(block)
            ? block.getChildren().map((child) =>
                $isTextNode(child)
                  ? {
                      text: child.getTextContent(),
                      bold: child.hasFormat("bold"),
                      italic: child.hasFormat("italic"),
                      underline: child.hasFormat("underline"),
                    }
                  : { text: child.getTextContent(), type: child.getType() },
              )
            : [],
        })),
    );
  },

  selectionOffsets() {
    return editor.getEditorState().read(() => {
      const selection = $getSelection();
      if (!$isRangeSelection(selection)) {
        return null;
      }
      return {
        anchor: selection.anchor.offset,
        focus: selection.focus.offset,
      };
    });
  },

  setFormattedDocument() {
    editor.update(
      () => {
        const root = $getRoot();
        root.clear();
        const heading = $createHeadingNode("h1");
        heading.append($createTextNode("Title"));
        const paragraph = $createParagraphNode();
        const plain = $createTextNode("plain ");
        const bold = $createTextNode("bold");
        bold.toggleFormat("bold");
        const tail = $createTextNode(" end");
        paragraph.append(plain, bold, tail);
        root.append(heading, paragraph);
        const selection = bold.select(2, 2);
        selection.format = bold.getFormat();
      },
      { discrete: true },
    );
    return settleSelection();
  },

  setSelectedText(value, start, end) {
    editor.update(
      () => {
        const root = $getRoot();
        root.clear();
        const paragraph = $createParagraphNode();
        const node = $createTextNode(value);
        paragraph.append(node);
        root.append(paragraph);
        node.select(start, end);
      },
      { discrete: true },
    );
    return settleSelection();
  },
};
