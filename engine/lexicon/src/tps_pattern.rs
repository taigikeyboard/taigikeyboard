//! TPS ambiguity-aware FST pattern — one `fst::Automaton` matching every
//! reading of a typed key sequence in a single index walk.
//!
//! A TPS lookup key ("tps:ㄎㄛㆻㄫ") is built from the composing buffer,
//! whose dual-form glyphs are whatever the per-keystroke auto-correct
//! guessed. The pattern expands each such glyph to its full ambiguity
//! family (`phonetics::tps_ambiguity_family`) — `{ㄍ,ㆻ}`, `{ㄇ,ㆬ}`,
//! `{ㄫ,ㆭ,ㄥ}` — while every other byte (family prefix, vowels, tone
//! marks) stays fixed, so an intersection with the FST finds `tps:ㄎㄛㄍㆭ`
//! (考卷) without enumerating the up-to-2^k literal variants (measured
//! worst case 82,944 for a 9-syllable entry; the automaton only walks
//! branches the FST actually has).
//!
//! Positional restriction: callers pass the byte offsets of slots that sit
//! immediately before a HARD syllable close (a stripped separator / 連字
//! barrier, or a tone mark). Those slots keep only `Final`-role members —
//! an `Initial` reading there would need a following nucleus the user has
//! explicitly closed off. Everything else keeps the whole family;
//! buffer-end stays unrestricted (the user may keep typing — deferring is
//! the point).
//!
//! Wire contracts (Codex pre-impl 2026-08-19):
//! - `Exact`: `pattern` then EOF — `syllables.fst` inventory probes.
//! - `ExactWire`: `pattern || 0xFF || exactly 4 rowid bytes || EOF` —
//!   `dictionary.fst` entries (`prefix_index` wire format). The rowid is
//!   4 arbitrary bytes, never scanned for.
//! - `StartsWith`: `pattern` then anything — partial-prefix scans.
//!
//! All family members are 3-byte glyphs, so every match has the same byte
//! structure as the literal key; [`substitution_count`] is a plain
//! charwise diff.

// TPS 歧義感知 FST pattern — 單次索引走訪同時比對按鍵序列的所有讀法。
// 雙形 glyph 展開成家族({ㄍㆻ}/{ㄇㆬ}/{ㄫㆭㄥ}),其餘 byte 固定;與 FST 交集即找到替代讀法,
//   免枚舉最壞 82,944 條字串變體。分隔符/調號前一格限 Final 形;buffer 尾端不設限(延後裁決)。
// 三種 wire 契約:Exact(inventory)、ExactWire(pattern||0xFF||4 rowid bytes)、StartsWith(partial)。
// 家族成員皆 3-byte → 命中 key 與字面 key 同 byte 結構,substitution_count 為逐字 diff。

use fst::Automaton;
use phonetics::{tps_ambiguity_family, TpsGlyphRole};

/// How the pattern terminates against the stored key bytes.
// pattern 對儲存 key 的終止方式。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WireMode {
    /// Key is exactly the pattern (syllable inventory entries).
    Exact,
    /// Key is `pattern || 0xFF || rowid_le_4` (dictionary.fst wire).
    ExactWire,
    /// Key starts with the pattern (partial-prefix scans; wire suffix and
    /// any extension bytes are accepted).
    StartsWith,
}

/// One pattern slot: the alternative byte sequences this position accepts.
/// Fixed bytes are a single-alternative slot; an ambiguity family
/// contributes one alternative per (positionally legal) member.
// 一個 slot = 該位置接受的替代 byte 序列;固定 byte 為單一替代。
#[derive(Clone, Debug)]
struct Slot {
    alternatives: Vec<Vec<u8>>,
}

/// Ambiguity-aware pattern over a TPS FST key.
// TPS FST key 的歧義感知 pattern。
pub struct TpsKeyPattern {
    slots: Vec<Slot>,
    wire: WireMode,
}

/// Automaton state. `slot`/`offset` walk the pattern; `viable` is the
/// bitmask of alternatives still consistent with the bytes seen in the
/// current slot (families have ≤3 members + the literal, so u8 suffices).
// 自動機狀態 — slot/offset 走 pattern;viable 為當前 slot 仍一致的替代 bitmask。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PatternState {
    Pattern {
        slot: u16,
        offset: u8,
        viable: u8,
    },
    /// Saw the 0xFF wire separator; consuming the fixed 4 rowid bytes.
    RowId {
        consumed: u8,
    },
    /// Pattern fully matched and the wire mode accepts anything further.
    Done,
    Dead,
}

impl TpsKeyPattern {
    /// Build the pattern for `key` (e.g. "tps:ㄎㄛㆻㄫ").
    ///
    /// `final_only_offsets` are BYTE offsets into `key` of chars that sit
    /// immediately before a hard syllable close — family expansion at
    /// those offsets keeps only `Final`-role members. A tone-mark hard
    /// close is derived internally (the char before a TPS tone mark is
    /// final-only); separator barriers can only be supplied by the caller
    /// because the separator byte itself is already stripped from `key`.
    ///
    /// Non-family chars are fixed single-alternative slots, so a TL/POJ
    /// key builds a degenerate pattern equal to a plain exact lookup —
    /// but callers gate by the `tps:` prefix and never pay for that.
    // 由 key 建 pattern。final_only_offsets = 「緊鄰硬性音節收尾」char 的 byte 偏移
    //   (分隔符 barrier 由 caller 提供 — 分隔符本身已被剝除;調號前一格在此函式內部推導)。
    pub fn new(key: &str, wire: WireMode, final_only_offsets: &[usize]) -> Self {
        let chars: Vec<(usize, char)> = key.char_indices().collect();
        let mut slots: Vec<Slot> = Vec::with_capacity(chars.len());
        for (i, &(offset, c)) in chars.iter().enumerate() {
            let next_is_tone_mark = chars
                .get(i + 1)
                .is_some_and(|&(_, n)| phonetics::is_tps_tone_mark(n));
            let final_only = next_is_tone_mark || final_only_offsets.contains(&offset);
            let alternatives = match tps_ambiguity_family(c) {
                Some(family) => {
                    let mut alts: Vec<Vec<u8>> = family
                        .iter()
                        .filter(|m| !final_only || m.role == TpsGlyphRole::Final)
                        .map(|m| m.glyph.to_string().into_bytes())
                        .collect();
                    // The literal glyph must stay viable even when its role
                    // is filtered out: the user may have long-pressed the
                    // Initial form explicitly, and dropping the literal
                    // would make the pattern reject the buffer's own text.
                    let literal = c.to_string().into_bytes();
                    if !alts.contains(&literal) {
                        alts.push(literal);
                    }
                    alts
                }
                None => vec![c.to_string().into_bytes()],
            };
            slots.push(Slot { alternatives });
        }
        TpsKeyPattern { slots, wire }
    }

    fn start_state(&self) -> PatternState {
        if self.slots.is_empty() {
            return match self.wire {
                WireMode::Exact => PatternState::Done, // degenerate: empty key
                WireMode::StartsWith => PatternState::Done,
                WireMode::ExactWire => PatternState::Pattern {
                    slot: 0,
                    offset: 0,
                    viable: u8::MAX,
                },
            };
        }
        PatternState::Pattern {
            slot: 0,
            offset: 0,
            viable: u8::MAX,
        }
    }

    /// Whether a state at end-of-key is an accepting match.
    fn accepts(&self, state: &PatternState) -> bool {
        match (self.wire, state) {
            (WireMode::Exact, PatternState::Pattern { slot, offset, .. }) => {
                *slot as usize == self.slots.len() && *offset == 0
            }
            (WireMode::ExactWire, PatternState::RowId { consumed }) => *consumed == 4,
            (_, PatternState::Done) => true,
            _ => false,
        }
    }

    fn step(&self, state: &PatternState, byte: u8) -> PatternState {
        match *state {
            PatternState::Dead => PatternState::Dead,
            PatternState::Done => match self.wire {
                WireMode::StartsWith => PatternState::Done,
                _ => PatternState::Dead,
            },
            PatternState::RowId { consumed } => {
                if consumed < 4 {
                    PatternState::RowId {
                        consumed: consumed + 1,
                    }
                } else {
                    PatternState::Dead
                }
            }
            PatternState::Pattern {
                slot,
                offset,
                viable,
            } => {
                let slot_idx = slot as usize;
                if slot_idx >= self.slots.len() {
                    // Pattern exhausted; only the wire tail may continue.
                    return match self.wire {
                        WireMode::ExactWire if byte == 0xFF => PatternState::RowId { consumed: 0 },
                        WireMode::StartsWith => PatternState::Done,
                        _ => PatternState::Dead,
                    };
                }
                let alternatives = &self.slots[slot_idx].alternatives;
                let mut next_viable: u8 = 0;
                let mut advanced_done = false;
                for (alt_index, alt) in alternatives.iter().enumerate() {
                    if viable & (1 << alt_index) == 0 {
                        continue;
                    }
                    let position = offset as usize;
                    if alt.get(position) == Some(&byte) {
                        if position + 1 == alt.len() {
                            advanced_done = true;
                        } else {
                            next_viable |= 1 << alt_index;
                        }
                    }
                }
                // Alternatives within a slot share their byte length (all
                // family members are 3-byte glyphs; fixed slots have one
                // alternative), so "one alternative completed" and "others
                // still mid-way" cannot coexist.
                debug_assert!(
                    !(advanced_done && next_viable != 0),
                    "slot alternatives must share byte length"
                );
                if advanced_done {
                    PatternState::Pattern {
                        slot: slot + 1,
                        offset: 0,
                        viable: u8::MAX,
                    }
                } else if next_viable != 0 {
                    PatternState::Pattern {
                        slot,
                        offset: offset + 1,
                        viable: next_viable,
                    }
                } else {
                    PatternState::Dead
                }
            }
        }
    }
}

impl Automaton for &TpsKeyPattern {
    type State = PatternState;

    fn start(&self) -> PatternState {
        self.start_state()
    }

    fn is_match(&self, state: &PatternState) -> bool {
        self.accepts(state)
    }

    fn can_match(&self, state: &PatternState) -> bool {
        *state != PatternState::Dead
    }

    fn will_always_match(&self, state: &PatternState) -> bool {
        // Only the StartsWith wire has an absorbing accept state.
        matches!(state, PatternState::Done)
    }

    fn accept(&self, state: &PatternState, byte: u8) -> PatternState {
        self.step(state, byte)
    }
}

/// True when `text` contains at least one ambiguity-family glyph — i.e.
/// a pattern over it could match anything beyond the literal bytes.
/// Cheap pre-check that lets the hot paths (syllabifier probes, exact
/// lookups) skip building an automaton for unambiguous keys entirely.
// text 是否含任何歧義家族 glyph;無 → pattern 等同字面,熱路徑可直接跳過 automaton。
pub fn has_ambiguous_glyph(text: &str) -> bool {
    text.chars().any(|c| tps_ambiguity_family(c).is_some())
}

/// Charwise substitution count between a matched key and the literal key
/// the pattern was built from. Both have identical byte structure (family
/// members share the 3-byte glyph length), so a plain char zip suffices.
/// `0` = the user's literal text.
// 命中 key 與字面 key 的逐字替換數;byte 結構相同故 zip 即可。0 = 使用者字面。
pub fn substitution_count(literal_key: &str, matched_key: &str) -> u32 {
    literal_key
        .chars()
        .zip(matched_key.chars())
        .filter(|(a, b)| a != b)
        .count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use fst::{IntoStreamer, Set, Streamer};

    fn set_of(keys: &[&str]) -> Set<Vec<u8>> {
        let mut sorted: Vec<&str> = keys.to_vec();
        sorted.sort_unstable();
        Set::from_iter(sorted).expect("build set")
    }

    fn matches(set: &Set<Vec<u8>>, pattern: &TpsKeyPattern) -> Vec<String> {
        let mut out = Vec::new();
        let mut stream = set.search(pattern).into_stream();
        while let Some(key) = stream.next() {
            out.push(String::from_utf8_lossy(key).into_owned());
        }
        out
    }

    #[test]
    fn exact_pattern_matches_every_family_reading() {
        // 考卷: literal buffer ㄎㄛㆻㄫ must reach the stored ㄎㄛㄍㆭ.
        let set = set_of(&["tps:ㄎㄛㄍㆭ", "tps:ㄎㄛㆻㄫ", "tps:ㄎㄛㆻㆭ", "tps:ㄎㄚ"]);
        let pattern = TpsKeyPattern::new("tps:ㄎㄛㆻㄫ", WireMode::Exact, &[]);
        assert_eq!(
            matches(&set, &pattern),
            vec![
                "tps:ㄎㄛㄍㆭ".to_string(),
                "tps:ㄎㄛㆻㄫ".to_string(),
                "tps:ㄎㄛㆻㆭ".to_string(),
            ],
        );
    }

    #[test]
    fn exact_pattern_keeps_fixed_glyphs_fixed() {
        // Vowels never expand: ㄎㄚ must not match ㄎㄛ anything.
        let set = set_of(&["tps:ㄎㄛ"]);
        let pattern = TpsKeyPattern::new("tps:ㄎㄚ", WireMode::Exact, &[]);
        assert!(matches(&set, &pattern).is_empty());
    }

    #[test]
    fn final_only_offset_drops_initial_readings() {
        // ㆻ immediately before a barrier: the ㄍ (Initial) reading is
        // dropped, the coda reading and the literal survive.
        let set = set_of(&["tps:ㄎㄛㄍ", "tps:ㄎㄛㆻ"]);
        let byte_of_third_char = "tps:".len() + "ㄎㄛ".len();
        let pattern = TpsKeyPattern::new("tps:ㄎㄛㆻ", WireMode::Exact, &[byte_of_third_char]);
        assert_eq!(matches(&set, &pattern), vec!["tps:ㄎㄛㆻ".to_string()]);
    }

    #[test]
    fn tone_mark_makes_preceding_slot_final_only() {
        // ㄇ before a tone mark cannot read as the Initial ㄇ of a next
        // syllable — but ㆬ (Final) is offered, and the literal survives.
        let set = set_of(&["tps:ㆬ˫", "tps:ㄇ˫"]);
        let pattern = TpsKeyPattern::new("tps:ㄇ˫", WireMode::Exact, &[]);
        let got = matches(&set, &pattern);
        assert!(got.contains(&"tps:ㆬ˫".to_string()), "{got:?}");
        assert!(
            got.contains(&"tps:ㄇ˫".to_string()),
            "literal kept: {got:?}"
        );
    }

    #[test]
    fn exact_wire_requires_separator_and_exactly_four_rowid_bytes() {
        let mut good = b"tps:\xE3\x84\x87".to_vec(); // ㄇ
        good.push(0xFF);
        good.extend_from_slice(&7u32.to_le_bytes());
        let mut short = b"tps:\xE3\x84\x87".to_vec();
        short.push(0xFF);
        short.extend_from_slice(&[1, 2, 3]);
        let mut long = good.clone();
        long.push(9);
        let set = Set::from_iter({
            let mut v = vec![good.clone(), short, long];
            v.sort();
            v
        })
        .unwrap();
        let pattern = TpsKeyPattern::new("tps:ㄇ", WireMode::ExactWire, &[]);
        let mut stream = set.search(&pattern).into_stream();
        let mut got: Vec<Vec<u8>> = Vec::new();
        while let Some(key) = stream.next() {
            got.push(key.to_vec());
        }
        assert_eq!(got, vec![good]);
    }

    #[test]
    fn starts_with_accepts_extensions() {
        let set = set_of(&["tps:ㆬㄒㄧ", "tps:ㆬㄊㄤ", "tps:ㄇㄧㄚ", "tps:ㄎㄚ"]);
        let pattern = TpsKeyPattern::new("tps:ㄇ", WireMode::StartsWith, &[]);
        assert_eq!(
            matches(&set, &pattern),
            vec![
                "tps:ㄇㄧㄚ".to_string(),
                "tps:ㆬㄊㄤ".to_string(),
                "tps:ㆬㄒㄧ".to_string(),
            ],
        );
    }

    #[test]
    fn substitution_count_is_charwise() {
        assert_eq!(substitution_count("tps:ㄎㄛㆻㄫ", "tps:ㄎㄛㆻㄫ"), 0);
        assert_eq!(substitution_count("tps:ㄎㄛㆻㄫ", "tps:ㄎㄛㆻㆭ"), 1);
        assert_eq!(substitution_count("tps:ㄎㄛㆻㄫ", "tps:ㄎㄛㄍㆭ"), 2);
    }
}
