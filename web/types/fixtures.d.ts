interface CodeMirror5Position {
  line: number;
  ch: number;
}

interface CodeMirror5Instance {
  getValue(): string;
  setValue(value: string): void;
  clearHistory(): void;
  getCursor(): CodeMirror5Position;
  setCursor(pos: CodeMirror5Position): void;
  setSelection(from: CodeMirror5Position, to: CodeMirror5Position): void;
  setOption(option: string, value: boolean | string): void;
  getWrapperElement(): HTMLElement;
  focus(): void;
}

interface CodeMirror5Fixture {
  text(): string;
  cursor(): CodeMirror5Position;
  setDocument(value: string, line: number, ch: number): void;
  setSelection(
    value: string,
    from: CodeMirror5Position,
    to: CodeMirror5Position,
  ): void;
  setReadOnly(value: boolean | string): void;
  openDialogInput(): void;
}

interface CodeMirror6Fixture {
  text(): string;
  selection(): { anchor: number; head: number };
  setDocument(doc: string, pos?: number): Promise<void>;
  setSelectedText(doc: string, anchor: number, head: number): Promise<void>;
}

interface LexicalBlockRun {
  text: string;
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
  type?: string;
}

interface LexicalBlock {
  type: string;
  runs: LexicalBlockRun[];
}

interface LexicalFixture {
  text(): string;
  blocks(): LexicalBlock[];
  selectionOffsets(): { anchor: number; focus: number } | null;
  setFormattedDocument(): Promise<void>;
  setSelectedText(value: string, start: number, end: number): Promise<void>;
}

interface HiddenFixture {
  clearLog(): void;
  editContextText(): string;
  setEditContextReadOnly(value: boolean): void;
  focusDocsIframe(): void;
  focusSheetsGrid(): void;
  openSheetsCell(): void;
  sheetsCellText(): string;
}

interface QuillInstance {
  getText(): string;
  setText(text: string): void;
  getContents(): { ops: Array<Record<string, unknown>> };
  setContents(ops: Array<Record<string, unknown>>): void;
  getSelection(): { index: number; length: number } | null;
  setSelection(index: number, length: number): void;
  history: {
    undo(): void;
  };
}

declare class Quill {
  constructor(container: string | Element, options?: Record<string, unknown>);
  getText(): string;
  setText(text: string): void;
  getContents(): { ops: Array<Record<string, unknown>> };
  setContents(ops: Array<Record<string, unknown>>): void;
  getSelection(): { index: number; length: number } | null;
  setSelection(index: number, length: number): void;
  history: {
    undo(): void;
  };
}

interface TaigiEditorHandle {
  insertText(text: string): boolean;
  deleteBefore(count: number): boolean;
}

interface HTMLElement {
  taigiEditor?: TaigiEditorHandle;
}

interface Window {
  CodeMirror: (
    element: HTMLElement | null,
    options: Record<string, unknown>,
  ) => CodeMirror5Instance;
  cm5Fixture: CodeMirror5Fixture;
  cm6Fixture: CodeMirror6Fixture;
  lexicalFixture: LexicalFixture;
  hiddenFixture: HiddenFixture;
  quill: QuillInstance;
  inlineRan?: boolean;
  violations?: string[];
  cspViolations?: Array<{ directive: string; blockedURI: string }>;
  lastPrevented?: boolean;
  flashed?: Array<string | null>;
  inputs?: Array<[string, string | null]>;
  copies?: Array<{
    ids: number;
    hidden: string | null;
    pointer: string;
    removed: boolean;
  }>;
}
