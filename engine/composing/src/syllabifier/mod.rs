//! Pure-function syllable boundary scanners for v3.5.8 連續輸入 Phase 3.
//!
//! Two domain-specific entry points:
//! - `tl::valid_span_endings` — BFS-with-FST over a `SyllableInventory`,
//!   returning every byte offset reachable from `pos` by a chain of 1..=
//!   `max_syllables` valid syllables. Caller must supply input already
//!   canonicalized for the chosen mode (TL ASCII for TL/English via
//!   `phonetics::canonicalize_syllable`; POJ ASCII for POJ via
//!   `phonetics::canonicalize_poj_syllable` — v3.5.9 B-1 added the POJ
//!   axis and B-2 made the inventory mode-aware via
//!   `SyllableInventory::contains_in(mode, ...)`).
//! - `tps::valid_span_endings` — O(n) scan over Bopomofo Extended tone
//!   marks + entering-coda small letters + 8th-tone combining/encode-safe
//!   dots (unambiguous terminators per `docs/releases/v3.5.8/plan.md` § 走查範例 Case D — TPS), plus
//!   implicit tone-1 boundaries via the "next initial seen" rule
//!   (v3.5.8 Phase 9 Item 7) — the syllabifier itself emits the
//!   tone-1 endings; no dispatcher post-processing.
//!
//! Both functions return `Vec<usize>` of ascending, deduplicated byte
//! offsets. In production these endings flow into
//! `composing::continuous::assemble_candidates`, which builds the
//! mode-aware `<prefix>:<toneless>` keys (`tl:` / `poj:`) and calls
//! `lexicon::fetch_candidates_for_keys` directly. The legacy
//! `lexicon::fetch_candidates_for_endings` wrapper that used to consume
//! these `Vec<usize>` is now `#[doc(hidden)]` test-only after v3.5.9
//! D7+D8 (#306). The scanners never panic on partial UTF-8 or
//! out-of-range `pos`; see each entry's `pos` validation contract.
//!
//! Design choice (`docs/releases/v3.5.8/plan.md` § Phase 3 — 純函數 syllabifier 設計裁定): multi-cut, span-local
//! lookup is the middle ground between khiin-rs pure longest-match
//! (which would miss `珠 (tsu, span=3)` when the user types `tsua`) and
//! librime's global lattice (over-engineered). BFS with FST membership
//! and a `max_syllables` depth cap fits this slot exactly.

// 中文: v3.5.8 連續輸入 Phase 3 的純函式音節邊界掃描器,提供 TL 與 TPS 兩個入口。
// 中文: TL 走 BFS+SyllableInventory.contains;TPS 走 O(n) 終止符 + 隱式第 1 聲掃描;結果為遞增 byte 位移。

pub mod tl;
pub mod tps;
