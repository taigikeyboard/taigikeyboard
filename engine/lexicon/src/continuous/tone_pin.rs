//! Post-lookup tone pins: which stored readings a typed tone digit, TPS tone
//! mark, keyboard space or typed `-` boundary still admits.

use std::borrow::Cow;

use phonetics::KeyFamily;

/// Post-lookup tone constraint on the readings one continuous lookup
/// returns. The FST only has two romanization key families per record —
/// the fused toneless one and the fully-toned one
/// (`dictionary/build/create_fst.py`) — so every tone the user typed
/// that the key cannot express is re-applied HERE, after the lookup, by
/// checking each candidate's canonical reading against what was typed.
/// Built by `composing::shadow::span_key` (per key) and
/// `composing::shadow::whole_buffer_tone_pin` (whole buffer); consumed
/// through [`TonePin::admits`] on every candidate path — span-local,
/// walker slot 0, partial-prefix, custom merge — so the layers cannot
/// disagree on which readings qualify.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum TonePin {
    /// No tone constraint: a toneless TL/POJ span (the deliberate
    /// "show all tones" affordance, §17 case 2), any English span (a
    /// digit there is not a tone), or a TPS span with no tone mark and no
    /// space in it or at its end.
    #[default]
    None,
    /// §17 case 4 + §41 — a TPS span carrying a tone mark or a keyboard
    /// space: every [`TpsClose`] is a point where the user closed a
    /// syllable, so a reading must end a syllable there and carry the tone
    /// the close names. `ㄉㄞ˫ㄨㄢ` keeps 大員 `tāi-uân` and drops 台灣
    /// `tâi-uân`; `ㄌㄧˋㄫ` drops 靈 `lîng` (no syllable ends after ㄧ);
    /// `ㄒㄧ`␣ keeps 詩 si1 and drops 是 sī. `body` is the fused
    /// `tps_notone` body of the span / buffer, which the whole-buffer
    /// sources align against; the exact / walker / partial-prefix paths
    /// align against the MATCHED key instead (a §35 substituted reading
    /// reconstructs to the matched key, never the typed one).
    TpsTyped { body: String, closes: Vec<TpsClose> },
    /// §17 case 3 — a TL/POJ span carrying at least one ASCII tone digit:
    /// the hyphenless typed body with its digits (`teng5sek`) and the
    /// mode whose numeric-tone face it is read against. Every syllable
    /// the user toned must match that tone; a syllable left untoned is
    /// unconstrained, so `teng5sek` keeps 程式 `têng-sek` (5 + 4 stop
    /// coda) and drops 等式 `téng-sek` / 中式 `teng-sek`. Also set for a
    /// fully-toned span (`teng5sek4`): the verbatim toned key already
    /// filters the dictionary hits there, but custom entries are matched
    /// toneless and need the pin. §52 — `boundaries` are the byte offsets
    /// into `typed` where the user typed a `-`: a reading must end a
    /// syllable on every one, so `khi|ah` keeps 去啊 `khì--ah` and drops
    /// 隙 `khiah`. A span with a boundary but no digit is pinned too.
    /// `one_syllable_stretches` are the `(start, end)` ranges of `typed` a
    /// typed `-` closed as one inventory syllable: a reading may end no
    /// syllable strictly inside one, so `ai-` keeps 愛 `ài` and drops 阿姨
    /// `a-î`, both under the key `ai`.
    TypedTones {
        mode: phonetics::InputMode,
        typed: String,
        boundaries: Vec<TypedBoundary>,
        one_syllable_stretches: Vec<(usize, usize)>,
    },
}

/// §52 — one `-` run the user typed inside a span: where it sits in the
/// typed body, and whether it was the khinsiann `--` (two or more) rather
/// than a plain hyphen `-`. A reading must end a syllable there AND, when
/// another of its syllables follows, separate the two the same way: a
/// typed `--` keeps 去啊 `khì--ah` and drops 忍氣 `jím-khì`-shaped words;
/// a typed `-` does the reverse (a dictionary space counts as plain).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedBoundary {
    pub at: usize,
    pub khinsiann: bool,
}

/// One syllable close the user typed inside (or at the end of) a TPS span:
/// `body_end` is the byte offset into the span's TONELESS body where the
/// closed syllable ends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpsClose {
    pub body_end: usize,
    pub kind: TpsCloseKind,
}

/// What a [`TpsClose`] asks of the syllable ending there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TpsCloseKind {
    /// A typed tone mark: the syllable carries exactly this numeric tone.
    Marked(char),
    /// The keyboard space closing the span's unmarked last syllable (§41):
    /// tone 1 on an open rime, 4 on a stop coda.
    SpaceClosed,
    /// A space inside the span: a syllable boundary of any tone, never a
    /// tone-1 instruction (§31 `ㄍㄠ`␣`ㄉㄞ˪` 交代, §35 `ㄇ`␣`ㄒㄧ` 毋是).
    Boundary,
}

impl TonePin {
    /// Does `reading` (a canonical TL reading — `DictionaryRecord.tl`, or
    /// a custom roman folded through `phonetics::api::canonical_tl_form`)
    /// satisfy this pin? `matched_key` is the FST key the reading came
    /// back under on the exact / walker / partial-prefix paths (for a
    /// strict-prefix extension it runs past the typed text — only the TPS
    /// pin reads it, cut at each close); the custom and learned merges
    /// pass `None` and the pin aligns against its own carried body, since
    /// a custom entry has no key at all.
    pub fn admits(&self, matched_key: Option<&str>, reading: &str) -> bool {
        match self {
            TonePin::None => true,
            TonePin::TpsTyped { body, closes } => {
                reading_passes_tps_closes(matched_key.unwrap_or(body), closes, reading)
            }
            TonePin::TypedTones {
                mode,
                typed,
                boundaries,
                one_syllable_stretches,
            } => reading_passes_typed_tones(
                *mode,
                typed,
                boundaries,
                one_syllable_stretches,
                reading,
            ),
        }
    }

    /// [`TonePin::admits`] for a `custom_dictionary.db` entry: custom
    /// entries are synthesized whole-buffer (no matched key), and their
    /// roman may be stored in the user's native POJ form, so it is folded
    /// to canonical TL first — the same fold `custom_entry_to_candidate`
    /// applies for the identity key. TPS mode's identity fold leaves the
    /// Latin roman as stored, so there the pin reads the same
    /// [`phonetics::api::custom_search_reading`] the TPS search keys and
    /// [`custom_reach`] are derived from. The fold is skipped when nothing
    /// is pinned, the common toneless-buffer case.
    ///
    /// [`custom_reach`]: super::reach::custom_reach
    pub fn admits_custom(&self, roman: &str, mode: phonetics::InputMode) -> bool {
        if matches!(self, TonePin::None) {
            return true;
        }
        let reading = match mode {
            phonetics::InputMode::Tps => phonetics::api::custom_search_reading(roman),
            _ => phonetics::api::canonical_tl_form(roman, mode),
        };
        self.admits(None, &reading)
    }
}

/// §17 case 3 + §52: [`phonetics::typed_syllable_walk`] must consume the typed body
/// (`teng5sek` against 程式 `têng-sek`: `teng` + `5` matches, `sek` is
/// free), and every typed boundary must be a syllable end of the reading
/// with the same separator kind — `--` vs `-` / space
/// (`phonetics::tl_syllable_khinsiann_flags`; a dictionary space counts
/// as plain). The last syllable's end carries no kind: a trailing typed
/// `--` constrains the NEXT word, not this one. A boundary at 0 is the
/// run typed right before the span: only a reading that itself opens with
/// `--` (a custom `--ah`) reads it, and then the run must be `--` too.
/// No syllable of the reading may end strictly inside a one-syllable
/// stretch.
fn reading_passes_typed_tones(
    mode: phonetics::InputMode,
    typed: &str,
    boundaries: &[TypedBoundary],
    one_syllable_stretches: &[(usize, usize)],
    reading: &str,
) -> bool {
    let Some(walk) =
        phonetics::typed_syllable_walk(mode, typed, reading).filter(|w| w.typed_consumed)
    else {
        return false;
    };
    let khinsiann_before = if boundaries.is_empty() {
        Vec::new()
    } else {
        phonetics::tl_syllable_khinsiann_flags(reading)
    };
    let opens_khinsiann = khinsiann_before.first().copied().unwrap_or(false);
    boundaries.iter().all(|b| {
        if b.at == 0 {
            return !opens_khinsiann || b.khinsiann;
        }
        walk.ends.iter().enumerate().any(|(index, &at)| {
            at == b.at
                && khinsiann_before
                    .get(index + 1)
                    .is_none_or(|&k| k == b.khinsiann)
        })
    }) && !walk.ends.iter().any(|&at| {
        one_syllable_stretches
            .iter()
            .any(|&(start, end)| start < at && at < end)
    })
}

/// A3 (§41) — the two tones TPS writes with no mark: 1 on an open rime,
/// 4 on a stop coda (ㆴ/ㆵ/ㆻ/ㆷ). Pressing the keyboard's space closes a
/// syllable, and an unmarked closed syllable can only be one of these
/// two, so they are exactly the tones a space-pinned candidate may carry.
/// Tone 8 shares tone 4's coda but writes a dot, so it is excluded here —
/// that is the 一 (`tsit8`) vs 這 (`tsit4`) split the bug report hit.
fn is_unmarked_tps_tone(tone: char) -> bool {
    matches!(tone, '1' | '4')
}

/// §17 case 4 + §41 — does `reading` honour every syllable close the user
/// typed? For each [`TpsClose`] the reading must end a syllable exactly at
/// `body_end` of the toneless body and that syllable's tone must fit the
/// close's [`TpsCloseKind`] — one walk of the reading for every close
/// ([`phonetics::tps_notone_boundary_tones`]).
///
/// `tps_body` is either a full `tps:`-family FST key — the MATCHED key on
/// the exact / walker / partial-prefix dictionary paths, since §35
/// substitutions reconstruct to the matched form and the literal query key
/// would reject legitimate ambiguity-family hits — or a bare body, which is
/// the shape the whole-buffer sources (custom entries, the walker's custom
/// override) carry. Its tone marks are stripped before aligning (a
/// fully-toned span matched the verbatim toned key; every §35 substitution
/// is byte-length preserving), and it may run past the last close (a
/// partial-prefix extension's later syllables). A key in any other family
/// passes through: TL/POJ tones are ASCII digits, pinned by
/// [`TonePin::TypedTones`].
///
/// A missing boundary is a REJECT, not a pass: the user closed a syllable
/// there, so a reading that runs through that point mid-syllable is not
/// the word they typed.
///
/// `reading` is a canonical TL reading — `DictionaryRecord.tl` for a
/// dictionary hit, `CustomEntry.roman` for a custom entry.
fn reading_passes_tps_closes(tps_body: &str, closes: &[TpsClose], reading: &str) -> bool {
    let body = match tps_body.strip_prefix(KeyFamily::Tps.prefix()) {
        Some(body) => body,
        // Bare body — the whole-buffer form, TPS by construction.
        None if !tps_body.contains(':') => tps_body,
        // Another FST family (`tl:` / `poj:` / `hanzi:`) — never pinned.
        None => return true,
    };
    // A partial-tone span keys the toneless body, so most matched keys
    // carry no mark and need no copy.
    let toneless: Cow<str> = if body.chars().any(phonetics::is_tps_tone_mark) {
        Cow::Owned(
            body.chars()
                .filter(|&c| !phonetics::is_tps_tone_mark(c))
                .collect(),
        )
    } else {
        Cow::Borrowed(body)
    };
    let ends = closes.iter().map(|close| close.body_end);
    phonetics::tps_notone_boundary_tones(reading, &toneless, ends).is_some_and(|tones| {
        closes
            .iter()
            .zip(tones)
            .all(|(close, tone)| match close.kind {
                TpsCloseKind::Marked(typed) => tone == typed,
                TpsCloseKind::SpaceClosed => is_unmarked_tps_tone(tone),
                TpsCloseKind::Boundary => true,
            })
    })
}

#[cfg(test)]
mod typed_tone_pin_tests {
    use super::{reading_passes_typed_tones, TonePin, TpsClose, TpsCloseKind, TypedBoundary};
    use phonetics::InputMode::{self, Poj, Tl};

    fn passes(mode: InputMode, typed: &str, reading: &str) -> bool {
        reading_passes_typed_tones(mode, typed, &[], &[], reading)
    }

    // §17 case 3 — the reported shape. Faces via `poj_num_syllable_ends_from_tl`:
    // trace: 程式 tîng-sik → POJ têng-sek → `teng5sek4` ends [5, 9];
    //        等式 tíng-sik → `teng2sek4`; 中式 ting-sik → `teng1sek4`.
    // Typed `teng5sek`: `teng` + `5` == 5 ✓, `sek` + no digit → free.
    #[test]
    fn partial_tone_pins_the_toned_syllable_and_frees_the_rest() {
        assert!(passes(Poj, "teng5sek", "tîng-sik"));
        assert!(!passes(Poj, "teng5sek", "tíng-sik"));
        assert!(!passes(Poj, "teng5sek", "ting-sik"));
    }

    // Reverse partial (`tengsek4`): first syllable free, second pinned to 4.
    // 程式 sik4 ✓; a tone-8 second syllable is rejected.
    // trace: 色 sik → `sek4`; 熟 si̍k → `sek8`.
    #[test]
    fn untoned_leading_syllable_is_free_and_later_digit_still_pins() {
        assert!(passes(Poj, "tengsek4", "tîng-sik"));
        assert!(!passes(Poj, "tengsek4", "tîng-si̍k"));
    }

    // Fully toned: every syllable pinned (the verbatim key covers dictionary
    // hits; custom entries rely on this).
    #[test]
    fn fully_toned_pins_every_syllable() {
        assert!(passes(Tl, "ting5sik4", "tîng-sik"));
        assert!(!passes(Tl, "ting5sik8", "tîng-sik"));
        assert!(!passes(Tl, "ting2sik4", "tîng-sik"));
    }

    // Partial-prefix path: the typed text may end at a boundary or
    // mid-syllable; everything typed so far must still be honored.
    // trace: 中西 ting-se → `teng1se1`; 程世 tîng-sè → `teng5se3`.
    #[test]
    fn prefix_typed_text_ending_early_keeps_the_pins_so_far() {
        assert!(passes(Poj, "teng5se", "tîng-sè"));
        assert!(!passes(Poj, "teng5se", "ting-se"));
        assert!(passes(Poj, "teng5s", "tîng-sik"));
        assert!(!passes(Poj, "teng2s", "tîng-sik"));
        // Typed runs past a 2-syllable reading into a 3rd: 中西區 ting-se-khu.
        assert!(passes(Poj, "teng1sekhu", "ting-se-khu"));
        assert!(!passes(Poj, "teng5sekhu", "ting-se-khu"));
    }

    // Fail-closed: typed text the face cannot account for is a reject —
    // a 3-syllable typed body against a 2-syllable reading.
    // trace: 中西 ting-se → `teng1se1`, exhausted with `khu` unconsumed.
    #[test]
    fn typed_text_running_past_the_face_rejects() {
        assert!(!passes(Poj, "teng1sekhu", "ting-se"));
    }

    // Nasal-`oo` alias: the typed spelling `hoonn` reaches 好 `hònn` through
    // the alias key; the pin must still place the digit and reject a
    // wrong tone rather than fail open on the spelling difference.
    // trace: 好玄 hònn-hiân → tl_num `honn3hian5`; alias per syllable `hoonn`.
    #[test]
    fn nasal_oo_alias_spelling_still_pins_the_tone() {
        assert!(passes(Tl, "hoonn3hian", "hònn-hiân"));
        assert!(!passes(Tl, "hoonn5hian", "hònn-hiân"));
        assert!(passes(Tl, "honn3hian", "hònn-hiân"));
    }

    // Codex post-impl 2026-09-14: the typed text may end INSIDE an alias
    // spelling too. Production: `si7honn` offers 是乎 `sī-honnh`, so
    // `si7hoonn` (alias, one letter short of `hoonnh`) must as well.
    // trace: sī-honnh → tl_num `si7honnh4`; syllable 2 letters `honnh`,
    //        alias `hoonnh`; rest `hoonn` is a prefix of the alias.
    #[test]
    fn typed_text_ending_inside_an_alias_spelling_passes() {
        assert!(passes(Tl, "si7honn", "sī-honnh"));
        assert!(passes(Tl, "si7hoonn", "sī-honnh"));
        assert!(passes(Tl, "si7hoonnh", "sī-honnh"));
        assert!(!passes(Tl, "si2hoonn", "sī-honnh"));
    }

    // A syllable whose letters are not the next thing typed cannot place
    // its digit → reject (the toneless guards own spelling; this must not
    // fail open on a mismatch). `ho5onn` is `ho` + `onn`, not the alias.
    // trace: 好 hònn → `honn3`, alias `hoonn` — neither is a prefix of `ho5onn`.
    #[test]
    fn misaligned_letters_reject() {
        assert!(!passes(Tl, "ho5onn", "hònn"));
        // 和唔 hô-onn → `ho5onn1` aligns and passes.
        assert!(passes(Tl, "ho5onn", "hô-onn"));
    }

    // ---- §52 typed `-` boundaries (USER 2026-09-22) ----
    fn passes_boundaries(
        mode: InputMode,
        typed: &str,
        boundaries: &[usize],
        reading: &str,
    ) -> bool {
        let plain: Vec<(usize, bool)> = boundaries.iter().map(|&at| (at, false)).collect();
        passes_kinds(mode, typed, &plain, reading)
    }

    fn passes_kinds(
        mode: InputMode,
        typed: &str,
        boundaries: &[(usize, bool)],
        reading: &str,
    ) -> bool {
        let boundaries: Vec<TypedBoundary> = boundaries
            .iter()
            .map(|&(at, khinsiann)| TypedBoundary { at, khinsiann })
            .collect();
        reading_passes_typed_tones(mode, typed, &boundaries, &[], reading)
    }

    /// Plain `-` boundaries at the ends of `stretches`, each one syllable.
    fn passes_stretches(typed: &str, stretches: &[(usize, usize)], reading: &str) -> bool {
        let boundaries: Vec<TypedBoundary> = stretches
            .iter()
            .map(|&(_, at)| TypedBoundary {
                at,
                khinsiann: false,
            })
            .collect();
        reading_passes_typed_tones(Tl, typed, &boundaries, stretches, reading)
    }

    // trace: typed `ai-` → stretch 0..2.
    //   愛 ài → one syllable over `ai`, walk ends [2] ✓ nothing inside.
    //   阿姨 a-î → `a` + `i`, walk ends [1, 2]; 1 lies inside 0..2 ✗.
    //   `siamtioh`, stretch 0..4: 閃著 siám-tio̍h ends [4, 8] ✓;
    //   si-am-tio̍h ends [2, 4, 8]; 2 lies inside 0..4 ✗.
    #[test]
    fn a_reading_may_not_split_a_one_syllable_stretch() {
        assert!(passes_stretches("ai", &[(0, 2)], "ài"));
        assert!(!passes_stretches("ai", &[(0, 2)], "a-î"));
        assert!(passes_stretches("siamtioh", &[(0, 4)], "siám-tio̍h"));
        assert!(!passes_stretches("siamtioh", &[(0, 4)], "si-am-tio̍h"));
        assert!(passes_stretches("aiai", &[(0, 2), (2, 4)], "ài-ài"));
        assert!(!passes_stretches("aiai", &[(0, 2), (2, 4)], "ài-a-î"));
        // A boundary with no one-syllable stretch keeps the split (`taigi-`).
        assert!(passes_boundaries(Tl, "ai", &[2], "a-î"));
    }

    // The kind of the typed run must match the reading's own separator at
    // that boundary; a plain `-` and a dictionary space are the same kind.
    #[test]
    fn typed_boundary_kind_must_match_the_readings_separator() {
        assert!(passes_kinds(Tl, "khiah", &[(3, true)], "khì--ah"));
        assert!(!passes_kinds(Tl, "khiah", &[(3, false)], "khì--ah"));
        assert!(passes_kinds(Tl, "jimkhi", &[(3, false)], "jím-khì"));
        assert!(!passes_kinds(Tl, "jimkhi", &[(3, true)], "jím-khì"));
        assert!(passes_kinds(Tl, "iasi", &[(2, false)], "iā sī"));
        assert!(!passes_kinds(Tl, "iasi", &[(2, true)], "iā sī"));
        // A trailing `--` constrains the next word, not this reading.
        assert!(passes_kinds(Tl, "tai", &[(3, true)], "tâi"));
        assert!(passes_kinds(Tl, "tai", &[(3, false)], "tâi"));
        // …but a reading that continues past it must continue the same way.
        assert!(passes_kinds(Tl, "tai", &[(3, true)], "tâi--uân"));
        assert!(!passes_kinds(Tl, "tai", &[(3, true)], "tâi-uân"));
        // The run typed right before the span (`at: 0`): read only by a
        // reading that opens with `--` itself (a custom `--ah` row).
        assert!(passes_kinds(Tl, "ah", &[(0, true)], "--ah"));
        assert!(!passes_kinds(Tl, "ah", &[(0, false)], "--ah"));
        assert!(passes_kinds(Tl, "ah", &[(0, true)], "ah"));
        assert!(passes_kinds(Tl, "ah", &[(0, false)], "ah"));
    }

    // trace: typed `khiah` with `-` after `khi` → boundary 3.
    //   去啊 khì--ah → `khi3ah4` ends [4, 7]; `khi` consumed → cursor 3 ✓.
    //   隙 khiah → `khiah4` one syllable; letters `khiah` run past 3 ✗.
    //   齒仔 khí-á → `khi2a2`; `khi` ✓ then `a` leaves `h` unconsumed ✗.
    #[test]
    fn typed_boundary_must_land_on_a_syllable_end_of_the_reading() {
        assert!(passes_kinds(Tl, "khiah", &[(3, true)], "khì--ah"));
        assert!(passes_boundaries(Tl, "khiah", &[3], "khì-ah"));
        assert!(!passes_boundaries(Tl, "khiah", &[3], "khiah"));
        assert!(!passes_boundaries(Tl, "khiah", &[3], "khia̍h"));
        assert!(!passes_boundaries(Tl, "khiah", &[3], "khí-á"));
        // Same reading, no boundary typed: everything still passes.
        assert!(passes_boundaries(Tl, "khiah", &[], "khiah"));
        assert!(passes_kinds(Poj, "khiah", &[(3, true)], "khì--ah"));
    }

    // A typed digit sits before the boundary: `khi3` + `-` + `ah` → 4.
    #[test]
    fn typed_boundary_and_typed_tone_are_both_required() {
        assert!(passes_kinds(Tl, "khi3ah", &[(4, true)], "khì--ah"));
        assert!(!passes_kinds(Tl, "khi3ah", &[(4, true)], "khí--ah"));
        assert!(!passes_boundaries(Tl, "khi3ah", &[4], "khiah"));
    }

    // Trailing `-` (`tai-`): the boundary is the span end; a reading whose
    // first syllable is exactly `tai` may continue (custom / learned rows
    // are matched on the typed prefix), one that runs past it may not.
    #[test]
    fn trailing_boundary_closes_the_syllable_the_user_typed() {
        assert!(passes_boundaries(Tl, "tai", &[3], "tâi"));
        assert!(passes_boundaries(Tl, "tai", &[3], "tâi-uân"));
        assert!(!passes_boundaries(Tl, "tai", &[3], "tâin"));
    }

    // `typed` is the lowercased shadow; a custom reading keeps its capital.
    #[test]
    fn boundary_check_reads_case_blind() {
        assert!(passes_kinds(Tl, "khiah", &[(3, true)], "Khì--ah"));
        assert!(!passes_boundaries(Tl, "khiah", &[3], "Khiah"));
    }

    #[test]
    fn admits_dispatches_per_variant() {
        assert!(TonePin::None.admits(None, "tíng-sik"));
        let typed = TonePin::TypedTones {
            mode: Poj,
            typed: "teng5sek".to_owned(),
            boundaries: Vec::new(),
            one_syllable_stretches: Vec::new(),
        };
        assert!(typed.admits(Some("poj:tengsek"), "tîng-sik"));
        assert!(!typed.admits(Some("poj:tengsek"), "tíng-sik"));
        // Custom entries: folded to canonical TL first (POJ-form roman in
        // POJ mode), and never folded when nothing is pinned.
        assert!(typed.admits_custom("têng-sek", Poj));
        assert!(!typed.admits_custom("téng-sek", Poj));
        assert!(TonePin::None.admits_custom("téng-sek", Poj));
        // TPS: exact path aligns on the matched key, whole-buffer on the body.
        // trace: 詩 si → tps notone `ㄒㄧ`, tone 1 → passes; 是 sī tone 7 → rejected.
        let space = TonePin::TpsTyped {
            body: "ㄒㄧ".to_owned(),
            closes: vec![TpsClose {
                body_end: "ㄒㄧ".len(),
                kind: TpsCloseKind::SpaceClosed,
            }],
        };
        assert!(space.admits(None, "si"));
        assert!(!space.admits(None, "sī"));
        assert!(space.admits(Some("tps:ㄒㄧ"), "si"));
        assert!(!space.admits(Some("tps:ㄒㄧ"), "sī"));
    }
}

#[cfg(test)]
mod tps_close_pin_tests {
    use super::{reading_passes_tps_closes, TpsClose, TpsCloseKind};
    use TpsCloseKind::{Boundary, Marked, SpaceClosed};

    fn close(body_end: usize, kind: TpsCloseKind) -> TpsClose {
        TpsClose { body_end, kind }
    }

    // trace: `ㄉㄞ˫ㄨㄢ` → toneless `ㄉㄞㄨㄢ`, ˫ = 7 closes at `ㄉㄞ`.len() = 6.
    // 大員 tāi-uân: tai7 ends at 6 ✓; 台灣 tâi-uân: tai5 ✗; uan is free.
    #[test]
    fn marked_close_keeps_the_typed_tone_only() {
        let closes = [close(6, Marked('7'))];
        assert!(reading_passes_tps_closes("ㄉㄞㄨㄢ", &closes, "tāi-uân"));
        assert!(!reading_passes_tps_closes("ㄉㄞㄨㄢ", &closes, "tâi-uân"));
        assert!(reading_passes_tps_closes(
            "tps:ㄉㄞㄨㄢ",
            &closes,
            "tāi-uan"
        ));
    }

    // trace: `ㄌㄧˋㄫ` → toneless `ㄌㄧㄫ`, ˋ = 2 at 6; §35 matched key
    // `ㄌㄧㄥ` for 靈 lîng accumulates `ㄌㄧㄥ` (9 bytes) — no syllable ends
    // at 6 → reject; 你 lí ends at 6 with tone 2 ✓.
    #[test]
    fn marked_close_rejects_a_reading_that_runs_through_the_mark() {
        let closes = [close(6, Marked('2'))];
        assert!(!reading_passes_tps_closes("tps:ㄌㄧㄥ", &closes, "lîng"));
        assert!(!reading_passes_tps_closes("tps:ㄌㄧㄥ", &closes, "líng"));
        assert!(reading_passes_tps_closes("tps:ㄌㄧㄫ", &closes, "lí-n̂g"));
    }

    // trace: `ㄌㄧ`␣`ㄫ` — interior space at 6. 靈 lîng ✗ (no end at 6);
    // 你硬 lí-ngē ✓ whatever the tones; the space sets no tone.
    #[test]
    fn boundary_close_needs_a_syllable_end_but_no_tone() {
        let closes = [close(6, Boundary)];
        assert!(!reading_passes_tps_closes(
            "tps:ㄌㄧㄥㆤ",
            &closes,
            "lîng-ē"
        ));
        assert!(reading_passes_tps_closes("tps:ㄌㄧㄫㆤ", &closes, "lí-ngē"));
        assert!(reading_passes_tps_closes("tps:ㄌㄧㄫㆤ", &closes, "li-ngē"));
    }

    // trace: S23 `ㄇ`␣`ㄒㄧ` — §35 matches `tps:ㆬㄒㄧ` for 毋是 m̄-sī; m7's
    // notone `ㆬ` ends at 3 ✓. The bare typed body keeps `ㄇ`, which no
    // reading reconstructs — the matched key is what makes the hit align.
    #[test]
    fn boundary_close_aligns_on_the_substituted_matched_key() {
        let closes = [close(3, Boundary)];
        assert!(reading_passes_tps_closes("tps:ㆬㄒㄧ", &closes, "m̄-sī"));
        assert!(!reading_passes_tps_closes("ㄇㄒㄧ", &closes, "m̄-sī"));
    }

    // trace: §41 — tsit4 這 ✓ (unmarked stop), tsit8 一 ✗ (dotted), si1 ✓, sī ✗.
    #[test]
    fn space_closed_keeps_tone_one_and_four_only() {
        let at = |body: &str| [close(body.len(), SpaceClosed)];
        assert!(reading_passes_tps_closes("ㄐㄧㆵ", &at("ㄐㄧㆵ"), "tsit"));
        assert!(!reading_passes_tps_closes("ㄐㄧㆵ", &at("ㄐㄧㆵ"), "tsi̍t"));
        assert!(reading_passes_tps_closes("ㄒㄧ", &at("ㄒㄧ"), "si"));
        assert!(!reading_passes_tps_closes("ㄒㄧ", &at("ㄒㄧ"), "sī"));
    }

    // trace: `ㄏㄛˋㄇㄚ`␣ → ˋ 2 at 6 + space closing `ㄇㄚ` at 12.
    #[test]
    fn marked_and_space_closes_combine() {
        let closes = [close(6, Marked('2')), close(12, SpaceClosed)];
        assert!(reading_passes_tps_closes("ㄏㄛㄇㄚ", &closes, "hó-ma"));
        assert!(!reading_passes_tps_closes("ㄏㄛㄇㄚ", &closes, "hó-má"));
        assert!(!reading_passes_tps_closes("ㄏㄛㄇㄚ", &closes, "hō-ma"));
    }

    // A fully-toned span matched the verbatim toned key: marks are stripped
    // before aligning, and ˙ (tone 8) sits on the toneless offsets too.
    // trace: `ㄐㄧㆵ˙ㆤˊ` (key form U+0307) → toneless `ㄐㄧㆵㆤ`; closes
    // 9 (8) and 12 (5). 一个 tsi̍t-ê ✓; 這个 tsit-ê (tone 4) ✗.
    #[test]
    fn verbatim_toned_matched_key_aligns_on_toneless_offsets() {
        let closes = [close(9, Marked('8')), close(12, Marked('5'))];
        assert!(reading_passes_tps_closes(
            "tps:ㄐㄧㆵ\u{0307}ㆤˊ",
            &closes,
            "tsi̍t-ê"
        ));
        assert!(!reading_passes_tps_closes(
            "tps:ㄐㄧㆵ\u{0307}ㆤˊ",
            &closes,
            "tsit-ê"
        ));
        let toned = [close(6, Marked('7')), close(12, Marked('5'))];
        assert!(reading_passes_tps_closes(
            "tps:ㄉㄞ˫ㄨㄢˊ",
            &toned,
            "tāi-uân"
        ));
        assert!(!reading_passes_tps_closes(
            "tps:ㄉㄞ˫ㄨㄢˊ",
            &toned,
            "tâi-uân"
        ));
    }

    // A partial-prefix extension's matched key runs past the last close;
    // only the typed part is checked. A close past the body is a reject.
    #[test]
    fn closes_ignore_the_matched_key_past_the_last_close() {
        let closes = [close(6, Marked('7'))];
        assert!(reading_passes_tps_closes(
            "tps:ㄉㄞㄨㄢㄒㄧ",
            &closes,
            "tāi-uân-si"
        ));
        assert!(!reading_passes_tps_closes(
            "ㄉㄞ",
            &[close(9, Boundary)],
            "tāi"
        ));
        assert!(!reading_passes_tps_closes(
            "ㄉㄞ",
            &[close(4, Boundary)],
            "tāi"
        ));
    }

    #[test]
    fn other_key_families_pass_through() {
        let closes = [close(1, Marked('2'))];
        assert!(reading_passes_tps_closes("tl:tai", &closes, "tâi"));
    }
}
