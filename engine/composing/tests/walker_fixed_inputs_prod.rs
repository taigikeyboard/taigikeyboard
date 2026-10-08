//! E1 fixed regression inputs over the production lexicon with a fresh
//! install's sources: slot 0 is the word the user means
//! (`docs/architecture/unified-word-frequency-roadmap.md` §5). `kau3siu7` is
//! the E1 bug (到受 composed over 教授 before P3), `kausiu` its toneless twin
//! (狗岫 picked for the edge before P5a); the rest pin what the walker already
//! got right.

use crate::common::{
    config, default_sources_bitmask, fetch_cells, production_lexicon_ready, Fetch,
};

/// A fresh install's sources, no literal roman row.
fn fresh_install_fetch() -> Fetch {
    Fetch {
        enabled_sources_bitmask: default_sources_bitmask(),
        literal_roman_candidate_disabled: true,
        ..Default::default()
    }
}

#[test]
fn fixed_inputs_put_the_intended_word_at_slot_0() {
    if !production_lexicon_ready() {
        eprintln!("production artifacts absent — run `make dict`; skipping.");
        return;
    }
    // trace (candidate_dump, default sources, E1 P3 walker_cost):
    // kau3siu7 教授 9,938 → 7.978 < 到 5,636 + 受 6,856 → 10.028;
    // kausiu (P5a edge pick by walker_cost, N + αV = 5,302,173): 教授 (count
    // 246) 9,938 < 狗岫 (count 1) 13,086 < 狗巢 / 交收 (unseen) 13,181 —
    // 狗岫 led on the old frequency (25 = 50 ÷ 2 syllables vs 教授 10);
    // hoogua: the walker's 予我 shows the dictionary's `hōo--guá` (§22);
    // ginalangtsiahpngbesai: 食飯 (bitmask 52) is outside the default
    // sources, so 食 and 飯 are two words.
    let cases = [
        ("kau3siu7", "教授", "kàu-siū"),
        ("kausiu", "教授", "kàu-siū"),
        ("tai5gi2", "台語", "tâi-gí"),
        ("hoogua", "予我", "hōo--guá"),
        ("taiuan", "台灣", "tâi-uân"),
        (
            "ginalangtsiahpngbesai",
            "囡仔人食飯袂使",
            "gín-á-lâng tsia̍h pn̄g bē-sái",
        ),
    ];
    for (raw, hanji, roman) in cases {
        let cells = fetch_cells(&config("tl"), raw, fresh_install_fetch());
        let (slot0_hanji, slot0_roman, _, _) = cells.first().expect("slot 0");
        assert_eq!(
            (slot0_hanji.as_deref(), slot0_roman.as_str()),
            (Some(hanji), roman),
            "{raw}"
        );
    }
}

#[test]
fn list_orders_the_words_after_slot_0_by_corpus_count() {
    if !production_lexicon_ready() {
        eprintln!("production artifacts absent — run `make dict`; skipping.");
        return;
    }
    // E1 P5b — the list shares slot 0's corpus scale. trace (word_unigrams,
    // default sources): taiuanlang's partial spans 台灣/tâi-uân 4,084 > 大/tāi
    // 1,281 > 台/tâi 1,006; on the old frequency 台 (31,281) led 台灣 (1,379).
    let cells = fetch_cells(&config("tl"), "taiuanlang", fresh_install_fetch());
    let position = |hanji: &str| {
        cells
            .iter()
            .position(|(cell_hanji, ..)| cell_hanji.as_deref() == Some(hanji))
            .unwrap_or_else(|| panic!("{hanji} missing from {cells:?}"))
    };
    assert!(position("台灣") < position("大"));
    assert!(position("大") < position("台"));
}
