//! Branch coverage for the TPS input adjuster (`phonetics::tps_input_adjust`),
//! the auto-correct the composing `TpsKey` op runs on every TPS key. The
//! fixtures were the iOS / Android `CharacterInputPipeline` tests before the
//! phones moved onto `TpsKey`.

fn tps_adjust(incoming: &str, raw: &str) -> (String, Option<String>) {
    phonetics::tps_input_adjust(incoming, raw)
}

#[test]
fn tps_adjust_dual_form_keeps_initial_on_empty_buffer() {
    for ch in ["ㄇ", "ㄋ", "ㄫ", "ㄅ", "ㄉ", "ㄍ", "ㄏ"] {
        let (adjusted, replace) = tps_adjust(ch, "");
        assert_eq!(adjusted, ch, "{ch} on empty buffer should stay initial");
        assert_eq!(replace, None);
    }
}

#[test]
fn tps_adjust_m_after_vowel_returns_final() {
    let (adjusted, replace) = tps_adjust("ㄇ", "ㄚ");
    assert_eq!(adjusted, "ㆬ");
    assert_eq!(replace, None);
}

#[test]
fn tps_adjust_ng_after_i_returns_ing() {
    let (adjusted, replace) = tps_adjust("ㄫ", "ㄧ");
    assert_eq!(adjusted, "ㄥ");
    assert_eq!(replace, None);
}

#[test]
fn tps_adjust_ng_after_other_vowel_returns_syllabic_ng() {
    let (adjusted, replace) = tps_adjust("ㄫ", "ㄚ");
    assert_eq!(adjusted, "ㆭ");
    assert_eq!(replace, None);
}

#[test]
fn tps_adjust_ainn_after_i_becomes_aunn() {
    let (adjusted, replace) = tps_adjust("ㆮ", "ㄧ");
    assert_eq!(adjusted, "ㆯ");
    assert_eq!(replace, None);
}

#[test]
fn tps_adjust_tone_after_m_replaces_with_syllabic_m() {
    let (adjusted, replace) = tps_adjust("\u{02ca}", "ㄇ");
    assert_eq!(adjusted, "\u{02ca}");
    assert_eq!(replace, Some("ㆬ".to_string()));
}

#[test]
fn tps_adjust_tone_after_ng_replaces_with_syllabic_ng() {
    let (adjusted, replace) = tps_adjust("\u{02ca}", "ㄫ");
    assert_eq!(adjusted, "\u{02ca}");
    assert_eq!(replace, Some("ㆭ".to_string()));
}

#[test]
fn tps_adjust_tone_after_other_consonant_no_replace() {
    let (adjusted, replace) = tps_adjust("\u{02ca}", "ㄉ");
    assert_eq!(adjusted, "\u{02ca}");
    assert_eq!(replace, None);
}

#[test]
fn tps_adjust_palatalization_i_after_ts_replaces() {
    let (adjusted, replace) = tps_adjust("ㄧ", "ㄗ");
    assert_eq!(adjusted, "ㄧ");
    assert_eq!(replace, Some("ㄐ".to_string()));
}

#[test]
fn tps_adjust_palatalization_inn_after_ts_replaces() {
    let (adjusted, replace) = tps_adjust("ㆪ", "ㄗ");
    assert_eq!(adjusted, "ㆪ");
    assert_eq!(replace, Some("ㄐ".to_string()));
}

#[test]
fn tps_adjust_palatalization_negative_non_trigger() {
    let (adjusted, replace) = tps_adjust("ㄚ", "ㄗ");
    assert_eq!(adjusted, "ㄚ");
    assert_eq!(replace, None);
}

#[test]
fn tps_adjust_palatalization_negative_non_affricate() {
    let (adjusted, replace) = tps_adjust("ㄧ", "ㄆ");
    assert_eq!(adjusted, "ㄧ");
    assert_eq!(replace, None);
}

#[test]
fn tps_adjust_dual_form_after_hyphen_returns_final() {
    // Locks the hyphen quirk — "-" is NOT in syllableBoundaryChars on iOS or
    // Android, so dual-form key after hyphen still gets final form. If
    // parity audit later flags this, fix as a separate parity: PR.
    let (adjusted, _) = tps_adjust("ㄇ", "ㄚ-");
    assert_eq!(adjusted, "ㆬ");
}

#[test]
fn tps_adjust_disjoint_trigger_invariant() {
    // Syllabic-nasal trigger set ({ㄇ, ㄫ}) is disjoint from palatalization
    // trigger set ({ㄗ, ㄘ, ㄙ, ㆡ}). Both effects can never fire on the
    // same call, so the `?:` short-circuit in dispatch is observationally
    // identical to running both checks.
    for last in ["ㄇ", "ㄫ"] {
        for incoming in ["ㄧ", "ㆪ"] {
            let (adjusted, replace) = tps_adjust(incoming, last);
            assert_eq!(adjusted, incoming);
            assert_eq!(
                replace, None,
                "{incoming} after {last} must not trigger either"
            );
        }
    }
}

#[test]
fn tps_adjust_reported_bug_repro_taiuan_tai() {
    // Reported 2026-05-27: TPS continuous input `ㄉㄞㄨㄢㄉㄞㆣㄧ`
    // (intended `Tâi-uân-tâi-gí` = 臺灣台語) corrupted to
    // `ㄉㄞㄨㄢㆵㄞㆣㄧ`. The second `ㄉ` lands after precomposed nasal
    // coda `ㄢ` (U+3122), which was missing from SYLLABLE_BOUNDARY_CHARS.
    let (adjusted, replace) = tps_adjust("ㄉ", "ㄉㄞㄨㄢ");
    assert_eq!(
        adjusted, "ㄉ",
        "ㄉ after precomposed nasal coda ㄢ must stay initial"
    );
    assert_eq!(replace, None);
}

#[test]
fn tps_adjust_precomposed_nasal_coda_finals_are_boundary() {
    // Every precomposed nasal-coda compound final (am/an/ang/om/ong) ends
    // the syllable for every dual-form initial. Mirrors the existing
    // syllabic-nasal coverage of `ㆬ ㄣ ㆭ ㄥ`.
    let finals = ["ㆬ", "ㄣ", "ㆭ", "ㄥ", "ㆰ", "ㄢ", "ㄤ", "ㆱ", "ㆲ"];
    let initials = ["ㄇ", "ㄋ", "ㄫ", "ㄅ", "ㄉ", "ㄍ", "ㄏ"];
    for last in finals {
        for incoming in initials {
            let raw = format!("ㄚ{last}");
            let (adjusted, _) = tps_adjust(incoming, &raw);
            assert_eq!(
                adjusted, incoming,
                "{incoming} after nasal-coda final {last} must stay initial"
            );
        }
    }
}

#[test]
fn tps_adjust_nasalized_vowel_is_boundary_for_all_initials() {
    // Nasalized vowels (ainn/aunn/ann/enn/inn/onn/unn) are complete syllables
    // and end the syllable for EVERY dual-form initial — including `ㄏ`. The
    // earlier `ㄏ`-exception (treating `<nasalized-vowel> + ㄏ` as the
    // nasalized checked final `-nnh`) was reverted because it broke common
    // continuous-input cases like `ㄏㄨㆩㄏㄧ` (歡喜 = huann-hi); see the
    // dedicated regression test `tps_adjust_continuous_input_huann_hi_keeps_h_initial`.
    // Each fixture uses a realistic raw-input prefix that produces a legal
    // standalone nasalized syllable.
    let cases: [(&str, &str); 7] = [
        ("ㆪ", "ㄒㄧㆪ"), // sinn (新)
        ("ㆥ", "ㄙㆥ"),   // senn
        ("ㆧ", "ㆧ"),     // onn standalone
        ("ㆫ", "ㄏㄧㆫ"), // hiunn
        ("ㆩ", "ㄍㄧㆩ"), // kiann (驚)
        ("ㆮ", "ㄆㆮ"),   // phainn (歹)
        ("ㆯ", "ㄎㆯ"),   // khaunn
    ];
    let all_dual_initials = ["ㄇ", "ㄋ", "ㄫ", "ㄅ", "ㄉ", "ㄍ", "ㄏ"];
    for (last, raw) in cases {
        for incoming in all_dual_initials {
            let (adjusted, _) = tps_adjust(incoming, raw);
            assert_eq!(
                adjusted, incoming,
                "{incoming} after nasalized vowel {last} (raw `{raw}`) must stay initial"
            );
        }
    }
}

#[test]
fn tps_adjust_continuous_input_huann_hi_keeps_h_initial() {
    // Regression for Codex PR #348 r3308735236 — dictionary entry `歡喜`
    // (huann-hi) has TPS key `ㄏㄨㆩㄏㄧ`. The second `ㄏ` lands after the
    // nasalized vowel `ㆩ` and MUST stay as initial; otherwise it converts
    // to `ㆷ`, producing `ㄏㄨㆩㆷㄧ` which misses the FST key.
    let (adjusted, replace) = tps_adjust("ㄏ", "ㄏㄨㆩ");
    assert_eq!(
        adjusted, "ㄏ",
        "ㄏ after nasalized vowel ㆩ in continuous input (huann-hi 歡喜) must stay initial"
    );
    assert_eq!(replace, None);
}

#[test]
fn tps_adjust_standalone_nasalized_vowel_plus_h_no_longer_synthesizes_nnh() {
    // Pin the documented trade-off (docs/phonetics/tps-auto-correct-rules.md
    // Rule 2): TPS auto-correct deliberately does NOT synthesize the
    // nasalized checked final `-nnh` from `<nasalized-vowel> + ㄏ` because
    // it cannot disambiguate from cross-syllable `<nasalized-vowel> + ㄏ-initial`
    // (e.g. 歡喜 = ㄏㄨㆩㄏㄧ). `-nnh` dictionary entries store `ㆷ`
    // directly and surface via lookup, not via input rewriting.
    for raw in ["ㆩ", "ㆥ", "ㆪ", "ㆫ", "ㆧ", "ㆮ", "ㆯ"] {
        let (adjusted, _) = tps_adjust("ㄏ", raw);
        assert_eq!(
            adjusted, "ㄏ",
            "ㄏ after standalone nasalized vowel {raw} must stay initial (no -nnh synthesis)"
        );
    }
}

#[test]
fn tps_adjust_pure_vowel_still_triggers_entering_tone_final() {
    // Regression guard: pure-vowel finals are INTENTIONALLY excluded from
    // the boundary set so single-syllable entering-tone input still works
    // (ㄍㄚ + ㄉ → ㄍㄚㆵ for `kat`). Pinning the documented examples from
    // docs/phonetics/tps-auto-correct-rules.md §Rule 2.
    let cases = [
        ("ㄅ", "ㄍㄚ", "ㆴ"),
        ("ㄉ", "ㄍㄚ", "ㆵ"),
        ("ㄍ", "ㄍㄚ", "ㆻ"),
        ("ㄏ", "ㄍㄚ", "ㆷ"),
        ("ㄇ", "ㄚ", "ㆬ"),
        ("ㄋ", "ㄚ", "ㄣ"),
    ];
    for (incoming, raw, expected) in cases {
        let (adjusted, _) = tps_adjust(incoming, raw);
        assert_eq!(
            adjusted, expected,
            "{incoming} after pure vowel `{raw}` must convert to {expected}"
        );
    }
}

#[test]
fn tps_adjust_dual_form_keeps_initial_after_a_syllable_end() {
    // A space, a tone mark, a checked final and a nasal final each end the
    // syllable, so the next dual-form key starts one.
    for raw in ["ㄚ ", "ㄚˊ", "ㄚㆴ", "ㄚㆬ"] {
        assert_eq!(
            tps_adjust("ㄇ", raw),
            ("ㄇ".to_string(), None),
            "after {raw:?}"
        );
    }
}

#[test]
fn tps_adjust_dual_form_after_vowel_returns_final() {
    // trace: initial → final pairs ㄋ→ㄣ, ㄅ→ㆴ, ㄉ→ㆵ, ㄍ→ㆻ, ㄏ→ㆷ after ㄚ.
    for (initial, coda) in [
        ("ㄋ", "ㄣ"),
        ("ㄅ", "ㆴ"),
        ("ㄉ", "ㆵ"),
        ("ㄍ", "ㆻ"),
        ("ㄏ", "ㆷ"),
    ] {
        assert_eq!(
            tps_adjust(initial, "ㄚ"),
            (coda.to_string(), None),
            "{initial}"
        );
    }
}

#[test]
fn tps_adjust_non_dual_form_passes_through() {
    for ch in ["ㄆ", "ㄊ", "ㄎ", "ㄈ", "ㄌ"] {
        assert_eq!(tps_adjust(ch, "ㄚ"), (ch.to_string(), None), "{ch}");
    }
}

#[test]
fn tps_adjust_ainn_after_other_vowel_unchanged() {
    assert_eq!(tps_adjust("ㆮ", "ㄚ"), ("ㆮ".to_string(), None));
    assert_eq!(tps_adjust("ㄚ", "ㄧ"), ("ㄚ".to_string(), None));
}

#[test]
fn tps_adjust_no_replace_without_its_trigger() {
    // Vowel after ㄇ, tone or ㄧ on an empty buffer: neither replacement fires.
    assert_eq!(tps_adjust("ㄚ", "ㄇ"), ("ㄚ".to_string(), None));
    assert_eq!(tps_adjust("\u{02ca}", ""), ("\u{02ca}".to_string(), None));
    assert_eq!(tps_adjust("ㄧ", ""), ("ㄧ".to_string(), None));
}

#[test]
fn tps_adjust_palatalization_every_affricate() {
    for (last, palatal) in [("ㄘ", "ㄑ"), ("ㄙ", "ㄒ"), ("ㆡ", "ㆢ")] {
        assert_eq!(
            tps_adjust("ㄧ", last),
            ("ㄧ".to_string(), Some(palatal.to_string())),
            "{last}"
        );
    }
}

#[test]
fn tps_adjust_dual_form_after_leading_or_post_boundary_hyphen_stays_initial() {
    for (incoming, raw) in [("ㄍ", "-"), ("ㄍ", "--"), ("ㄏ", "ㄒㄧㄚˋ--"), ("ㄇ", "--")] {
        assert_eq!(
            tps_adjust(incoming, raw),
            (incoming.to_string(), None),
            "{incoming:?} after {raw:?}"
        );
    }
}
