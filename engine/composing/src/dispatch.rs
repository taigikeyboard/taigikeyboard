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
use crate::syllabifier::{tl as tl_syll, tps as tps_syll};
use lexicon::{
    fetch_candidates_for_keys, ConsumedSpan, EngineHandle as LexiconHandle, RawCandidate,
};
use phonetics::{contains_tps, tps_to_tl};
use protos::engine::{
    composing_request, AppConfig, CandidateMessage, ComposingRequest, ComposingResponse,
    ContinuousResponse, FrequencyEntry,
};
use ranking::{build_frequency_map, FrequencyMap};

/// Cap on syllabifier BFS depth for Phase 6 fetches. Matches the
/// `max_syllables=8` budget called out in `docs/roadmap.md:231` and
/// keeps the worst-case lookup at O(n × 3 × 8) FST hits.
// 中文: TL syllabifier BFS 深度上限,對應 roadmap §Phase 3 的 8 syllable 估算。
const MAX_SYLLABLES: usize = 8;

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
        },
        Method::CommitContinuous(m) => Intent::CommitContinuous {
            display_text: m.display_text,
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
        } => Ok(handle_fetch_at_pos(
            engine,
            position,
            &frequency_entries,
            now_ms,
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
fn handle_fetch_at_pos(
    engine: &Engine,
    position: u32,
    frequency_entries: &[FrequencyEntry],
    now_ms: i64,
    config: &AppConfig,
) -> ComposingResponse {
    let snapshot = engine.snapshot(config);
    let state = engine.snapshot_state();
    let Phase::Continuous { raw, .. } = &state.phase else {
        return snapshot;
    };
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
    let keys = if contains_tps(raw) {
        build_keys_tps(raw)
    } else {
        build_keys_tl(raw)
    };
    if keys.is_empty() {
        return with_continuous(snapshot, ContinuousResponse::default());
    }
    // Phase 9.3a: hoist proto-shaped `FrequencyEntry[]` into the
    // domain-typed `FrequencyMap` once per fetch; `lexicon` consumes
    // `&FrequencyMap` and stays proto-agnostic. Empty list → empty
    // map → `user_freq_boost(0) = 1.0` for every candidate (backward
    // -compatible with PR-9.2 platform builds that have not wired
    // user-frequency plumbing yet).
    let freq_map = build_frequency_map(frequency_entries);
    let candidates = fetch_via_lexicon(&keys, raw.len() as u32, &freq_map, now_ms);
    with_continuous(
        snapshot,
        ContinuousResponse {
            candidates: candidates.into_iter().map(raw_to_proto_candidate).collect(),
        },
    )
}

/// Build TL/POJ FST keys from `Phase::Continuous { raw }`. Uses
/// `lexicon::EngineHandle::with_state` to acquire the syllable
/// inventory; if the inventory is unavailable (platform did not
/// supply `syllables.fst` at install time, or lexicon was never
/// installed), returns an empty list so `FetchAtPos` degrades to
/// "no candidates" rather than panicking.
///
/// **Input contract** (mirrors Phase 5 `fetch_candidates_for_endings`,
/// `engine/lexicon/src/continuous.rs:24-33`): `raw` MUST be canonical
/// TL ASCII — toneless (`tsua`) or numeric tone (`tsua7`, `tai1bak4`).
/// The toneless FST key is built by lowercasing and dropping every
/// ASCII digit, mirroring the digit half of the upstream
/// `notone.py::remove_tone` regex `[\d\-]`
/// (`dictionary/common/notone.py`); this is what the Phase 1b fused-
/// toneless FST keys contain (`engine/lexicon/tests/fused_toneless_key.rs`).
///
/// **Phase 6 limitations** (deferred to Phase 9 dogfood per
/// `feedback_no_future_planning.md`):
/// - **Hyphen-separated input** (`tai-bak`, `pe̍h-ōe-jī`) NOT supported:
///   `tl_syll::valid_span_endings` walks contiguous syllable bytes via
///   `inv.contains(...)` and the inventory has no hyphenated entries
///   (`engine/composing/src/syllabifier/tl.rs:73-76`), so the
///   syllabifier returns endings only up to the first hyphen. Stripping
///   hyphens here would not help. Phase 9 needs either a hyphen-aware
///   syllabifier or a hyphenless-shadow-with-offset-map dispatch
///   pre-pass.
/// - **POJ-display input** (`pe̍h`, `chóa`, `peⁿ`) NOT supported: the
///   `to_ascii_lowercase` pass does not strip diacritics; per-syllable
///   POJ→TL canonicalization is deferred.
// 中文: TL/POJ key 構造 — 經 lexicon SyllableInventory 切音節後,把 lower(raw[0..end]) 去掉所有 ASCII 數字形成 fused toneless key,加 "tl:" 前綴。
// 中文: 對應 dictionary/common/notone.py 的 [\d] 部分 (digit half of [\d\-] regex);FST 端只存 fused toneless,所以 numeric tone 輸入要先 strip。
// 中文: 帶連字號 (tai-bak) 的輸入 syllabifier 也走不過去,Phase 9 才補;POJ 帶調符同樣留 Phase 9。
fn build_keys_tl(raw: &str) -> Vec<(ConsumedSpan, String)> {
    let endings = match LexiconHandle::with_state(|state| {
        let Some(inv) = state.syllable_inventory.as_ref() else {
            return Ok(Vec::new());
        };
        Ok(tl_syll::valid_span_endings(raw, 0, inv, MAX_SYLLABLES))
    }) {
        Ok(endings) => endings,
        Err(_) => return Vec::new(),
    };
    if endings.is_empty() {
        return Vec::new();
    }
    let lower = raw.to_ascii_lowercase();
    let mut out = Vec::with_capacity(endings.len());
    for end in endings {
        if end == 0 || end > lower.len() || !lower.is_char_boundary(end) {
            continue;
        }
        let toneless = strip_ascii_tone_digits(&lower[..end]);
        if toneless.is_empty() {
            continue;
        }
        out.push(((0u32, end as u32), format!("tl:{toneless}")));
    }
    out
}

/// Drop every ASCII digit from `s`. Equivalent to the digit half of
/// `dictionary/common/notone.py::remove_tone()` regex `[\d\-]` under
/// the canonical-ASCII TL input contract — Python `\d` matches every
/// Unicode decimal digit, but TL canonical input only ever uses
/// ASCII `0..=9`, so `is_ascii_digit()` is sound here. Hyphens are NOT
/// stripped (the syllabifier already cannot walk past them; see
/// `build_keys_tl` Phase-6 limitations note).
// 中文: 對應 notone.py [\d\-] 中的 \d (ASCII contract 等價);hyphen 不剝因為 syllabifier 也走不過去,Phase 9 再一併處理。
fn strip_ascii_tone_digits(s: &str) -> String {
    s.chars().filter(|c| !c.is_ascii_digit()).collect()
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
/// **Phase 6 limitations** (deferred to Phase 9 dogfood per `docs/roadmap.md`):
/// - Tone-1 (no mark) syllables are NOT detected by `tps::valid_span_endings`
///   today (`engine/composing/src/syllabifier/tps.rs:6-8`); FetchAtPos
///   will not surface candidates whose TPS prefix ends on tone-1.
/// - Malformed fragments where `tps_to_tl` yields output not ending in a
///   `1..=9` tone digit (e.g. partial Bopomofo) are skipped — the partial
///   prefix would corrupt the cumulative fused key for downstream endings.
/// - Endings beyond `MAX_SYLLABLES` (= 8) are dropped to mirror the
///   `build_keys_tl` BFS depth bound, keeping per-keystroke FST lookup
///   and candidate scoring complexity bounded across modes. The TPS
///   syllabifier itself still scans the full input
///   (`tps::valid_span_endings` is unparameterised today); pushing the
///   cap into the syllabifier is a follow-up if profiling shows the
///   linear scan is hot.
// 中文: TPS key 構造 — 累加每個 Bopomofo 音節的 toneless TL,於每個 TPS ending 釋出 fused key (對應 build_keys_tl 的 lower[0..end])。
// 中文: consumed_span 仍以 Bopomofo bytes 為單位;tone-1 / 不合法片段一律 Phase 9 才補。
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
        // The TL converter ALWAYS appends a tone digit on a well-formed
        // single-syllable Bopomofo span (`engine/phonetics/src/tps.rs:301`).
        // If the trailing char is not a 1..=9 digit the fragment is
        // malformed (partial Bopomofo / unsupported sequence); skipping
        // it would corrupt the cumulative fused key for subsequent
        // endings, so we abort the whole TPS key build instead.
        let last = tl_numeric.as_bytes().last();
        let well_formed = matches!(last, Some(b) if b.is_ascii_digit() && *b != b'0');
        if !well_formed {
            return Vec::new();
        }
        let toneless = strip_trailing_tone_digit(&tl_numeric).to_ascii_lowercase();
        if toneless.is_empty() {
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
/// value matching `CommittedSegment.syllable_count: u8`.
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
    fn build_keys_tps_returns_empty_on_no_terminator() {
        // Bopomofo without any tone mark / entering coda → no endings.
        let keys = build_keys_tps("ㄉㄧㄠ");
        assert!(keys.is_empty(), "expected empty keys, got {keys:?}");
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

    #[test]
    fn strip_ascii_tone_digits_drops_all_ascii_digits() {
        // Equivalent to digit half of `notone.py::remove_tone()` regex
        // `[\d\-]` under the canonical-ASCII TL contract.
        assert_eq!(strip_ascii_tone_digits("tsua"), "tsua");
        assert_eq!(strip_ascii_tone_digits("tsua7"), "tsua");
        assert_eq!(strip_ascii_tone_digits("tai1bak4"), "taibak");
        // '0' is not a tone marker per phonetics::syllable.rs:18-20 but
        // notone.py drops every ASCII digit; mirror that here.
        assert_eq!(strip_ascii_tone_digits("a0b"), "ab");
        // Hyphen NOT stripped at this layer (syllabifier already can't
        // walk past it; see `build_keys_tl` Phase-6 limitations note).
        assert_eq!(strip_ascii_tone_digits("tai-bak"), "tai-bak");
    }

    #[test]
    fn clamp_syllable_count_saturates() {
        assert_eq!(clamp_syllable_count(0), 0);
        assert_eq!(clamp_syllable_count(1), 1);
        assert_eq!(clamp_syllable_count(255), 255);
        assert_eq!(clamp_syllable_count(256), 255);
        assert_eq!(clamp_syllable_count(u32::MAX), 255);
    }
}
