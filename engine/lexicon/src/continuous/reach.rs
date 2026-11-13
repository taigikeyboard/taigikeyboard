//! Syllable reach: whether typed input reaches into a record's final syllable,
//! for custom entries and partial-prefix extensions.

use super::toneless_match::{tps_face_starts_with, with_tps_or_variant, KeyFace};

/// Where a `custom_dictionary.db` entry stands against the typed whole-buffer
/// key — the dictionary's own rules applied to a word the user added
/// (product owner 2026-10-12, reversing the 2026-08-21 custom exemption: a
/// custom 陳建中 `Tân Kiàn-tiong` surfaced from `t`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CustomReach {
    /// The reading's face IS the typed body — a full-buffer word.
    Exact,
    /// A strict-prefix extension the typed body reaches into the final
    /// syllable of ([`SyllableReach`]).
    Extension,
    /// Anything else: a syllable never typed into, an acronym, or a face
    /// that does not start with the typed body at all.
    Unreached,
}

/// The whole-buffer key a custom entry is measured against: typed spaces and
/// hyphens dropped, as the store's query key drops them — a face carries
/// none, and the tone pin already holds every typed boundary to a syllable
/// end. Built once per merge, then handed to [`custom_reach`] per entry.
pub(super) fn fused_custom_typed_key(typed_key: &str) -> String {
    typed_key
        .chars()
        .filter(|c| !matches!(c, ' ' | '-'))
        .collect()
}

/// [`CustomReach`] of the custom `roman` under `fused_key`
/// ([`fused_custom_typed_key`]).
///
/// Measured on [`phonetics::api::custom_search_reading`] — the reading the
/// store's search keys were derived from, so a row the store matched is
/// judged on the same surface (not the identity fold
/// `canonical_tl_form`, which keeps a TL-literal `eng` the search keys
/// fold).
///
/// Fail-closed where [`SyllableReach::admits`] fails open: a face that does
/// not cover the typed body is exactly what the store's loose prefix query
/// lets through (an acronym, a longer spelling), so it is not offered.
pub(super) fn custom_reach(fused_key: &str, roman: &str) -> CustomReach {
    let Some(reach) = SyllableReach::new(fused_key) else {
        return CustomReach::Unreached;
    };
    let body = &fused_key[reach.family.len()..];
    let reading = phonetics::api::custom_search_reading(roman).to_lowercase();
    let Some(ends) = reach.syllable_ends(body, &reading) else {
        return CustomReach::Unreached;
    };
    if ends
        .last()
        .is_some_and(|&end| end as usize == reach.typed_body_len)
    {
        CustomReach::Exact
    } else if reach.reaches_final_syllable(&ends) {
        CustomReach::Extension
    } else {
        CustomReach::Unreached
    }
}

/// Whether one of the custom `roman`'s abbreviation search keys in
/// `abbrev_key`'s family (`tl-abbrev:tkt`) is exactly the key body — read
/// off the store's own write-side bundle
/// ([`phonetics::api::derive_custom_search_keys`], TPS `ㄜ`→`ㄛ` variant
/// included), so the engine and the store's whole-abbreviation match cannot
/// drift.
pub(super) fn custom_abbreviation_matches(abbrev_key: &str, roman: &str) -> bool {
    let Some((tag, body)) = phonetics::split_abbrev_family_key(abbrev_key) else {
        return false;
    };
    phonetics::api::derive_custom_search_keys(roman)
        .iter()
        .any(|key| key.family == tag && key.form == "abbrev" && key.key == body)
}

/// The typed input must reach INTO a record's FINAL syllable for a
/// strict-prefix extension hit to be offered as a continuous candidate.
///
/// This is the engine-wide rule "a candidate never carries more syllables than
/// the user has typed" (product owner, 2026-08-21, all three platforms).
/// Before it, `lookup_prefix` hydrated every row whose key merely STARTS with
/// what was typed, so `tsuisi` (2 syllables) surfaced 水社寮 `tsuí-siā-liâu`
/// (3) and `kesithau` (3) surfaced 家私頭仔 `ke-si-thâu-á` (4) — the whole last
/// syllable was never typed at all.
///
/// Stated per-syllable rather than as a syllable-count comparison because "how
/// many syllables did the user type" has no single answer: `aia` reads as 2
/// hops (`ai`+`a`) or 3 (`a`+`i`+`a`), and 阿姨仔 `a-î-á` — an EXACT key hit,
/// not an extension — must survive. Measuring how far the typed bytes reach
/// into the record's OWN syllable chain answers the product question directly
/// and leaves exact hits untouched (their key IS the typed body, so the head of
/// any multi-syllable reading is strictly shorter than it).
///
/// Built once per lookup from the typed key, then asked about each hydrated
/// row: the family and the typed body's length are the same for every row.
///
/// **Subsumes, deliberately does not replace, its siblings.** The prefix test
/// inside [`SyllableReach::syllable_ends`] is the same `starts_with` the three
/// `matches_continuous_*_toneless_prefix_key` guards run earlier in
/// `fetch_partial_prefix_candidates_unbounded`'s hydration loop,
/// and `phonetics::tps_notone_prefix_boundary_tone` (§41) walks the same
/// per-syllable accumulation to answer the adjacent question "does the typed
/// body land ON a boundary, and at what tone". Folding them into one walk is
/// the right end state, but it would tighten three shipped guards — they
/// fail-open on a tone-bearing body, this one measures it — so it belongs to a
/// refactor round with its own behaviour-freeze list, not here. Do not add a
/// fifth independent reconstruction.
pub(super) struct SyllableReach<'a> {
    /// `<family>:`, including the colon.
    family: &'a str,
    typed_body_len: usize,
}

impl<'a> SyllableReach<'a> {
    /// `None` for a key with no `<family>:` prefix — nothing to measure
    /// against, so the caller leaves every hit alone.
    pub(super) fn new(typed_key: &'a str) -> Option<Self> {
        let family_end = typed_key.find(':')? + 1;
        Some(Self {
            family: &typed_key[..family_end],
            typed_body_len: typed_key.len() - family_end,
        })
    }

    /// Whether `record_tl` may be offered for the hit that came back as
    /// `matched_key`.
    ///
    /// The reading is selected by `matched_key` — that is the face the row was
    /// found under, and for the §35 TPS ambiguity families it is a substituted,
    /// FULL stored key rather than the typed prefix
    /// (`lookup_prefix_shortest_first_tps_readings`). The reach is measured
    /// against the TYPED body, since substitution is charwise and cannot
    /// lengthen the typed prefix.
    ///
    /// Fail-open: a hit this cannot place on a reconstructable face keeps its
    /// pre-rule behaviour rather than being dropped on a derivation miss. Two
    /// ways that happens, both narrow: a family with no romanization face at
    /// all (`hanzi:`, or one added after this was written), and a body the
    /// reconstructed face does not cover — an acronym face, which the sibling
    /// guards in the same loop already reject on their own. Every production
    /// romanization face IS reconstructable: `tests/roman_num_face_parity.rs`
    /// and `tests/tps_notone_parity.rs` pin all five columns byte-for-byte
    /// against the shipped CSV, so a fail-open here means a real drift, not a
    /// tolerated gap.
    pub(super) fn admits(&self, matched_key: &str, record_tl: &str) -> bool {
        let Some(matched_body) = matched_key.strip_prefix(self.family) else {
            return true;
        };
        let Some(ends) = self.syllable_ends(matched_body, record_tl) else {
            return true;
        };
        self.reaches_final_syllable(&ends)
    }

    /// Whether the typed body runs past the second-to-last of `ends` — into
    /// the final syllable. A single-syllable reading is reached by any
    /// non-empty typed prefix.
    fn reaches_final_syllable(&self, ends: &[u32]) -> bool {
        match ends.len() {
            0 | 1 => true,
            count => (ends[count - 2] as usize) < self.typed_body_len,
        }
    }

    /// Where each syllable of `record_tl` ends, on the key surface
    /// `matched_body` was found under — `None` when no reconstructable surface
    /// covers it.
    ///
    /// Surface is read off the body itself, the same way the sibling guards
    /// read it: an ASCII digit means the numeric-tone family (`tl:<tl_num>` /
    /// `poj:<poj_num>`), a Bopomofo tone mark means `tps:<tps_num>`, anything
    /// else is the fused toneless family. The C-3a `er`↔`or` dialect variant is
    /// a second face of the same reading, so it is tried when the primary does
    /// not cover the body; the substitution is one Bopomofo scalar for another
    /// of the same width, so the boundaries carry over unchanged.
    fn syllable_ends(&self, matched_body: &str, record_tl: &str) -> Option<Vec<u32>> {
        match KeyFace::of(self.family, matched_body)? {
            KeyFace::TpsNum | KeyFace::TpsNotone => {
                let (primary, ends) = if matched_body.chars().any(phonetics::is_tps_tone_mark) {
                    phonetics::tps_num_syllable_ends_from_tl(record_tl)
                } else {
                    phonetics::tps_notone_syllable_ends_from_tl(record_tl)
                };
                let faces = with_tps_or_variant(primary);
                debug_assert!(
                    faces
                        .1
                        .as_ref()
                        .is_none_or(|variant| variant.len() == faces.0.len()),
                    "or-variant substitution must preserve byte offsets",
                );
                tps_face_starts_with(&faces, matched_body).then_some(ends)
            }
            face @ (KeyFace::TlNum | KeyFace::TlNotone) => {
                let num = phonetics::tl_num_syllable_ends_from_tl(record_tl);
                Self::covering(matched_body, num, face == KeyFace::TlNotone)
            }
            face @ (KeyFace::PojNum | KeyFace::PojNotone) => {
                let num = phonetics::poj_num_syllable_ends_from_tl(record_tl);
                Self::covering(matched_body, num, face == KeyFace::PojNotone)
            }
        }
    }

    /// `ends` when `face` covers `matched_body`, dropping the tone digits first
    /// when the body came from the toneless surface.
    ///
    /// The toneless columns ARE the numeric ones with the digits removed
    /// (`dictionary/common/notone.py::remove_tone`), so one derivation answers
    /// both surfaces: dropping a digit shortens that syllable and every
    /// boundary after it by one. A syllable that is nothing but its tone digit
    /// disappears entirely, and must not leave a boundary behind — that would
    /// count a syllable the toneless face does not have.
    fn covering(
        matched_body: &str,
        (num, ends): (String, Vec<u32>),
        toneless: bool,
    ) -> Option<Vec<u32>> {
        if !toneless {
            return num.starts_with(matched_body).then_some(ends);
        }
        let mut face = String::with_capacity(num.len());
        let mut face_ends = Vec::with_capacity(ends.len());
        let mut cursor = 0usize;
        for end in ends {
            let syllable = num.get(cursor..end as usize)?;
            cursor = end as usize;
            let before = face.len();
            face.extend(syllable.chars().filter(|c| !c.is_ascii_digit()));
            if face.len() != before {
                face_ends.push(face.len() as u32);
            }
        }
        face.starts_with(matched_body).then_some(face_ends)
    }
}
