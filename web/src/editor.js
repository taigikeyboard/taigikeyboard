// The editor: a Lexical rich-text root on `#text` that shows formatting and
// never its markers. Its document is the stored Markdown (`markdown.js`), read
// in and written out as blocks. Every write the page makes goes through here.

import { encodeUrl, parse, serialize } from "./markdown.js";
import {
  $createHeadingNode,
  $createLineBreakNode,
  $createLinkNode,
  $createListItemNode,
  $createListNode,
  $createParagraphNode,
  $createQuoteNode,
  $createTextNode,
  $findMatchingParent,
  $getRoot,
  $getSelection,
  $isElementNode,
  $isHeadingNode,
  $isLineBreakNode,
  $isLinkNode,
  $isListItemNode,
  $isListNode,
  $isQuoteNode,
  $isRangeSelection,
  $isTextNode,
  $setBlocksType,
  $setSelection,
  $toggleLink,
  COMMAND_PRIORITY_CRITICAL,
  createEditor,
  createEmptyHistoryState,
  FORMAT_TEXT_COMMAND,
  HeadingNode,
  HISTORY_PUSH_TAG,
  INSERT_ORDERED_LIST_COMMAND,
  INSERT_UNORDERED_LIST_COMMAND,
  KEY_DOWN_COMMAND,
  LinkNode,
  ListItemNode,
  ListNode,
  QuoteNode,
  REMOVE_LIST_COMMAND,
  registerHistory,
  registerList,
  registerRichText,
} from "./vendor/lexical.js";

/** @typedef {import("../types/shared.d.ts").RunSpan} RunSpan */
/** @typedef {import("../types/shared.d.ts").MarkdownBlock} MarkdownBlock */
/** @typedef {import("lexical").BaseSelection} BaseSelection */
/** @typedef {import("lexical").ElementNode} ElementNode */
/** @typedef {import("lexical").LexicalEditor} LexicalEditor */
/** @typedef {import("lexical").LexicalNode} LexicalNode */
/** @typedef {import("lexical").RangeSelection} RangeSelection */
/** @typedef {import("lexical").TextFormatType} TextFormatType */
/** @typedef {import("@lexical/rich-text").HeadingTagType} HeadingTagType */
/** @typedef {import("@lexical/list").ListType} ListType */
/** @typedef {import("@lexical/history").HistoryState} HistoryState */

/**
 * Updates that bring a whole text in (a stored draft, another tab's save, the
 * intro): no user edit, so Draft does not save them back.
 */
export const LOAD_TAG = "taigi-load";

/** The classes style.css draws formatting with. */
const THEME = {
  text: { bold: "is-bold", italic: "is-italic", underline: "is-underline" },
  link: "is-link",
};

/** @type {Array<"bold" | "italic" | "underline">} */
const FORMATS = ["bold", "italic", "underline"];

/**
 * A run's text as nodes: `\n` is a line break.
 *
 * @param {RunSpan} run
 * @returns {LexicalNode[]}
 */
function textNodes(run) {
  /** @type {LexicalNode[]} */
  const nodes = [];
  run.text.split("\n").forEach((part, index) => {
    if (index > 0) {
      nodes.push($createLineBreakNode());
    }
    if (part !== "") {
      const node = $createTextNode(part);
      for (const format of FORMATS) {
        if (run[format]) {
          node.toggleFormat(format);
        }
      }
      nodes.push(node);
    }
  });
  return nodes;
}

/**
 * Runs as inline nodes, a link's runs inside one link node.
 *
 * @param {RunSpan[]} runs
 * @returns {LexicalNode[]}
 */
function inlineNodes(runs) {
  /** @type {LexicalNode[]} */
  const nodes = [];
  for (let index = 0; index < runs.length; ) {
    const { url } = runs[index];
    if (url === null) {
      nodes.push(...textNodes(runs[index]));
      index += 1;
      continue;
    }
    const link = $createLinkNode(url);
    while (index < runs.length && runs[index].url === url) {
      link.append(...textNodes(runs[index]));
      index += 1;
    }
    nodes.push(link);
  }
  return nodes;
}

/**
 * @param {MarkdownBlock} block
 * @returns {ElementNode}
 */
function blockNode(block) {
  switch (block.type) {
    case "heading": {
      const tag = /** @type {HeadingTagType} */ (`h${block.level}`);
      return $createHeadingNode(tag).append(...inlineNodes(block.runs));
    }
    case "quote":
      return $createQuoteNode().append(...inlineNodes(block.runs));
    case "list":
      return $createListNode(
        block.ordered ? "number" : "bullet",
        block.start,
      ).append(
        ...block.items.map((runs) =>
          $createListItemNode().append(...inlineNodes(runs)),
        ),
      );
    default:
      return $createParagraphNode().append(...inlineNodes(block.runs));
  }
}

/**
 * An element's inline content as runs. A nested list (pasted, or indented)
 * is left out: `listItems` takes its items. A pasted link's URL is encoded as
 * the URL field's is, so the Markdown link holds.
 *
 * @param {ElementNode} element
 * @param {string | null} [url]
 * @returns {RunSpan[]}
 */
function runsOf(element, url = null) {
  /** @type {RunSpan[]} */
  const runs = [];
  for (const child of element.getChildren()) {
    if ($isListNode(child)) {
      continue;
    }
    if ($isTextNode(child) || $isLineBreakNode(child)) {
      /** @param {TextFormatType} format */
      const has = (format) => $isTextNode(child) && child.hasFormat(format);
      runs.push({
        text: child.getTextContent(),
        bold: has("bold"),
        italic: has("italic"),
        underline: has("underline"),
        url,
      });
    } else if ($isLinkNode(child)) {
      runs.push(...runsOf(child, encodeUrl(child.getURL())));
    } else if ($isElementNode(child)) {
      runs.push(...runsOf(child, url));
    }
  }
  return runs;
}

/**
 * A list's items, a nested list's items after the item holding them: the
 * Markdown has one level.
 *
 * @param {ListNode} list
 * @returns {RunSpan[][]}
 */
function listItems(list) {
  return list
    .getChildren()
    .filter($isListItemNode)
    .flatMap((item) => {
      const runs = runsOf(item);
      const nested = item.getChildren().filter($isListNode).flatMap(listItems);
      return runs.length === 0 ? nested : [runs, ...nested];
    });
}

/**
 * The document as blocks. A pasted h4–h6 reads as h3; any block the page has
 * no Markdown for reads as a paragraph.
 *
 * @returns {MarkdownBlock[]}
 */
function $blocks() {
  return $getRoot()
    .getChildren()
    .map((node) => {
      if ($isHeadingNode(node)) {
        const level = Math.min(3, Number(node.getTag().slice(1)));
        return { type: "heading", level, runs: runsOf(node) };
      }
      if ($isQuoteNode(node)) {
        return { type: "quote", runs: runsOf(node) };
      }
      if ($isListNode(node)) {
        return {
          type: "list",
          ordered: node.getListType() === "number",
          start: node.getStart(),
          items: listItems(node),
        };
      }
      return {
        type: "paragraph",
        runs: $isElementNode(node) ? runsOf(node) : [],
      };
    });
}

/**
 * @param {string} text
 * @returns {boolean}
 */
function $insertText(text) {
  const selection = $getSelection();
  if (!$isRangeSelection(selection)) {
    return false;
  }
  selection.insertText(text);
  return true;
}

/**
 * The last text node at or before `node`, inside an inline element (a link)
 * or out of one.
 *
 * @param {LexicalNode | null} node
 * @param {ElementNode | null} parent
 * @returns {LexicalNode | null}
 */
function $textBefore(node, parent) {
  let current = node;
  let holder = parent;
  while (current === null && holder !== null && holder.isInline()) {
    current = holder.getPreviousSibling();
    holder = holder.getParent();
  }
  while ($isElementNode(current) && current.isInline()) {
    current = current.getLastChild();
  }
  return current;
}

/**
 * Deletes `count` Unicode scalars before the caret, in its paragraph. Not
 * Lexical's character deletion, which goes by grapheme: a combining tone mark
 * is a scalar of its own.
 *
 * @param {number} count
 * @returns {void}
 */
function $deleteBefore(count) {
  const selection = $getSelection();
  if (!$isRangeSelection(selection) || !selection.isCollapsed()) {
    return;
  }
  const anchorNode = selection.anchor.getNode();
  // A format armed at the caret (Command+B with nothing selected) outlives the
  // deletion; otherwise the commit writes in the text's own.
  const armed =
    $isTextNode(anchorNode) && selection.format !== anchorNode.getFormat()
      ? selection.format
      : null;
  /** @type {LexicalNode | null} */
  let node = anchorNode;
  let offset = selection.anchor.offset;
  if ($isElementNode(node)) {
    node = $textBefore(node.getChildAtIndex(offset - 1), node);
    offset = $isTextNode(node) ? node.getTextContentSize() : 0;
  }
  let remaining = count;
  while (remaining > 0 && node !== null) {
    const parent = node.getParent();
    const previous = node.getPreviousSibling();
    if ($isTextNode(node)) {
      const text = node.getTextContent();
      let cut = offset;
      while (remaining > 0 && cut > 0) {
        const unit = text.charCodeAt(cut - 1);
        cut -= unit >= 0xdc00 && unit <= 0xdfff && cut >= 2 ? 2 : 1;
        remaining -= 1;
      }
      node.spliceText(cut, offset - cut, "", true);
      if (remaining === 0) {
        break;
      }
    } else if ($isLineBreakNode(node)) {
      node.remove();
      remaining -= 1;
    } else {
      break;
    }
    node = $textBefore(previous, parent);
    offset = $isTextNode(node) ? node.getTextContentSize() : 0;
    if ($isTextNode(node)) {
      node.select(offset, offset);
    }
  }
  // A caret the page placed takes no format of its own.
  const caret = $getSelection();
  const anchor = $isRangeSelection(caret) ? caret.anchor.getNode() : null;
  if ($isRangeSelection(caret) && $isTextNode(anchor)) {
    caret.format = armed ?? anchor.getFormat();
    caret.style = anchor.getStyle();
  }
}

/**
 * @param {string} markdown
 * @returns {void}
 */
function $setMarkdown(markdown) {
  const root = $getRoot();
  root.clear();
  const blocks = parse(markdown);
  root.append(
    ...(blocks.length === 0 ? [$createParagraphNode()] : blocks.map(blockNode)),
  );
}

/**
 * The top-level blocks a selection touches.
 *
 * @param {RangeSelection} selection
 * @returns {LexicalNode[]}
 */
function $selectedBlocks(selection) {
  return [
    ...new Set(
      selection.getNodes().map((node) => node.getTopLevelElement() ?? node),
    ),
  ];
}

/**
 * The link the caret or selection is in.
 *
 * @param {RangeSelection} selection
 * @returns {LinkNode | null}
 */
function $selectedLink(selection) {
  return $findMatchingParent(selection.anchor.getNode(), $isLinkNode);
}

export class Editor {
  /** @type {LexicalEditor} */
  #lexical;
  /** @type {HTMLElement} */
  #root;
  /** @type {HistoryState} */
  #history = createEmptyHistoryState();
  /** Inside `batch`: writes join its update. */
  #isBatching = false;

  /** @param {HTMLElement} root */
  constructor(root) {
    this.#root = root;
    this.#lexical = createEditor({
      namespace: "taigi",
      theme: THEME,
      nodes: [HeadingNode, QuoteNode, ListNode, ListItemNode, LinkNode],
      editable: false,
      onError: (error) => {
        throw error;
      },
    });
    this.#lexical.setRootElement(root);
    // For tests and devtools, as Lexical keeps `__lexicalEditor` on the root.
    /** @type {HTMLElement & { taigiEditor?: Editor }} */ (root).taigiEditor =
      this;
    registerRichText(this.#lexical);
    registerList(this.#lexical);
    registerHistory(this.#lexical, this.#history, 300);
    // A key the engine took (`ime.js`, capture on `#editor`) is not Lexical's:
    // no Enter paragraph, no Backspace, no shortcut.
    this.#lexical.registerCommand(
      KEY_DOWN_COMMAND,
      (event) => event.defaultPrevented,
      COMMAND_PRIORITY_CRITICAL,
    );
  }

  /** @returns {HTMLElement} */
  get root() {
    return this.#root;
  }

  /**
   * `contenteditable` follows: Lexical only stops its own handlers.
   *
   * @param {boolean} isEditable
   * @returns {void}
   */
  setEditable(isEditable) {
    this.#lexical.setEditable(isEditable);
    this.#root.contentEditable = String(isEditable);
  }

  /**
   * The caret where Lexical last had it, the text's end the first time.
   *
   * @returns {void}
   */
  focus() {
    this.#lexical.focus(undefined, { defaultSelection: "rootEnd" });
  }

  /** @returns {string} */
  markdown() {
    return this.#lexical.getEditorState().read(() => serialize($blocks()));
  }

  /**
   * A whole text from outside the page's editing: the undo history starts
   * over and the caret goes to the end. `alignLastRight` draws the last
   * paragraph against the right edge (the intro's signature).
   *
   * @param {string} markdown
   * @param {{ alignLastRight?: boolean }} [options]
   * @returns {void}
   */
  setMarkdown(markdown, { alignLastRight = false } = {}) {
    this.#lexical.update(
      () => {
        $setMarkdown(markdown);
        if (alignLastRight) {
          const last = $getRoot().getLastChild();
          if ($isElementNode(last)) {
            last.setFormat("right");
          }
        }
        $getRoot().selectEnd();
      },
      { discrete: true, tag: LOAD_TAG },
    );
    this.#resetHistory();
  }

  /**
   * The history starts over at the current state. Not CLEAR_HISTORY_COMMAND:
   * it leaves no current state, so the first edit after a load could not be
   * undone.
   *
   * @returns {void}
   */
  #resetHistory() {
    const history = this.#history;
    history.undoStack.length = 0;
    history.redoStack.length = 0;
    history.current = {
      editor: this.#lexical,
      editorState: this.#lexical.getEditorState(),
    };
  }

  /**
   * One undo step for everything `edit` writes (one engine reply). The writes
   * inside run in this update at once: a nested update would only be queued.
   *
   * @param {() => void} edit
   * @returns {void}
   */
  batch(edit) {
    this.#lexical.update(
      () => {
        this.#isBatching = true;
        try {
          edit();
        } finally {
          this.#isBatching = false;
        }
      },
      { discrete: true, tag: HISTORY_PUSH_TAG },
    );
  }

  /**
   * `write` inside the running batch, or in an update of its own.
   *
   * @template T
   * @param {() => T} write
   * @returns {T}
   */
  #write(write) {
    if (this.#isBatching) {
      return write();
    }
    /** @type {T | undefined} */
    let result;
    this.#lexical.update(
      () => {
        result = write();
      },
      { discrete: true },
    );
    return /** @type {T} */ (result);
  }

  /**
   * Replaces the selection, or inserts at the caret, in the caret's format.
   * Answers whether there was a caret to write at.
   *
   * @param {string} text
   * @returns {boolean}
   */
  insertText(text) {
    return this.#write(() => $insertText(text));
  }

  /**
   * Deletes `count` Unicode scalars before the caret, in its paragraph.
   *
   * @param {number} count
   * @returns {void}
   */
  deleteBefore(count) {
    this.#write(() => $deleteBefore(count));
  }

  /**
   * The text and the selection's points, to tell a change the page did not
   * make: Lexical may make a new state for a selection it only normalizes.
   *
   * @returns {string}
   */
  snapshot() {
    return this.#lexical.getEditorState().read(() => {
      const selection = $getSelection();
      const points = $isRangeSelection(selection)
        ? [
            selection.anchor.key,
            selection.anchor.offset,
            selection.focus.key,
            selection.focus.offset,
          ].join(",")
        : "";
      return `${points}\n${$getRoot().getTextContent()}`;
    });
  }

  /**
   * Whether some text is selected.
   *
   * @returns {boolean}
   */
  hasSelection() {
    return this.#lexical.getEditorState().read(() => {
      const selection = $getSelection();
      return $isRangeSelection(selection) && !selection.isCollapsed();
    });
  }

  /**
   * Bold, italic or underline on the selection.
   *
   * @param {TextFormatType} format
   * @returns {void}
   */
  toggleFormat(format) {
    this.#lexical.dispatchCommand(FORMAT_TEXT_COMMAND, format);
  }

  /**
   * `h1`–`h3` or `quote` on the blocks the selection touches; off, back to
   * paragraphs, when every one of them already is.
   *
   * @param {"h1" | "h2" | "h3" | "quote"} kind
   * @returns {void}
   */
  toggleBlock(kind) {
    this.#lexical.update(() => {
      const selection = $getSelection();
      if (!$isRangeSelection(selection)) {
        return;
      }
      const isOn = $selectedBlocks(selection).every((block) =>
        kind === "quote"
          ? $isQuoteNode(block)
          : $isHeadingNode(block) && block.getTag() === kind,
      );
      $setBlocksType(selection, () => {
        if (isOn) {
          return $createParagraphNode();
        }
        return kind === "quote" ? $createQuoteNode() : $createHeadingNode(kind);
      });
    });
  }

  /**
   * A bulleted (`bullet`) or numbered (`number`) list over the blocks the
   * selection touches; off when they all are that list.
   *
   * @param {ListType} type
   * @returns {void}
   */
  toggleList(type) {
    const isOn = this.#lexical.getEditorState().read(() => {
      const selection = $getSelection();
      return (
        $isRangeSelection(selection) &&
        $selectedBlocks(selection).every(
          (block) => $isListNode(block) && block.getListType() === type,
        )
      );
    });
    if (isOn) {
      this.#lexical.dispatchCommand(REMOVE_LIST_COMMAND, undefined);
    } else {
      this.#lexical.dispatchCommand(
        type === "number"
          ? INSERT_ORDERED_LIST_COMMAND
          : INSERT_UNORDERED_LIST_COMMAND,
        undefined,
      );
    }
  }

  /**
   * The selection as it is, to come back to after the URL field had focus.
   *
   * @returns {BaseSelection | null}
   */
  saveSelection() {
    return this.#lexical
      .getEditorState()
      .read(() => $getSelection()?.clone() ?? null);
  }

  /**
   * The URL of the link `selection` is in, or `null`.
   *
   * @param {BaseSelection | null} selection
   * @returns {string | null}
   */
  linkAt(selection) {
    return this.#lexical.getEditorState().read(() => {
      if (!$isRangeSelection(selection)) {
        return null;
      }
      return $selectedLink(selection)?.getURL() ?? null;
    });
  }

  /**
   * `selection` back, focused; with `url`, made a link (`null` takes one off).
   *
   * @param {BaseSelection | null} selection
   * @param {string | null} [url]
   * @returns {void}
   */
  restoreSelection(selection, url) {
    this.#lexical.update(
      () => {
        if (selection !== null) {
          $setSelection(selection.clone());
        }
        if (url !== undefined) {
          $toggleLink(url);
        }
      },
      { discrete: true },
    );
    this.#root.focus({ preventScroll: true });
  }

  /**
   * `listener(tags)` after each update that changed the text, not only the
   * selection.
   *
   * @param {(tags: Set<string>) => void} listener
   * @returns {() => void}
   */
  onTextChange(listener) {
    return this.#lexical.registerUpdateListener(
      ({ dirtyElements, dirtyLeaves, tags }) => {
        if (dirtyElements.size > 0 || dirtyLeaves.size > 0) {
          listener(tags);
        }
      },
    );
  }

  /**
   * `listener()` after an update that moved or changed the selection.
   *
   * @param {() => void} listener
   * @returns {() => void}
   */
  onSelectionChange(listener) {
    return this.#lexical.registerUpdateListener(
      ({ editorState, prevEditorState }) => {
        const before = prevEditorState.read(() => $getSelection());
        const after = editorState.read(() => $getSelection());
        if (before === null ? after !== null : !before.is(after)) {
          listener();
        }
      },
    );
  }

  /**
   * The state object: a new one after any update, so an unchanged one means
   * nothing happened.
   *
   * @returns {import("lexical").EditorState}
   */
  state() {
    return this.#lexical.getEditorState();
  }
}
