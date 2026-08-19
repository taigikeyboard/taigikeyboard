//! TPS lookup-ambiguity families — the glyph sets one physical key can
//! mean, resolved at LOOKUP time instead of keystroke time.
//!
//! The TPS keyboard has one key for each initial/terminal dual-form pair
//! (`ㄍ` key = onset `ㄍ` or coda `ㆻ`; `ㄇ` key = onset `ㄇ` or syllabic
//! `ㆬ`). The per-keystroke auto-correct (`tps_adjust`) picks ONE form so
//! the preedit reads as standard TPS notation, but under toneless
//! continuous input the correct pick is often not locally decidable — the
//! answer depends on later keys and on the lexicon (考卷 `ㄎㄛ`+`ㄍ`+`ㄫ`:
//! the `ㄍ` folds to `ㆻ` because `khok` is a real syllable, yet the word
//! needs the onset reading). These families let the segmenter and the
//! dictionary lookup consider EVERY reading of the pressed key, so the
//! auto-correct's guess stops deciding reachability (its display role is
//! unchanged).
//!
//! **Membership is deliberately narrow** — only glyph pairs the
//! auto-correct actually rewrites (dual-form stops + nasals, rules 2/2b
//! of `knowledge/tps-auto-correct-rules.md`):
//! - NOT the palatalization pairs (`ㄗㄐ` …): palatalization before ㄧ/ㆪ
//!   is phonologically unconditional, the keystroke rule is always
//!   correct, and the whole-dictionary replay found zero readings lost
//!   to it.
//! - NOT the same-key vowel callouts (`ㆤㄝ`, `ㆮㆯ`, `ㆰㆱ`, `ㆪㆳ`):
//!   those are DIFFERENT phonemes (e/ee, ainn/aunn, am/om, inn/innn)
//!   that merely share a keycap for space reasons; treating them as
//!   interchangeable would be cross-phoneme fuzzy matching, a separate
//!   product decision (Codex pre-impl 2026-08-19 BLOCK).
//!
//! Every member is a 3-byte Bopomofo / Bopomofo-Extended glyph, so any
//! substitution is byte-length preserving — lattice spans and offset maps
//! stay valid, the same property `INVARIANT_TPS_DEFOLD_ENUMERATE` (§35)
//! has always relied on.

// 中文: TPS 查詢歧義家族 — 同一顆實體鍵可能代表的 glyph 集合,解歧延後到查詢時。
// 中文: 只收 per-keystroke auto-correct 真正會改寫的雙形(塞音+鼻音);顎化規則音韻上恆真(0 個字受害)、
// 中文:   同鍵母音 callout 是不同音位(跨音位模糊比對是另一個產品決策)→ 皆不收。
// 中文: 全部成員為 3-byte 注音 glyph → 替換不改 byte 長度,lattice span / offset map 不變(§35 同性質)。

/// Where a family member may legally sit inside a syllable. Drives the
/// positional restriction: a slot immediately before a hard syllable
/// close (separator / tone mark) may only take `Final`-role members —
/// an `Initial` reading there would need a following nucleus the user
/// has explicitly said is not coming.
// 中文: 成員的音節內合法位置。分隔符/調號前一格只許 Final 形 — Initial 讀法需要後接韻核,
// 中文:   而使用者已明示音節在此收掉。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TpsGlyphRole {
    /// Syllable onset form (`ㄍ`, `ㄇ`, …) — must be followed by a nucleus.
    Initial,
    /// Coda / syllabic / terminal form (`ㆻ`, `ㆬ`, `ㆭ`, `ㄥ`, …) — closes
    /// a syllable.
    Final,
}

/// One reading of an ambiguous key: the glyph plus its positional role.
// 中文: 歧義鍵的一個讀法 = glyph + 位置角色。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TpsFamilyMember {
    pub glyph: char,
    pub role: TpsGlyphRole,
}

const fn member(glyph: char, role: TpsGlyphRole) -> TpsFamilyMember {
    TpsFamilyMember { glyph, role }
}

use TpsGlyphRole::{Final, Initial};

/// The seven ambiguity families. Each family is the full reading set of
/// one physical key; the sub-slices below are exposed through
/// [`tps_ambiguity_family`] keyed by ANY member (auto-correct may have
/// left either form in the buffer, and a long-pressed explicit form is
/// a member too).
///
/// `ㄫ`'s two Final forms mirror [`super::tps_adjust`]'s context rule
/// (`ㄥ` = the `-ing` coda after `ㄧ`, `ㆭ` = syllabic / `-ng` coda
/// elsewhere); the pattern layer offers both and lets the FST decide,
/// so no context look-back is re-implemented here.
// 中文: 七個歧義家族(= 七顆雙形鍵的完整讀法集合);以任一成員查得整個家族。
// 中文: ㄫ 的兩個 Final 形對應 tps_adjust 的前文規則(ㄧ 後 ㄥ,否則 ㆭ)— 此層兩者都給,由 FST 裁決。
const FAMILIES: &[&[TpsFamilyMember]] = &[
    &[member('ㄅ', Initial), member('ㆴ', Final)],
    &[member('ㄉ', Initial), member('ㆵ', Final)],
    &[member('ㄍ', Initial), member('ㆻ', Final)],
    &[member('ㄏ', Initial), member('ㆷ', Final)],
    &[member('ㄇ', Initial), member('ㆬ', Final)],
    &[member('ㄋ', Initial), member('ㄣ', Final)],
    &[
        member('ㄫ', Initial),
        member('ㆭ', Final),
        member('ㄥ', Final),
    ],
];

/// The ambiguity family containing `glyph`, or `None` when the glyph is
/// unambiguous (vowels, tone marks, palatalized initials, separators —
/// everything a lookup pattern treats as fixed bytes).
// 中文: 回傳含 glyph 的歧義家族;無歧義字元(母音/調號/顎化聲母/分隔符)回 None → pattern 視為固定 bytes。
pub fn tps_ambiguity_family(glyph: char) -> Option<&'static [TpsFamilyMember]> {
    FAMILIES
        .iter()
        .copied()
        .find(|family| family.iter().any(|m| m.glyph == glyph))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_lookup_is_symmetric_across_members() {
        // Any member keys the whole family — the buffer may hold either
        // form depending on what the auto-correct / a long-press did.
        for family in FAMILIES {
            for m in family.iter() {
                assert_eq!(
                    tps_ambiguity_family(m.glyph),
                    Some(*family),
                    "{} must key its own family",
                    m.glyph
                );
            }
        }
    }

    #[test]
    fn families_cover_exactly_the_auto_correct_dual_forms() {
        // The keystroke path (`dual_final_form`) and this table must agree
        // glyph-for-glyph — one policy, two consumers.
        for (initial, previous, expected_final) in [
            ('ㄅ', 'ㄚ', 'ㆴ'),
            ('ㄉ', 'ㄚ', 'ㆵ'),
            ('ㄍ', 'ㄚ', 'ㆻ'),
            ('ㄏ', 'ㄚ', 'ㆷ'),
            ('ㄇ', 'ㄚ', 'ㆬ'),
            ('ㄋ', 'ㄚ', 'ㄣ'),
            ('ㄫ', 'ㄚ', 'ㆭ'),
            ('ㄫ', 'ㄧ', 'ㄥ'),
        ] {
            let family = tps_ambiguity_family(initial).expect("dual-form initial has a family");
            assert!(
                family.contains(&member(expected_final, Final)),
                "{initial}+{previous} keystroke fold target {expected_final} must be a Final member",
            );
        }
    }

    #[test]
    fn unambiguous_glyphs_have_no_family() {
        // Vowels, palatalized initials, tone marks, ASCII: fixed bytes.
        for c in ['ㄚ', 'ㆤ', 'ㄝ', 'ㄐ', 'ㄗ', 'ㄒ', 'ㆪ', '\u{02cb}', ' ', 't'] {
            assert_eq!(tps_ambiguity_family(c), None, "{c:?} must be unambiguous");
        }
    }

    #[test]
    fn all_members_are_three_byte_glyphs() {
        // Byte-length preservation is what keeps lattice offset maps valid.
        for family in FAMILIES {
            for m in family.iter() {
                assert_eq!(m.glyph.len_utf8(), 3, "{} must be 3 bytes", m.glyph);
            }
        }
    }

    #[test]
    fn every_family_offers_both_roles() {
        // A slot before a hard close keeps at least one legal reading
        // (the Final form), and an open position keeps the Initial one.
        for family in FAMILIES {
            assert!(family.iter().any(|m| m.role == Initial));
            assert!(family.iter().any(|m| m.role == Final));
        }
    }
}
