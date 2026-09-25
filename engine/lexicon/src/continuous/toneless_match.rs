//! Continuous toneless-key guards: does a record's reconstructed toneless face
//! (`tl_notone` / `poj_notone` / `tps_notone`) match the key it was found under.

/// v3.5.8 — continuous-input abbreviation-collision guard. Returns
/// `true` iff `record_tl` genuinely matched the queried `key` via its
/// toneless spelling (`tl_notone`), not via its acronym (`tl_abbrev`).
///
/// `dictionary/build/create_fst.py` indexes `tl:<tl_notone>`,
/// `tl:<tl_num>` AND `tl:<tl_abbrev>` under one shared `tl:` FST
/// prefix. For NORMAL IME autocomplete (`lexicon::search::search`)
/// acronym matching is intentional — typing `gi` should surface 外夷
/// (`guā-î`, `tl_abbrev == "gi"`). For whole-sentence CONTINUOUS input
/// the user types phonetic syllables, so the first-syllable key `tl:gi`
/// also returning 外夷 is letter-mismatched noise: the candidate shares
/// no spelling with what was typed.
///
/// Keep a record only when its real TL display reduces to the queried
/// toneless body. `normalize_input` yields a per-syllable numeric-tone
/// form (e.g. `gín-á-lâng` → `gin2a2lang5`), so every ASCII tone digit
/// is dropped to reach the stored fused `tl_notone` surface
/// (`remove_tone(to_numeric_tone(tl)) == tl_notone` holds for every
/// `dictionary.csv` row — verified pre-impl).
///
/// Scope: only `tl:` keys are guarded by this fn; v3.5.9 B-2 added the
/// POJ analog [`matches_continuous_poj_toneless_key`] for `poj:` keys
/// and the prefix-aware dispatcher [`matches_continuous_toneless_key`].
/// `hanzi:` keys pass through both guards untouched. A `tl:` key whose
/// body still carries an ASCII digit is a numeric-tone (`tl:<tl_num>`)
/// key, NOT a continuous toneless key, so the guard is skipped rather
/// than silently filtering a non-continuous caller ([`toneless_body`]).
fn matches_continuous_tl_toneless_key(key: &str, record_tl: &str) -> bool {
    toneless_body(key, "tl:")
        .is_none_or(|body| face_eq_with_nasal_oo_alias(&tl_toneless_face(record_tl), body))
}

/// Body of `key` when it is a toneless-surface key of `family` (`"tl:"` /
/// `"poj:"` / `"tps:"`, colon included); `None` for any other family and
/// for a tone-bearing body — an ASCII digit for TL/POJ, a Bopomofo tone
/// mark for TPS. Those are numeric-tone keys (`tl:<tl_num>` /
/// `poj:<poj_num>` / `tps:<tps_num>`) reserved for the non-continuous
/// paths, so every toneless guard passes them through unfiltered rather
/// than silently filtering a non-continuous caller. The surface
/// classification is [`KeyFace::of`]'s.
fn toneless_body<'a>(key: &'a str, family: &str) -> Option<&'a str> {
    let body = key.strip_prefix(family)?;
    matches!(
        KeyFace::of(family, body)?,
        KeyFace::TlNotone | KeyFace::PojNotone | KeyFace::TpsNotone
    )
    .then_some(body)
}

/// The record's `tl_notone` face — what `create_fst.py` indexes under
/// `tl:` — reconstructed from `record.tl`. `normalize_input` yields a
/// per-syllable numeric-tone form (`gín-á-lâng` → `gin2a2lang5`), so every
/// ASCII tone digit is dropped to reach the stored fused surface
/// (`remove_tone(to_numeric_tone(tl)) == tl_notone` holds for every
/// `dictionary.csv` row — verified pre-impl; `tests/roman_num_face_parity.rs`).
fn tl_toneless_face(record_tl: &str) -> String {
    phonetics::normalize_input(record_tl)
        .chars()
        .filter(|c| !c.is_ascii_digit())
        .collect()
}

/// `face == body`, also accepting the nasal-`oo` alias respelling of the face.
///
/// The dictionary build indexes the `o͘ⁿ` rendering of the nasal final beside
/// the canonical `onn`, so a row legitimately comes back under a key its own
/// reconstructed face does not equal — 好 `hònn` reconstructs `honn` but was
/// found under `tl:hoonn`. This is the same shape as the `er↔or` dialect
/// alias two guards down ([`matches_continuous_tps_toneless_key`] →
/// `tps_notone_or_variant`), and exists for the same reason: a build-time
/// spelling alias needs its runtime counterpart in the face reconstruction, or
/// every row it indexes is filtered back out.
///
/// Accepting the respelled face cannot admit a wrong row: `oonn` never occurs
/// in a canonical key, so the respelling is disjoint from every canonical face
/// and can only match a body the build itself emitted.
fn face_eq_with_nasal_oo_alias(face: &str, body: &str) -> bool {
    face == body || phonetics::nasal_oo_alias_spelling(face).is_some_and(|alias| alias == body)
}

/// The record's `poj_notone` face — v3.5.9 B-2 POJ counterpart of [`tl_toneless_face`].
/// `record.tl` is the only romanization the engine record carries
/// (Codex BLOCK #1 — `dict.bin` has no `poj` field), so we derive the
/// `poj_notone` surface at runtime byte-for-byte the way
/// `dictionary/build/merge_csv.py:226-240` does in production:
///   1. [`phonetics::tl_display_to_poj_display`] rewrites the TL display
///      form into POJ display (`tsiah → chiah`, `gín-á-lâng →
///      gín-á-lâng`).
///   2. Split on both `-` AND space. `record.tl` carries multi-syllable
///      records as either `gín-á-lâng` (hyphenated) or `iā sī`
///      (space-separated; 998 rows in `dictionary.csv` have spaces) —
///      Codex pre-impl BLOCK caught the hyphen-only split.
///   3. **Per token**: NFD-walk, drop the 8 POJ / TL combining tone
///      marks (`U+0300, U+0301, U+0302, U+0304, U+0306, U+030B,
///      U+030C, U+030D`), lowercase, then apply
///      [`phonetics::NORMALIZE_TO_POJ_RULES`] (`ou→oo`,
///      `o\u{0358}→oo`, `\u{207f}→nn`, `\u{1d3a}→nn`) and strip any
///      ASCII digits the normalize stage emits.
///   4. Concatenate the per-token results into the final body.
///
/// Step 3's normalize MUST run **per token before concat**, not over
/// the concatenated surface — see [`derive_poj_notone_for_match`]'s
/// own contract (`ou→oo` would mis-fire across hyphen boundaries on
/// `tó-uī → toui`; concat-then-normalize would drift to `tooi`).
///
/// Encoding-only — no phonotactic gating — because that is what the
/// build pipeline does. A phonotactic gate (Codex post-impl SHOULD #1)
/// would silently reject ~10 legitimate dictionary rows whose TL has
/// shapes the syllable table does not enumerate (e.g. `hehⁿ` →
/// `poj_notone=hehnn`; `tl_notone=hehⁿ`; both legitimately indexed in
/// the shipped `dictionary.fst`). The non-golden parity test
/// `engine/lexicon/tests/poj_notone_parity.rs` pins this against
/// every row of `dictionary/output/dictionary.csv`.
///
fn poj_toneless_face(record_tl: &str) -> String {
    derive_poj_notone_for_match(&phonetics::api::tl_display_to_poj_display(record_tl))
}

/// v3.5.9 B-2 — POJ analog of [`matches_continuous_tl_toneless_key`]:
/// `record_tl`'s [`poj_toneless_face`] equals the `poj:` body. Scope
/// mirrors the TL guard: a `poj:` key whose body still carries an ASCII
/// digit is treated as a numeric-tone key and passed through.
fn matches_continuous_poj_toneless_key(key: &str, record_tl: &str) -> bool {
    toneless_body(key, "poj:")
        .is_none_or(|body| face_eq_with_nasal_oo_alias(&poj_toneless_face(record_tl), body))
}

/// v3.5.9 B-2 — encoding-only POJ-notone derivation; helper for
/// [`matches_continuous_poj_toneless_key`]. Per-token: NFD-walk, drop
/// the 8 combining tone marks, lowercase, apply
/// [`phonetics::NORMALIZE_TO_POJ_RULES`], strip ASCII digits — then
/// concatenate tokens. Pure / deterministic — testable directly.
///
/// Per-token application is load-bearing: the `ou → oo` alias must not
/// fire across a hyphen boundary. For a 2-syllable record like
/// `tó-uī`, hand-concatenation would form `toui` and fire `ou → oo`,
/// drifting from the build pipeline's `to_numeric_tone +
/// remove_tone(remove_hyphens)` output `toui`. Per-token, `tó` → `to`,
/// `uī` → `ui`, concat `toui` (no `ou` formed). The non-golden parity
/// test `engine/lexicon/tests/poj_notone_parity.rs` pins this against
/// every row of `dictionary/output/dictionary.csv`.
fn derive_poj_notone_for_match(poj_display: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    let mut out = String::with_capacity(poj_display.len());
    for token in poj_display.split(['-', ' ']) {
        if token.is_empty() {
            continue;
        }
        let mut token_buf = String::with_capacity(token.len());
        for ch in token.nfd() {
            if phonetics::is_combining_tone_mark(ch) {
                continue;
            }
            for lower_ch in ch.to_lowercase() {
                token_buf.push(lower_ch);
            }
        }
        for c in phonetics::normalize_to_poj(&token_buf).chars() {
            if !c.is_ascii_digit() {
                out.push(c);
            }
        }
    }
    out
}

/// v3.5.9 D / C-3b — TPS analog of [`matches_continuous_tl_toneless_key`] /
/// [`matches_continuous_poj_toneless_key`]. The continuous walker emits
/// `tps:<bopomofo_toneless>` keys (Bopomofo tone marks stripped via
/// `composing::shadow::strip_tones_for_mode` on the shadow slice); the
/// FST may return a record indexed under `tps_abbrev` (per-syllable first
/// chars) that happens to share the same key body. Reject those: the
/// continuous user typed phonetic syllables, not an abbreviation.
///
/// **Variant acceptance (C-3a er↔or dual emit)**: C-3a's build pipeline
/// emits BOTH `tps:<tps_notone>` (primary, with ㄜ for `er`/`or`) AND
/// `tps:<tps_notone_var>` (ㄛ for `or`) per row whose primary contains
/// ㄜ. The guard accepts either form so a user typing the ㄛ variant
/// (e.g. `ㄉㄛ` for TL `tor`/`tór`) does not get rejected as an
/// abbrev-collision.
///
/// Derivation: [`phonetics::tps_notone_from_tl`] mirrors the build
/// pipeline `merge_csv.py:265 tps_notone = remove_tps_tone(tps_num)`
/// (per-token TL→TPS via [`phonetics::tps::to_zhuyin`] with the Node
/// bridge default `or_maps_to_er = true`, then drop 8 Bopomofo tone
/// marks + hyphen + whitespace). Variant form derived via
/// [`phonetics::tps_notone_or_variant`] (ㄜ→ㄛ substitution; matches
/// `dictionary/common/notone.py::apply_or_dialect_variant`).
///
/// A `tps:` key whose body still carries a TPS tone mark is a
/// numeric-tone (`tps:<tps_num>`) key, not the toneless continuous one,
/// so the guard passes through (mirrors TL/POJ guards' digit-in-body
/// bypass).
fn matches_continuous_tps_toneless_key(key: &str, record_tl: &str) -> bool {
    toneless_body(key, "tps:").is_none_or(|body| tps_face_eq(&tps_toneless_faces(record_tl), body))
}

/// The record's `tps_notone` face plus its C-3a or→er dialect variant
/// (`None` when the primary has no ㄜ to substitute) — the two TPS
/// toneless keys the build emits per row.
fn tps_toneless_faces(record_tl: &str) -> (String, Option<String>) {
    with_tps_or_variant(phonetics::tps_notone_from_tl(record_tl))
}

/// Pair a TPS face with its C-3a or→er dialect variant
/// ([`phonetics::tps_notone_or_variant`]; matches
/// `dictionary/common/notone.py::apply_or_dialect_variant`).
pub(super) fn with_tps_or_variant(primary: String) -> (String, Option<String>) {
    let variant = phonetics::tps_notone_or_variant(&primary);
    (primary, (!variant.is_empty()).then_some(variant))
}

/// `body` IS one of the two faces.
fn tps_face_eq((primary, variant): &(String, Option<String>), body: &str) -> bool {
    primary == body || variant.as_deref() == Some(body)
}

/// One of the two faces starts with `body`.
pub(super) fn tps_face_starts_with(
    (primary, variant): &(String, Option<String>),
    body: &str,
) -> bool {
    primary.starts_with(body) || variant.as_deref().is_some_and(|v| v.starts_with(body))
}

/// Select the toneless-key guard by FST key family. Production span-local
/// and walker paths both route through here so a `poj:` key cannot hit the
/// TL guard (which would always reject a POJ body) or vice versa; `tps:`
/// routes to [`matches_continuous_tps_toneless_key`]. `hanzi:` and any
/// unknown prefix pass through (`matches_continuous_tl_toneless_key`
/// returns `true` for keys lacking the `tl:` prefix).
pub(super) fn matches_continuous_toneless_key(key: &str, record_tl: &str) -> bool {
    if key.starts_with("poj:") {
        matches_continuous_poj_toneless_key(key, record_tl)
    } else if key.starts_with("tps:") {
        matches_continuous_tps_toneless_key(key, record_tl)
    } else {
        matches_continuous_tl_toneless_key(key, record_tl)
    }
}

/// Prefix-aware analog of [`matches_continuous_toneless_key`] for the
/// partial-prefix path. The span-local + walker filters check for exact
/// equality (`reconstructed_toneless == body`) because their key body IS
/// the full toneless. The partial-prefix path's key body is a **strict
/// prefix** of the toneless — `fetch_partial_prefix_candidates` hydrates
/// every rowid whose FST key starts with that body, which includes both
/// (a) genuine phonetic prefix-extension hits where the rowid's
/// `tl_notone` starts with the body and (b) acronym collisions where
/// the rowid's `tl_abbrev` starts with the body. The Codex bot
/// (PR #351 r3319500948) caught the missing filter: a continuous typist
/// at `tl:taigi` should see `tâi-gí`/`tâi-gír` extensions but NOT a
/// rowid whose `tl_abbrev` happens to start with `taigi`.
///
/// The variant returns `true` when `reconstructed_toneless.starts_with(
/// body)`. Reuses the per-family face reconstruction ([`tl_toneless_face`] /
/// [`poj_toneless_face`] / [`tps_toneless_faces`]) and [`toneless_body`] so
/// the reconstruction path is byte-identical to the equality guards. The
/// digit-in-body pass-through stays identical too — numeric-tone keys
/// are reserved for non-continuous code paths.
///
/// Sibling of [`matches_continuous_toneless_key`]; the two share the
/// reconstruction code and only differ in `==` vs `starts_with`.
pub(super) fn matches_continuous_toneless_prefix_key(key: &str, record_tl: &str) -> bool {
    if key.starts_with("poj:") {
        matches_continuous_poj_toneless_prefix_key(key, record_tl)
    } else if key.starts_with("tps:") {
        matches_continuous_tps_toneless_prefix_key(key, record_tl)
    } else {
        matches_continuous_tl_toneless_prefix_key(key, record_tl)
    }
}

fn matches_continuous_tl_toneless_prefix_key(key: &str, record_tl: &str) -> bool {
    toneless_body(key, "tl:")
        .is_none_or(|body| starts_with_face_or_nasal_oo_alias(&tl_toneless_face(record_tl), body))
}

/// `face.starts_with(body)`, also accepting the nasal-`oo` alias respelling of
/// the face. The dictionary build indexes the `o͘ⁿ` rendering of the nasal
/// final beside the canonical `onn`, so a row legitimately comes back under a
/// key its own reconstructed face does not start with — 好 `hònn` reconstructs
/// `honn` but was found under `tl:hoonn`. Exactly the shape the `er↔or`
/// dialect alias already needs one guard down
/// (`matches_continuous_tps_toneless_prefix_key` → `tps_notone_or_variant`).
///
/// Admitting the respelled face cannot admit a wrong row: `oonn` never occurs
/// in a canonical key, so the respelling is disjoint from every canonical face
/// and only ever matches a body the build itself emitted.
fn starts_with_face_or_nasal_oo_alias(face: &str, body: &str) -> bool {
    if face.starts_with(body) {
        return true;
    }
    // The alias arm requires the TYPED body to carry the alias spelling
    // itself. A respelled face is a strict superstring of the canonical one,
    // so without this gate every prefix of it would match too: `tl:hoo`
    // (予/戶/雨, one of the most common buffers there is) would start matching
    // 好/否/呼/齁 because their respelled face is `hoonn`. That is not a
    // spelling the user has chosen yet, and the extension pool is capped, so
    // the extra rows displace real ones (護欄 / 虎貓 / 好學 fell off `hoo`).
    // Once `oonn` is actually typed the choice is unambiguous.
    body.contains(phonetics::NASAL_OO_ALIAS_SPELLING)
        && phonetics::nasal_oo_alias_spelling(face).is_some_and(|alias| alias.starts_with(body))
}

fn matches_continuous_poj_toneless_prefix_key(key: &str, record_tl: &str) -> bool {
    toneless_body(key, "poj:")
        .is_none_or(|body| starts_with_face_or_nasal_oo_alias(&poj_toneless_face(record_tl), body))
}

fn matches_continuous_tps_toneless_prefix_key(key: &str, record_tl: &str) -> bool {
    toneless_body(key, "tps:")
        .is_none_or(|body| tps_face_starts_with(&tps_toneless_faces(record_tl), body))
}

/// Which stored key surface a hydrated hit's body belongs to.
///
/// `create_fst.py` emits two romanization faces per family — the numeric-tone
/// column (`tl_num` / `poj_num`, where the digits double as syllable
/// separators) and the fused toneless one, plus their TPS equivalents — and
/// they are different coordinate systems: a toneless head measured against a
/// toned body is short by one digit per syllable. Naming the classification
/// keeps that decision in one place instead of re-deriving
/// `any(is_ascii_digit)` at each site.
///
/// The sibling guards in this file read the same two predicates but act on them
/// the OPPOSITE way: `matches_continuous_toneless_prefix_key` and friends
/// fail-open on a tone-bearing body (numeric-tone keys are reserved for the
/// non-continuous paths), while [`SyllableReach`](super::SyllableReach) must measure on the toned
/// face or `tai5` mis-drops. Deliberate divergence, not drift.
///
/// `None` for a family with no romanization face at all (`hanzi:`), and for any
/// family added later — the caller fails open rather than guessing a face.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum KeyFace {
    TlNum,
    TlNotone,
    PojNum,
    PojNotone,
    TpsNum,
    TpsNotone,
}

impl KeyFace {
    pub(super) fn of(family: &str, body: &str) -> Option<Self> {
        let numeric_tone = body.bytes().any(|b| b.is_ascii_digit());
        match family {
            "tl:" => Some(if numeric_tone {
                KeyFace::TlNum
            } else {
                KeyFace::TlNotone
            }),
            "poj:" => Some(if numeric_tone {
                KeyFace::PojNum
            } else {
                KeyFace::PojNotone
            }),
            "tps:" => Some(if body.chars().any(phonetics::is_tps_tone_mark) {
                KeyFace::TpsNum
            } else {
                KeyFace::TpsNotone
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod abbrev_collision_guard_tests {
    //! v3.5.8 — `matches_continuous_tl_toneless_key`: continuous input
    //! must reject `tl_abbrev` acronym collisions that share the `tl:`
    //! FST namespace with a different word's real toneless key. The
    //! motivating bug: typing `ginalangtsiahpngbesai` surfaced 外夷
    //! (`guā-î`, `tl_abbrev == "gi"`) because the first-syllable key
    //! `tl:gi` also indexes the acronym.
    use super::matches_continuous_tl_toneless_key as guard;

    #[test]
    fn genuine_single_syllable_toneless_match_is_kept() {
        // 語 `gí` → normalize "gi2" → toneless "gi" == key body.
        assert!(guard("tl:gi", "gí"));
    }

    #[test]
    fn abbrev_collision_is_rejected() {
        // 外夷 `guā-î` (`tl_abbrev == "gi"`) → toneless "guai" != "gi".
        assert!(!guard("tl:gi", "guā-î"));
        assert!(!guard("tl:gi", "guā-i")); // 外衣, same collision
    }

    #[test]
    fn genuine_multi_syllable_left_anchored_key_is_kept() {
        // 囡仔人 `gín-á-lâng` → toneless "ginalang" == key body.
        assert!(guard("tl:ginalang", "gín-á-lâng"));
    }

    #[test]
    fn no_diacritic_checked_syllable_is_kept() {
        // 鴨 `ah` (tone 4, no diacritic): both sides digitless "ah".
        assert!(guard("tl:ah", "ah"));
    }

    #[test]
    fn numeric_tone_key_skips_guard() {
        // Codex pre-impl BLOCK: a `tl:<tl_num>` key body carries an
        // ASCII digit and is NOT a continuous toneless key — the guard
        // must pass it through so a non-continuous caller is never
        // silently filtered.
        assert!(guard("tl:gua2", "guā-î"));
    }

    #[test]
    fn non_tl_family_passes_through() {
        // v3.5.9 B-2 — the TL guard itself still passes through `poj:`
        // and `hanzi:` keys; the dispatcher
        // `matches_continuous_toneless_key` is what actually routes
        // `poj:` keys to the dedicated POJ guard. Tested in
        // `poj_abbrev_collision_guard_tests` + `dispatcher_tests` below.
        assert!(guard("poj:goa", "goá"));
        assert!(guard("hanzi:外夷", "guā-î"));
    }
}

#[cfg(test)]
mod poj_abbrev_collision_guard_tests {
    //! v3.5.9 B-2 — `matches_continuous_poj_toneless_key`: continuous
    //! POJ input must reject `poj_abbrev` acronym collisions that
    //! share the `poj:` FST namespace with a different word's real
    //! POJ toneless key. Mirrors `abbrev_collision_guard_tests` for
    //! the TL family.
    use super::matches_continuous_poj_toneless_key as guard;

    #[test]
    fn genuine_single_syllable_poj_toneless_match_is_kept() {
        // 語 TL `gí` → POJ display `gí` → poj_notone "gi" == key body.
        assert!(guard("poj:gi", "gí"));
    }

    #[test]
    fn poj_abbrev_collision_is_rejected() {
        // 外夷 TL `guā-î` → POJ display `goā-î` → poj_notone "goai"
        // != key body "gi". Fail-closed drops the acronym hit.
        assert!(!guard("poj:gi", "guā-î"));
    }

    #[test]
    fn genuine_multi_syllable_hyphen_record_is_kept() {
        // 囡仔人 TL `gín-á-lâng` → POJ display `gín-á-lâng` → split
        // on `-` → per-syllable canonicalize → "ginalang".
        assert!(guard("poj:ginalang", "gín-á-lâng"));
    }

    #[test]
    fn genuine_space_separated_record_is_kept() {
        // 998 dictionary.csv rows carry spaces in `tl` (e.g.
        // 也是 `iā sī`). Codex pre-impl BLOCK on hyphen-only split.
        // Split on `['-', ' ']` and concat → "iasi".
        assert!(guard("poj:iasi", "iā sī"));
    }

    #[test]
    fn diverged_tl_vs_poj_form_matches_poj() {
        // 食 TL `tsia̍h` → POJ display `chia̍h` → poj_notone "chiah".
        // Key body "chiah" matches; "tsiah" would NOT (and that is
        // exactly the B-2 family separation we are testing).
        assert!(guard("poj:chiah", "tsia̍h"));
        assert!(!guard("poj:tsiah", "tsia̍h"));
    }

    #[test]
    fn numeric_tone_key_skips_guard() {
        // A `poj:<poj_num>` key body carries an ASCII digit and is NOT
        // a continuous toneless key — the guard passes it through so a
        // non-continuous caller is never silently filtered (parity with
        // the TL guard's numeric-tone skip).
        assert!(guard("poj:goa2", "guā-î"));
    }

    #[test]
    fn malformed_record_does_not_falsely_match() {
        // v3.5.9 B-2 — Codex post-impl SHOULD #1 switched the runtime
        // derive to encoding-only (matches the build pipeline's
        // `to_numeric_tone + remove_tone` path; no phonotactic gating).
        // A malformed record like `xyz` flows through as itself and
        // the comparison against the key body settles the match — no
        // gate-driven false positives.
        assert!(!guard("poj:goa", "xyz"));
        // Identity case: `xyz` against `poj:xyz` IS a match in the
        // encoding-only scheme. Pinning this so a future tightening
        // (e.g. re-introducing a syllable-table gate) does not slip in
        // silently without an updated parity story.
        assert!(guard("poj:xyz", "xyz"));
    }

    #[test]
    fn poj_diacritic_record_with_non_syllabified_nn_match_is_kept() {
        // v3.5.9 B-2 — Codex post-impl SHOULD #1 motivating row:
        // 嚇 / 拀 / 煞 / 省 all have `tl=hehⁿ`-style display whose
        // `poj_notone` shipped in `dictionary.fst` is `hehnn` (POJ
        // encoding-only). Pre-fix the strict gate rejected these.
        assert!(guard("poj:hehnn", "hehⁿ"));
        assert!(guard("poj:sahnn", "sahⁿ"));
    }

    #[test]
    fn non_poj_family_passes_through() {
        // `tl:` / `hanzi:` keys are not POJ continuous toneless keys.
        assert!(guard("tl:gua", "guá"));
        assert!(guard("hanzi:外夷", "guā-î"));
    }

    // v3.5.9 B-2 (Codex post-impl SHOULD #2) — extra delimiter / form
    // edge cases derived from real `dictionary.csv` rows. The
    // implementation splits on both `-` and ` ` and skips empty tokens,
    // so these are the fragile shapes most likely to break a future
    // refactor.

    #[test]
    fn mixed_space_and_hyphen_record() {
        // `bô iàu-kín` (`dictionary.csv:5775`): TL display has a space
        // between the first syllable and the hyphenated tail. Split on
        // both delimiters → ["bô", "iàu", "kín"] → poj_notone "boiaukin".
        // Pin: `bô` POJ-display is `bô`, `iàu` is `iàu`, `kín` is `kín`,
        // canonicalize each: "bo", "iau", "kin" → "boiaukin".
        assert!(guard("poj:boiaukin", "bô iàu-kín"));
    }

    #[test]
    fn leading_hyphen_record_skips_empty_token() {
        // `-tiong-tàu` (`dictionary.csv:95675`): leading `-` produces an
        // empty token after split; guard must skip empty tokens (not
        // fail-closed on them) and concat the rest → "tiongtau".
        assert!(guard("poj:tiongtau", "-tiong-tàu"));
    }

    #[test]
    fn trailing_hyphen_record_skips_empty_token() {
        // `thàu-tiong-tàu-` (`dictionary.csv:141481`): trailing `-`
        // produces an empty trailing token; same skip-empty path.
        assert!(guard("poj:thautiongtau", "thàu-tiong-tàu-"));
    }
}

#[cfg(test)]
mod dispatcher_tests {
    //! v3.5.9 B-2 — `matches_continuous_toneless_key` dispatcher: routes
    //! `poj:` keys to the POJ guard and everything else (`tl:`, unknown
    //! prefixes) to the TL guard.
    use super::matches_continuous_toneless_key as dispatch;

    #[test]
    fn poj_key_routes_to_poj_guard() {
        // POJ-notone match is kept under POJ routing (TL guard would
        // wrongly accept any `poj:` key because of its early-return).
        assert!(dispatch("poj:chiah", "tsia̍h"));
        // POJ-acronym mismatch is rejected.
        assert!(!dispatch("poj:gi", "guā-î"));
    }

    #[test]
    fn tl_key_routes_to_tl_guard() {
        assert!(dispatch("tl:gi", "gí"));
        assert!(!dispatch("tl:gi", "guā-î"));
    }

    #[test]
    fn hanzi_key_passes_through() {
        // Falls into the TL branch which itself passes through any key
        // lacking the `tl:` prefix → guard returns `true`.
        assert!(dispatch("hanzi:外夷", "guā-î"));
    }

    #[test]
    fn dispatcher_decides_by_prefix_not_content() {
        // Same body ("chiah"), different prefix → different guard fires.
        // `poj:chiah` against TL `tsia̍h` → POJ guard accepts (poj_notone
        // matches). `tl:chiah` against TL `tsia̍h` → TL guard rejects
        // (`normalize_input(tsia̍h) → tsiah` ≠ "chiah").
        assert!(dispatch("poj:chiah", "tsia̍h"));
        assert!(!dispatch("tl:chiah", "tsia̍h"));
    }
}

#[cfg(test)]
mod nasal_oo_alias_face_tests {
    use super::{matches_continuous_toneless_key, matches_continuous_toneless_prefix_key};

    // The dictionary build indexes the `o͘ⁿ` rendering of the nasal final
    // beside the canonical `onn`, so a row comes back under a key its own
    // reconstructed face does not equal. Without the alias arm in the face
    // guards every such row is hydrated and then filtered straight back out —
    // the failure mode that made the build-time keys look like no-ops.
    // trace: 好 hònn → normalize_input → `honn3` → digits dropped → `honn`;
    //        respelled `hoonn` == the matched body.
    #[test]
    fn equality_guard_admits_the_alias_key() {
        assert!(matches_continuous_toneless_key("tl:honn", "hònn"));
        assert!(matches_continuous_toneless_key("tl:hoonn", "hònn"));
        assert!(matches_continuous_toneless_key("poj:honn", "hònn"));
        assert!(matches_continuous_toneless_key("poj:hoonn", "hònn"));
        assert!(matches_continuous_toneless_key("tl:honnhian", "hònn-hiân"));
        assert!(matches_continuous_toneless_key("tl:hoonnhian", "hònn-hiân"));
    }

    #[test]
    fn prefix_guard_admits_the_alias_key() {
        assert!(matches_continuous_toneless_prefix_key(
            "tl:hoonnh",
            "hònn-hiân"
        ));
        assert!(matches_continuous_toneless_prefix_key(
            "poj:hoonnh",
            "hònn-hiân"
        ));
    }

    // A respelled face is a strict superstring of the canonical one, so a
    // prefix guard that consulted it unconditionally would match every prefix
    // of it too: typing `hoo` (予/戶/雨) would start pulling in 好/否/呼/齁,
    // whose respelled face is `hoonn`. Measured on production artifacts before
    // this gate: `hoo` gained 20 rows, `khoo` 18, `tshioo` 30, and because the
    // extension pool is capped the new rows displaced real ones — 護欄, 虎貓
    // and 好學 fell off `hoo` / `hoon`. The alias only applies once the user
    // has actually typed it.
    #[test]
    fn prefix_guard_does_not_leak_the_alias_into_a_canonical_prefix() {
        // `hoo` is a prefix of the respelled `hoonn`, but not of `honn`.
        assert!(!matches_continuous_toneless_prefix_key("tl:hoo", "hònn"));
        assert!(!matches_continuous_toneless_prefix_key("poj:hoo", "hònn"));
        assert!(!matches_continuous_toneless_prefix_key(
            "tl:hoon",
            "hònn-hiân"
        ));
        // Its own row is untouched: `hoo` still reaches 戶 `hōo`.
        assert!(matches_continuous_toneless_prefix_key("tl:hoo", "hōo"));
    }

    // The alias arm must not turn the guard into a pass-through: a body that
    // is neither the face nor its respelling is still rejected.
    #[test]
    fn still_rejects_an_unrelated_body() {
        assert!(!matches_continuous_toneless_key("tl:tai", "hònn"));
        assert!(!matches_continuous_toneless_key("tl:hooonn", "hònn"));
        assert!(!matches_continuous_toneless_prefix_key(
            "tl:hoonnx",
            "hònn-hiân"
        ));
    }

    // 滷卵 `lóo-nn̄g` reconstructs `loonng`, where the `onn` spans the
    // `loo`|`nng` seam and is not a nasal final at all. Its own key still
    // matches, and `lonng` — what the retired whole-buffer fold produced, and
    // the spelling that made the word unreachable — is still rejected.
    //
    // The guard reconstructs a FUSED face, so it cannot see that seam and does
    // respell it to `looonng`. That imprecision is unreachable rather than
    // wrong: the dictionary build respells per syllable, so no key carries a
    // cross-seam body, and this guard only ever runs on rows a key already
    // hydrated. Tightening it would mean a fifth per-syllable face
    // reconstruction, which `SyllableReach`'s doc explicitly rules out.
    #[test]
    fn leaves_a_cross_seam_face_alone() {
        assert!(matches_continuous_toneless_key("tl:loonng", "lóo-nn̄g"));
        assert!(!matches_continuous_toneless_key("tl:lonng", "lóo-nn̄g"));
    }
}
