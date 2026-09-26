//! ⁿ becomes ᴺ in capitals (`AppConfig.force_lowercase_nasal_marker`, USER 2026-09-22)
//! integration test (`behavioral-invariants.md` §53). One nasal-marker case
//! rule for the §34 literal (preedit) and every candidate's `roman`: by
//! default (proto `false`, the switch ON) the marker follows the typed
//! capital — `ᴺ` U+1D3A, as the preedit has since PR #102; with the flag
//! (the switch OFF) it is always `ⁿ` U+207F. The identity sidechannels
//! (`display_text`, `canonical_tl`) are the same either way.
//!
//! Hermetic `LexiconHandle` install comes from `tests/common/mod.rs`.

use protos::engine::AppConfig;

mod common;
use common::Fetch;
use common::{
    build_dictionary_fst, build_syllables_fst, build_tkdb_v3, cell_with_hanji, config,
    empty_association_bin, engine_install_lock, fetch_cells, install_lexicon, write_temp, Cell,
    Row,
};
use lexicon::CustomEntry;

fn fixture_rows() -> Vec<Row> {
    vec![
        Row {
            toneless_key: "siann",
            hanzi: "聲",
            tl: "siann",
            syll: 1,
            freq: 9000,
        },
        Row {
            toneless_key: "siannim",
            hanzi: "聲音",
            tl: "siann-im",
            syll: 2,
            freq: 9000,
        },
        Row {
            toneless_key: "im",
            hanzi: "音",
            tl: "im",
            syll: 1,
            freq: 100,
        },
    ]
}

fn install_fixture() {
    let rows = fixture_rows();
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v3(&rows));
    let fst_path = build_dictionary_fst(&rows);
    let assoc_path = write_temp("association.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst(&["siann1", "im1"]);
    install_lexicon(&fst_path, &dict_path, &assoc_path, &syllables_path);
}

/// POJ with the `nn` double tap on, so `SIANN` reaches the tone chain as
/// `SIAⁿ` — the shape the mobile `nn` key and every desktop produce.
fn fetch(raw: &str, force_lowercase_nasal_marker: bool) -> Vec<Cell> {
    fetch_with_custom(raw, "poj", force_lowercase_nasal_marker, vec![])
}

fn fetch_with_custom(
    raw: &str,
    input_mode: &str,
    force_lowercase_nasal_marker: bool,
    custom: Vec<CustomEntry>,
) -> Vec<Cell> {
    let cfg = AppConfig {
        nn_doubletap_enabled: true,
        force_lowercase_nasal_marker,
        ..config(input_mode)
    };
    let fetch = Fetch {
        custom,
        ..Default::default()
    };
    fetch_cells(&cfg, raw, fetch)
}

/// A custom 聲 stored with the capital marker.
fn custom_capital_siann() -> Vec<CustomEntry> {
    vec![CustomEntry {
        roman: "SIA\u{1d3a}".into(),
        hanji: Some("聲".into()),
    }]
}

#[test]
fn the_rule_is_triggered_by_the_marker_not_the_mode() {
    let _lock = engine_install_lock();
    install_fixture();
    // TL has no nasal marker of its own, but a custom row that carries one
    // follows the same rule the TL preedit already applies to a typed
    // marker: as stored under the default (`SIAᴺ` after capitals), `ⁿ`
    // when forced.
    for (force_lowercase, expected) in [(false, "SIA\u{1d3a}"), (true, "SIA\u{207f}")] {
        let cells = fetch_with_custom("SIANN", "tl", force_lowercase, custom_capital_siann());
        assert!(
            cells
                .iter()
                .any(|c| c.0.as_deref() == Some("聲") && c.1 == expected),
            "force_lowercase={force_lowercase}: {cells:?}"
        );
    }
}

#[test]
fn caps_lock_default_writes_the_capital_nasal_marker() {
    let _lock = engine_install_lock();
    install_fixture();
    // trace: literal "SIANN" → nn double tap "SIAⁿ" → no tone digit →
    // `adjust_nasal_marker_case` after the capital A → "SIAᴺ". Candidate
    // 聲: `recase_all` raises "siann" → "SIANN", the POJ render title-cases
    // to "Siaⁿ" and `raise_case` (CapsLocked) leaves the marker → "SIAⁿ";
    // Step 5 then applies the same rule as the literal → "SIAᴺ".
    let cells = fetch("SIANN", false);
    assert_eq!(cells[0].1, "SIA\u{1d3a}", "the §34 literal");
    assert_eq!(cell_with_hanji(&cells, "聲").1, "SIA\u{1d3a}");
    assert_eq!(
        cell_with_hanji(&fetch("SIANNIM", false), "聲音").1,
        "SIA\u{1d3a}-IM"
    );
}

#[test]
fn caps_lock_keeps_a_custom_entry_with_a_nasal_marker_all_caps_in_poj_mode() {
    let _lock = engine_install_lock();
    install_fixture();
    // Retro Codex review of #89 (2026-09-22): the stored `ⁿ` is an
    // alphabetic lowercase char, so the whole-string case read behind the
    // POJ render called the raised row "title case" and lowered it to
    // `Sia-Sia` … per token now. trace: custom "sia-siaⁿ" → `recase_all`
    // raises to "SIA-SIAⁿ" → POJ render title-cases "Sia-Siaⁿ" → per-token
    // `match_case` re-raises "SIA-SIAⁿ" → §53 pass → "SIA-SIAᴺ".
    let custom = vec![CustomEntry {
        roman: "sia-sia\u{207f}".into(),
        hanji: Some("聲聲".into()),
    }];
    let cells = fetch_with_custom("SIASIANN", "poj", false, custom.clone());
    assert_eq!(cell_with_hanji(&cells, "聲聲").1, "SIA-SIA\u{1d3a}");
    // Forced lowercase marker: the letters still keep Caps Lock.
    let cells = fetch_with_custom("SIASIANN", "poj", true, custom.clone());
    assert_eq!(cell_with_hanji(&cells, "聲聲").1, "SIA-SIA\u{207f}");
    // Negative control: lowercase typing leaves the row as stored.
    let cells = fetch_with_custom("siasiann", "poj", false, custom);
    assert_eq!(cell_with_hanji(&cells, "聲聲").1, "sia-sia\u{207f}");
}

#[test]
fn force_lowercase_nasal_marker_renders_the_literal_and_every_candidate_with_a_lowercase_marker() {
    let _lock = engine_install_lock();
    install_fixture();
    let on = fetch("SIANN", true);
    assert_eq!(on[0].1, "SIA\u{207f}", "the §34 literal");
    assert_eq!(on[0].2, on[0].1, "the literal stays WYSIWYG (§34)");
    let siann = cell_with_hanji(&on, "聲");
    assert_eq!(siann.1, "SIA\u{207f}", "rendered roman");
    // Same rows, same order, same identity — only `roman` differs.
    let off = fetch("SIANN", false);
    assert_eq!(on.len(), off.len());
    for (a, b) in on.iter().zip(&off).skip(1) {
        assert_eq!((&a.0, &a.2, &a.3), (&b.0, &b.2, &b.3), "{a:?} vs {b:?}");
    }
}

#[test]
fn force_lowercase_nasal_marker_keeps_the_letters_capital_across_a_compound() {
    let _lock = engine_install_lock();
    install_fixture();
    let cells = fetch("SIANNIM", true);
    let siann_im = cell_with_hanji(&cells, "聲音");
    assert_eq!(siann_im.1, "SIA\u{207f}-IM", "only the marker is lowered");
}

#[test]
fn force_lowercase_nasal_marker_changes_nothing_for_lowercase_typing() {
    let _lock = engine_install_lock();
    install_fixture();
    let on = fetch("siann", true);
    let off = fetch("siann", false);
    assert_eq!(cell_with_hanji(&on, "聲").1, "sia\u{207f}");
    // Same rows, same order, same identity, same roman.
    assert_eq!(on, off);
}

#[test]
fn under_a_roman_only_display_the_literal_and_the_dictionary_row_now_read_the_same_and_collapse() {
    let _lock = engine_install_lock();
    install_fixture();
    // Romanization Only (`candidate_display_mode` = ROMAN_ONLY): before this rule the
    // Caps Lock literal `SIAᴺ` and the 聲 row `SIAⁿ` differed only in the
    // marker's case and showed as two cells; now they read the same, so
    // `dedupe_display_roman` keeps one and the literal adopts 聲's identity
    // (`adopt_collapsed_dict_identity`) — whichever way the switch is set.
    for (force_lowercase, expected) in [(false, "SIA\u{1d3a}"), (true, "SIA\u{207f}")] {
        let cfg = AppConfig {
            nn_doubletap_enabled: true,
            force_lowercase_nasal_marker: force_lowercase,
            ..common::config_with_display_mode("poj", 2)
        };
        let cells = fetch_cells(&cfg, "SIANN", Fetch::default());
        assert_eq!(cells[0].1, expected, "force_lowercase={force_lowercase}");
        assert_eq!(cells[0].2, "聲", "the literal carries 聲's 詞頻 key");
        assert_eq!(
            cells.iter().filter(|c| c.1 == expected).count(),
            1,
            "one cell reads {expected}: {cells:?}"
        );
    }
}
