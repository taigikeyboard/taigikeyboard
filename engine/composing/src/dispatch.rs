//! Decode `ComposingRequest` → `Intent`, apply against `Engine`, encode the
//! `ComposingResponse`. The generation-mismatch reset path also lives here
//! (per plan §5b.2).
//!
//! `Intent::FetchAtPos` is the v3.5.8 Phase 6 read-only continuous-input
//! candidate query. Unlike the other 12+ intents (which mutate engine
//! state via `transition::apply`), `FetchAtPos` needs lexicon state
//! (`lexicon::EngineHandle::with_state` — prefix index + dictionary +
//! syllable inventory) and so it is resolved here in dispatch outside
//! the pure transition table. The mode-aware key construction
//! (TL/POJ → `tl:<lowered>`; TPS → `phonetics::tps_to_tl` per Bopomofo
//! span → strip trailing tone digit → `tl:<toneless>`) also runs here,
//! per the Phase 5 module contract pinned in
//! `engine/lexicon/src/continuous.rs:24-33`.

// 中文: 將 protobuf ComposingRequest 解碼成 Intent,套用到 Engine 後產出回應。
// 中文: 純函式分派層,不處理 generation 同步 (那由 EngineHandle 負責)。
// 中文: Phase 6 新增 — FetchAtPos 在 dispatch 短路處理 (需要 lexicon state + 模式相關 key 構造)。

use crate::api::{ComposingError, Engine, Intent, Phase};
use crate::shadow::{
    build_partial_prefix_key_tl, build_shadow_lattice, custom_toneless_key,
    greedy_longest_syllabification, left_anchored_keys_from_lattice, span_min_syllable_count,
    strip_ascii_tone_digits, MAX_SYLLABLES,
};
use crate::syllabifier::tps as tps_syll;
use lexicon::{
    best_candidate_for_key, classification::is_hanzi, derive_mode, fetch_candidates_for_keys,
    fetch_partial_prefix_candidates, ConsumedSpan, CustomEntry, EngineHandle as LexiconHandle,
    RawCandidate, SyllableInventory, COVERAGE_KIND_FULL, FORM_NOTONE,
};
use phonetics::{contains_tps, tps_to_tl};
use protos::engine::{
    composing_request, AppConfig, CandidateMessage, ComposingRequest, ComposingResponse,
    ContinuousResponse, CustomDictEntry, FrequencyEntry,
};
use ranking::{build_frequency_map, decayed_user_weight_delta, recency_rank, FrequencyMap};

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
        },
        Method::CommitContinuous(m) => Intent::CommitContinuous {
            display_text: m.display_text,
            canonical_text: m.canonical_text,
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
        } => Ok(handle_fetch_at_pos(
            engine,
            position,
            &frequency_entries,
            now_ms,
            &custom_entries,
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
// 中文: Phase 6 — 連續輸入候選讀取入口;短路處理,不經過 transition.rs。
// 中文: Phase 9.3a — 帶平台 FrequencyEntry[] + now_ms;dispatch 端建立 FrequencyMap 後送進 lexicon。
// 中文: Phase 9 Item 12 — 加帶平台 CustomDictEntry[];dispatch hoist 成 CustomEntry domain 後送進 lexicon 合成 + 去重。
fn handle_fetch_at_pos(
    engine: &Engine,
    position: u32,
    frequency_entries: &[FrequencyEntry],
    now_ms: i64,
    custom_entries: &[CustomDictEntry],
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
    // Pick syllabifier path by inspecting the raw buffer rather than
    // `config.input_mode`: existing platform `AppConfig` builders map
    // TPS to `"tl"` for legacy reasons (`ios/.../RustEngineBridge.swift`
    // appConfig builder, `android/.../ime/text/composing/ComposingManager.kt::resolveMode`),
    // so config alone would mis-classify a TPS buffer as TL. Bopomofo
    // chars are unambiguous (`contains_tps` mirrors the same
    // detection used in `engine/composing/src/derived.rs:17`).
    let is_tps = contains_tps(raw);
    // Input mode is parsed here (ahead of key construction) because the
    // POJ→TL canonicalize step inside `build_keys_tl` is mode-gated:
    // toneless pure-ASCII POJ (`chiah`, `chhia`, `goa`) must fold POJ
    // spelling into TL or every ch-/oa-/oe- word yields zero continuous
    // candidates, while TL ASCII must keep the identity fast-path (the
    // F3C `tó-uī`→`toui` gate — see `canonicalize_poj_shadow`). Unlike
    // the TPS-vs-TL distinction above (which `config.input_mode` cannot
    // make because platform builders legacy-map TPS→`"tl"`), POJ-vs-TL
    // IS reliable from config: builders map POJ→`"poj"` (the same signal
    // the POJ-render block below already trusts). TPS routes through
    // `build_keys_tps` so `is_poj` never reaches it.
    // 中文: input mode 提前 parse — build_keys_tl 的 POJ→TL canonicalize 需 mode-gate;
    // 中文:   無調號純 ASCII POJ 須摺成 TL,否則 ch-/oa-/oe- 連續候選全空;TL ASCII 維持 identity(F3C gate)。
    let mode = phonetics::api::parse_input_mode(&config.input_mode);
    let is_poj = mode == phonetics::InputMode::Poj;
    let keys = if is_tps {
        build_keys_tps(raw)
    } else {
        build_keys_tl(raw, is_poj)
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
    // `mode` (parsed above for the key-construction gate) also drives two
    // presentation steps below: per-segment recasing (else branch) and,
    // for POJ, the TL→POJ-display render of the assembled candidate list.
    // It is in scope here so the partial-prefix branch's candidates are
    // covered by the POJ pass too.
    // 中文: 上方已 parse 的 mode 同時驅動下方 recase 與 POJ render;partial-prefix 候選也吃到 POJ pass。
    let mut candidates = if keys.is_empty() {
        // v3.5.8 Phase 9 Item 10 — partial-prefix fallthrough. The
        // syllabifier produced no valid ending (e.g. `raw = "gu"`,
        // `"t"`), so the lookup-exact path is dead. Try a TL/POJ
        // `lookup_prefix` instead so the user still sees engine
        // candidates while typing toward the first syllable
        // boundary. Spec: `docs/engine/continuous-candidate-display.md`
        // §15.3.D + §15.5. TPS partial-prefix is out of scope —
        // there is no TPS → TL partial-syllable mapping (a leading
        // Bopomofo initial like `ㄉ` carries no terminator, so
        // `phonetics::tps_to_tl` cannot produce a valid `tl:` prefix).
        // 中文: Item 10 — syllabifier 切不出邊界時改走 TL/POJ partial-prefix;TPS 沒對應 partial map,跳過。
        if is_tps {
            Vec::new()
        } else {
            fetch_via_lexicon_partial(raw, &freq_map, now_ms, &custom, is_poj)
        }
    } else {
        let raw_len = raw.len() as u32;
        let mut c = fetch_via_lexicon(&keys, raw_len, &freq_map, now_ms, &custom);
        // v3.5.8 (Codex pre-impl 2A locus = candidate construction):
        // case each span-local candidate's roman to mirror the user's
        // raw input for its consumed span, so the displayed candidate
        // already shows `Hit` / `tui` and tap commits it verbatim
        // (display == commit). The legacy platform
        // `SuggestionCaseTransformer` is bypassed for continuous, so
        // this is the single casing source. Only the presentation
        // `roman` is touched — `display_text` (canonical freq/NextWord
        // key) and `hanji` are deliberately left intact.
        // 中文: 連續候選 roman 依該段 raw 還原大小寫(display==commit);
        // 中文:   平台 SuggestionCaseTransformer 對連續 bypass,此為唯一源;
        // 中文:   只改呈現 roman,canonical display_text / hanji 不動。
        for cand in &mut c {
            let (cs, ce) = cand.consumed_span;
            if let Some(seg) = raw.get(cs as usize..ce as usize) {
                cand.roman = recase_roman(&cand.roman, seg, mode);
            }
        }
        // v3.5.8 S2 — whole-sentence walker. TPS excluded (S1 Codex Q5
        // deferred TPS multi-start; the lattice builder is TL/POJ
        // only). The synthesized full-buffer best path is explicitly
        // prepended at slot 0 (Codex pre-impl S2 Q1 — the 8-dim
        // `SortKey` cannot guarantee slot 0 on its own). Span-aware
        // de-dup against the synth (Codex pre-impl S2 Q1d): drop any
        // span-local candidate identical on
        // `(roman, hanji, consumed_span)` so slot 0 is unique (e.g. a
        // real left-anchored full-buffer dict word equal to the walker
        // path — keep the walker's at slot 0, not a duplicate slot N).
        // 中文: S2 — 全句 walker (TPS 排除,S1 Q5 deferred)。合成全 buffer 最佳路徑
        // 中文:   explicit prepend slot 0 (Codex S2 Q1);與 synth 同 (roman,hanji,span)
        // 中文:   的 span-local 候選去掉,保 slot 0 唯一 (Codex S2 Q1d)。
        if !is_tps {
            if let Some(slot0) = fetch_walker_slot0(raw, raw_len, &freq_map, now_ms, mode, &custom)
            {
                c.retain(|x| {
                    !(x.roman == slot0.roman
                        && x.hanji == slot0.hanji
                        && x.consumed_span == slot0.consumed_span)
                });
                c.insert(0, slot0);
            }
        }
        c
    };
    // v3.5.8 — POJ-display render. The Continuous platform builders are
    // mode-agnostic by design (Item 13: "the engine owns input-mode
    // handling"); mirror the engine-side NextWord POJ render
    // (`engine/nextword/src/filter.rs`). Rewrite the presentation
    // `roman` TL→POJ (`oo`→`o͘`, `nn`→`ⁿ`, …) for EVERY emitted
    // candidate — span-local (recased above), the walker slot-0 best
    // candidate inserted at index 0, and the partial-prefix branch — so
    // display AND the platform-formatted commit (both derive from
    // `roman`) are POJ. `display_text` / `hanji` (the canonical commit +
    // `user_frequency.db` key) are deliberately untouched. No-op for
    // TL / English / (legacy-mapped) TPS.
    // 中文: POJ 顯示渲染。平台 Continuous builder 刻意 mode-agnostic(Item 13:engine 管 input mode);
    // 中文:   對齊引擎端 NextWord POJ render。對「每一筆」候選(span-local / walker slot-0 最佳候選 /
    // 中文:   partial-prefix)把呈現 roman TL→POJ,讓顯示與平台 commit(皆源自 roman)一致為 POJ;
    // 中文:   canonical display_text / hanji 不動。TL/English/(legacy 映射)TPS 為 no-op。
    // 中文:   render 可能讓 custom POJ 與 dict TL 兩筆變相同 → dedupe_rendered_continuous 收尾去重。
    if mode == phonetics::InputMode::Poj {
        for cand in &mut candidates {
            cand.roman = render_roman_for_mode(&cand.roman, mode);
        }
        dedupe_rendered_continuous(&mut candidates);
    }
    with_continuous(
        snapshot,
        ContinuousResponse {
            candidates: candidates.into_iter().map(raw_to_proto_candidate).collect(),
        },
    )
}

/// Build TL/POJ FST keys from `Phase::Continuous { raw }`. Thin wrapper
/// that pulls the `SyllableInventory` out of the lexicon singleton and
/// delegates to [`build_keys_tl_with_inventory`]; if the inventory is
/// unavailable (platform did not supply `syllables.fst` at install
/// time, or lexicon was never installed) it degrades to an empty list
/// rather than panicking. The pure-function split exists so dispatch
/// tests can exercise the hyphen-shadow + key-build pipeline against a
/// hermetic inventory without touching the global handle.
///
/// **Input contract** (mirrors Phase 5 `fetch_candidates_for_endings`,
/// `engine/lexicon/src/continuous.rs:24-33`): `raw` is either canonical
/// TL ASCII — toneless (`tsua`) or numeric tone (`tsua7`, `tai1bak4`) —
/// or POJ-display ASCII / non-ASCII (`pe̍h-ōe-jī`, `chóa`, `so͘`, `peⁿ`,
/// `tâi5-ban3`). Hyphen `-` is accepted as a syllable-boundary marker
/// (Phase 9 Item 8), reflecting POJ/TL convention (`tâi-uân`,
/// `pe̍h-ōe-jī`). The toneless FST key is built by lowercasing,
/// canonicalizing POJ-display to ASCII TL (Phase 9 Item 9), dropping
/// every ASCII `-` (Item 8), then dropping every ASCII tone digit —
/// converging on the upstream `dictionary/common/notone.py::remove_tone`
/// surface (`[\d\-]` plus the diacritic / `o\u{0358}` / `\u{207f}`
/// substitutions baked into the dictionary builder).
// 中文: TL/POJ key 構造 — 經 lexicon SyllableInventory 切音節後,把 lower(shadow[0..end]) 去掉所有 ASCII 數字形成 fused toneless key,加 "tl:" 前綴。
// 中文: 對應 dictionary/common/notone.py 的 [\d\-] 規則 + 顯示層 POJ diacritic / o\u{0358} / \u{207f} 等非 ASCII 寫法:
// 中文:   digit 半邊在 strip_ascii_tone_digits;hyphen 半邊在 build_hyphen_shadow;POJ-display 半邊在 canonicalize_poj_shadow (Phase 9 Item 9)。
fn build_keys_tl(raw: &str, is_poj: bool) -> Vec<(ConsumedSpan, String)> {
    LexiconHandle::with_state(|state| {
        let Some(inv) = state.syllable_inventory.as_ref() else {
            return Ok(Vec::new());
        };
        Ok(build_keys_tl_with_inventory(raw, inv, is_poj))
    })
    .unwrap_or_default()
}

/// Inventory-injected variant of [`build_keys_tl`]. Runs the v3.5.8
/// Phase 9 Items 8 + 9 canonicalize → hyphen-shadow → syllabify
/// pipeline:
///
/// 1. Lowercase `raw` (ASCII only — non-ASCII codepoints stay untouched
///    here so the canonicalize stage can detect them).
/// 2. [`crate::shadow::canonicalize_poj_shadow`] (Item 9) folds POJ-display
///    input (`pe̍h`, `chóa`, `peⁿ`, `so͘`) into ASCII TL spelling, returning
///    `(canonical, canonical_to_raw_end)`. Pure-ASCII input is passed
///    through identity in TL mode (preserving every Item 8 hyphen-shadow
///    contract pin); in POJ mode (`is_poj`) it ALSO runs the POJ→TL
///    spelling chain so toneless ASCII POJ like `chiah`/`goa` keys into
///    `tl:tsiah`/`tl:gua` instead of returning zero candidates.
/// 3. [`crate::shadow::build_hyphen_shadow`] (Item 8) strips ASCII `-` from
///    the canonical buffer and returns `(shadow, shadow_to_canonical_end)`.
/// 4. The two byte-offset maps compose into a single
///    `shadow_to_raw_end` so downstream `consumed_span_end` lines up
///    with what platform UI slices on commit.
/// 5. The lattice builder (`crate::lattice::build_lattice`) walks the
///    shadow via `valid_span_endings` against the
///    inventory (which has no hyphenated entries and no POJ-display
///    keys — both transforms run upstream).
/// 6. For each shadow ending, build the fused toneless `tl:<key>` from
///    `shadow[..end]` (digit strip).
///
/// Public (but `#[doc(hidden)]`) so the workspace integration tests
/// `engine/composing/tests/build_keys_tl_*` can drive a hermetic
/// inventory without installing the global `LexiconHandle` singleton.
/// Production callers go through [`build_keys_tl`]; the `doc(hidden)`
/// attribute keeps this symbol off the public docs and signals that it
/// is a test-injection seam, not a stable API.
// 中文: build_keys_tl 的可注入測試版 — 直接吃 SyllableInventory,跑「lowercase → canonicalize_poj_shadow (Item 9) → hyphen-shadow (Item 8) → 音節切分 → fused toneless key + raw byte offset」。
// 中文: 兩條 offset map (canonical→raw, shadow→canonical) 在此 compose 成單一 shadow→raw,供 consumed_span_end 使用。
// 中文: 開放 pub 是為了 integration test 可以 inject hermetic inventory;`#[doc(hidden)]` 標示其為 test seam 非穩定 API。
#[doc(hidden)]
pub fn build_keys_tl_with_inventory(
    raw: &str,
    inv: &SyllableInventory,
    is_poj: bool,
) -> Vec<(ConsumedSpan, String)> {
    let (shadow, shadow_to_raw_end, lattice) = build_shadow_lattice(raw, inv, is_poj);
    left_anchored_keys_from_lattice(&shadow, &shadow_to_raw_end, &lattice)
}

/// v3.5.8 S2 (Codex post-impl P1, 2026-05-16) — `consumed_span` for
/// the synthesized slot-0 candidate, or `None` to suppress the synth.
///
/// The walker spans the **shadow** (`0..shadow_len`); its slot-0
/// candidate commits in **raw** byte space under forward-only Model B.
/// They line up only when the shadow's end maps back to the full raw
/// buffer. A **trailing** hyphen (`tai-`, `tai-bak-`) is stripped from
/// the shadow and the S1 `build_hyphen_shadow` contract keeps it in
/// the pending raw buffer, so `shadow_to_raw_end[shadow_len] <
/// raw_len`. Emitting a `(0, raw_len)` full-buffer synth there would
/// wrongly consume / drop that pending `-`, violating the S1
/// trailing-hyphen contract — so suppress the synth and leave the
/// span-local list untouched (pre-S2 behavior). Leading / internal
/// hyphens fold INTO the consumed prefix
/// (`shadow_to_raw_end[shadow_len] == raw_len`) and still synth.
// 中文: S2 (Codex post-impl P1) — walker 跨 shadow,slot-0 在 raw 空間 commit。
// 中文:   尾端 `-` 被 shadow 剝掉且 S1 契約留在 pending → shadow_to_raw_end[len] < raw_len;
// 中文:   此時發 (0,raw_len) 會誤吃 pending `-`,故抑制 synth(維持 pre-S2)。leading/internal `-` 已併入前綴仍 synth。
fn synth_consumed_span(
    shadow_to_raw_end: &[usize],
    shadow_len: usize,
    raw_len: u32,
) -> Option<(u32, u32)> {
    // `shadow_to_raw_end` always has length `shadow_len + 1`
    // (`build_hyphen_shadow` / `canonicalize_poj_shadow` contract),
    // so indexing `[shadow_len]` is in bounds.
    (shadow_to_raw_end[shadow_len] as u32 == raw_len).then_some((0, raw_len))
}

/// v3.5.8 S2 — the whole-sentence walker's single synthesized
/// full-buffer best-path candidate, or `None` when no edge chain
/// spans the buffer (sub-syllable partial-prefix input — leaves the
/// existing span-local list untouched, pre-S2 behavior preserved).
///
/// The caller ([`handle_fetch_at_pos`]) **explicitly prepends** this
/// at candidate slot 0 (Codex pre-impl S2 Q1: the 8-dim `SortKey`
/// alone cannot guarantee slot 0 — a high-`frequency` left-anchored
/// full-buffer dict hit would tie on coverage then beat the synth on
/// score). The walker stays pure + shadow-space native; this fn is
/// the composing↔lexicon seam (Codex pre-impl S2 Q1b): it holds
/// `LexiconHandle` state and injects an edge-content provider that
/// reuses `lexicon::best_candidate_for_key` (NOT a duplicated
/// `record_to_candidate`).
///
/// Synthesis (Codex pre-impl S2 Q1-roman: existing columns suffice):
/// - `roman` = each edge's roman joined with one ASCII space
///   (`docs/engine/continuous-input-ranking.md` §10.2 segmented rule:
///   roman line gets word-boundary spaces).
/// - `hanji` = `Some(edge hanji joined with NO space)` iff **every**
///   edge had a dict hanji, else `None` (a no-hanji path → the
///   synthesized roman best-path; subsumes paused Bug 2 / §1 — NOT a
///   special-case fallback, `feedback_no_redundant_fallback`).
/// - `display_text` = `hanji.unwrap_or(roman)` (mirrors
///   `record_to_candidate` at the path level — same commit /
///   `user_frequency.db` write-key contract).
/// - `consumed_span = (0, raw_len)` (via [`synth_consumed_span`] —
///   suppressed entirely on a trailing-hyphen buffer so the pending
///   `-` is not mis-committed, Codex post-impl S2 P1), `coverage_kind
///   = FULL`, `is_custom = false`, `frequency = 0` (synthesized — not
///   a dict freq; irrelevant since slot 0 is an explicit prepend).
///
/// TPS is excluded (S1 Codex Q5 deferred TPS multi-start; the lattice
/// builder is TL/POJ only) — caller gates on `!is_tps`.
// 中文: S2 — 全句 walker 的單一合成全 buffer 最佳路徑候選 (無法整段覆蓋時 None,維持 pre-S2)。
// 中文: 由 handle_fetch_at_pos explicit prepend 到 slot 0 (Codex S2 Q1:SortKey 無法保證 slot 0)。
// 中文: walker 純 shadow-space;此函式 = composing↔lexicon seam (Q1b),持 LexiconHandle state、
// 中文:   注入重用 best_candidate_for_key 的 edge provider (不複製 record_to_candidate)。
// 中文: roman = 各 edge roman 以單空格 join (§10.2);hanji = 全 edge 皆有 hanji 才 Some(無空格 join),
// 中文:   否則 None → 合成羅馬字最佳路徑 (吞 Bug2/§1,非 fallback)。TPS 排除 (S1 Q5 deferred)。
/// v3.5.8 — derive the keyboard `LetterCase` intent from a user raw
/// input segment so a continuous candidate's roman mirrors the case the
/// user actually typed for *that* span (`Hittui` → segment `Hit`
/// Uppercased, segment `tui` Lowercased). Only alphabetic chars count
/// (tone digits / hyphens ignored); no alpha → `Lowercased`. This is
/// the engine-owned continuous casing of Codex pre-impl 2A locus =
/// candidate construction; the legacy platform `SuggestionCaseTransformer`
/// is bypassed for continuous candidates so this is the single source.
// 中文: 由使用者該段 raw 推導 LetterCase,讓連續候選 roman 跟著使用者實際打的大小寫
// 中文:   (Hittui → Hit 大寫、tui 小寫)。只看英文字母;無字母 → Lowercased。
// 中文:   2A locus = 候選構造;平台 SuggestionCaseTransformer 對連續候選 bypass,此為唯一源。
fn raw_segment_letter_case(raw_seg: &str) -> phonetics::case_transform::LetterCase {
    use phonetics::case_transform::LetterCase;
    let mut alphas = raw_seg.chars().filter(|c| c.is_alphabetic());
    let Some(first) = alphas.next() else {
        return LetterCase::Lowercased;
    };
    let rest_all_upper = alphas.all(|c| c.is_uppercase());
    if first.is_uppercase() && rest_all_upper {
        LetterCase::CapsLocked
    } else if first.is_uppercase() {
        LetterCase::Uppercased
    } else {
        LetterCase::Lowercased
    }
}

/// Apply [`raw_segment_letter_case`] of `raw_seg` to `roman` via the
/// tone-letter-aware `phonetics::case_transform::transform_input_case`
/// (handles POJ/TL diacritic letters; the raw span is used only to
/// derive the case intent, never sliced against the roman, so a
/// toneless-ASCII raw vs tone-diacritic roman length mismatch is a
/// non-issue — Codex pre-impl 2026-05-18).
// 中文: 用 raw_seg 推得的 case 經 transform_input_case(tone-aware)套到 roman;
// 中文:   raw 只用來決定 case 意圖,不與 roman 對齊切片,故長度不一致無妨。
fn recase_roman(roman: &str, raw_seg: &str, mode: phonetics::InputMode) -> String {
    phonetics::case_transform::transform_input_case(roman, raw_segment_letter_case(raw_seg), mode)
}

/// v3.5.8 — render a continuous candidate's presentation `roman` for the
/// active input mode. POJ: rewrite TL-display → POJ-display
/// (`oo`→`o͘`, `nn`→`ⁿ`, `ua`→`oa`, …) via
/// [`phonetics::api::tl_display_to_poj_display`], then re-impose the
/// roman's own letter-case POJ-grapheme-aware. The TL→POJ rewriter only
/// title-cases (it checks `first.is_uppercase()` then stops), so a
/// CapsLocked candidate (`HOO`) would otherwise collapse to title case
/// (`Ho͘`); `transform_input_case` with the detected `LetterCase` and
/// `InputMode::Poj` restores it (`HO͘`) — the same helper `recase_roman`
/// already trusts for POJ diacritics (Codex pre-impl 2026-05-19 BLOCK).
/// Case detection reads the (already-recased) roman's own alpha chars
/// via [`raw_segment_letter_case`]: span-local candidates carry one
/// uniform case so this is exact. A multi-segment walker path with
/// heterogeneous casing collapses to one bucket derived from the whole
/// string — first alpha lowercase ⇒ all-lowercase; first upper but not
/// all remaining uppercase ⇒ leading-cap only; uniformly upper ⇒
/// all-caps — a bounded POJ-only edge far outside normal use, not the
/// reported `oo`/`nn` defect. TL / English / (legacy-mapped) TPS:
/// identity. Presentation only — never feed `display_text` (the
/// canonical commit / `user_frequency.db` key) here.
// 中文: 依 input mode 渲染連續候選呈現 roman。POJ:TL→POJ-display 後,
// 中文:   以候選自身字母 case 經 transform_input_case(POJ-grapheme-aware)還原大小寫
// 中文:   — TL→POJ rewriter 只 title-case,CapsLock(HOO)會塌成 Ho͘,此處還原成 HO͘
// 中文:   (Codex pre-impl 2026-05-19 BLOCK)。span-local 單段 case 均勻故精確;
// 中文:   walker 多段異質大小寫依整串首字母塌成單一桶(首小寫⇒全小寫;
// 中文:   首大寫但其餘非全大寫⇒僅首字大寫;全大寫⇒全大寫)— POJ-only 邊角,非回報 bug。
// 中文:   TL/English/(legacy 映射)TPS = identity。只處理呈現 roman,勿傳 display_text。
fn render_roman_for_mode(roman: &str, mode: phonetics::InputMode) -> String {
    if mode != phonetics::InputMode::Poj {
        return roman.to_string();
    }
    let case = raw_segment_letter_case(roman);
    let poj = phonetics::api::tl_display_to_poj_display(roman);
    phonetics::case_transform::transform_input_case(&poj, case, phonetics::InputMode::Poj)
}

/// v3.5.8 — collapse continuous candidates that became identical only
/// after the POJ render. The pre-render `(roman, hanji, consumed_span)`
/// dedupe (`lexicon::dedupe_by_roman_hanji_span`) runs on TL spellings,
/// so a custom entry stored in POJ display form (`gô͘`) and a `dict.bin`
/// entry in TL (`gôo`) sharing one hanji + span both survive it, then
/// [`render_roman_for_mode`] renders both to `gô͘` → a visible
/// duplicate. First-wins keeps the earlier row, so the prepended
/// whole-sentence best candidate at index 0 is never dropped. It is
/// behavior-neutral whenever `hanji` is `Some`: the collision key pins
/// the same hanji and `display_text` (the commit / `user_frequency.db`
/// key) is that hanji for BOTH the custom and the `dict.bin` candidate,
/// so which row survives cannot change what commits. The only bounded
/// asymmetry is a romanization-only (`hanji == None`) custom-after-dict
/// collision: the survivor's `display_text` is the earlier (dict TL)
/// form — a frequency-key granularity nuance only; the committed
/// document text is the rendered `roman`, identical for both. Called
/// in the POJ branch only — for TL/English/TPS the render is identity
/// so the pre-render dedupe already settled every key.
// 中文: POJ render 後才相等的候選去重(custom 存 POJ `gô͘` vs dict TL `gôo`,
// 中文:   同 hanji+span 過不了 TL 拼寫的 pre-render 去重,render 後皆 `gô͘`)。
// 中文:   first-wins → index 0 整句最佳候選不被丟。hanji 存在時行為中性
// 中文:   (碰撞鍵鎖同一 hanji,custom/dict 的 display_text 都是該 hanji,commit 不變);
// 中文:   唯 hanji==None 的 custom-after-dict 留下 dict TL 形 display_text(僅 freq-key 粒度,
// 中文:   commit 的文件字串是 render 後 roman,兩者相同)。只在 POJ 分支呼叫。
fn dedupe_rendered_continuous(candidates: &mut Vec<RawCandidate>) {
    use std::collections::HashSet;
    let mut seen: HashSet<(String, Option<String>, ConsumedSpan)> =
        HashSet::with_capacity(candidates.len());
    candidates.retain(|c| seen.insert((c.roman.clone(), c.hanji.clone(), c.consumed_span)));
}

fn fetch_walker_slot0(
    raw: &str,
    raw_len: u32,
    freq_map: &FrequencyMap,
    now_ms: i64,
    mode: phonetics::InputMode,
    custom: &[CustomEntry],
) -> Option<RawCandidate> {
    LexiconHandle::with_state(|state| {
        let Some(inv) = state.syllable_inventory.as_ref() else {
            return Ok(None);
        };
        let Some(prefix) = state.prefix_index.as_ref() else {
            return Ok(None);
        };
        let Some(dict) = state.dictionary.as_ref() else {
            return Ok(None);
        };
        // Single POJ-mode source for this fetch: the SAME `is_poj` must
        // reach both the walker edge provider (`build_shadow_lattice`)
        // and the custom-dict keying (`custom_toneless_key`) so the S6
        // byte-identity invariant (custom key == lattice edge key) holds
        // — a split-brain (POJ-aware edges, mode-blind custom keys) would
        // silently drop custom matches in POJ mode.
        // 中文: 本次 fetch 單一 is_poj 來源 — walker edge 與 custom key 必須同值,維持 S6 byte-identical。
        let is_poj = mode == phonetics::InputMode::Poj;
        let (shadow, shadow_to_raw_end, lattice) = build_shadow_lattice(raw, inv, is_poj);
        // v3.5.8 S6 (Codex pre-impl S6 Q2/Q6, 2026-05-17) — per-fetch
        // map from a custom entry's normalized toneless key to the
        // entry. `custom_toneless_key` reuses the SAME shadow pipeline
        // the edge keys use, so a hit here is byte-identical to a
        // lattice edge's `tl:{toneless}` (Q2 BLOCK: a plain
        // tone-digit-strip would not fold a POJ/diacritic custom roman
        // like `tâi-uân`). `or_insert` = **first-wins** on a duplicate
        // key (Codex Q6: explicit, not `HashMap` overwrite/iteration).
        // 中文: S6 — per-fetch「custom toneless key → entry」表;custom_toneless_key 重用同一 shadow pipeline
        // 中文:   → 命中與 lattice edge key byte-identical(Q2 BLOCK:POJ/diacritic 須先 canonicalize);
        // 中文:   同 key 重複 = or_insert first-wins(Codex Q6,非 HashMap 覆寫)。
        let mut custom_map: std::collections::HashMap<String, &CustomEntry> =
            std::collections::HashMap::with_capacity(custom.len());
        for entry in custom {
            if let Some(k) = custom_toneless_key(&entry.roman, is_poj) {
                custom_map.entry(k).or_insert(entry);
            }
        }
        // Codex post-impl S2 P1: suppress the synth when a trailing
        // hyphen leaves the shadow short of the raw buffer (a
        // `(0, raw_len)` synth would mis-commit the pending `-`).
        // Cheap early-out before walking.
        let Some(consumed_span) = synth_consumed_span(&shadow_to_raw_end, shadow.len(), raw_len)
        else {
            return Ok(None);
        };

        let path = crate::lattice::walk_best(&lattice, shadow.len(), |start, end| {
            // Edges come from the syllabifier-built lattice so they are
            // well-formed by construction; the guard is defensive
            // (mirrors the `build_keys_tl_with_inventory` projection
            // guard) and also drops a digit-only / empty toneless span.
            if start >= end
                || end > shadow.len()
                || !shadow.is_char_boundary(start)
                || !shadow.is_char_boundary(end)
            {
                return None;
            }
            let toneless = strip_ascii_tone_digits(&shadow[start..end]);
            if toneless.is_empty() {
                return None;
            }
            let raw_span = (
                shadow_to_raw_end[start] as u32,
                shadow_to_raw_end[end] as u32,
            );
            let key = format!("tl:{toneless}");
            // v3.5.8 S6 (Codex pre-impl S6 Q3, 2026-05-17) — a
            // `custom_dictionary.db` entry whose normalized toneless
            // roman equals this edge's key OVERRIDES the `dict.bin`
            // best candidate for the edge (checked BEFORE
            // `best_candidate_for_key`). Same source-rank-0 precedence
            // custom has in the span-local `(roman,hanji,consumed_span)`
            // dedupe — an unconditional edge-content override, NOT a
            // cost competition (segmentation safety comes from the
            // `CUSTOM_EFFECTIVE_FREQ` proxy + existing single-syllable
            // user-delta damping, not from out-scoring dict here).
            // 中文: S6 — custom 命中該 edge key → 覆寫 dict.bin 最佳候選(在 best_candidate_for_key 之前查);
            // 中文:   = span-local source-rank-0 同語意,無條件 override 非 cost 競爭
            // 中文:   (切分安全靠 CUSTOM_EFFECTIVE_FREQ proxy + 既有單音節阻尼,不靠在此贏分)。
            if let Some(entry) = custom_map.get(key.as_str()) {
                // `display_text` = the exact key the platform writes to
                // `user_frequency.db` on commit, mirroring
                // `lexicon::custom_entry_to_candidate` (hanji else
                // roman) so a user-selected custom entry's decayed
                // weight folds into the path objective identically to a
                // dict edge (S3 Q4d seam). Single-syllable custom is
                // damped by `WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE`
                // in `edge_cost`, same as dict.
                let display_text = entry.hanji.clone().unwrap_or_else(|| entry.roman.clone());
                let fd = freq_map.get(&display_text).copied().unwrap_or_default();
                let count = u32::try_from(fd.count).unwrap_or(0);
                let user_weight_delta = decayed_user_weight_delta(count, now_ms, fd.last_used_ms);
                // Syllable count = greedy-longest segment count of the
                // edge's shadow span (the edge came from the
                // syllabifier-built lattice so it segments cleanly;
                // clamp ≥ 1 defends a leaked empty). `CustomEntry`
                // carries no syllable model — this mirrors what the
                // dict path reads off `DictionaryRecord.syllable_count`
                // for the same span and feeds the khiin `n_syls` bias.
                let syllable_count = greedy_longest_syllabification(&shadow[start..end], inv)
                    .map(|segs| segs.len())
                    .unwrap_or(1)
                    .clamp(1, u8::MAX as usize) as u8;
                return Some(crate::lattice::EdgeChoice {
                    roman: entry.roman.clone(),
                    hanji: entry.hanji.clone(),
                    // S6 Q7 (Codex BLOCK guard): a custom entry IS a
                    // lexicon-backed hit (not OOV roman synthesis) —
                    // keeps the all-OOV carve-out from firing on a
                    // custom-only path. NEVER infer no-dict from
                    // hanji/freq/is_custom.
                    dict_hit: true,
                    is_custom: true,
                    // S6 Q1 (Codex BLOCK): effective-frequency proxy —
                    // custom still pays the same ln(CORPUS)
                    // normalization toll, NOT a cost floor/discount.
                    frequency: crate::lattice::CUSTOM_EFFECTIVE_FREQ,
                    syllable_count,
                    toneless_len: toneless.chars().count(),
                    user_weight_delta,
                });
            }
            match best_candidate_for_key(&key, raw_span, freq_map, now_ms, prefix, dict) {
                Some(c) => {
                    // v3.5.8 S3 (Codex pre-impl Q4d seam, 2026-05-16):
                    // fold this edge's time-decayed user-frequency
                    // weight into the walker path objective (closes
                    // Continuous-input Gap B → G2). `c.display_text`
                    // is the exact key the platform writes to
                    // `user_frequency.db` on commit (set by
                    // `lexicon::record_to_candidate`), so the same
                    // snapshot the span-local path consults applies
                    // here. Looked up BEFORE the field moves below.
                    // `best_candidate_for_key` / `record_to_candidate`
                    // are deliberately UNTOUCHED — their internal
                    // `user_freq_boost` answers "which record wins
                    // inside this edge" (homophone disambiguation);
                    // this answers "which segmentation path wins".
                    // Same user signal, two orthogonal decision
                    // levels, no double counting.
                    // 中文: S3 — 把本 edge 的時間衰減 user-freq 權重接進 walker 路徑目標
                    // 中文:   (收斂 Gap B → G2)。display_text = 平台 commit 寫 user_frequency.db
                    // 中文:   的同一 key;best_candidate_for_key/record_to_candidate 不動
                    // 中文:   (其 boost 管 edge 內選 record,此管選哪條切分路徑,正交不重複計)。
                    let fd = freq_map.get(&c.display_text).copied().unwrap_or_default();
                    // `FrequencyData.count` is i32 (legacy cap domain);
                    // re-widen to u32 the same way
                    // `record_to_candidate` does (negative → 0).
                    let count = u32::try_from(fd.count).unwrap_or(0);
                    let user_weight_delta =
                        decayed_user_weight_delta(count, now_ms, fd.last_used_ms);
                    Some(crate::lattice::EdgeChoice {
                        roman: c.roman,
                        hanji: c.hanji,
                        // S5 (Codex pre-impl Q2 BLOCK): explicit
                        // dict-hit flag — the `Some(c)` branch IS a
                        // dictionary record. The no-dict carve-out must
                        // key off this, never `hanji.is_none()` /
                        // `frequency == 0`.
                        dict_hit: true,
                        // S6: a `dict.bin` record is not custom.
                        is_custom: false,
                        frequency: c.frequency,
                        syllable_count: c.syllable_count,
                        // khiin `word_len` for the S5 length
                        // normalization (Codex pre-impl Q1 BLOCK).
                        toneless_len: toneless.chars().count(),
                        user_weight_delta,
                    })
                }
                // No dict hit: the edge's own toneless roman, SAME code
                // path as a dict edge (`feedback_no_redundant_fallback`)
                // but flagged `dict_hit: false`. A synthesized roman
                // edge has no `user_frequency.db` history key so its
                // walker user weight stays neutral
                // (`user_weight_delta = 0.0`). The user-facing
                // per-syllable romanization for a wholly no-dict buffer
                // is produced by the explicit no-dict carve-out below
                // (keyed off `dict_hit`), NOT by an edge-cost tie lever.
                //
                // v3.5.8 RC0: what stops a multi-syllable OOV blob
                // from undercutting a dict-covering path is now
                // `edge_cost`'s char-keyed per-char `BIG` penalty
                // (`OOV_PER_CHAR_PENALTY * toneless_len`), NOT the
                // syllable count. `syllable_count` here is metadata
                // only — it flows into the synthesized slot-0
                // candidate's syllable sum, so it is still kept honest
                // (not a hardcoded `1`) via a **guaranteed-reachable**
                // min-syllable-hop walk over the lattice's own
                // single-syllable step primitive
                // ([`span_min_syllable_count`]) — NOT
                // `greedy_longest_syllabification(...).unwrap_or(1)`,
                // which can dead-end on a span still lattice-
                // syllabifiable via a non-greedy split and then
                // under-count the synth syllable sum (Codex pre-impl
                // RC0 Q3; PR #290 P1 r3255035136). `None` is
                // unreachable for a real lattice edge (Codex pre-impl
                // Q1/Q2 OK); if the edge/provider invariant is ever
                // broken, fail-closed by dropping the edge — the buffer
                // is still spanned via finer edges.
                None => {
                    let toneless_len = toneless.chars().count();
                    let syllable_count = span_min_syllable_count(&shadow[start..end], inv)?
                        .clamp(1, u8::MAX as usize) as u8;
                    Some(crate::lattice::EdgeChoice {
                        roman: toneless,
                        hanji: None,
                        dict_hit: false,
                        // S6: a synthesized OOV roman edge is not custom.
                        is_custom: false,
                        frequency: 0,
                        syllable_count,
                        toneless_len,
                        user_weight_delta: 0.0,
                    })
                }
            }
        });

        let Some(path) = path else {
            return Ok(None);
        };
        if path.choices.is_empty() {
            return Ok(None);
        }

        // v3.5.8 S5 (Codex pre-impl Q2, 2026-05-17) — no-dict carve-out.
        // "No dictionary hit anywhere" is detected via the explicit
        // `dict_hit` flag, NOT `hanji.is_none()` / `frequency == 0` (a
        // dict record may be roman-only; zero frequency is
        // representable — Codex BLOCK). Under the S5 min-cost objective
        // an all-OOV buffer collapses to the fewest-edge blob
        // (`taiuantai`), wrong for a romanization the user wants to
        // read. Re-derive the user-facing roman as the greedy
        // longest-syllable syllabification (`tai uan tai`) — an
        // explicit rule OUTSIDE `edge_cost`, not an OOV cost tuned to
        // fight the corpus normalization. Greedy-longest (the canonical
        // romanization reading) is used rather than literal
        // max-syllable-count: the latter would over-split into
        // sub-syllables (`ta i u an …`), reproducing the very
        // over-segmentation S5 removes, and contradicts the documented
        // `taiuantai → tai uan tai` expectation.
        //
        // v3.5.8 #288 (per-segment case-from-raw) composes with the
        // dict-hit branch: each chosen edge's canonical roman is recased
        // to mirror the user's raw input for that edge's own byte span
        // (per-edge, NOT whole buffer: `hitTUI` keeps segment 2 upper).
        // `path.edges` are shadow offsets; `shadow_to_raw_end` maps them
        // back to the raw buffer, then join with the §10.2 slot-0 word
        // space. The all-OOV carve-out stays unrecased — it is a
        // synthesized reading, not an edge the user typed a case intent
        // for; greedy-longest segments are independent of `path.edges`.
        // 中文: S5 no-dict carve-out — 用 dict_hit 旗標判定(非 hanji/freq,Codex BLOCK)。
        // 中文:   全 OOV 在 min-cost 會塌成最少段 blob → 改 greedy-longest 音節切分羅馬字
        // 中文:   (canonical 讀法,對齊文件 taiuantai→tai uan tai;literal max-segment 會重現過度切分)。
        // 中文: #288 逐 edge case-from-raw 與 dict-hit 分支組合;OOV carve-out 為合成讀法,不還原大小寫。
        let any_dict = path.choices.iter().any(|c| c.dict_hit);
        let (roman, syllable_count) = if any_dict {
            let r = path
                .edges
                .iter()
                .zip(path.choices.iter())
                .map(
                    |(&(s, e), c)| match raw.get(shadow_to_raw_end[s]..shadow_to_raw_end[e]) {
                        Some(seg) => recase_roman(&c.roman, seg, mode),
                        None => c.roman.clone(),
                    },
                )
                .collect::<Vec<_>>()
                .join(" ");
            let s = path
                .choices
                .iter()
                .map(|c| u32::from(c.syllable_count))
                .sum::<u32>()
                .min(u32::from(u8::MAX)) as u8;
            (r, s)
        } else {
            match greedy_longest_syllabification(&shadow, inv) {
                Some(segs) if !segs.is_empty() => {
                    let r = segs
                        .iter()
                        .map(|&(s, e)| strip_ascii_tone_digits(&shadow[s..e]))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let s = segs.len().min(u8::MAX as usize) as u8;
                    (r, s)
                }
                // Cannot cleanly syllabify the buffer → leave the
                // span-local list untouched (pre-S2 behavior, mirrors
                // the synth_consumed_span trailing-hyphen suppression).
                _ => return Ok(None),
            }
        };
        let all_hanji = path.choices.iter().all(|c| c.hanji.is_some());
        let hanji: Option<String> = if all_hanji {
            Some(
                path.choices
                    .iter()
                    .filter_map(|c| c.hanji.as_deref())
                    .collect::<String>(),
            )
        } else {
            None
        };
        let display_text = hanji.clone().unwrap_or_else(|| roman.clone());
        let last_used_ms = freq_map
            .get(&display_text)
            .map(|d| d.last_used_ms)
            .unwrap_or(0);
        // Classify via the lexicon single-source-of-truth so the
        // synth's `CandidateMessage.mode` matches span-local / custom
        // candidates exactly — including MIXED when the concatenated
        // hanji contains a Latin letter (e.g. a path through `…hip相`).
        // Codex PR #285 P2: the earlier `hanji.is_some()` binary
        // mis-emitted HANT for mixed-script full-buffer paths, breaking
        // platform dual-line render parity with regular candidates.
        // 中文: 用 lexicon 單一真相 derive_mode 分類,synth mode 與 span-local/custom 一致
        // 中文:   (hanji 內含 Latin → MIXED);Codex PR #285 P2 修正 binary 漏 MIXED。
        let mode = derive_mode(hanji.as_deref());
        Ok(Some(RawCandidate {
            consumed_span,
            syllable_count,
            display_text,
            roman,
            hanji,
            // S5: `path.cost` is a min-cost (lower = better) total;
            // `RawCandidate.score` is higher-better elsewhere. Slot 0
            // is an explicit prepend so this is informational only
            // (same rationale as `frequency = 0` below) — negate to
            // keep the higher-better monotonic ordering if anything
            // ever does read it.
            score: -(path.cost as f32),
            form: FORM_NOTONE,
            frequency: 0,
            bitmask: 0,
            mode,
            recency_rank: recency_rank(now_ms, last_used_ms),
            coverage_kind: COVERAGE_KIND_FULL,
            // v3.5.8 S6 (Codex pre-impl S6 Q4): provenance truth — a
            // synthesized full-buffer path containing ≥1 custom edge is
            // custom-influenced. Informational at slot 0 (explicit
            // prepend, not sorted; the dispatch.rs dedupe keys on
            // `(roman,hanji,consumed_span)` not `is_custom`), but a
            // truthful flag keeps future ranking/dedupe changes sound.
            is_custom: path.choices.iter().any(|c| c.is_custom),
        }))
    })
    .unwrap_or_default()
}

/// Build TPS FST keys from `Phase::Continuous { raw }`. Each Bopomofo
/// span returned by `tps::valid_span_endings` is converted to numeric-
/// tone TL via `phonetics::tps_to_tl` (e.g., `ㄉㄧㄠˊ` → `tiau5`); the
/// trailing `1..=9` tone digit is stripped and each fragment is
/// concatenated into a fused toneless TL key (Phase 1b guarantee:
/// multi-syllable entries store the fused form, e.g. `珠仔 → tl:tsua`).
/// At each TPS ending the cumulative key from byte 0 is emitted so the
/// FST surface mirrors `build_keys_tl`'s `lower[0..end]` slice — both
/// produce identical key sequences against the same logical input.
/// `consumed_span` stays in the caller's TPS byte space so platform UI
/// can slice the correct number of Bopomofo characters on commit;
/// only the FST-side key uses the canonical TL form.
///
/// Tone-1 (no mark) syllables ARE detected since v3.5.8 Phase 9 Item 7
/// (`tps::valid_span_endings` next-initial-seen rule); a tone-1 span
/// converts to a digitless toneless TL form (`ㄉㄞ` → `tai`) which is
/// accepted directly as the fused key fragment.
///
/// Remaining bounds:
/// - Malformed fragments where `tps_to_tl` yields a non-ASCII result
///   (partial / unconvertible Bopomofo emitted as a raw `Part::Other`
///   symbol) abort the whole build — a partial prefix would corrupt
///   the cumulative fused key for downstream endings.
/// - Endings beyond `MAX_SYLLABLES` (= 8) are dropped to mirror the
///   `build_keys_tl` BFS depth bound, keeping per-keystroke FST lookup
///   and candidate scoring complexity bounded across modes. The TPS
///   syllabifier itself still scans the full input
///   (`tps::valid_span_endings` is unparameterised today); pushing the
///   cap into the syllabifier is a follow-up if profiling shows the
///   linear scan is hot.
// 中文: TPS key 構造 — 累加每個 Bopomofo 音節的 toneless TL,於每個 TPS ending 釋出 fused key (對應 build_keys_tl 的 lower[0..end])。
// 中文: consumed_span 仍以 Bopomofo bytes 為單位;第 1 聲自 Item 7 起支援 (轉出無數字 toneless 形,直接當 key 片段);不合法 Bopomofo 仍中止整批。
// 中文: 與 build_keys_tl 對齊,輸出最多取前 MAX_SYLLABLES (8) 個 ending,避免長 preedit 引發無上限 FST 查詢。
fn build_keys_tps(raw: &str) -> Vec<(ConsumedSpan, String)> {
    let endings = tps_syll::valid_span_endings(raw, 0);
    if endings.is_empty() {
        return Vec::new();
    }
    let cap = endings.len().min(MAX_SYLLABLES);
    let mut out = Vec::with_capacity(cap);
    let mut prev_end = 0usize;
    let mut fused_toneless = String::new();
    for end in endings.into_iter().take(MAX_SYLLABLES) {
        if end <= prev_end || end > raw.len() || !raw.is_char_boundary(end) {
            continue;
        }
        let span_text = &raw[prev_end..end];
        prev_end = end;
        let tl_numeric = tps_to_tl(span_text);
        let toneless = strip_trailing_tone_digit(&tl_numeric).to_ascii_lowercase();
        // Partial / unconvertible Bopomofo makes `tps_to_tl` emit the
        // raw symbol as a non-ASCII `Part::Other` char; failing this
        // check aborts the whole build so the cumulative fused key
        // cannot be corrupted for later endings. Tone-marked and
        // digitless tone-1 (`ㄉㄞ` → `tai`, Item 7) spans both pass.
        let well_formed = !toneless.is_empty() && toneless.bytes().all(|b| b.is_ascii_lowercase());
        if !well_formed {
            return Vec::new();
        }
        fused_toneless.push_str(&toneless);
        out.push(((0u32, end as u32), format!("tl:{fused_toneless}")));
    }
    out
}

/// Drop a trailing ASCII tone digit (`1..=9`) per
/// `engine/composing/src/syllabifier/tl.rs:94-101`: digit `0` is not a
/// tone marker so it is preserved.
// 中文: 剝掉尾端 1..=9 的 tone digit ('0' 不是 tone marker)。
fn strip_trailing_tone_digit(s: &str) -> &str {
    match s.as_bytes().last() {
        Some(b) if b.is_ascii_digit() && *b != b'0' => &s[..s.len() - 1],
        _ => s,
    }
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
// 中文: Item 12 — proto CustomDictEntry[] → domain CustomEntry;roman/hanji 是 custom_dictionary.db 原始欄位,
// 中文:   不是 legacy 顯示大寫化形式,確保 (roman,hanji) 去重鍵能與 dict.bin 正確碰撞。
fn build_custom_entries(entries: &[CustomDictEntry]) -> Vec<CustomEntry> {
    entries
        .iter()
        .map(|e| CustomEntry {
            roman: e.roman.clone(),
            hanji: e.hanji.clone(),
        })
        .collect()
}

/// Acquire lexicon state and run the span-local fetch. Empty result on
/// any state-availability failure (mirrors `build_keys_tl` policy).
///
/// `raw_len` is the byte length of the original pending buffer
/// (`Phase::Continuous { raw }.len()`). It is the predicate input
/// for the Phase 9.1 Tier 1 rule (`consumed_span_end == raw_len`)
/// inside `fetch_candidates_for_keys`. Same byte space as the
/// caller-built keys' `consumed_span` (TL ASCII or TPS Bopomofo
/// bytes, depending on input mode).
///
/// **Filter default** (deferred per `feedback_no_future_planning.md`):
/// `enabled_sources_bitmask = u32::MAX` (all sources on). PR-9.6 will
/// plumb the platform dictionary toggles through `FetchAtPos`.
///
/// **User-frequency plumb** (Phase 9.3a): `freq_map` + `now_ms` are
/// caller-built from `FetchAtPos.frequency_entries` and
/// `FetchAtPos.now_ms` (see `handle_fetch_at_pos`). Empty map +
/// `now_ms = 0` reproduces the cold-start neutral-boost behavior
/// (`user_freq_boost(0) = 1.0`, `recency_rank = 1` everywhere) so
/// PR-9.2 platform builds keep working until PR-9.3b/c plumb the
/// platform SQLite query.
// 中文: 取出 lexicon 內的 prefix_index + dictionary,呼 fetch_candidates_for_keys;狀態不可用時回傳空。
// 中文: raw_len = pending buffer 長度,用於 Phase 9.1 Tier 1 判定 (consumed_span_end == raw_len)。
// 中文: Phase 9.3a — freq_map + now_ms 由呼叫端從 FrequencyEntry[] 建好;空 map = 中性 boost。
fn fetch_via_lexicon(
    keys: &[(ConsumedSpan, String)],
    raw_len: u32,
    freq_map: &FrequencyMap,
    now_ms: i64,
    custom: &[CustomEntry],
) -> Vec<RawCandidate> {
    LexiconHandle::with_state(|state| {
        let Some(prefix) = state.prefix_index.as_ref() else {
            return Ok(Vec::new());
        };
        let Some(dict) = state.dictionary.as_ref() else {
            return Ok(Vec::new());
        };
        Ok(fetch_candidates_for_keys(
            keys,
            raw_len,
            u32::MAX,
            freq_map,
            now_ms,
            custom,
            prefix,
            dict,
        ))
    })
    .unwrap_or_default()
}

/// Acquire lexicon state and run the partial-prefix fetch. Empty
/// result on any state-availability failure (mirrors
/// [`fetch_via_lexicon`] policy). Returns `Vec::new()` when
/// [`build_partial_prefix_key_tl`] declines (empty toneless key).
///
/// `enabled_sources_bitmask = u32::MAX` matches the full-syllable
/// path's behaviour; PR-9.6 will plumb the platform dictionary
/// toggles uniformly to both paths.
// 中文: Item 10 — partial-prefix 入口的 lexicon glue;同 fetch_via_lexicon 的 state-availability 降級政策。
// 中文: Item 12 — custom 命中也併進 partial-prefix 路徑(標 COVERAGE_KIND_PARTIAL_PREFIX),legacy custom dict prefix-visible 行為對齊。
fn fetch_via_lexicon_partial(
    raw: &str,
    freq_map: &FrequencyMap,
    now_ms: i64,
    custom: &[CustomEntry],
    is_poj: bool,
) -> Vec<RawCandidate> {
    let Some(key) = build_partial_prefix_key_tl(raw, is_poj) else {
        return Vec::new();
    };
    let raw_len = raw.len() as u32;
    LexiconHandle::with_state(|state| {
        let Some(prefix) = state.prefix_index.as_ref() else {
            return Ok(Vec::new());
        };
        let Some(dict) = state.dictionary.as_ref() else {
            return Ok(Vec::new());
        };
        Ok(fetch_partial_prefix_candidates(
            &key,
            raw_len,
            u32::MAX,
            freq_map,
            now_ms,
            custom,
            prefix,
            dict,
        ))
    })
    .unwrap_or_default()
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
        // `handle_fetch_at_pos` POJ pass). `hanji` is a proto3
        // `optional string` so prost serializes `None` as wire-absent
        // (distinguishes TAILO from defective empty-string emission).
        // See `docs/engine/continuous-candidate-display.md` §4.2.
        // 中文: Item 5 — roman 永有值,為當前 input mode 的顯示羅馬字(TL,或經 POJ pass 後的 POJ);
        // 中文:   hanji 為 proto optional,TAILO 候選送 None,wire 上是「absent」而非空字串。
        roman: c.roman,
        hanji: c.hanji,
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
    //! Unit tests for the pure helpers used by `handle_fetch_at_pos`.
    //! Dispatch-level integration (decode round-trip, degraded paths) lives
    //! in `engine/composing/tests/dispatch_continuous.rs` because it needs
    //! the `Engine` + lexicon singletons.

    use super::*;

    #[test]
    fn strip_trailing_tone_digit_drops_one_to_nine() {
        assert_eq!(strip_trailing_tone_digit("tiau5"), "tiau");
        assert_eq!(strip_trailing_tone_digit("tai1"), "tai");
        assert_eq!(strip_trailing_tone_digit("bak4"), "bak");
        assert_eq!(strip_trailing_tone_digit("khih8"), "khih");
        assert_eq!(strip_trailing_tone_digit("khoo9"), "khoo");
    }

    #[test]
    fn strip_trailing_tone_digit_preserves_zero_and_letters() {
        // '0' is not a tone marker per phonetics::syllable.rs:18-20.
        assert_eq!(strip_trailing_tone_digit("tai0"), "tai0");
        assert_eq!(strip_trailing_tone_digit("tai"), "tai");
        assert_eq!(strip_trailing_tone_digit(""), "");
    }

    #[test]
    fn build_keys_tps_emits_cumulative_fused_keys() {
        // ㄉㄧㄠˊㄨㄢˊ → tone-5 + tone-5 → tiau + uan (after digit strip),
        // fused into a multi-syllable toneless TL key per Phase 1b.
        let raw = "ㄉㄧㄠˊㄨㄢˊ";
        let keys = build_keys_tps(raw);
        let texts: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
        assert_eq!(texts, vec!["tl:tiau", "tl:tiauuan"], "{texts:?}");

        // consumed_span end values must be at TPS Bopomofo byte offsets.
        // ㄉ/ㄧ/ㄠ/ㄨ/ㄢ each 3 bytes (Bopomofo block U+3100-U+312F);
        // ˊ is 2 bytes (U+02CA, modifier-letter range).
        // First syllable: 3+3+3+2 = 11.
        assert_eq!(keys[0].0, (0u32, 11u32));
        // Whole input: 11 + 3+3+2 = 19.
        assert_eq!(keys[1].0, (0u32, 19u32));
    }

    #[test]
    fn build_keys_tps_tone1_no_mark_emits_key() {
        // Item 7: Bopomofo with no tone mark is a tone-1 syllable. The
        // syllabifier's next-initial-seen rule ends it at EOI; the
        // digitless toneless TL form (`ㄉㄧㄠ` → `tiau`) is accepted
        // directly as the fused key fragment. ㄉ/ㄧ/ㄠ = 3 bytes each.
        let keys = build_keys_tps("ㄉㄧㄠ");
        let texts: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
        assert_eq!(texts, vec!["tl:tiau"], "{texts:?}");
        assert_eq!(keys[0].0, (0u32, 9u32));
    }

    #[test]
    fn build_keys_tps_tone1_chain_emits_cumulative_fused_keys() {
        // ㄉㄞㆣㄧ — "tâi-gí" (台語) typed with no tone marks. The
        // syllabifier splits ㄉㄞ | ㆣㄧ via next-initial-seen; each
        // span converts to a clean toneless TL fragment and the
        // cumulative fused keys mirror the TL path (`tl:tai`, then
        // `tl:taigi`). Each Bopomofo char = 3 bytes.
        let keys = build_keys_tps("\u{3109}\u{311e}\u{31a3}\u{3127}");
        let texts: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
        assert_eq!(texts, vec!["tl:tai", "tl:taigi"], "{texts:?}");
        assert_eq!(keys[0].0, (0u32, 6u32));
        assert_eq!(keys[1].0, (0u32, 12u32));
    }

    #[test]
    fn build_keys_tps_mixed_tone1_and_tone_marked() {
        // ㄉㄞㆣㄧˊ — tone-1 ㄉㄞ then tone-5 ㆣㄧˊ. Mixed boundary
        // kinds still produce cumulative fused keys; ˊ = U+02CA (2
        // bytes), so the second span ends at 6 + 3+3+2 = 14.
        let keys = build_keys_tps("\u{3109}\u{311e}\u{31a3}\u{3127}\u{02ca}");
        let texts: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
        assert_eq!(texts, vec!["tl:tai", "tl:taigi"], "{texts:?}");
        assert_eq!(keys[0].0, (0u32, 6u32));
        assert_eq!(keys[1].0, (0u32, 14u32));
    }

    #[test]
    fn build_keys_tps_caps_at_max_syllables() {
        // `MAX_SYLLABLES + 1` well-formed `ㄉㄧㄠˊ` (= tiau5) syllables →
        // syllabifier emits one more ending than the budget, but
        // `build_keys_tps` must cap at MAX_SYLLABLES to mirror
        // `build_keys_tl`'s BFS depth bound and keep per-keystroke FST
        // lookup complexity bounded.
        // Each `ㄉㄧㄠˊ` syllable = ㄉ(3) + ㄧ(3) + ㄠ(3) + ˊ(2) = 11 bytes.
        const SYLLABLE_BYTES: usize = 11;
        let raw = "ㄉㄧㄠˊ".repeat(MAX_SYLLABLES + 1);
        let keys = build_keys_tps(&raw);
        assert_eq!(keys.len(), MAX_SYLLABLES, "{keys:?}");
        // Last consumed-span end must sit on the cap-th syllable boundary,
        // not the full input — catches accidental overconsumption that
        // would mis-align platform commit slicing.
        assert_eq!(
            keys[MAX_SYLLABLES - 1].0,
            (0u32, (SYLLABLE_BYTES * MAX_SYLLABLES) as u32)
        );
        // Last fused key concatenates exactly MAX_SYLLABLES toneless `tiau` fragments.
        assert_eq!(
            keys[MAX_SYLLABLES - 1].1,
            format!("tl:{}", "tiau".repeat(MAX_SYLLABLES))
        );
    }

    // ----- v3.5.8 S2 — synth_consumed_span trailing-hyphen guard -----
    // (Codex post-impl S2 P1 regression). Production runs
    // `canonicalize_poj_shadow` then `build_hyphen_shadow`; for the
    // pure-ASCII inputs here canonicalize is identity (see the
    // matching `canonicalize_poj_shadow_pure_ascii_is_identity_fast_path`
    // case in `composing::shadow`'s test module), so
    // `crate::shadow::build_hyphen_shadow(raw)` yields the same
    // `shadow_to_raw_end` the walker path computes.

    #[test]
    fn synth_consumed_span_trailing_hyphen_suppresses_synth() {
        use crate::shadow::build_hyphen_shadow;
        // `tai-` / `tai-bak-`: the trailing `-` stays in the pending
        // raw buffer (S1 contract) so the shadow is short of raw_len —
        // the full-buffer synth MUST be suppressed (returns None)
        // rather than mis-commit / drop the pending `-`.
        for raw in ["tai-", "tai-bak-"] {
            let (shadow, map) = build_hyphen_shadow(raw);
            assert!(
                synth_consumed_span(&map, shadow.len(), raw.len() as u32).is_none(),
                "trailing-hyphen {raw:?} must suppress slot-0 synth"
            );
        }
    }

    #[test]
    fn synth_consumed_span_full_coverage_emits_zero_to_raw_len() {
        use crate::shadow::build_hyphen_shadow;
        // No trailing hyphen → shadow maps to the whole raw buffer.
        // Leading (`-tai`) and internal (`tai-bak`) hyphens fold INTO
        // the consumed prefix, so those still get a `(0, raw_len)`
        // full-buffer synth.
        for raw in ["taibak", "tai-bak", "-tai"] {
            let (shadow, map) = build_hyphen_shadow(raw);
            assert_eq!(
                synth_consumed_span(&map, shadow.len(), raw.len() as u32),
                Some((0, raw.len() as u32)),
                "{raw:?} fully covered → (0, raw_len) synth"
            );
        }
    }

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
            score: 1.5,
            form: 1,
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
    }

    #[test]
    fn raw_to_proto_candidate_emits_none_hanji_for_tailo() {
        let raw = RawCandidate {
            consumed_span: (0, 3),
            syllable_count: 1,
            display_text: "tāi".to_owned(),
            roman: "tāi".to_owned(),
            hanji: None,
            score: 0.5,
            form: 1,
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

    // ----- v3.5.8 2A — per-segment case-from-raw -----

    #[test]
    fn raw_segment_letter_case_maps_typed_intent() {
        use phonetics::case_transform::LetterCase;
        // Titlecase raw (the `Hittui → Hit` segment): first alpha upper,
        // rest not all upper → Uppercased (capitalize-first).
        assert_eq!(raw_segment_letter_case("Hit"), LetterCase::Uppercased);
        // Lowercase raw (the `tui` segment) → Lowercased.
        assert_eq!(raw_segment_letter_case("tui"), LetterCase::Lowercased);
        // All-caps → CapsLocked (full upper).
        assert_eq!(raw_segment_letter_case("HIT"), LetterCase::CapsLocked);
        // Tone digits / hyphens are not alphabetic → ignored.
        assert_eq!(raw_segment_letter_case("Tai5"), LetterCase::Uppercased);
        assert_eq!(raw_segment_letter_case("tai-uan"), LetterCase::Lowercased);
        // No alphabetic char → defaults to Lowercased (never panics).
        assert_eq!(raw_segment_letter_case("123"), LetterCase::Lowercased);
        assert_eq!(raw_segment_letter_case(""), LetterCase::Lowercased);
    }

    // ----- v3.5.8 — POJ-display render of the continuous candidate roman -----

    #[test]
    fn render_roman_for_mode_poj_rewrites_oo_and_nn() {
        let poj = phonetics::InputMode::Poj;
        // The reported bug: in POJ mode the continuous best candidate
        // must show `oo`→`o͘` (o + U+0358) and `nn`→`ⁿ` (U+207F),
        // not raw TL.
        assert_eq!(render_roman_for_mode("oo", poj), "o\u{0358}");
        assert_eq!(render_roman_for_mode("goo", poj), "go\u{0358}");
        assert_eq!(render_roman_for_mode("sann", poj), "sa\u{207f}");
        // Whole-sentence walker path: space-joined multi-syllable roman
        // (`tl_display_to_poj_display` splits on ` `/`-` per token).
        assert_eq!(
            render_roman_for_mode("goo sann", poj),
            "go\u{0358} sa\u{207f}"
        );
        assert_eq!(render_roman_for_mode("tai-oo", poj), "tai-o\u{0358}");
        // Tone-marked TL → POJ: the tone sits between `o` and the
        // U+0358 dot (`kòo` 顧 → `kò͘`). Exact, not just "contains".
        assert_eq!(render_roman_for_mode("kòo", poj), "k\u{f2}\u{0358}");
    }

    #[test]
    fn render_roman_for_mode_poj_preserves_letter_case() {
        let poj = phonetics::InputMode::Poj;
        // Lowercase (normal typing) stays lowercase.
        assert_eq!(render_roman_for_mode("goo", poj), "go\u{0358}");
        // Sentence-start capital (the `Hittui → Hit` segment class).
        assert_eq!(render_roman_for_mode("Goo", poj), "Go\u{0358}");
        // CapsLock — the Codex pre-impl BLOCK: `tl_display_to_poj_display`
        // only title-cases, so without the `transform_input_case`
        // restore an all-caps candidate would collapse to `Go͘`. Pin
        // the all-caps form survives.
        assert_eq!(render_roman_for_mode("OO", poj), "O\u{0358}");
        // `nn` under CapsLock: the base letters go all-caps while the
        // POJ nasal `ⁿ` (U+207F) is preserved as-is (correct POJ — no
        // uppercase nasal hook). Without the case restore this would
        // collapse to title case `Sa\u{207f}`, the exact BLOCK
        // regression; pin the all-caps form survives.
        assert_eq!(render_roman_for_mode("SANN", poj), "SA\u{207f}");
    }

    #[test]
    fn render_roman_for_mode_non_poj_is_identity() {
        // TL / English keep raw TL (asymmetry is POJ-only, matching the
        // dictionary-search path `inputMode == .poj ? tlToPoj : raw`).
        // (legacy-mapped) TPS resolves to `Tl` upstream, so it is
        // covered by the Tl identity here.
        assert_eq!(render_roman_for_mode("oo", phonetics::InputMode::Tl), "oo");
        assert_eq!(
            render_roman_for_mode("sann", phonetics::InputMode::Tl),
            "sann"
        );
        assert_eq!(
            render_roman_for_mode("Goo", phonetics::InputMode::Tl),
            "Goo"
        );
        assert_eq!(
            render_roman_for_mode("oo", phonetics::InputMode::English),
            "oo"
        );
    }

    #[test]
    fn dedupe_rendered_continuous_drops_post_render_collision_keeping_first() {
        fn mk(roman: &str, hanji: Option<&str>, span: (u32, u32), is_custom: bool) -> RawCandidate {
            RawCandidate {
                consumed_span: span,
                syllable_count: 1,
                display_text: hanji.map(String::from).unwrap_or_else(|| roman.to_string()),
                roman: roman.to_string(),
                hanji: hanji.map(String::from),
                score: 0.0,
                form: FORM_NOTONE,
                frequency: 0,
                bitmask: 0,
                mode: lexicon::CandidateMode::Tailo,
                recency_rank: 1,
                coverage_kind: COVERAGE_KIND_FULL,
                is_custom,
            }
        }
        // Post-render: a custom entry stored `gô͘` and a dict.bin entry
        // stored `gôo`; both already rendered to `gô͘` here, same hanji
        // + span ⇒ a visible duplicate. First-wins keeps index 0 so the
        // prepended whole-sentence best candidate is never dropped.
        let mut collide = vec![
            mk("gô\u{0358}", Some("鵝"), (0, 6), false), // dict, rendered
            mk("gô\u{0358}", Some("鵝"), (0, 6), true),  // custom, rendered
        ];
        dedupe_rendered_continuous(&mut collide);
        assert_eq!(collide.len(), 1);
        assert!(
            !collide[0].is_custom,
            "first-wins must keep the earlier (index-0) row"
        );
        // Distinct on ANY of (roman, hanji, consumed_span) ⇒ untouched
        // (the S2 same-word-different-span invariant survives).
        let mut keep = vec![
            mk("go\u{0358}", Some("鵝"), (0, 6), false),
            mk("go\u{0358}", Some("吳"), (0, 6), false), // different hanji
            mk("go\u{0358}", Some("鵝"), (0, 3), false), // different span
            mk("sa\u{207f}", None, (0, 6), false),       // different roman
        ];
        let before = keep.len();
        dedupe_rendered_continuous(&mut keep);
        assert_eq!(keep.len(), before, "distinct keys must all survive");
    }

    #[test]
    fn recase_roman_mirrors_raw_segment_case_tone_aware() {
        let tl = phonetics::InputMode::Tl;
        // The motivating bug: dict roman is canonical lowercase; the
        // user's raw for each span drives the displayed case so a
        // continuous segment reads `Hit` / `tui`, not `Tui`.
        assert_eq!(recase_roman("hit", "Hit", tl), "Hit");
        assert_eq!(recase_roman("tui", "tui", tl), "tui");
        // Tone-diacritic roman, toneless ASCII raw (length mismatch is a
        // non-issue — raw only derives the case intent).
        assert_eq!(recase_roman("tâi", "Tai", tl), "Tâi");
        assert_eq!(recase_roman("tâi", "tai", tl), "tâi");
    }
}
