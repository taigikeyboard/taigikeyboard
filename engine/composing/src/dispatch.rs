//! Decode `ComposingRequest` → `Intent`, apply against `Engine`, encode the
//! `ComposingResponse`. The generation-mismatch reset path also lives here
//! (per plan §5b.2).
//!
//! `Intent::FetchAtPos` is the v3.5.8 Phase 6 read-only continuous-input
//! candidate query. Unlike the other 12+ intents (which mutate engine
//! state via `transition::apply`), `FetchAtPos` needs lexicon state
//! (`lexicon::EngineHandle::with_state` — prefix index + dictionary +
//! syllable inventory) and so it is resolved here in dispatch outside
//! the pure transition table. After v3.5.9 A2 the candidate-assembly
//! 6-step seam lives in [`crate::continuous::assemble_candidates`];
//! dispatch only handles the phase/hanzi/position guards, the proto →
//! domain hoists (`mode`, `freq_map`, `custom`), and wire encoding.
//!
//! The mode-aware key construction lives in `composing::continuous`
//! per the Phase 5 module contract pinned in
//! `engine/lexicon/src/continuous.rs`: TL/English emit `tl:<lowered>`,
//! POJ emits `poj:<lowered>` (v3.5.9 B-2 PR #309 promoted POJ to a
//! first-class FST key family via `composing::shadow::mode_key_prefix`),
//! and TPS emits `tps:<bopomofo_toneless>` against the C-0 emit of
//! `dictionary.fst` (v3.5.9 D / C-3b promoted TPS to first-class via
//! the same shadow → lattice path TL/POJ already walk; the legacy
//! `build_keys_tps` `tl:`-folded path is retired).

// 中文: 將 protobuf ComposingRequest 解碼成 Intent,套用到 Engine 後產出回應。
// 中文: 純函式分派層,不處理 generation 同步 (那由 EngineHandle 負責)。
// 中文: Phase 6 — FetchAtPos 在 dispatch 短路處理 (需要 lexicon state + 模式相關 key 構造);
// 中文:   v3.5.9 A2 後候選組裝的 6-step seam 已移到 composing::continuous,
// 中文:   dispatch 只剩 phase/hanzi/position guard + proto→domain hoist + wire encode。

use crate::api::{ComposingError, Engine, Intent, Phase};
use crate::continuous::assemble_candidates;
use crate::shadow::{build_shadow_lattice, left_anchored_keys_from_lattice};
use lexicon::{
    classification::is_hanzi, derive_mode, ConsumedSpan, CustomEntry, RawCandidate,
    SyllableInventory, COVERAGE_KIND_FULL, FORM_NOTONE,
};
use phonetics::contains_tps;
use protos::engine::{
    composing_request, AppConfig, CandidateMessage, ComposingRequest, ComposingResponse,
    ContinuousResponse, CustomDictEntry, FrequencyEntry,
};
use ranking::build_frequency_map;

/// Decode the proto request into a typed `Intent`. Returns `MissingMethod`
/// when `oneof method` is empty.
// 中文: 把 proto 請求解碼為型別化 Intent,oneof method 缺漏時回傳 MissingMethod。
pub(crate) fn decode_intent(req: &ComposingRequest) -> Result<Intent, ComposingError> {
    use composing_request::Method;
    let Some(method) = req.method.clone() else {
        return Err(ComposingError::MissingMethod);
    };
    Ok(match method {
        Method::Start(m) => Intent::Start { text: m.text },
        Method::Append(m) => Intent::Append { ch: m.char },
        Method::AppendHyphen(_) => Intent::AppendHyphen,
        Method::ReplaceLast(m) => Intent::ReplaceLast {
            replacement: m.replacement,
        },
        Method::DeleteBackward(_) => Intent::DeleteBackward,
        Method::CommitDerived(_) => Intent::CommitDerived,
        Method::CommitRaw(_) => Intent::CommitRaw,
        Method::SelectSuggestion(m) => Intent::SelectSuggestion { text: m.text },
        Method::CommitPreeditThenInsertExternal(m) => {
            Intent::CommitPreeditThenInsertExternal { text: m.text }
        }
        Method::Reset(_) => Intent::Reset,
        Method::SetSelectedCandidateIndex(m) => {
            Intent::SetSelectedCandidateIndex { index: m.index }
        }
        Method::QueryState(_) => Intent::QueryState,
        Method::EnterContinuous(_) => Intent::EnterContinuous,
        Method::FetchAtPos(m) => Intent::FetchAtPos {
            position: m.position,
            frequency_entries: m.frequency_entries,
            now_ms: m.now_ms,
            custom_entries: m.custom_entries,
            enabled_sources_bitmask: m.enabled_sources_bitmask,
            literal_roman_candidate_disabled: m.literal_roman_candidate_disabled,
        },
        Method::CommitContinuous(m) => Intent::CommitContinuous {
            display_text: m.display_text,
            canonical_text: m.canonical_text,
            association_tl: m.association_tl,
            consumed_bytes: m.consumed_bytes as usize,
            syllable_count: clamp_syllable_count(m.syllable_count),
        },
        Method::ResetContinuous(_) => Intent::ResetContinuous,
    })
}

/// Pure dispatch entry: decode the proto request into an `Intent` and
/// apply it against `engine`. Generation-mismatch handling lives one
/// layer up in `EngineHandle::handle` (`handle.rs`); this fn is the
/// in-process Rust API also used directly by the workspace tests.
///
/// `FetchAtPos` is short-circuited here (not via `engine.apply`) so the
/// pure transition table stays free of lexicon access. See module docs.
// 中文: 純分派入口:解碼後套用到 engine。generation 同步由上層 EngineHandle 處理。
// 中文: FetchAtPos 在此短路處理,讓 transition.rs 維持 pure (不接觸 lexicon)。
pub fn handle(
    req: &ComposingRequest,
    engine: &mut Engine,
    config: &AppConfig,
) -> Result<ComposingResponse, ComposingError> {
    let intent = decode_intent(req)?;
    match intent {
        Intent::QueryState => Ok(engine.snapshot(config)),
        Intent::FetchAtPos {
            position,
            frequency_entries,
            now_ms,
            custom_entries,
            enabled_sources_bitmask,
            literal_roman_candidate_disabled,
        } => Ok(handle_fetch_at_pos(
            engine,
            position,
            &frequency_entries,
            now_ms,
            &custom_entries,
            enabled_sources_bitmask,
            literal_roman_candidate_disabled,
            config,
        )),
        intent => Ok(engine.apply(intent, config)),
    }
}

/// Phase 6 read-only continuous-input candidate query.
///
/// Returns a `ComposingResponse` snapshot of the current engine state
/// with `continuous: Some(ContinuousResponse { candidates })` populated
/// when `Phase::Continuous` and lexicon state is available; in any
/// other case returns the snapshot with `continuous = None` (an empty
/// candidate list is encoded as the carrier present with empty
/// `candidates`, distinct from "FetchAtPos was a no-op because state
/// was wrong").
///
/// `position != 0` is reserved for future partial-fetch use; the
/// engine treats it as an empty result today (matches the
/// `FetchAtPos.position` proto comment).
///
/// v3.5.9 A2: the candidate-assembly 6-step seam lives in
/// [`crate::continuous::assemble_candidates`]; this fn does the
/// phase/hanzi/position guards, the proto→domain hoists, and the wire
/// encoding around it.
// 中文: Phase 6 — 連續輸入候選讀取入口;短路處理,不經過 transition.rs。
// 中文: Phase 9.3a — 帶平台 FrequencyEntry[] + now_ms;dispatch 端建立 FrequencyMap 後送進 lexicon。
// 中文: Phase 9 Item 12 — 加帶平台 CustomDictEntry[];dispatch hoist 成 CustomEntry domain 後送進 lexicon 合成 + 去重。
// 中文: v3.5.9 A2 — 候選組裝 6-step seam 已移到 composing::continuous;此處只剩 guard + hoist + wire encode。
fn handle_fetch_at_pos(
    engine: &Engine,
    position: u32,
    frequency_entries: &[FrequencyEntry],
    now_ms: i64,
    custom_entries: &[CustomDictEntry],
    enabled_sources_bitmask: u32,
    literal_roman_candidate_disabled: bool,
    config: &AppConfig,
) -> ComposingResponse {
    let snapshot = engine.snapshot(config);
    let state = engine.snapshot_state();
    let Phase::Continuous { raw, .. } = &state.phase else {
        return snapshot;
    };
    // v3.5.8 Phase 9 Item 11 — hanzi guard (§15.3.E). The only input
    // modes are TL/POJ/TPS romanization; CJK never legitimately enters
    // the composing buffer. When it leaks in (paste, stale selection
    // residue) short-circuit to an empty candidate carrier instead of
    // letting the syllabifier / lexicon scan garbage. Ports the platform
    // D-8 guard (`LexiconService` Hanzi classification) into the engine
    // so the behavior survives the Item 13 platform-fallback retire.
    // Runs ahead of the reserved-position check because contaminated
    // `raw` is dead regardless of `position`.
    // 中文: Item 11 — 漢字誤入 composing buffer 時短路回空候選(carrier present),
    // 中文:   把平台 D-8 兜底搬進 engine,Item 13 retire 平台 fallback 後行為不流失。
    if is_hanzi(raw) {
        return with_continuous(snapshot, ContinuousResponse::default());
    }
    if position != 0 {
        // Position field is reserved (always 0 in v3.5.8); non-zero
        // returns an empty candidate carrier so the caller can still
        // tell "FetchAtPos was reached" vs "wrong phase".
        return with_continuous(snapshot, ContinuousResponse::default());
    }
    // v3.5.9 D / C-3b — mode upgrade: the raw buffer trumps
    // `config.input_mode` when TPS Bopomofo is detected. Existing
    // platform `AppConfig` builders map TPS to `"tl"` / `"poj"` for
    // legacy reasons (`ios/.../RustEngineBridge+Composing.swift:584-599`,
    // `android/.../engine/RustEngineBridge.kt:1270-1288` — both fold
    // TPS into `is_translate_swapped` and keep `input_mode` as the
    // underlying romanization choice), so the config string alone
    // would mis-classify a TPS buffer. Bopomofo chars are unambiguous
    // (`phonetics::contains_tps` mirrors the same detection used in
    // `engine/composing/src/derived.rs:27`), so we promote the parsed
    // mode to `InputMode::Tps` whenever any Bopomofo char is present.
    //
    // Single-source mode flow into `assemble_candidates`: the seam
    // derives every TPS-gated branch from `mode == InputMode::Tps`
    // internally — pre-C-3b had a parallel `is_tps: bool` arg that
    // duplicated this axis (`is_tps = contains_tps(raw)`, dual source
    // of truth). Dropping the bool eliminates split-brain risk.
    //
    // The POJ-vs-TL/English branch in the seam is unaffected: platform
    // builders DO map POJ → `"poj"` so config is reliable for that
    // axis; only TPS needs the `contains_tps` override.
    // 中文: D / C-3b — mode 升級:有 Bopomofo 字元時 raw 凌駕 config。
    // 中文:   現平台 AppConfig 把 TPS 折成 "tl"/"poj" + is_translate_swapped 旗標,
    // 中文:   config 字串無法判 TPS,故以 contains_tps(raw) 為唯一真相升級到 InputMode::Tps。
    // 中文:   單一 mode 直流入 assemble_candidates,刪掉 is_tps 雙軸 split-brain。
    // 中文:   POJ vs TL 不受影響(config 字串可靠),只有 TPS 需要 raw 偵測覆寫。
    let mode = if contains_tps(raw) {
        phonetics::InputMode::Tps
    } else {
        phonetics::api::parse_input_mode(&config.input_mode)
    };
    // Phase 9.3a: hoist proto-shaped `FrequencyEntry[]` into the
    // domain-typed `FrequencyMap` once per fetch; `lexicon` consumes
    // `&FrequencyMap` and stays proto-agnostic. Empty list → empty
    // map → `user_freq_boost(0) = 1.0` for every candidate (backward
    // -compatible with PR-9.2 platform builds that have not wired
    // user-frequency plumbing yet).
    let freq_map = build_frequency_map(frequency_entries);
    // v3.5.8 Phase 9 Item 12: hoist proto-shaped `CustomDictEntry[]`
    // into the domain-typed `CustomEntry` list once per fetch (mirror
    // of `build_frequency_map` above); `lexicon` consumes
    // `&[CustomEntry]` and stays proto-agnostic. Empty list = no
    // custom matches / feature disabled → zero synthesized candidates
    // and the `(roman, hanji)` dedupe is a no-op (backward-compatible
    // with builds that never set `FetchAtPos.custom_entries`).
    // 中文: Item 12 — proto CustomDictEntry[] → domain CustomEntry,空 list = 無 custom,合成 0 筆、去重 no-op。
    let custom = build_custom_entries(custom_entries);
    // PR-9.6 — normalise the source-toggle bitmask at the proto→domain
    // boundary: proto3 default `0` means "platform did not wire this"
    // (older / un-wired build) and maps to `u32::MAX` (legacy all-on),
    // reproducing pre-PR-9.6 behaviour where continuous candidates
    // ignored toggles. A real bitmask is never `0` because
    // `compute_filters` always sets the `dev` bit, so `0` is an
    // unambiguous absence marker. Normalising here (mirroring the
    // `build_frequency_map` / `build_custom_entries` hoists) keeps
    // `assemble_candidates` taking an already-resolved enabled bitmask —
    // no domain code has to know about the wire sentinel.
    // 中文: PR-9.6 — 在 proto→domain 邊界正規化 source-toggle bitmask;
    // 中文:   0 (proto3 預設,平台未接線) → u32::MAX (legacy 全開),重現 PR-9.6 前忽略 toggle 的行為。
    // 中文:   真實 bitmask 因 dev bit 必設故絕不為 0,0 為明確 absence sentinel;在此正規化讓 assemble_candidates 只收已解析值。
    let enabled_sources_bitmask = if enabled_sources_bitmask == 0 {
        u32::MAX
    } else {
        enabled_sources_bitmask
    };
    // v3.5.9 A2 seam — the 6-step assemble_candidates contract
    // (key build → span-local/partial fetch → recase → walker slot-0
    // prepend → POJ presentation pass → return). Wire encoding (step 6)
    // happens below via `raw_to_proto_candidate` + `with_continuous`.
    // PR-9.6 — `enabled_sources_bitmask` (already sentinel-normalised
    // above) flows into `ContinuousFetchCtx` so the span-local + partial
    // -prefix fetchers apply the same `Filter` the Tab3 browse path uses.
    let mut candidates = assemble_candidates(
        raw,
        &freq_map,
        now_ms,
        &custom,
        mode,
        enabled_sources_bitmask,
    );
    // INVARIANT_CONTINUOUS_LITERAL_ROMAN_CANDIDATE (§34): whenever composing
    // in TL/POJ — tone or no tone — surface the current composing result
    // (= the preedit WYSIWYG) as a roman-only candidate at index 0, so 漢羅
    // mixing commits the romanization in one tap without toggling 文/A and
    // the list does not jump when a tone is added. Display-layer prepend —
    // the segmentation / cost primitive (`assemble_candidates`) is never
    // touched (incidents S5/§18/S9). NOT re-run through Step 5's POJ recase:
    // `derived_display` is already the mode-correct POJ/TL literal.
    // 中文: §34 — TL/POJ 組字時(不論有無標調)把目前組字結果(= preedit WYSIWYG)當
    // 中文:   roman-only 候選放 index 0,漢羅一鍵上屏免切 文/A,加調時候選列不跳動。
    // 中文:   display 層 prepend,不碰 assemble_candidates 切詞/cost primitive;不跑 Step 5 POJ recase。
    //
    // §34 / S22 toggle (顯示羅馬字): when the user turns the setting OFF
    // the platform sends `literal_roman_candidate_disabled = true` and the
    // forced prepend is skipped — the dedupe `retain` lives inside this
    // block so it is skipped too. This suppresses ONLY the §34 WYSIWYG
    // prepend; any roman-only / OOV-synth candidate `assemble_candidates`
    // produced on its own stays. Inverted sentinel: proto3 default `false`
    // = show (legacy always-on), so un-wired builds are unaffected.
    // 中文: §34/S22 開關 — 使用者關閉時平台送 disabled=true,跳過 prepend(含內層去重);
    // 中文:   只關 §34 強制 prepend,assemble_candidates 自然產生的 roman 候選保留。
    if !literal_roman_candidate_disabled {
        if let Some(literal) = literal_roman_candidate(raw, config, mode) {
            // Drop a pre-existing IDENTICAL bare-roman (hanji-absent Tailo)
            // so the literal is not duplicated; dict rows with hanji stay (a
            // `tâi`/台 dict candidate and a bare `tâi` commit differ — Codex
            // pre-impl F5). 中文: 去重同 roman 的純羅馬字 Tailo;帶漢字的字典候選保留。
            candidates.retain(|c| !(c.hanji.is_none() && c.roman == literal.roman));
            candidates.insert(0, literal);
        }
    }
    with_continuous(
        snapshot,
        ContinuousResponse {
            candidates: candidates.into_iter().map(raw_to_proto_candidate).collect(),
        },
    )
}

/// Build the literal-roman candidate for 漢羅 fast input
/// (`INVARIANT_CONTINUOUS_LITERAL_ROMAN_CANDIDATE` §34 / dogfood S22).
///
/// Surfaces the **current composing result** — the preedit literal
/// (`derived_display`) — as a roman-only candidate **whenever** composing
/// in TL/POJ, tone or no tone. The candidate always mirrors the underline,
/// so the list does not jump when a tone is added: `tai`→`tai`,
/// `tai5`→`tâi`, `nng7`→`nn̄g`, `taigi`→`taigi`, `tai5-gi2`→`tâi-gí`. This
/// makes 漢羅 (mixed Han + roman) input commit the romanization in one tap
/// without toggling 文/A, even in 漢字 mode (PhahTaigi parity — the lomaji
/// candidate is always present, USER 2026-06-06 "邏輯 should consist").
///
/// Returns `Some` when:
/// * `mode` is TL or POJ — TPS is hanji-first (diacritic-glyph tones,
///   promoted to `InputMode::Tps` upstream); English excluded.
/// * the preedit literal is non-empty.
///
/// The candidate is roman-only (`hanji = None` → `CandidateMode::Tailo`),
/// with `roman == display_text ==` the preedit literal — WYSIWYG with the
/// underline (§30 literal-no-fold: tone marks only, no spelling fold). It
/// mirrors the preedit EXACTLY, so a tone-1/4 syllable or an unhyphenated
/// multi-syllable blob keeps its raw digits as the underline shows them
/// (`tai1`, `goa2ai3li2` — the engine does not auto-syllabify, §10.2). It
/// carries `canonical_tl` via `canonical_tl_form` so 詞頻 / 詞關聯 learn the
/// canonical `(∅, TL)` identity on commit (Core Principle #7; §24/§28).
// 中文: §34/S22 — 漢羅快速輸入的字面 roman 候選;TL/POJ 組字時「一律」顯示目前組字
// 中文:   結果(= preedit derived_display),不論有無標調(USER「邏輯 should consist」),
// 中文:   候選恆鏡 preedit → 加調時候選列不跳動。roman-only(hanji=None→Tailo),
// 中文:   恆等 preedit(§30 字面);canonical_tl 走 canonical_tl_form 保 #7 身分。
fn literal_roman_candidate(
    raw: &str,
    config: &AppConfig,
    mode: phonetics::InputMode,
) -> Option<RawCandidate> {
    if !matches!(mode, phonetics::InputMode::Tl | phonetics::InputMode::Poj) {
        return None;
    }
    let literal = crate::derived::derived_display(raw, config);
    if literal.is_empty() {
        return None;
    }
    let canonical_tl = phonetics::api::canonical_tl_form(&literal, mode);
    Some(RawCandidate {
        consumed_span: (0, raw.len() as u32),
        syllable_count: literal.split('-').count().min(u8::MAX as usize) as u8,
        display_text: literal.clone(),
        roman: literal,
        hanji: None,
        canonical_tl,
        score: 0.0,
        form: FORM_NOTONE,
        frequency: 0,
        bitmask: 0,
        mode: derive_mode(None),
        recency_rank: 0,
        coverage_kind: COVERAGE_KIND_FULL,
        is_custom: false,
    })
}

/// Inventory-injected hermetic test seam for the shadow + key projection
/// pipeline. v3.5.9 A2 moved the production path through
/// [`crate::continuous::assemble_candidates`], which builds the shadow
/// lattice once and derives the left-anchored keys inline (D1 fold).
/// This wrapper survives only so the
/// `engine/composing/tests/build_keys_tl_{lattice,hyphen,poj_diacritic}.rs`
/// integration tests can drive a hermetic inventory without installing
/// the global `LexiconHandle` singleton. Public (`#[doc(hidden)]`) for
/// the test crate boundary; production callers must NOT reach for it
/// (use the seam).
///
/// Pipeline (mirrors A1 [`crate::shadow::build_shadow_lattice`] +
/// [`crate::shadow::left_anchored_keys_from_lattice`]):
///
/// 1. Lowercase `raw` (ASCII only).
/// 2. `shadow::canonicalize_poj_shadow` (Phase 9 Item 9; v3.5.9 B-2
///    reshape) emits POJ ASCII in POJ mode (`chiah` stays `chiah`) and
///    TL ASCII identity in TL mode (`tó-uī` stays `tó-uī`).
/// 3. `shadow::build_hyphen_shadow` (Phase 9 Item 8) strips ASCII `-`.
/// 4. The two byte-offset maps compose into a single
///    `shadow_to_raw_end` so downstream `consumed_span_end` lines up
///    with platform UI commit slicing.
/// 5. The lattice builder (`crate::lattice::build_lattice`) walks the
///    shadow against the inventory family selected by `mode` (B-2:
///    `tl:` vs `poj:`) — both transforms ran upstream.
/// 6. The left-anchored projection (`start == 0`) emits a fused
///    toneless `{mode_prefix}:<key>` per ending (`tl:` in TL/English
///    mode, `poj:` in POJ mode).
// 中文: build_keys_tl 的可注入測試版 — 直接吃 SyllableInventory,跑「lowercase →
// 中文:   canonicalize_poj_shadow (Item 9) → hyphen-shadow (Item 8) → 音節切分 →
// 中文:   fused toneless key + raw byte offset」。
// 中文: 兩條 offset map (canonical→raw, shadow→canonical) 在此 compose 成單一 shadow→raw,
// 中文:   供 consumed_span_end 使用。
// 中文: v3.5.9 A2 後 production 走 composing::continuous::assemble_candidates(D1 fold:
// 中文:   build_shadow_lattice 單建);此 wrapper 只剩 integration test 用。
/// Injectable test seam for the FULL continuous-input key set — the base
/// reading's keys PLUS every alternate reading's
/// (`INVARIANT_TPS_DEFOLD_ENUMERATE` §35, TPS-only). Same shared
/// [`crate::shadow::build_continuous_keys`] production runs, so an
/// integration test can pin an alternate reading against a hermetic
/// inventory instead of only against production artifacts.
///
/// [`build_keys_tl_with_inventory`] stays the BASE-only seam: the pre-§35
/// tests that pin exact base key sets must keep seeing exactly those.
// 中文: 完整連續輸入鍵集的可注入測試接縫 = base 讀法 + 各替代讀法(§35,TPS-only),
// 中文:   與 production 共用 build_continuous_keys,故 integration test 可用 hermetic inventory 釘替代讀法。
// 中文: build_keys_tl_with_inventory 維持「只有 base」的接縫,§35 之前釘死鍵集的測試不受影響。
#[doc(hidden)]
pub fn build_continuous_keys_with_inventory(
    raw: &str,
    inv: &SyllableInventory,
    mode: phonetics::InputMode,
) -> Vec<(ConsumedSpan, String)> {
    crate::shadow::build_continuous_keys(raw, inv, mode).0
}

#[doc(hidden)]
pub fn build_keys_tl_with_inventory(
    raw: &str,
    inv: &SyllableInventory,
    mode: phonetics::InputMode,
) -> Vec<(ConsumedSpan, String)> {
    let (shadow, shadow_to_raw_end, lattice) = build_shadow_lattice(raw, inv, mode);
    // v3.5.9 B-2 — `left_anchored_keys_from_lattice` takes `mode` so the
    // emitted key prefix matches the inventory family the shadow lattice
    // was built against. It also takes `inv` for longest-match prefix
    // suppression (`INVARIANT_CONTINUOUS_LONGEST_MATCH_PREFIX`).
    left_anchored_keys_from_lattice(&shadow, &shadow_to_raw_end, &lattice, inv, mode)
}

/// v3.5.8 Phase 9 Item 12 — hoist proto-shaped `CustomDictEntry[]`
/// into the domain-typed [`CustomEntry`] list. Mirror of
/// `ranking::build_frequency_map`'s proto→domain boundary, kept here
/// so `lexicon` stays proto-agnostic. `roman` / `hanji` are the raw
/// stored `custom_dictionary.db` columns the platform marshalled
/// verbatim (NOT the legacy display-capitalized form) so the
/// `(roman, hanji)` dedupe key collides correctly against
/// `dict.bin`'s `DictionaryRecord.tl` / `.hanzi`. proto3 `optional
/// hanji` absent → `None` (romanization-only entry); present (even
/// empty) → `Some`.
///
/// v3.5.9 B-4 — `roman` is kept in its raw stored form here (TL or
/// POJ display, whichever the user typed). Canonicalization to TL
/// happens downstream at the `display_text` synthesis site only
/// (`lexicon::custom_entry_to_candidate` →
/// `phonetics::api::canonical_tl_form(roman, mode)`), NOT here. The
/// lattice / FST-key matching (`composing::shadow::custom_toneless_key`)
/// needs the user's native form to align with the mode-tagged
/// inventory introduced by B-1 / B-2; rewriting `roman` at this seam
/// would break that alignment for POJ-mode custom entries
/// (Codex pre-impl BLOCK #1, 2026-05-21).
// 中文: Item 12 — proto CustomDictEntry[] → domain CustomEntry;roman/hanji 是 custom_dictionary.db 原始欄位,
// 中文:   不是 legacy 顯示大寫化形式,確保 (roman,hanji) 去重鍵能與 dict.bin 正確碰撞。
// 中文: B-4 — roman 在此保留原 form(用戶 native),canonical TL fold 只發生在 display_text 合成端
// 中文:   (custom_entry_to_candidate → canonical_tl_form)。在此 rewrite roman 會破 B-2 POJ-family lattice 對齊。
fn build_custom_entries(entries: &[CustomDictEntry]) -> Vec<CustomEntry> {
    entries
        .iter()
        .map(|e| CustomEntry {
            roman: e.roman.clone(),
            hanji: e.hanji.clone(),
        })
        .collect()
}

fn raw_to_proto_candidate(c: RawCandidate) -> CandidateMessage {
    CandidateMessage {
        consumed_span_start: c.consumed_span.0,
        consumed_span_end: c.consumed_span.1,
        syllable_count: c.syllable_count as u32,
        display_text: c.display_text,
        score: c.score,
        form: c.form as u32,
        mode: c.mode.to_proto_i32(),
        // v3.5.8 Phase 9 Item 5 — `roman` is always non-empty for a
        // dictionary-sourced candidate; it is the display romanization
        // for the active input mode (TL, or POJ-display after the
        // continuous seam's POJ presentation pass). `hanji` is a proto3
        // `optional string` so prost serializes `None` as wire-absent
        // (distinguishes TAILO from defective empty-string emission).
        // See `docs/engine/continuous-candidate-display.md` §4.2.
        // 中文: Item 5 — roman 永有值,為當前 input mode 的顯示羅馬字(TL,或經 POJ pass 後的 POJ);
        // 中文:   hanji 為 proto optional,TAILO 候選送 None,wire 上是「absent」而非空字串。
        roman: c.roman,
        hanji: c.hanji,
        // v3.6.1 R2 — identity sidechannel: canonical TL (NOT the
        // POJ-rendered display `roman`). The platform round-trips it back
        // into `CommitContinuous.association_tl` on tap. Empty only for
        // TPS-OOV hanji-absent candidates with no recoverable dict TL.
        // 中文: R2 — canonical TL 身分 sidechannel(非 POJ render 後的顯示 roman);
        // 中文:   平台 tap 時 round-trip 回 CommitContinuous.association_tl。
        canonical_tl: c.canonical_tl,
    }
}

/// `snapshot` already has the right `preedit` / `effect` / `is_composing` /
/// `selected_candidate_index` for `Phase::Continuous`; only the continuous
/// carrier needs population.
fn with_continuous(
    mut snapshot: ComposingResponse,
    continuous: ContinuousResponse,
) -> ComposingResponse {
    snapshot.continuous = Some(continuous);
    snapshot
}

/// Clamp Phase-4-pinned syllable count range. The Phase 5 builder
/// caps at 4 (`engine/lexicon/src/continuous.rs:91-93`); the proto
/// field is u32 so callers could in principle send larger values.
/// Saturate to `u8::MAX` so Continuous transition.rs sees a typed
/// value matching `NailedSegment.syllable_count: u8`.
// 中文: 把 wire 上的 u32 壓回 u8,避免錯誤值 panic。
fn clamp_syllable_count(value: u32) -> u8 {
    value.min(u8::MAX as u32) as u8
}

#[cfg(test)]
mod tests {
    //! Unit tests for the pure helpers that stayed in dispatch.rs after
    //! v3.5.9 A2 (`raw_to_proto_candidate`, `clamp_syllable_count`).
    //! Helpers that moved into [`crate::continuous`] carry their tests
    //! into that module verbatim. Dispatch-level integration (decode
    //! round-trip, degraded paths) lives in
    //! `engine/composing/tests/dispatch_continuous.rs` because it needs
    //! the `Engine` + lexicon singletons.

    use super::*;
    use lexicon::FORM_NOTONE;

    #[test]
    fn clamp_syllable_count_saturates() {
        assert_eq!(clamp_syllable_count(0), 0);
        assert_eq!(clamp_syllable_count(1), 1);
        assert_eq!(clamp_syllable_count(255), 255);
        assert_eq!(clamp_syllable_count(256), 255);
        assert_eq!(clamp_syllable_count(u32::MAX), 255);
    }

    /// v3.5.8 Phase 9 Item 5 — `raw_to_proto_candidate` must propagate
    /// `roman` and `hanji` onto the wire. HANT records carry both;
    /// TAILO records emit `roman` only and leave proto `hanji` as
    /// `None` (proto3 `optional string` wire-absent, NOT `Some("")`).
    // 中文: Item 5 — raw_to_proto_candidate 把 roman + hanji 寫到 wire 的 hermetic 測試。
    // 中文:   HANT 帶兩者;TAILO 的 hanji 為 None,proto 上 wire-absent。
    #[test]
    fn raw_to_proto_candidate_propagates_roman_and_some_hanji() {
        let raw = RawCandidate {
            consumed_span: (0, 7),
            syllable_count: 2,
            display_text: "臺灣".to_owned(),
            roman: "tâi-uân".to_owned(),
            hanji: Some("臺灣".to_owned()),
            canonical_tl: "tâi-uân".to_owned(),
            score: 1.5,
            form: FORM_NOTONE,
            frequency: 12,
            bitmask: 0,
            mode: lexicon::CandidateMode::Hant,
            recency_rank: 1,
            coverage_kind: lexicon::COVERAGE_KIND_FULL,
            is_custom: false,
        };
        let proto = raw_to_proto_candidate(raw);
        assert_eq!(proto.roman, "tâi-uân");
        assert_eq!(proto.hanji.as_deref(), Some("臺灣"));
        assert_eq!(proto.display_text, "臺灣");
        // R2 — canonical TL sidechannel propagates onto the wire.
        assert_eq!(proto.canonical_tl, "tâi-uân");
    }

    #[test]
    fn raw_to_proto_candidate_emits_none_hanji_for_tailo() {
        let raw = RawCandidate {
            consumed_span: (0, 3),
            syllable_count: 1,
            display_text: "tāi".to_owned(),
            roman: "tāi".to_owned(),
            hanji: None,
            canonical_tl: "tāi".to_owned(),
            score: 0.5,
            form: FORM_NOTONE,
            frequency: 3,
            bitmask: 0,
            mode: lexicon::CandidateMode::Tailo,
            recency_rank: 1,
            coverage_kind: lexicon::COVERAGE_KIND_FULL,
            is_custom: false,
        };
        let proto = raw_to_proto_candidate(raw);
        assert_eq!(proto.roman, "tāi");
        assert!(proto.hanji.is_none());
        assert_eq!(proto.display_text, "tāi");
    }

    // ----- INVARIANT_CONTINUOUS_LITERAL_ROMAN_CANDIDATE (§34) -----
    // 漢羅 fast input: TL/POJ + written tone → roman-only literal candidate
    // (= preedit WYSIWYG) injected at index 0. Tests assert the GATE +
    // self-consistency; the literal string is asserted against
    // `derived_display` (the source of truth) rather than a hard-coded
    // diacritic so the two never drift.

    fn config_tl() -> AppConfig {
        AppConfig {
            tone_mode: String::new(),
            input_mode: "tl".to_string(),
            oo_doubletap_enabled: false,
            nn_doubletap_enabled: false,
            is_translate_swapped: false,
            is_association_recording_enabled: false,
            platform_id: 0,
            output_both_scripts: false,
        }
    }

    fn config_poj() -> AppConfig {
        AppConfig {
            input_mode: "poj".to_string(),
            ..config_tl()
        }
    }

    #[test]
    fn literal_roman_candidate_tl_single_syllable_is_tailo_wysiwyg() {
        // Headline case: `nng7` → the literal tone-marked roman (`nn̄g`),
        // roman-only (hanji None → Tailo), matching the preedit byte-for-byte.
        let cfg = config_tl();
        let cand = literal_roman_candidate("nng7", &cfg, phonetics::InputMode::Tl)
            .expect("nng7 must yield a literal roman candidate");
        assert_eq!(cand.roman, crate::derived::derived_display("nng7", &cfg));
        assert_eq!(cand.display_text, cand.roman); // WYSIWYG: display == roman
        assert_ne!(cand.roman, "nng7"); // a tone-mark conversion happened
        assert!(cand.hanji.is_none());
        assert_eq!(cand.mode, lexicon::CandidateMode::Tailo);
        assert_eq!(cand.consumed_span, (0, 4));
        assert!(!cand.canonical_tl.is_empty()); // #7 identity sidechannel set
    }

    #[test]
    fn literal_roman_candidate_tl_goa2_keeps_literal_spelling() {
        // §30 literal: TL `goa2` displays `goá` (mark on `a`, NOT folded to
        // `guá`); the identity `canonical_tl` DOES fold to canonical TL `guá`.
        let cfg = config_tl();
        let cand = literal_roman_candidate("goa2", &cfg, phonetics::InputMode::Tl).unwrap();
        assert_eq!(cand.roman, crate::derived::derived_display("goa2", &cfg));
        assert_eq!(cand.canonical_tl, "gu\u{e1}"); // guá — cross-mode #7 identity
    }

    #[test]
    fn literal_roman_candidate_poj_uses_poj_diacritics() {
        // POJ mode is literal too: `goa2` → `góa` (mark on `o`).
        let cfg = config_poj();
        let cand = literal_roman_candidate("goa2", &cfg, phonetics::InputMode::Poj).unwrap();
        assert_eq!(cand.roman, crate::derived::derived_display("goa2", &cfg));
        assert!(cand.hanji.is_none());
    }

    #[test]
    fn literal_roman_candidate_hyphenated_multisyllable_included() {
        // Hyphenated fully-toned multi-syllable converts → included.
        let cfg = config_tl();
        let cand = literal_roman_candidate("tai5-gi2", &cfg, phonetics::InputMode::Tl).unwrap();
        assert_eq!(
            cand.roman,
            crate::derived::derived_display("tai5-gi2", &cfg)
        );
        assert!(cand.roman.contains('-'));
        assert_eq!(cand.syllable_count, 2);
    }

    #[test]
    fn literal_roman_candidate_toneless_now_shown() {
        // USER 2026-06-06 「邏輯 should consist」: toneless input ALSO surfaces
        // the composing literal (= preedit), not only when toned. `taigi`
        // → `taigi`. (Previously gated on a written tone — now always shown.)
        let cfg = config_tl();
        let cand = literal_roman_candidate("taigi", &cfg, phonetics::InputMode::Tl).unwrap();
        assert_eq!(cand.roman, crate::derived::derived_display("taigi", &cfg));
        assert_eq!(cand.roman, "taigi");
        assert!(cand.hanji.is_none());
    }

    #[test]
    fn literal_roman_candidate_partial_single_syllable_shown() {
        // The candidate mirrors the preedit at every keystroke so the list
        // does not jump as the user types toward / past a tone. `ta` → `ta`.
        let cfg = config_tl();
        let cand = literal_roman_candidate("ta", &cfg, phonetics::InputMode::Tl).unwrap();
        assert_eq!(cand.roman, "ta");
    }

    #[test]
    fn literal_roman_candidate_mirrors_preedit_verbatim_with_digits() {
        // The candidate is EXACTLY the preedit (§30 / §10.2): a tone-1/4
        // syllable and an unhyphenated multi-syllable blob keep their raw
        // digits as the underline shows them — the engine does not
        // auto-syllabify, and the candidate must not diverge from the
        // underline (consistency).
        let cfg = config_tl();
        for raw in ["tai1", "goa2ai3li2", "tai5gi2"] {
            let cand = literal_roman_candidate(raw, &cfg, phonetics::InputMode::Tl).unwrap();
            assert_eq!(cand.roman, crate::derived::derived_display(raw, &cfg));
        }
    }

    #[test]
    fn literal_roman_candidate_trailing_hyphen_mirrors_preedit() {
        // A trailing hyphen mirrors the preedit too (`tai5-` → `tâi-`); the
        // candidate stays consistent with the underline.
        let cfg = config_tl();
        let cand = literal_roman_candidate("tai5-", &cfg, phonetics::InputMode::Tl).unwrap();
        assert_eq!(cand.roman, crate::derived::derived_display("tai5-", &cfg));
    }

    #[test]
    fn literal_roman_candidate_empty_is_none() {
        // No composing content → no candidate.
        assert!(literal_roman_candidate("", &config_tl(), phonetics::InputMode::Tl).is_none());
    }

    #[test]
    fn literal_roman_candidate_tps_and_english_excluded() {
        // TPS is hanji-first (diacritic-glyph tones, not ASCII digits);
        // English is not Taigi romanization. Both excluded by the mode gate.
        let cfg = config_tl();
        assert!(literal_roman_candidate("nng7", &cfg, phonetics::InputMode::Tps).is_none());
        assert!(literal_roman_candidate("nng7", &cfg, phonetics::InputMode::English).is_none());
    }

    #[test]
    fn literal_roman_candidate_poj_doubletap_mirrors_preedit() {
        // POJ `oo`/`nn` doubletap rewrites the spelling (`oo1`→`o͘1`); the
        // candidate mirrors the preedit verbatim (including any residual
        // tone-1/4 digit) — consistency with the underline, no special gate.
        let cfg = AppConfig {
            input_mode: "poj".to_string(),
            oo_doubletap_enabled: true,
            nn_doubletap_enabled: true,
            ..config_tl()
        };
        for raw in ["oo1", "oo4", "oo2"] {
            let cand = literal_roman_candidate(raw, &cfg, phonetics::InputMode::Poj).unwrap();
            assert_eq!(cand.roman, crate::derived::derived_display(raw, &cfg));
        }
    }
}
