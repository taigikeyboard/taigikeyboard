/* tslint:disable */
/* eslint-disable */

export class Session {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Hands in one of `dictionaryFiles`.
   */
  addDictionary(name: string, bytes: Uint8Array | number[]): void;
  /**
   * ChromeOS: a text field gained focus.
   */
  beginSession(): string;
  caretMoved(): void;
  /**
   * Empties the three learning stores. Custom words stay.
   */
  clearLearningRecords(): string;
  /**
   * A click on a cell: selects, never commits (`click_from_panel`).
   * `position`: within the current page, or under a `layout` the cell's
   * index in the list.
   */
  clickCandidate(position: number): string;
  commitComposition(): string;
  /**
   * Removes the custom word with `id`.
   */
  deleteCustomEntry(id: string): string;
  /**
   * The files `addDictionary` takes.
   */
  dictionaryFiles(): string[];
  /**
   * ChromeOS: the field confirmed the preedit itself (blur, reset).
   */
  endSession(): string;
  /**
   * The open composition's segmentation and ranking, JSON, or `null`.
   */
  explain(): string;
  /**
   * One of `userDataFiles` as it is now, for the page to keep. Every write
   * has landed: the stores' queues drain on this thread (`database.rs`).
   */
  exportUserData(name: string): Uint8Array;
  focusLost(): void;
  /**
   * Hands in one of `userDataFiles` as the page kept it. Before
   * `openUserData` only: an open file is refused.
   */
  importUserData(name: string, bytes: Uint8Array | number[]): void;
  /**
   * Installs the dictionaries handed in and answers the record count.
   * Keys before this type romanization (`runtime.rs` `install_lexicon`).
   */
  install(): bigint;
  keyDown(key_json: string): string;
  keyUp(key_json: string): string;
  /**
   * The custom dictionary as JSON `{ "entries": [...] }`.
   */
  listCustomEntries(): string;
  /**
   * The mode chip under the editor, JSON: `isEnglish`, `mode`, `script`
   * and `other`, in the display language. Again on `modeChanged` and a settings change.
   */
  modeLine(): string;
  /**
   * `settings_json`: the document the page keeps, `{}` for the defaults.
   * `platform`: `"macos"`, `"windows"` or `"linux"`. `locale`:
   * `navigator.language`. The page passes the platform, which sets the
   * caret chord (`keys/intent.rs`) and how Option reads (`key.rs`).
   */
  constructor(settings_json: string, platform: string, locale: string);
  /**
   * Opens the stores over what was imported, a new file for each one not,
   * and finishes before it returns. From here on the engine learns. Once
   * per instance; `false` when the engine did not acknowledge it.
   */
  openUserData(): boolean;
  /**
   * Adds a custom word. A refusal (no romanization, a full dictionary)
   * is an error whose message is the engine's.
   */
  saveCustomEntry(roman: string, hanji: string): string;
  /**
   * A one-word best path joins the twelve stages. Several words stay pinned.
   */
  setRankOneWord(enabled: boolean): string;
  setSettings(settings_json: string): void;
  setSortOrder(json: string): string;
  /**
   * A malformed document is refused and the previous one stays.
   * The sidebar's Shortcuts section; JSON, `shortcuts.rs`.
   */
  shortcuts(): string;
  /**
   * The Telex guide's card, JSON: `title` and `rows` of `key` and
   * `meaning`, in the display language.
   */
  telexGuide(): string;
  /**
   * Taigi to English or back, as a Shift tap does.
   */
  toggleLanguageMode(): string;
  /**
   * A click on a TPS key cap: `glyph` is what the cap types.
   */
  tpsKeyPress(glyph: string): string;
  /**
   * The TPS key panel's rows, JSON. They never change, so once is enough.
   */
  tpsKeyboardRows(): string;
  /**
   * A click on the panel's page arrows.
   */
  turnPage(is_forward: boolean): string;
  /**
   * The files the page keeps the user's data in, by name.
   */
  userDataFiles(): string[];
  /**
   * The wheel moved the unfolded grid: the keys read the same viewport.
   */
  viewportScrolled(offset: number): void;
}

export type InitInput =
  | RequestInfo
  | URL
  | Response
  | BufferSource
  | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_session_free: (a: number, b: number) => void;
  readonly session_addDictionary: (
    a: number,
    b: number,
    c: number,
    d: number,
    e: number,
  ) => void;
  readonly session_beginSession: (a: number) => [number, number];
  readonly session_caretMoved: (a: number) => void;
  readonly session_clearLearningRecords: (
    a: number,
  ) => [number, number, number, number];
  readonly session_clickCandidate: (a: number, b: number) => [number, number];
  readonly session_commitComposition: (a: number) => [number, number];
  readonly session_deleteCustomEntry: (
    a: number,
    b: number,
    c: number,
  ) => [number, number, number, number];
  readonly session_dictionaryFiles: (a: number) => [number, number];
  readonly session_endSession: (a: number) => [number, number];
  readonly session_explain: (a: number) => [number, number];
  readonly session_exportUserData: (
    a: number,
    b: number,
    c: number,
  ) => [number, number, number, number];
  readonly session_focusLost: (a: number) => void;
  readonly session_importUserData: (
    a: number,
    b: number,
    c: number,
    d: number,
    e: number,
  ) => [number, number];
  readonly session_install: (a: number) => [bigint, number, number];
  readonly session_keyDown: (
    a: number,
    b: number,
    c: number,
  ) => [number, number, number, number];
  readonly session_keyUp: (
    a: number,
    b: number,
    c: number,
  ) => [number, number, number, number];
  readonly session_listCustomEntries: (
    a: number,
  ) => [number, number, number, number];
  readonly session_modeLine: (a: number) => [number, number];
  readonly session_new: (
    a: number,
    b: number,
    c: number,
    d: number,
    e: number,
    f: number,
  ) => [number, number, number];
  readonly session_openUserData: (a: number) => number;
  readonly session_saveCustomEntry: (
    a: number,
    b: number,
    c: number,
    d: number,
    e: number,
  ) => [number, number, number, number];
  readonly session_setRankOneWord: (a: number, b: number) => [number, number];
  readonly session_setSettings: (
    a: number,
    b: number,
    c: number,
  ) => [number, number];
  readonly session_setSortOrder: (
    a: number,
    b: number,
    c: number,
  ) => [number, number, number, number];
  readonly session_shortcuts: (a: number) => [number, number];
  readonly session_telexGuide: (a: number) => [number, number];
  readonly session_toggleLanguageMode: (a: number) => [number, number];
  readonly session_tpsKeyPress: (
    a: number,
    b: number,
    c: number,
  ) => [number, number];
  readonly session_tpsKeyboardRows: (a: number) => [number, number];
  readonly session_turnPage: (a: number, b: number) => [number, number];
  readonly session_userDataFiles: (a: number) => [number, number];
  readonly session_viewportScrolled: (a: number, b: number) => void;
  readonly rust_sqlite_wasm_abort: () => void;
  readonly rust_sqlite_wasm_assert_fail: (
    a: number,
    b: number,
    c: number,
    d: number,
  ) => void;
  readonly rust_sqlite_wasm_calloc: (a: number, b: number) => number;
  readonly rust_sqlite_wasm_free: (a: number) => void;
  readonly rust_sqlite_wasm_getentropy: (a: number, b: number) => number;
  readonly rust_sqlite_wasm_localtime: (a: number) => number;
  readonly rust_sqlite_wasm_malloc: (a: number) => number;
  readonly rust_sqlite_wasm_realloc: (a: number, b: number) => number;
  readonly sqlite3_os_end: () => number;
  readonly sqlite3_os_init: () => number;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_externrefs: WebAssembly.Table;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_realloc: (
    a: number,
    b: number,
    c: number,
    d: number,
  ) => number;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly __externref_drop_slice: (a: number, b: number) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(
  module: { module: SyncInitInput } | SyncInitInput,
): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init(
  module_or_path?:
    | { module_or_path: InitInput | Promise<InitInput> }
    | InitInput
    | Promise<InitInput>,
): Promise<InitOutput>;
