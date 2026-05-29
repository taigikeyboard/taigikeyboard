//! v3.5.9 A2 — `composing::continuous` seam.
//!
//! `assemble_candidates` is the named seam between `dispatch::handle_fetch_at_pos`
//! and the lexicon resolution primitives. It owns the 6-step continuous-input
//! candidate-vector contract (post-A2 reframing; line refs are `main` after
//! the seam extract — re-grep before relying on numbers):
//!
//! 1. Build the FST keys for this fetch: ALL modes (Tl / Poj / Tps / English)
//!    run `shadow::build_shadow_lattice` ONCE + `shadow::
//!    left_anchored_keys_from_lattice` projection (single `LexiconHandle::
//!    with_state` scope around steps 1–4, **D1 fold**). v3.5.9 D / C-3b
//!    retired the legacy TPS-specific `build_keys_tps` short-circuit (which
//!    folded TPS into `tl:` keys via `phonetics::tps_to_tl`); TPS now walks
//!    the same shadow → lattice path as TL/POJ with the only difference
//!    being the family family the syllabifier + key emitter resolve through
//!    `mode` (`tps:` against the C-0 emit of `dictionary.fst`).
//! 2. `keys.is_empty()` → [`fetch_via_lexicon_partial_inner`] (Item 10
//!    fallthrough) for ALL modes (TL/POJ/English/TPS). Each mode emits its
//!    own family prefix via [`crate::shadow::mode_key_prefix`], so a TPS
//!    leading initial like `ㄉ` scans `tps:ㄉ` byte-range exactly as TL's
//!    `g` scans `tl:g`.
//!    Else → [`fetch_via_lexicon_inner`] (span-local fetch).
//! 3. Per-candidate recase loop (`recase_roman` over each
//!    `consumed_span`; presentation `roman` only —`display_text` / `hanji`
//!    untouched).
//! 4. `fetch_walker_slot0_inner(..) → Some(slot0)`: span-aware
//!    retain-dedupe on `(roman, hanji, consumed_span)` then `insert(0,
//!    raw_to_proto_slot0(slot0))`. The walker returns a [`WalkerSlot0`] —
//!    **D3 honest type** — and the seam converts to `RawCandidate` with
//!    `score = -(slot0.cost as f32)` (the negated-cost bridge IS the wire
//!    contract, `CandidateMessage.score` proto field 5;
//!    `frequency`/`bitmask` are walker-N/A and stay `0`).
//!
//!    **Step 4b — whole-input prefix-extension scan (else-branch only).**
//!    After the walker slot-0 prepend, run
//!    [`fetch_via_lexicon_partial_inner`] against the whole-buffer key
//!    from [`crate::shadow::build_partial_prefix_key`] so dict rows
//!    whose key extends BEYOND any lattice edge (e.g. `tl:taigir` /
//!    `tâi-gír` for input `taigi`) surface as
//!    `COVERAGE_KIND_PARTIAL_PREFIX` candidates. Gated by
//!    `synth_consumed_span` (skipped when raw carries a pending
//!    trailing hyphen, same suppression as walker slot-0). Mainstream
//!    IME parity (librime `LookupWords(predictive=true)`, khiin-rs
//!    `candidates_for_splittable`, our own `lexicon::search`
//!    exact++prefix merge); the empty-keys branch above already
//!    covers this for inputs with no valid syllable, Step 4b extends
//!    it to already-syllabified inputs. Cross-batch
//!    `(roman, hanji, consumed_span)` dedupe drops extension hits
//!    already present in the FULL block. `SortKey`'s leading
//!    `coverage_kind` dim pins the result strictly below the FULL
//!    block, so no global re-sort is needed.
//! 5. POJ presentation pass: `mode == Poj` → [`recase_tl_as_poj_display`]
//!    over every candidate's `roman` then [`dedupe_rendered_continuous`].
//!    Skipped (not identity-called) for TL / English / TPS — the gate sits
//!    at the caller so the helper itself is POJ-only by signature. Runs
//!    AFTER the step-4 retain-dedupe so the prepended slot-0 row never
//!    moves; both dedupes preserved.
//! 6. The dispatch caller wraps the resulting `Vec<RawCandidate>` into
//!    `ContinuousResponse` via `raw_to_proto_candidate` + `with_continuous`.
//!
//! **D1 fold (B2 lifecycle invariant).** Before A2 the shadow + lattice was
//! built twice per non-TPS `FetchAtPos` — once in `build_keys_tl_with_inventory`,
//! once at the head of `fetch_walker_slot0`. A2 hoists ONE `LexiconHandle::
//! with_state` scope into [`assemble_candidates`] and calls
//! `build_shadow_lattice(raw, inv, mode)` exactly once. Byte-identical
//! under the mobile-IME lifecycle invariant — `EngineHandle::install` is
//! reachable only from `lexicon::api::install_engine` (mobile bridge
//! startup) plus the two test fixtures (`lexicon/tests/parity.rs`,
//! `composing/tests/golden_fetch_at_pos.rs`), never from a composing
//! `FetchAtPos`. Re-grep `EngineHandle::install` reachability whenever
//! touching this seam.
//!
//! **B1 step ordering** is load-bearing — step 5 (POJ render + post-render
//! dedupe) MUST run after step 4 (slot-0 prepend). Reordering changes the
//! visible candidate vector.
//!
//! Spec source-of-truth: `docs/reports/2026-05-18-v358-refactor-design-spec.md`
//! § 2 + ⭐ B1 / B2 / S7 amendment block.

// 中文: A2 — composing::continuous seam。assemble_candidates 統一 6-step 契約 +
// 中文:   D1 fold (build_shadow_lattice 單建) + D3 honest type (WalkerSlot0)。
// 中文: with_state 單一,B1 step 順序載重,B2 lifecycle invariant = install 只在
// 中文:   mobile-IME startup,FetchAtPos 不可達。POJ render/dedupe 在 walker prepend 後。
// 中文: spec docs/reports/2026-05-18-v358-refactor-design-spec.md §2 + ⭐ B1/B2/S7。

use crate::shadow::{
    build_partial_prefix_key, build_shadow_lattice, custom_toneless_key,
    greedy_longest_syllabification, left_anchored_keys_from_lattice, span_min_syllable_count,
    strip_tones_for_mode,
};
use lexicon::dictionary_reader::DictionaryReader;
use lexicon::prefix_index::PrefixIndex;
use lexicon::{
    best_candidate_for_key, derive_mode, fetch_candidates_for_keys,
    fetch_partial_prefix_candidates, fetch_partial_prefix_candidates_unbounded, CandidateMode,
    ConsumedSpan, ContinuousFetchCtx, CustomEntry, EngineHandle as LexiconHandle, RawCandidate,
    SyllableInventory, COVERAGE_KIND_FULL, FORM_NOTONE, PARTIAL_PREFIX_OUTPUT_CAP,
};
use ranking::{decayed_user_weight_delta, recency_rank, FrequencyMap};

// ============================================================================
// D3 honest type — walker result struct names `cost`
// ============================================================================

/// v3.5.9 A2 (D3 honest type) — the slot-0 walker's native result.
///
/// `cost` is the min-cost objective the walker optimizes (`lower-better`);
/// the seam in [`assemble_candidates`] converts to the wire `RawCandidate`
/// with `score = -(cost as f32)` (preserving the proto field 5 contract on
/// `CandidateMessage`). `frequency` / `bitmask` are walker-N/A and stamped
/// `0` at the seam — slot 0 is an explicit prepend, never sort-compared,
/// so they are informational only. `form` is stamped `FORM_NOTONE` at the
/// seam (walker always emits the toneless representation).
// 中文: D3 honest type — walker 原生結果。cost 是 min-cost 目標 (低-best);
// 中文:   seam 轉 wire 時 score = -(cost as f32) (CandidateMessage.score field 5 契約)。
// 中文:   frequency/bitmask/form 由 seam 戳定 (walker 無關),slot 0 explicit prepend 不參與排序。
pub(crate) struct WalkerSlot0 {
    pub cost: f64,
    pub consumed_span: ConsumedSpan,
    pub syllable_count: u8,
    pub display_text: String,
    pub roman: String,
    pub hanji: Option<String>,
    pub mode: CandidateMode,
    pub recency_rank: u8,
    pub coverage_kind: u8,
    pub is_custom: bool,
}

// ============================================================================
// Pure helpers (no lexicon state)
// ============================================================================

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

/// v3.5.9 C-4 (renamed from `render_roman_for_mode`) — POJ display rewrite
/// for a continuous candidate's presentation `roman`: `oo`→`o͘`, `nn`→`ⁿ`,
/// `ua`→`oa`, … via [`phonetics::api::tl_display_to_poj_display`], plus a
/// POJ-aware case restore. Not mode routing. Not the TL↔POJ canonical-form
/// pipeline; canonical commit keys stay under
/// [`phonetics::api::canonical_tl_form`]. This helper is step 5 of
/// [`assemble_candidates`] and runs only when the caller is in POJ mode
/// (gate at `assemble_candidates`); the signature is POJ-only so the
/// gate cannot drift to silent identity-on-misuse.
///
/// The TL→POJ rewriter only title-cases (it checks `first.is_uppercase()`
/// then stops), so a CapsLocked candidate (`HOO`) would otherwise collapse
/// to title case (`Ho͘`); `transform_input_case` with the detected
/// `LetterCase` and `InputMode::Poj` restores it (`HO͘`) — the same helper
/// `recase_roman` already trusts for POJ diacritics (Codex pre-impl
/// 2026-05-19 BLOCK). Case detection reads the (already-recased) roman's
/// own alpha chars via [`raw_segment_letter_case`]: span-local candidates
/// carry one uniform case so this is exact. A multi-segment walker path
/// with heterogeneous casing collapses to one bucket derived from the
/// whole string — first alpha lowercase ⇒ all-lowercase; first upper but
/// not all remaining uppercase ⇒ leading-cap only; uniformly upper ⇒
/// all-caps — a bounded POJ-only edge far outside normal use, not the
/// reported `oo`/`nn` defect. Presentation only — never feed `display_text`
/// (the canonical commit / `user_frequency.db` key) here.
// 中文: C-4 — POJ display 重寫(TL→POJ-display:oo→o͘、nn→ⁿ、ua→oa…)加上 POJ-aware case 還原。
// 中文:   非 mode 路由,非 TL↔POJ canonical 管線(canonical commit key 走 canonical_tl_form)。
// 中文:   assemble_candidates step 5;只在 POJ mode 由 caller 呼叫,簽章 POJ-only 不接受 mode 漂移。
// 中文: TL→POJ rewriter 只 title-case,CapsLock(HOO)會塌成 Ho͘,此處還原成 HO͘
// 中文:   (Codex pre-impl 2026-05-19 BLOCK)。span-local 單段 case 均勻故精確;
// 中文:   walker 多段異質大小寫依整串首字母塌成單一桶(首小寫⇒全小寫;
// 中文:   首大寫但其餘非全大寫⇒僅首字大寫;全大寫⇒全大寫)— POJ-only 邊角,非回報 bug。
// 中文:   只處理呈現 roman,勿傳 display_text。
fn recase_tl_as_poj_display(roman: &str) -> String {
    let case = raw_segment_letter_case(roman);
    let poj = phonetics::api::tl_display_to_poj_display(roman);
    phonetics::case_transform::transform_input_case(&poj, case, phonetics::InputMode::Poj)
}

/// v3.5.8 — collapse continuous candidates that became identical only
/// after the POJ render. The pre-render `(roman, hanji, consumed_span)`
/// dedupe (`lexicon::dedupe_by_roman_hanji_span`) keys on the **raw**
/// stored romanization without canonicalization, so a hanji-bearing
/// custom entry stored in POJ display form (`roman = "gô͘"`,
/// `hanji = Some("吳")`) and a `dict.bin` entry in TL (`roman = "gôo"`,
/// `hanji = Some("吳")`) sharing one consumed span both survive it,
/// then [`recase_tl_as_poj_display`] rewrites both `roman` fields to
/// the POJ display form (`gô͘`) and produces a visible duplicate.
/// `dedupe_rendered_continuous` collapses that visible duplicate; the
/// `HashSet::retain` first-wins rule keeps whichever row sits earlier
/// in the **current candidate vector** at the moment this fn runs
/// (post-sort, post whole-sentence prepend), so the prepended
/// best-walk-path candidate at index 0 is never dropped. It is
/// behavior-neutral whenever `hanji` is `Some`: the collision key
/// pins the same hanji and `display_text` (the commit /
/// `user_frequency.db` key) is that hanji for BOTH the custom and the
/// `dict.bin` candidate, so which row survives cannot change what
/// commits.
///
/// **v3.5.9 B-4 closed the romanization-only (hanji-absent) freq-key
/// asymmetry but NOT this hanji-bearing visible-duplicate axis.**
/// Pre-B-4 the hanji-absent custom-after-dict collision could leave the
/// `display_text` as the dict-TL row's form while the custom row's raw
/// `entry.roman` was still POJ — a frequency-key granularity nuance
/// that `lexicon::custom_entry_to_candidate` now closes by folding the
/// fallback through `phonetics::api::canonical_tl_form`. B-4
/// deliberately leaves `entry.roman` raw (the walker / `custom_toneless_key`
/// need it in the user's native form so POJ-family lattice keys match
/// against POJ-form custom roman per `composing::shadow::custom_toneless_key`),
/// which means the hanji-bearing custom-vs-dict-POJ-form collision
/// still slips past pre-render dedupe and lands here. This dedupe is
/// load-bearing in steady state, not transitional. Called in the POJ
/// branch only — for TL/English the render is identity so the
/// pre-render dedupe already settled every key. TPS has its own
/// post-render visual collapse via [`dedupe_display_hanji_for_tps`]
/// because the TPS UI hides romanization entirely (hanji-only display),
/// so two `dict.bin` rows differing only on `roman` (e.g. `灣 / uan`
/// vs `灣 / uân` at the same TPS-toneless key `ㄨㄢ`) survive the
/// pre-render `(roman, hanji, span)` dedupe yet render identically.
// 中文: POJ render 後才相等的候選去重(custom 存 POJ `gô͘` vs dict TL `gôo`,
// 中文:   同 hanji+span 過不了 raw `roman` 的 pre-render 去重,render 後皆 `gô͘`)。
// 中文:   first-wins 取「執行時當下候選向量」中較前者(post-sort、post 整句 prepend),
// 中文:   故 index 0 整句最佳候選不被丟。hanji 存在時行為中性。
// 中文:   B-4 已關掉 romanization-only(hanji==None)子情境的 freq-key 不一致:
// 中文:   lexicon::custom_entry_to_candidate 走 canonical_tl_form。但 B-4 刻意保留 entry.roman
// 中文:   原樣(walker / custom_toneless_key 需 user 原形對齊 POJ-family lattice 鍵),
// 中文:   故 hanji-bearing 的 custom-vs-dict-POJ-form 視覺重複仍需此 post-render dedupe 兜底。
// 中文:   穩態 load-bearing,非過渡兜底。只在 POJ 分支呼叫。
// 中文:   TPS 走 dedupe_display_hanji_for_tps,因 TPS UI 只顯示漢字,純羅馬字差異(灣/uan vs 灣/uân
// 中文:   同 tps:ㄨㄢ)無法被 pre-render (roman,hanji,span) 去重攔下。
fn dedupe_rendered_continuous(candidates: &mut Vec<RawCandidate>) {
    use std::collections::HashSet;
    let mut seen: HashSet<(String, Option<String>, ConsumedSpan)> =
        HashSet::with_capacity(candidates.len());
    candidates.retain(|c| seen.insert((c.roman.clone(), c.hanji.clone(), c.consumed_span)));
}

/// TPS-mode visual dedupe: collapse candidates sharing `(hanji,
/// consumed_span)` because the TPS candidate strip hides romanization
/// (TPS = hanji-first input mode per `.claude/rules/phonetics.md`).
/// Two `dict.bin` rows like `灣 / uan` (tone 1) and `灣 / uân` (tone 5)
/// both index under `tps:ㄨㄢ`, so the build pipeline legitimately
/// emits both rowids at the same FST key. The pre-sort
/// `lexicon::dedupe_by_roman_hanji_span` keys on `(roman, hanji, span)`
/// and preserves romanization variants; that is correct for TL/POJ
/// (the UI shows distinct `uan / 灣` vs `uân / 灣` rows) but produces
/// a visible duplicate in TPS mode where only `灣` is rendered.
///
/// Runs AFTER the sort + walker slot-0 prepend + (no-op for TPS) POJ
/// render pass so first-seen = highest-ranked. `consumed_span` stays
/// in the key so the same hanji at different spans survives — e.g. a
/// 1-syllable `灣` covering only the first syllable `(0, n1)` and a
/// 1-syllable `灣` re-emitted by the walker at the full-buffer span
/// `(0, raw_len)` are distinct commit surfaces; the rationale matches
/// `lexicon::dedupe_by_roman_hanji_span`. `hanji = None` or empty
/// hanji passes through to mirror
/// `engine/ranking/src/dedup.rs::remove_display_duplicates` (legacy
/// `processCandidates` path, dead in production today); upstream
/// `lexicon::dedupe_by_roman_hanji_span` already collapses exact
/// `(roman, hanji, span)` duplicates so this branch never reaches a
/// truly identical pair.
// 中文: TPS 顯示去重 — (hanji, consumed_span) 為鍵;TPS UI 只顯示漢字,純羅馬字差異
// 中文:   (灣/uan vs 灣/uân 皆掛 tps:ㄨㄢ) 在 TPS 是視覺重複,在 TL/POJ 是合法獨立列。
// 中文:   排序+walker slot-0 prepend+(TPS no-op) POJ render 之後跑,first-seen = 最高排名。
// 中文:   保留 span 讓「同 hanji 不同 span」(partial vs full-buffer)合法候選不被誤併;
// 中文:   hanji 為 None 或空字串者一律保留(顯示為 TPS/羅馬字,以 roman 區隔)。
fn dedupe_display_hanji_for_tps(candidates: &mut Vec<RawCandidate>) {
    use std::collections::HashSet;
    let mut seen: HashSet<(String, ConsumedSpan)> = HashSet::with_capacity(candidates.len());
    candidates.retain(|c| match c.hanji.as_deref() {
        Some(h) if !h.is_empty() => seen.insert((h.to_owned(), c.consumed_span)),
        _ => true,
    });
}

// v3.5.9 D / C-3b — `build_keys_tps` + `strip_trailing_tone_digit`
// retired. TPS now walks the shared `build_shadow_lattice` +
// `left_anchored_keys_from_lattice` path (with mode-aware tone strip via
// `shadow::strip_tones_for_mode` and `tps:` family routing) the same way
// TL / POJ do; the legacy per-syllable `phonetics::tps_to_tl` fold into
// `tl:` keys was the one place TPS diverged from the shared seam and is
// no longer needed.
// 中文: D / C-3b — build_keys_tps 與 strip_trailing_tone_digit 退役。
// 中文:   TPS 改走與 TL/POJ 共用的 shadow → lattice 路徑(mode-aware tone strip
// 中文:   + tps: 家族),不再經 phonetics::tps_to_tl 折成 tl: 家族鍵。

// ============================================================================
// Inner fetchers — take pre-resolved lexicon state (no with_state inside).
// ============================================================================

/// v3.5.9 A2 — span-local fetch inner. Pre-A2 `fetch_via_lexicon`'s body
/// minus the `LexiconHandle::with_state` opener and the `prefix_index` /
/// `dictionary` `as_ref()?` guards (those moved into [`assemble_candidates`]
/// under the **D1 fold**). Behavior is byte-identical to pre-A2:
/// `enabled_sources_bitmask = u32::MAX` (PR-9.6 will plumb platform
/// toggles uniformly to both paths). Caller is responsible for the empty
/// `Vec::new()` return when state is unavailable.
///
/// v3.5.9 D7 — takes [`ContinuousFetchCtx`] for the six shared lexicon
/// args; `enabled_sources_bitmask` is pinned to `u32::MAX` at the seam
/// construction site (`assemble_candidates`), not here.
// 中文: A2 — fetch_via_lexicon 純內層;D1 fold 後 prefix/dict 由 seam 提取;bitmask 仍 u32::MAX,等 PR-9.6 兩條路徑同步 plumb。
// 中文: D7 改:六個共用 arg 收進 ContinuousFetchCtx,seam 端建一次傳兩個 inner。
fn fetch_via_lexicon_inner(
    keys: &[(ConsumedSpan, String)],
    raw_len: u32,
    ctx: &ContinuousFetchCtx<'_>,
) -> Vec<RawCandidate> {
    fetch_candidates_for_keys(keys, raw_len, ctx)
}

/// v3.5.9 A2 — partial-prefix fetch inner. Pre-A2 `fetch_via_lexicon_partial`'s
/// body minus the `LexiconHandle::with_state` opener.
/// [`build_partial_prefix_key`] (renamed from `_tl` suffix in v3.5.9
/// B-2 since the emitter is now mode-aware) is pure (shadow primitives
/// only) and runs ahead of any state borrow.
///
/// v3.5.9 D7 — takes [`ContinuousFetchCtx`]; `mode` is kept as a
/// separate arg because it is shadow-key construction input, not part
/// of the shared lexicon ctx.
///
/// v3.5.9 B-0c — `mode: phonetics::InputMode` replaces the prior
/// `is_poj: bool` (`mode == InputMode::Poj` preserves the old gate).
/// `mode` flows untouched through to [`build_partial_prefix_key`],
/// keeping the seam's single-source-of-truth invariant. v3.5.9 B-2
/// makes the emitter mode-aware so POJ partial-prefix keys land in the
/// `poj:` family rather than folding through the TL chain.
// 中文: A2 — fetch_via_lexicon_partial 純內層;build_partial_prefix_key 純函式先跑,prefix/dict 由 seam 提取。
// 中文: D7 改:六個共用 arg 收進 ContinuousFetchCtx;mode 為 shadow key 構造輸入,留 separate arg。
// 中文: B-0c 改:`mode: phonetics::InputMode` 取代 `is_poj: bool`,語意對齊(POJ 等價 mode == Poj)。
// 中文: B-2 改:build_partial_prefix_key (脫 `_tl` 後綴) emitter 改 mode-aware,POJ 走 `poj:` 家族。
fn fetch_via_lexicon_partial_inner(
    raw: &str,
    raw_len: u32,
    mode: phonetics::InputMode,
    ctx: &ContinuousFetchCtx<'_>,
) -> Vec<RawCandidate> {
    fetch_via_lexicon_partial_inner_impl(raw, raw_len, mode, ctx, /* bounded */ true)
}

/// v3.5.9 Step 4b — un-truncated partial-prefix fetch for the FULL +
/// PARTIAL cross-batch path. Same hydrate/dedupe/sort/recase as
/// [`fetch_via_lexicon_partial_inner`] but returns the pool without
/// applying the `PARTIAL_PREFIX_OUTPUT_CAP` truncate. Step 4b applies
/// its FULL-block exclude on the un-truncated pool, then truncates
/// itself — addresses Codex PR #351 r3321758666 (exact-key homophones
/// would otherwise eat the entire output cap before the dedupe ran).
///
/// The empty-keys branch keeps using
/// [`fetch_via_lexicon_partial_inner`] (bounded wrapper) because it has
/// no FULL block to exclude against.
// 中文: Step 4b 專用 — 不裁尾的 partial-prefix fetcher;Codex PR #351 r3321758666 反應的
// 中文:   bug:exact 同音字 ≥30 時,bounded 版會把 OUTPUT_CAP 全消耗在被 dedupe 剔除的 row。
// 中文:   un-bounded 版讓 Step 4b 在 caller 端套 exclude → 自己裁,確保延伸候選能浮現。
fn fetch_via_lexicon_partial_inner_unbounded(
    raw: &str,
    raw_len: u32,
    mode: phonetics::InputMode,
    ctx: &ContinuousFetchCtx<'_>,
) -> Vec<RawCandidate> {
    fetch_via_lexicon_partial_inner_impl(raw, raw_len, mode, ctx, /* bounded */ false)
}

fn fetch_via_lexicon_partial_inner_impl(
    raw: &str,
    raw_len: u32,
    mode: phonetics::InputMode,
    ctx: &ContinuousFetchCtx<'_>,
    bounded: bool,
) -> Vec<RawCandidate> {
    let Some(key) = build_partial_prefix_key(raw, mode) else {
        return Vec::new();
    };
    let mut out = if bounded {
        fetch_partial_prefix_candidates(&key, raw_len, ctx)
    } else {
        fetch_partial_prefix_candidates_unbounded(&key, raw_len, ctx)
    };
    // Partial-prefix candidates carry `consumed_span = (0, raw_len)` by
    // construction (`build_partial_prefix_key` pins the span). Recase against
    // the full raw buffer so the keys-empty branch and the Step 4b extension
    // path apply the SAME per-candidate case mapping the span-local Step 3
    // loop applies to its FULL-coverage hits. Without this, the case display
    // of a partial-prefix hit flips between branches as the user crosses the
    // first syllable boundary (e.g. typing `T` → keys.is_empty() arm,
    // unrecased; then `Ta` → else-arm + Step 4b, recased).
    // 中文: partial-prefix 候選 consumed_span 由 build_partial_prefix_key 釘成 (0,raw_len);
    // 中文:   集中在此 recase 讓 empty-keys 與 Step 4b 兩條路徑共用同一大小寫規則,
    // 中文:   避免使用者跨第一個音節邊界時同一字典 row 的大小寫忽然翻轉。
    if raw_len as usize <= raw.len() {
        let seg = &raw[..raw_len as usize];
        for cand in &mut out {
            cand.roman = recase_roman(&cand.roman, seg, mode);
        }
    }
    out
}

/// v3.5.9 A2 — slot-0 whole-sentence walker inner. Pre-A2
/// `fetch_walker_slot0`'s body minus the `LexiconHandle::with_state`
/// opener, the state `as_ref()?` guards, AND the `build_shadow_lattice`
/// call (now built ONCE in the seam under **D1 fold** and passed in).
/// Returns [`WalkerSlot0`] (D3 honest type): `cost` named explicitly;
/// the seam converts to wire via `score = -(cost as f32)`.
///
/// v3.5.9 B-0c — `mode: phonetics::InputMode` replaces the prior
/// `is_poj: bool` (Codex pre-impl SHOULD 2026-05-21 + 2026-05-20 enum
/// sweep). `mode` is taken as a parameter rather than re-derived
/// here so the single-source-of-truth invariant — same `mode` feeds
/// `build_shadow_lattice` (seam) and `custom_toneless_key` (here) —
/// cannot be silently broken by a future seam refactor.
///
/// Returns `None` when the synth must be suppressed (trailing-hyphen
/// shadow short of raw, or the walker cannot span the buffer) — caller
/// leaves the span-local list untouched (pre-S2 behavior preserved).
// 中文: A2 — fetch_walker_slot0 純內層;D1 fold 後 (shadow, shadow_to_raw_end, lattice, inv,
// 中文:   prefix, dict) 由 seam 預建傳入,內層只走 walker + tail。
// 中文: B-0c — mode 由 seam 顯式傳入(取代 is_poj bool);確保 build_shadow_lattice 與
// 中文:   custom_toneless_key 同一 mode,不可未來 refactor 漂移。
// 中文: 回傳 WalkerSlot0 (D3),seam 轉 wire 時 score = -(cost as f32);trailing-hyphen 抑制仍 None。
#[allow(clippy::too_many_arguments)]
fn fetch_walker_slot0_inner(
    raw: &str,
    raw_len: u32,
    freq_map: &FrequencyMap,
    now_ms: i64,
    mode: phonetics::InputMode,
    custom: &[CustomEntry],
    shadow: &str,
    shadow_to_raw_end: &[usize],
    lattice: &crate::lattice::Lattice,
    inv: &SyllableInventory,
    prefix: &PrefixIndex,
    dict: &DictionaryReader,
) -> Option<WalkerSlot0> {
    // v3.5.8 S6 (Codex pre-impl S6 Q2/Q6, 2026-05-17) — per-fetch
    // map from a custom entry's normalized toneless key to the
    // entry. `custom_toneless_key` reuses the SAME shadow pipeline
    // the edge keys use, so a hit here is byte-identical to a
    // lattice edge's `tl:{toneless}` (Q2 BLOCK: a plain
    // tone-digit-strip would not fold a POJ/diacritic custom roman
    // like `tâi-uân`). `or_insert` = **first-wins** on a duplicate
    // key (Codex Q6: explicit, not `HashMap` overwrite/iteration).
    //
    // S6 byte-identity invariant: the SAME `mode` feeds
    // `build_shadow_lattice` (caller) and `custom_toneless_key` (here);
    // taking it as a parameter (B-0c: enum sweep — replaces the
    // pre-B-0c `is_poj: bool` from Codex pre-impl SHOULD 2026-05-21)
    // makes the contract local — a split-brain (POJ-aware edges,
    // mode-blind custom keys) would silently drop custom matches in
    // POJ mode.
    // 中文: S6 — per-fetch「custom toneless key → entry」表;custom_toneless_key 重用同一 shadow pipeline
    // 中文:   → 命中與 lattice edge key byte-identical(Q2 BLOCK:POJ/diacritic 須先 canonicalize);
    // 中文:   同 key 重複 = or_insert first-wins(Codex Q6,非 HashMap 覆寫)。
    // 中文: S6 byte-identity — 同一 mode 餵 build_shadow_lattice 與 custom_toneless_key;
    // 中文:   參數傳入(B-0c enum sweep)而非 from-mode 內部推導,split-brain 不可能。
    let mut custom_map: std::collections::HashMap<String, &CustomEntry> =
        std::collections::HashMap::with_capacity(custom.len());
    for entry in custom {
        if let Some(k) = custom_toneless_key(&entry.roman, mode) {
            custom_map.entry(k).or_insert(entry);
        }
    }
    // Codex post-impl S2 P1: suppress the synth when a trailing
    // hyphen leaves the shadow short of the raw buffer (a
    // `(0, raw_len)` synth would mis-commit the pending `-`).
    // Cheap early-out before walking.
    let consumed_span = synth_consumed_span(shadow_to_raw_end, shadow.len(), raw_len)?;

    let path = crate::lattice::walk_best(lattice, shadow.len(), |start, end| {
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
        // v3.5.9 D / C-3b — mode-aware tone strip. TL/POJ/English drop
        // ASCII digits (byte-identical to legacy `strip_ascii_tone_digits`),
        // TPS drops the 8 Bopomofo tone marks so the body matches the
        // `tps:<tps_notone>` family from C-0.
        // 中文: D / C-3b — mode-aware tone 剝除;TPS 改剝注音聲調,對齊 tps_notone 家族。
        let toneless = strip_tones_for_mode(&shadow[start..end], mode);
        if toneless.is_empty() {
            return None;
        }
        let raw_span = (
            shadow_to_raw_end[start] as u32,
            shadow_to_raw_end[end] as u32,
        );
        // v3.5.9 B-2 — walker edge key is mode-aware: the same `mode`
        // feeding the shadow + lattice (`build_shadow_lattice`) above
        // also feeds the key prefix here, so the lookup family is
        // consistent with the inventory family that produced the edge.
        // 中文: B-2 — walker edge key mode-aware,單一 mode 同時驅動 shadow / lattice / key 前綴,
        // 中文:   不同家族 (tl/poj/tps) 不會由不同來源分歧。
        let key = format!(
            "{prefix}:{toneless}",
            prefix = crate::shadow::mode_key_prefix(mode)
        );
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
            // `canonical_tl_form(roman, mode)`) so a user-selected
            // custom entry's decayed weight folds into the path
            // objective identically to a dict edge (S3 Q4d seam).
            // v3.5.9 B-4 — the canonicalization keeps the commit key
            // mode-invariant when the custom entry stored its roman
            // in POJ display form (the user's native mode). Single-
            // syllable custom is damped by
            // `WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE` in
            // `edge_cost`, same as dict.
            // 中文: B-4 — display_text fold canonical TL,跨 mode freq key 合一;
            // 中文:   `entry.roman` 保留原樣(lattice key 比對需要)。
            let display_text = entry
                .hanji
                .clone()
                .unwrap_or_else(|| phonetics::api::canonical_tl_form(&entry.roman, mode));
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
            let syllable_count = greedy_longest_syllabification(&shadow[start..end], inv, mode)
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
                let user_weight_delta = decayed_user_weight_delta(count, now_ms, fd.last_used_ms);
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
                let syllable_count = span_min_syllable_count(&shadow[start..end], inv, mode)?
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

    let path = path?;
    if path.choices.is_empty() {
        return None;
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
        match greedy_longest_syllabification(shadow, inv, mode) {
            Some(segs) if !segs.is_empty() => {
                // v3.5.9 D / C-3b — mode-aware tone strip. TPS no-dict
                // synth strips Bopomofo tone marks per syllable; TL/POJ
                // keep the byte-identical ASCII-digit drop.
                // 中文: D / C-3b — TPS no-dict synth 改剝注音聲調符號;
                // 中文:   TL/POJ 保 byte-identical ASCII 數字剝除。
                let r = segs
                    .iter()
                    .map(|&(s, e)| strip_tones_for_mode(&shadow[s..e], mode))
                    .collect::<Vec<_>>()
                    .join(" ");
                let s = segs.len().min(u8::MAX as usize) as u8;
                (r, s)
            }
            // Cannot cleanly syllabify the buffer → leave the
            // span-local list untouched (pre-S2 behavior, mirrors
            // the synth_consumed_span trailing-hyphen suppression).
            _ => return None,
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
    // v3.5.9 B-4 (Codex pre-impl BLOCK #2 close): the walker
    // greedy-longest OOV synth builds `roman` from the
    // mode-aware shadow — POJ mode preserves POJ ASCII per B-2,
    // so a hanji-absent synth candidate would otherwise key
    // `user_frequency.db` as POJ ASCII (`chiah goa`) and split
    // from the same word's TL-mode commit key (`tsiah gua`).
    // `canonical_tl_form` folds POJ-mode synth roman to TL so
    // the freq key is mode-invariant; `roman` itself stays in
    // its native form so the downstream `recase_tl_as_poj_display`
    // pass renders correctly (POJ-mode roundtrip via
    // `tl_display_to_poj_display`). §9 #2 user-history contract
    // is now satisfied across all three candidate paths:
    // `dict.bin` (record.tl is canonical TL), custom dict (this
    // change to `custom_entry_to_candidate`), and walker synth
    // (this site).
    // 中文: B-4 — walker OOV synth roman 在 POJ mode 是 POJ ASCII;
    // 中文:   freq key 折成 canonical TL 跨 mode 合一,roman 自身留原 form
    // 中文:   讓 recase_tl_as_poj_display roundtrip 正確。§9 #2 三 path 全合規。
    let display_text = hanji
        .clone()
        .unwrap_or_else(|| phonetics::api::canonical_tl_form(&roman, mode));
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
    let candidate_mode = derive_mode(hanji.as_deref());
    Some(WalkerSlot0 {
        // S5: `path.cost` is a min-cost (lower = better) total.
        // The seam converts to wire `score = -(cost as f32)`
        // (`CandidateMessage` proto field 5) preserving the
        // higher-better wire monotonic ordering. Slot 0 is an
        // explicit prepend so the wire value is informational only
        // (never sort-compared), but the bridge IS the contract —
        // do not "fix" the sign.
        cost: path.cost,
        consumed_span,
        syllable_count,
        display_text,
        roman,
        hanji,
        mode: candidate_mode,
        recency_rank: recency_rank(now_ms, last_used_ms),
        coverage_kind: COVERAGE_KIND_FULL,
        // v3.5.8 S6 (Codex pre-impl S6 Q4): provenance truth — a
        // synthesized full-buffer path containing ≥1 custom edge is
        // custom-influenced. Informational at slot 0 (explicit
        // prepend, not sorted; the seam dedupe keys on
        // `(roman,hanji,consumed_span)` not `is_custom`), but a
        // truthful flag keeps future ranking/dedupe changes sound.
        is_custom: path.choices.iter().any(|c| c.is_custom),
    })
}

// ============================================================================
// Seam orchestrator — single `with_state` scope, 6-step contract.
// ============================================================================

/// v3.5.9 A2 seam — the 6-step `assemble_candidates` contract.
///
/// Replaces the inline `let mut candidates = if keys.is_empty() {...} else
/// {...}` block + POJ presentation pass that lived in
/// `dispatch::handle_fetch_at_pos` before A2. See module docs for the
/// 6-step contract and the **D1 fold** lifecycle invariant.
///
/// Inputs are the proto→domain hoists from `handle_fetch_at_pos`
/// (`mode` from `parse_input_mode` + `contains_tps` raw-buffer upgrade;
/// `freq_map` from `ranking::build_frequency_map`; `custom` from
/// `build_custom_entries`). Every TPS-vs-non-TPS branch derives from
/// `mode == InputMode::Tps` internally — v3.5.9 D / C-3b dropped the
/// parallel `is_tps: bool` arg (single mode axis, no split-brain).
/// `mode == Poj` similarly derives the POJ branch — v3.5.9 B-0c
/// dropped the prior `is_poj: bool` arg.
/// Output is the unwrapped `Vec<RawCandidate>` the caller maps to
/// `CandidateMessage` via `raw_to_proto_candidate`.
// 中文: A2 seam — 6-step assemble_candidates。取代 A2 前 handle_fetch_at_pos 內 inline 區塊。
// 中文: 入參皆為 dispatch hoist 過的 domain 型別,出參為 wire 前 RawCandidate vector。
// 中文: B-0c — `mode: InputMode` 取代 `is_poj: bool` 內部 derive。
// 中文: D / C-3b — `is_tps: bool` 退役;所有 TPS 分支由 mode == InputMode::Tps 推導,
// 中文:   平台 contains_tps(raw) 升級在 dispatch 端完成,seam 只看 mode。
// 中文: 單一 LexiconHandle::with_state 範圍包住 step 1–4;D1 fold = build_shadow_lattice 單建;
// 中文:   POJ render/dedupe (step 5) 在 walker prepend 之後。
pub(crate) fn assemble_candidates(
    raw: &str,
    freq_map: &FrequencyMap,
    now_ms: i64,
    custom: &[CustomEntry],
    mode: phonetics::InputMode,
) -> Vec<RawCandidate> {
    let raw_len = raw.len() as u32;
    LexiconHandle::with_state(|state| {
        let inv = state.syllable_inventory.as_ref();
        let prefix = state.prefix_index.as_ref();
        let dict = state.dictionary.as_ref();

        // v3.5.9 D7 — build the shared lexicon-fetch context once per
        // seam invocation. `Some` only when BOTH `prefix_index` and
        // `dictionary` resolved; either inner fetcher (span-local or
        // partial-prefix) needs both. `enabled_sources_bitmask =
        // u32::MAX` is the production wiring (Item: PR-9.6 will plumb
        // platform toggles uniformly).
        // 中文: D7 — 此次 seam 共用的 lexicon-fetch ctx 只建一次;只有 prefix+dict 兩個都解到時才 Some
        // 中文:   (兩個內層 fetcher 都需要);bitmask=u32::MAX 為當前 production 對齊 (待 PR-9.6 統一平台 plumb)。
        let lex_ctx = prefix
            .zip(dict)
            .map(|(prefix_index, dict)| ContinuousFetchCtx {
                enabled_sources_bitmask: u32::MAX,
                freq_map,
                now_ms,
                custom,
                prefix_index,
                dict,
                // v3.5.9 B-4 — thread the active input mode through so
                // `lexicon::custom_entry_to_candidate` can fold a
                // POJ-form custom roman's `display_text` (the
                // `user_frequency.db` commit key) to canonical TL,
                // keeping the freq key mode-invariant. Lattice / FST
                // key prefix is decided above and embedded in `keys`,
                // so this is purely a freq-key axis.
                // 中文: B-4 — mode 透到 lexicon 端 canonicalize hanji-absent display_text。
                // 中文: D / C-3b — TPS 改走 InputMode::Tps,canonical_tl_form 對 Tps 走 identity。
                mode,
            });

        // ---- Step 1: build keys + shadow/lattice (D1 fold).
        // v3.5.9 D / C-3b — all modes (Tl / Poj / Tps / English) share
        // the same shadow-pipeline path: build the shadow + lattice ONCE
        // here and project the left-anchored keys via mode-aware
        // [`shadow::mode_key_prefix`] + [`shadow::strip_tones_for_mode`];
        // walker reuses the SAME lattice (no rebuild). The legacy
        // `build_keys_tps` short-circuit retired with C-3b — TPS now
        // walks the same path TL/POJ already do, the only difference
        // being the inventory family selected by `mode` inside
        // [`crate::syllabifier::valid_span_endings_lowered`].
        // Inv absent → empty keys and walker skipped (graceful
        // degradation; matches pre-A2 / pre-C-3b behavior).
        // 中文: D / C-3b — 所有模式共用 shadow-pipeline 單路徑;舊 build_keys_tps 短路退役。
        // 中文:   inv 缺席時退化為空鍵 + walker 跳過(優雅退化,與 A2 前同)。
        let (keys, shadow_lattice) = match inv {
            Some(inv) => {
                let (shadow, shadow_to_raw_end, lattice) = build_shadow_lattice(raw, inv, mode);
                let keys =
                    left_anchored_keys_from_lattice(&shadow, &shadow_to_raw_end, &lattice, mode);
                (keys, Some((shadow, shadow_to_raw_end, lattice, inv)))
            }
            None => (Vec::new(), None),
        };

        // ---- Step 2: empty-keys partial-prefix vs span-local fetch.
        let mut candidates: Vec<RawCandidate> = if keys.is_empty() {
            // v3.5.8 Phase 9 Item 10 — partial-prefix fallthrough.
            // The syllabifier produced no valid ending (e.g. `raw =
            // "gu"`, `"t"`, `"ㄉ"`), so the lookup-exact path is dead.
            // Fall through to `lookup_prefix` so the user still sees
            // engine candidates while typing toward the first syllable
            // boundary. Spec: `docs/engine/
            // continuous-candidate-display.md` §15.3.D + §15.5.
            //
            // Mode-aware for ALL modes: TL/POJ/English emit `tl:`/`poj:`
            // family keys (since v3.5.9 B-2); TPS emits the `tps:` family
            // (Fork 7b activated by 2026-05-27 dogfood — a leading
            // Bopomofo initial `ㄉ` scans `tps:ㄉ` byte-range and returns
            // every dictionary row whose `tps_notone` starts with `ㄉ`,
            // matching librime / khiin-rs / McBopomofo leading-prefix
            // behavior). [`crate::shadow::build_partial_prefix_key`]
            // returns `None` for empty-after-tone-strip input (bare tone
            // mark / hyphen-only / digit-only), so no unbounded scan.
            // 中文: Item 10 — syllabifier 切不出邊界時走 partial-prefix。
            // 中文: 全模式 mode-aware:TPS 由 build_partial_prefix_key 發 `tps:` 家族鍵,
            // 中文:   leading 單注音字 (ㄉ) 走 tps:ㄉ byte-range scan,對齊主流 IME。
            if let Some(ctx) = lex_ctx.as_ref() {
                fetch_via_lexicon_partial_inner(raw, raw_len, mode, ctx)
            } else {
                Vec::new()
            }
        } else {
            // ---- Step 2b: span-local fetch.
            let mut c = if let Some(ctx) = lex_ctx.as_ref() {
                fetch_via_lexicon_inner(&keys, raw_len, ctx)
            } else {
                Vec::new()
            };
            // ---- Step 3: per-candidate recase loop.
            // v3.5.8 (Codex pre-impl 2A locus = candidate
            // construction): case each span-local candidate's roman
            // to mirror the user's raw input for its consumed span,
            // so the displayed candidate already shows `Hit` /
            // `tui` and tap commits it verbatim (display == commit).
            // The legacy platform `SuggestionCaseTransformer` is
            // bypassed for continuous, so this is the single casing
            // source. Only the presentation `roman` is touched —
            // `display_text` (canonical freq/NextWord key) and
            // `hanji` are deliberately left intact.
            // 中文: 連續候選 roman 依該段 raw 還原大小寫(display==commit);
            // 中文:   平台 SuggestionCaseTransformer 對連續 bypass,此為唯一源;
            // 中文:   只改呈現 roman,canonical display_text / hanji 不動。
            for cand in &mut c {
                let (cs, ce) = cand.consumed_span;
                if let Some(seg) = raw.get(cs as usize..ce as usize) {
                    cand.roman = recase_roman(&cand.roman, seg, mode);
                }
            }
            // ---- Step 4: walker slot-0 prepend (all modes).
            // v3.5.8 S2 — whole-sentence walker. The synthesized
            // full-buffer best path is explicitly prepended at slot 0
            // (Codex pre-impl S2 Q1 — the 8-dim `SortKey` cannot
            // guarantee slot 0 on its own). Span-aware de-dup against
            // the synth (Codex pre-impl S2 Q1d): drop any span-local
            // candidate identical on `(roman, hanji, consumed_span)` so
            // slot 0 is unique (e.g. a real left-anchored full-buffer
            // dict word equal to the walker path — keep the walker's
            // at slot 0, not a duplicate slot N).
            //
            // v3.5.9 D / C-3b — TPS now participates in the walker
            // (was excluded pre-C-3b because the lattice builder was
            // TL/POJ-only and `build_keys_tps` short-circuited the
            // shadow path). The unified `valid_span_endings_lowered`
            // dispatcher routes TPS through its own inventory-gated
            // scanner inside the same `build_shadow_lattice` /
            // `build_lattice` call, so the walker sees a real DAG and
            // can emit a slot-0 whole-sentence best path for TPS too.
            //
            // A2 D1 fold: the walker receives the pre-built
            // `(shadow, shadow_to_raw_end, lattice, inv)` plus
            // `prefix`/`dict` from the single seam `with_state`
            // scope — no second `build_shadow_lattice` per fetch.
            // A2 D3 honest type: the walker returns
            // [`WalkerSlot0`] (cost-named); convert to wire
            // `RawCandidate` with `score = -(cost as f32)` here.
            // 中文: S2 — 全句 walker。合成全 buffer 最佳路徑 explicit prepend slot 0
            // 中文:   (Codex S2 Q1);與 synth 同 (roman,hanji,span) 的 span-local 候選去掉,
            // 中文:   保 slot 0 唯一 (Codex S2 Q1d)。
            // 中文: D / C-3b — TPS 加入 walker(舊 build_keys_tps 短路造就 TPS 不接 walker;
            // 中文:   現透過統一 valid_span_endings_lowered + 共用 build_shadow_lattice 路徑,
            // 中文:   TPS 也能合成全句最佳路徑)。
            // 中文: A2 D1 fold — walker 收 seam 預建 shadow/lattice;A2 D3 honest type —
            // 中文:   walker 回 WalkerSlot0,score = -(cost as f32) 在此戳上 wire。
            {
                if let (Some((shadow, shadow_to_raw_end, lattice, inv)), Some(prefix), Some(dict)) =
                    (&shadow_lattice, prefix, dict)
                {
                    if let Some(slot0) = fetch_walker_slot0_inner(
                        raw,
                        raw_len,
                        freq_map,
                        now_ms,
                        mode,
                        custom,
                        shadow,
                        shadow_to_raw_end,
                        lattice,
                        inv,
                        prefix,
                        dict,
                    ) {
                        let slot0_cand = RawCandidate {
                            consumed_span: slot0.consumed_span,
                            syllable_count: slot0.syllable_count,
                            display_text: slot0.display_text,
                            roman: slot0.roman,
                            hanji: slot0.hanji,
                            // D3 honest conversion: wire `score`
                            // (proto field 5) = negated min-cost.
                            score: -(slot0.cost as f32),
                            form: FORM_NOTONE,
                            // walker-N/A; slot 0 explicit prepend.
                            frequency: 0,
                            bitmask: 0,
                            mode: slot0.mode,
                            recency_rank: slot0.recency_rank,
                            coverage_kind: slot0.coverage_kind,
                            is_custom: slot0.is_custom,
                        };
                        c.retain(|x| {
                            !(x.roman == slot0_cand.roman
                                && x.hanji == slot0_cand.hanji
                                && x.consumed_span == slot0_cand.consumed_span)
                        });
                        c.insert(0, slot0_cand);
                    }
                }
            }
            // ---- Step 4b: whole-input prefix-extension scan.
            //
            // Mainstream IME parity. Dict rows whose toneless FST key has
            // `raw` as a strict prefix (key extends BEYOND raw, e.g.
            // `tl:taigir` for input `taigi` → `tâi-gír`/`台語`) are not
            // reachable from the span-local + walker pair:
            // * `fetch_via_lexicon_inner` calls `lookup_exact` per
            //   left-anchored lattice edge, so it only matches keys EQUAL
            //   to a `(0, end)` toneless prefix.
            // * Walker spans `0..raw_len` only, so it cannot extend past
            //   the buffer length to surface a longer key.
            // * The `keys.is_empty()` partial-prefix branch only fires
            //   when the syllabifier produced no valid ending at all.
            //
            // Step 4b runs the same `fetch_partial_prefix_candidates`
            // machinery that branch uses against the whole-buffer key
            // built by `build_partial_prefix_key(raw, mode)`. The result
            // carries `COVERAGE_KIND_PARTIAL_PREFIX`, so `SortKey`'s
            // leading dim pins these strictly below the FULL block in
            // the merged vec — concat-without-resort preserves the
            // `[FULL...][PARTIAL...]` invariant.
            //
            // References:
            // - librime `src/rime/gear/table_translator.cc:178` →
            //   `dictionary.cc:308-309` `LookupWords(predictive=true)` →
            //   `prism_->ExpandSearch` is the default `table_translator`
            //   path; both user dict and main dict surface prefix
            //   extensions for an input already on a valid syllable
            //   boundary.
            // - khiin-rs `khiin/src/input/converter.rs:58-69`
            //   `candidates_for_splittable` keeps `elem.raw_text().len()
            //   >= query.len()` via `select_conversions_for_multiple` on
            //   prefix-trie hits.
            // - Our own `engine/lexicon/src/search.rs:124-129` normal-
            //   mode autocomplete already does `lookup_exact ++
            //   lookup_prefix` via `IndexSet` order-preserving merge.
            //   The continuous path is the outlier.
            //
            // **Trailing-hyphen gate.** When `synth_consumed_span` returns
            // `None` (shadow ends short of `raw_len` because the raw
            // buffer carries a pending hyphen — `tai-`, `tai-bak-`),
            // Step 4b must NOT emit a `(0, raw_len)`-span candidate that
            // would mis-commit the pending `-`. Mirrors the walker
            // slot-0 trailing-hyphen suppression at
            // `fetch_walker_slot0_inner`'s `synth_consumed_span?`
            // early-out, same contract.
            //
            // **Recase.** Recase is centralized in
            // [`fetch_via_lexicon_partial_inner`] so the empty-keys
            // branch and Step 4b apply the same per-candidate case
            // mapping — without that centralization the display of a
            // partial-prefix hit would case-flip as the user crosses
            // the first syllable boundary. Multi-syllable extension
            // candidates whose syllable count exceeds the raw segment
            // share the single-bucket case treatment documented at
            // [`recase_roman`] (per-syllable case intent is not
            // recoverable past the raw segment length).
            //
            // **Cross-batch dedupe.** Both fetchers run their own internal
            // `dedupe_by_roman_hanji_span` pre-sort, but that does not
            // span batches. A partial-prefix hit on the same dict row
            // already present in the FULL block (walker slot-0 recases
            // edge-by-edge in `fetch_walker_slot0_inner`; span-local
            // recases here at Step 3) normalizes to the same `(roman,
            // hanji, consumed_span)` triple and collapses.
            //
            // 中文: Step 4b — 主流 IME 對齊的 whole-buffer 前綴延伸掃描。
            // 中文:   librime predictive=true / khiin-rs candidates_for_splittable /
            // 中文:   本專案 lexicon::search 的 exact++prefix。沒這步,長於 buffer
            // 中文:   的字典詞(`tl:taigir` 對應輸入 `taigi`)永遠進不來。
            // 中文:   COVERAGE_KIND_PARTIAL_PREFIX + SortKey 首維讓延伸候選
            // 中文:   嚴格排在 FULL 之後;不需 re-sort。
            // 中文: 尾端 `-` 門檻 — synth_consumed_span None 時跳過,避免合成
            // 中文:   (0,raw_len) span 誤吃 pending `-`(對齊 walker slot-0 抑制)。
            // 中文: Recase — partial-prefix 的 consumed_span 一律 (0,raw_len),
            // 中文:   對整段 raw 還原大小寫,與 Step 3 span-local recase 一致。
            // 中文: 跨 batch 去重 — fetcher 內部去重不跨 batch;新增 FULL/PARTIAL
            // 中文:   交叉 (roman,hanji,consumed_span) 去重,把 walker slot-0 /
            // 中文:   span-local 已有的行從 partial 中剔除。
            if let (Some((shadow, shadow_to_raw_end, _, _)), Some(ctx)) =
                (&shadow_lattice, lex_ctx.as_ref())
            {
                if synth_consumed_span(shadow_to_raw_end, shadow.len(), raw_len).is_some() {
                    // v3.5.9 Codex PR #351 r3321758666 — use the un-truncated
                    // pool so the FULL-block exclude is applied BEFORE the
                    // `PARTIAL_PREFIX_OUTPUT_CAP` truncate. Otherwise, for
                    // inputs whose exact key has many homophones (e.g.
                    // `tl_notone = hong` has 30+ exact rows in production),
                    // the bounded fetcher would return 30 duplicates of the
                    // FULL block, and the post-fetch dedupe would drop all
                    // 30 — leaving zero extension candidates visible even
                    // though `hong-tshia` / `hong-thai` etc. exist in the
                    // prefix range. The un-bounded variant returns the
                    // sorted pool (≤ `PARTIAL_PREFIX_HYDRATE_CAP` after
                    // dedupe); composing-side filters out FULL duplicates,
                    // then applies `PARTIAL_PREFIX_OUTPUT_CAP` itself.
                    // 中文: Codex PR #351 r3321758666 — 走 un-truncated pool。
                    // 中文:   bounded 版會在 dedupe 之前先裁到 30,exact 同音字
                    // 中文:   ≥30 時 30 個名額全在 FULL 也有 → dedupe 全掉 → 0 個延伸。
                    // 中文:   composing 端先 exclude,再裁 OUTPUT_CAP。
                    let mut ext =
                        fetch_via_lexicon_partial_inner_unbounded(raw, raw_len, mode, ctx);
                    if !ext.is_empty() {
                        // Borrowed key shape — `c` is not mutated during the
                        // retain on `ext`, so HashSet entries can hold &str
                        // slices into `c` and avoid per-keystroke String allocs.
                        //
                        // Recase byte-identity holds for the dominant case:
                        // span-local single-edge FULL hits AND walker slot-0
                        // single-buffer paths, both at span (0, raw_len), both
                        // recased against the same raw segment as `ext`.
                        //
                        // Known divergence (Codex post-impl SHIP-WITH-FIXES
                        // 2026-05-29): walker slot-0 joins per-edge-recased
                        // syllables with a space (see [`fetch_walker_slot0_inner`]),
                        // whereas `ext` recases the full raw segment as one
                        // string. For multi-edge walker paths with mixed-case
                        // per-syllable intent like `TaIGi`, walker produces
                        // `Tâi Gí` while `ext` produces `Tâi-gí` for the same
                        // dict row — the exclude misses the duplicate. Visible
                        // only when ≥30 homophones share an exact key AND raw
                        // is mixed-case per-syllable, so deferred to a
                        // follow-up. The normal lowercase path (the v3.5.9
                        // continuous dogfood baseline) is unaffected.
                        // 中文: borrow 形式 HashSet key — `c` 在 ext.retain 期間不變;
                        // 中文:   span-local 單 edge FULL + walker slot-0 全 buffer 單 edge 都與 ext
                        // 中文:   recase 對齊。已知例外(Codex post-impl):walker 多 edge 用空格 join
                        // 中文:   而 ext 整段 recase,`TaIGi`-類 mixed-case 多音節輸入會分歧;
                        // 中文:   現實觸發條件 = ≥30 同音字 AND 逐音節 mixed-case,延後處理。
                        let existing: std::collections::HashSet<(
                            &str,
                            Option<&str>,
                            ConsumedSpan,
                        )> = c
                            .iter()
                            .map(|x| (x.roman.as_str(), x.hanji.as_deref(), x.consumed_span))
                            .collect();
                        ext.retain(|x| {
                            !existing.contains(&(
                                x.roman.as_str(),
                                x.hanji.as_deref(),
                                x.consumed_span,
                            ))
                        });
                        // Truncate AFTER exclude so the visible top-N is the
                        // best-scoring subset of the non-duplicate extensions,
                        // not the post-truncate dregs of a homophone-dominated
                        // pool. The lexicon-side sort is preserved (the
                        // un-truncated pool was already sorted by SortKey),
                        // so `truncate` keeps the global score order.
                        // 中文: 先 exclude 再裁,確保 UI top-N 是去 dup 後的最佳子集;
                        // 中文:   pool 已由 lexicon 端依 SortKey 排好,truncate 保排序。
                        ext.truncate(PARTIAL_PREFIX_OUTPUT_CAP);
                        c.extend(ext);
                    }
                }
            }
            c
        };
        // ---- Step 5: POJ presentation pass.
        // v3.5.8 — POJ-display render. The Continuous platform
        // builders are mode-agnostic by design (Item 13: "the
        // engine owns input-mode handling"); mirror the engine-side
        // NextWord POJ render (`engine/nextword/src/filter.rs`).
        // Rewrite the presentation `roman` TL→POJ (`oo`→`o͘`,
        // `nn`→`ⁿ`, …) for EVERY emitted candidate — span-local
        // (recased above), the walker slot-0 best candidate
        // inserted at index 0, and the partial-prefix branch — so
        // display AND the platform-formatted commit (both derive
        // from `roman`) are POJ. `display_text` / `hanji` (the
        // canonical commit + `user_frequency.db` key) are
        // deliberately untouched. No-op for TL / English /
        // (legacy-mapped) TPS.
        // 中文: POJ 顯示渲染。平台 Continuous builder 刻意 mode-agnostic(Item 13:engine 管 input mode);
        // 中文:   對齊引擎端 NextWord POJ render。對「每一筆」候選(span-local / walker slot-0 最佳候選 /
        // 中文:   partial-prefix)把呈現 roman TL→POJ,讓顯示與平台 commit(皆源自 roman)一致為 POJ;
        // 中文:   canonical display_text / hanji 不動。TL/English/(legacy 映射)TPS 為 no-op。
        // 中文:   render 可能讓 custom POJ 與 dict TL 兩筆變相同 → dedupe_rendered_continuous 收尾去重。
        if mode == phonetics::InputMode::Poj {
            for cand in &mut candidates {
                cand.roman = recase_tl_as_poj_display(&cand.roman);
            }
            dedupe_rendered_continuous(&mut candidates);
        }
        // TPS visual-dedupe — TPS UI hides romanization so two rows
        // identical on `(hanji, span)` but differing on `roman` are a
        // visible duplicate. Runs AFTER sort + walker slot-0 prepend +
        // POJ render (POJ render is a no-op for TPS so order with the
        // POJ branch above is irrelevant). See `dedupe_display_hanji_for_tps`
        // for the full rationale.
        // 中文: TPS 視覺去重 — TPS UI 隱藏羅馬字,同 (hanji, span) 不同 roman 為視覺重複。
        // 中文:   在 sort + walker slot-0 prepend + POJ render (TPS no-op) 之後跑。
        if mode == phonetics::InputMode::Tps {
            dedupe_display_hanji_for_tps(&mut candidates);
        }
        Ok(candidates)
    })
    .unwrap_or_default()
    // Step 6 (wrap into `ContinuousResponse` via `raw_to_proto_candidate`
    // + `with_continuous`) stays in `dispatch::handle_fetch_at_pos`.
}

#[cfg(test)]
mod tests {
    //! Unit tests for the pure helpers used by [`assemble_candidates`].
    //! Dispatch-level integration (decode round-trip, degraded paths) lives
    //! in `engine/composing/tests/dispatch_continuous.rs` because it needs
    //! the `Engine` + lexicon singletons. The S0 golden harness
    //! (`engine/composing/tests/golden_fetch_at_pos.rs`) is the
    //! wire-level behavior-neutrality gate for the A2 extraction.

    use super::*;

    // v3.5.9 D / C-3b — `build_keys_tps` + `strip_trailing_tone_digit`
    // tests retired; TPS now exercises the shared
    // `build_shadow_lattice` + `left_anchored_keys_from_lattice` path
    // (with `tps:` family prefix and Bopomofo tone-mark strip) tested
    // by `composing::shadow`'s tests plus the integration suite
    // (`engine/composing/tests/build_keys_tps.rs` covers the new
    // mode-aware TPS key emission against an inline `tps:` inventory).

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
    fn recase_tl_as_poj_display_rewrites_oo_and_nn() {
        // The reported bug: in POJ mode the continuous best candidate
        // must show `oo`→`o͘` (o + U+0358) and `nn`→`ⁿ` (U+207F),
        // not raw TL.
        assert_eq!(recase_tl_as_poj_display("oo"), "o\u{0358}");
        assert_eq!(recase_tl_as_poj_display("goo"), "go\u{0358}");
        assert_eq!(recase_tl_as_poj_display("sann"), "sa\u{207f}");
        // Whole-sentence walker path: space-joined multi-syllable roman
        // (`tl_display_to_poj_display` splits on ` `/`-` per token).
        assert_eq!(
            recase_tl_as_poj_display("goo sann"),
            "go\u{0358} sa\u{207f}"
        );
        assert_eq!(recase_tl_as_poj_display("tai-oo"), "tai-o\u{0358}");
        // Tone-marked TL → POJ: the tone sits between `o` and the
        // U+0358 dot (`kòo` 顧 → `kò͘`). Exact, not just "contains".
        assert_eq!(recase_tl_as_poj_display("kòo"), "k\u{f2}\u{0358}");
    }

    #[test]
    fn recase_tl_as_poj_display_preserves_letter_case() {
        // Lowercase (normal typing) stays lowercase.
        assert_eq!(recase_tl_as_poj_display("goo"), "go\u{0358}");
        // Sentence-start capital (the `Hittui → Hit` segment class).
        assert_eq!(recase_tl_as_poj_display("Goo"), "Go\u{0358}");
        // CapsLock — the Codex pre-impl BLOCK: `tl_display_to_poj_display`
        // only title-cases, so without the `transform_input_case`
        // restore an all-caps candidate would collapse to `Go͘`. Pin
        // the all-caps form survives.
        assert_eq!(recase_tl_as_poj_display("OO"), "O\u{0358}");
        // `nn` under CapsLock: the base letters go all-caps while the
        // POJ nasal `ⁿ` (U+207F) is preserved as-is (correct POJ — no
        // uppercase nasal hook). Without the case restore this would
        // collapse to title case `Sa\u{207f}`, the exact BLOCK
        // regression; pin the all-caps form survives.
        assert_eq!(recase_tl_as_poj_display("SANN"), "SA\u{207f}");
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
    fn dedupe_display_hanji_for_tps_collapses_same_hanji_same_span() {
        fn mk(roman: &str, hanji: Option<&str>, span: (u32, u32)) -> RawCandidate {
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
                is_custom: false,
            }
        }
        // The reported bug: TPS `ㄨㄢ` → dict has 灣/uan (tone 1) + 灣/uân
        // (tone 5) at the same `tps:ㄨㄢ` toneless key; same hanji, same
        // span, different roman. TPS UI shows hanji only ⇒ duplicate.
        let mut visible_dup = vec![mk("uan", Some("灣"), (0, 6)), mk("uân", Some("灣"), (0, 6))];
        dedupe_display_hanji_for_tps(&mut visible_dup);
        assert_eq!(visible_dup.len(), 1);
        assert_eq!(visible_dup[0].roman, "uan", "first-wins keeps top-ranked");
    }

    #[test]
    fn dedupe_display_hanji_for_tps_keeps_distinct_spans() {
        fn mk(roman: &str, hanji: Option<&str>, span: (u32, u32)) -> RawCandidate {
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
                is_custom: false,
            }
        }
        // Same hanji at different `consumed_span` is a legitimate partial
        // vs full-buffer surface — must NOT collapse (mirrors the S2
        // invariant pinned by `lexicon::dedupe_by_roman_hanji_span`).
        let mut spans = vec![mk("uan", Some("灣"), (0, 3)), mk("uan", Some("灣"), (0, 6))];
        let before = spans.len();
        dedupe_display_hanji_for_tps(&mut spans);
        assert_eq!(
            spans.len(),
            before,
            "distinct spans must all survive (partial vs full-buffer)"
        );
    }

    #[test]
    fn dedupe_display_hanji_for_tps_passes_through_none_and_empty_hanji() {
        fn mk(roman: &str, hanji: Option<&str>, span: (u32, u32)) -> RawCandidate {
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
                is_custom: false,
            }
        }
        // Hanji-absent rows render as TPS / roman in the UI and are
        // unique by roman — always survive regardless of repetition on
        // any other axis. Mirrors `ranking::dedup::remove_display_duplicates`.
        let mut roman_only = vec![
            mk("ㄉㄞ", None, (0, 3)),
            mk("ㄉㄞ", None, (0, 3)),     // duplicate roman, both kept
            mk("ㄨㄢ", Some(""), (0, 3)), // empty hanji also passes
        ];
        let before = roman_only.len();
        dedupe_display_hanji_for_tps(&mut roman_only);
        assert_eq!(roman_only.len(), before, "None / empty hanji all pass");
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
