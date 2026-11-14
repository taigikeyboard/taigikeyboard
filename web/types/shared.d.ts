import type { Session } from "./site/pkg/taigi_web.js";

export type Platform = "macos" | "windows" | "linux";

export interface BrowserKey {
  key: string;
  code: string;
  shift: boolean;
  ctrl: boolean;
  alt: boolean;
  meta: boolean;
  repeat: boolean;
}

export interface WireCell {
  text: string;
  annotation?: string;
}

export interface WireCollapsedCell {
  index: number;
  width: number;
}

export interface WireGridCell {
  index: number;
  start: number;
  span: number;
}

export interface WireRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface WireCellTrack {
  index: number;
  layer: "collapsed" | "expanded" | "shared";
  from: WireRect;
  to: WireRect;
  fromAlpha: number;
  toAlpha: number;
}

export interface WireTransition {
  isExpanding: boolean;
  cells: WireCellTrack[];
  chevron: {
    from: WireRect;
    to: WireRect;
    fromAlpha: number;
    toAlpha: number;
  };
}

export interface WireLayout {
  isColumn: boolean;
  isExpanded: boolean;
  leadCellIsUnkeyed: boolean;
  hasOverflow: boolean;
  collapsed: WireCollapsedCell[];
  rows: WireGridCell[][];
  columnWidth: number;
  itemHeight: number;
  rowHeight: number;
  windowHeight: number;
  gridHeight: number;
  peek: number;
  scrollY: number;
  columns: number;
  primaryColumn: number | null;
  keyed: number[];
  keyedBand: WireRect | null;
  isPaged: boolean;
  hasNextPage: boolean;
  animationMilliseconds: number;
  transition?: WireTransition;
}

export type WireEmit =
  | { type: "preedit"; text: string; caret: number }
  | { type: "clearPreedit" }
  | { type: "commit"; text: string }
  | { type: "deleteSurrounding"; offset: number; count: number }
  | {
      type: "lookupTable";
      cells: WireCell[];
      labels: string[];
      cursor: number;
      cursorVisible: boolean;
      pageSize: number;
      vertical: boolean;
      layout?: WireLayout;
    }
  | { type: "hideLookupTable" }
  | { type: "modeChanged"; symbol: string }
  | { type: "announceMode"; label: string }
  | { type: "openSettings" }
  | { type: "toggleTpsKeyboard" }
  | { type: "tpsKeyboardCapTyped"; row: number; cap: number };

export interface Reply {
  handled: boolean;
  emits: WireEmit[];
  settings?: string;
}

export interface WireTpsCap {
  label: string;
  glyph: string;
  shiftGlyph?: string;
}

export interface WireTpsRow {
  indent: number;
  caps: WireTpsCap[];
}

export interface WireTelexRow {
  key: string;
  meaning: string;
}

export interface WireTelexGuide {
  title: string;
  rows: WireTelexRow[];
}

export interface WireModeLine {
  isEnglish: boolean;
  mode: string;
  script: string;
  other: string;
}

export interface WireShortcutRow {
  label: string;
  keys: string;
  fixed_keys?: string;
}

export interface WireShortcutGroup {
  title: string;
  rows: WireShortcutRow[];
}

export interface WireShortcuts {
  groups: WireShortcutGroup[];
}

export interface CustomEntry {
  id: string;
  roman: string;
  hanji: string;
}

export interface CustomEntriesReply {
  entries: CustomEntry[];
}

export interface ExplainBufferPiece {
  text: string;
  gap: boolean;
}

export interface ExplainWordPiece {
  text: string;
  hanji: string | null;
  roman: string;
  kind: "dictionary" | "custom" | "reading" | "gap";
  userRow: boolean;
  picks: number;
  weight: number;
  gap?: boolean;
}

export interface ExplainCandidatePiece {
  hanji: string | null;
  roman: string;
  before: string;
  head: string;
  rest: string;
  role: "sentence" | "literal" | "word";
  placedBy: string;
  preference: "picked" | "custom" | "rest";
  coverage: "full" | "prefix" | "abbreviation";
  context: "learned" | "bundled" | "none";
  picks: number;
  syllables: number;
  weight: number;
  lastUsedMs: number;
  tier: "whole" | "front";
  score: number;
  frequency: number;
  walkerCost: number | null;
  coverageBytes: number;
  sourceRank: number;
  sources: string[];
  rankingSource: string;
}

export interface ExplainView {
  raw: string;
  cutBy: "walker" | "syllables" | "none";
  syllables: ExplainBufferPiece[];
  words: ExplainWordPiece[];
  candidates: ExplainCandidatePiece[];
}

export type SettingsValue = string | boolean;

export interface SettingsDocument {
  revision?: number;
  values: Record<string, SettingsValue>;
}

export interface SettingsStore {
  read(): SettingsDocument;
  write(json: string): void;
}

export interface RunSpan {
  text: string;
  bold: boolean;
  italic: boolean;
  underline: boolean;
  url: string | null;
}

export interface MarkdownParagraphBlock {
  type: "paragraph";
  runs: RunSpan[];
}

export interface MarkdownHeadingBlock {
  type: "heading";
  level: number;
  runs: RunSpan[];
}

export interface MarkdownQuoteBlock {
  type: "quote";
  runs: RunSpan[];
}

export interface MarkdownListBlock {
  type: "list";
  ordered: boolean;
  start: number;
  items: RunSpan[][];
}

export type MarkdownBlock =
  | MarkdownParagraphBlock
  | MarkdownHeadingBlock
  | MarkdownQuoteBlock
  | MarkdownListBlock;

export interface ShellEntry {
  path: string;
  sha256: string;
}

export interface DictionaryAsset {
  name: string;
  file: string;
  sha256: string;
  size: number;
}

export interface AssetsManifest {
  build: string;
  shell: ShellEntry[];
  dictionaries: DictionaryAsset[];
}

export type SessionRunner = <T>(
  action: (session: Session) => T,
) => T | undefined;

export interface CaretPoint {
  left: number;
  top: number;
  lineHeight: number;
}

export interface SurfaceEditor {
  batch(writes: () => void): void;
  insertText(text: string): boolean;
  deleteBefore(count: number): void;
  state(): unknown;
  snapshot(): unknown;
}
