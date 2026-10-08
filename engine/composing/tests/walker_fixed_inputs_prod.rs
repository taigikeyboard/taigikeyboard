//! E1 fixed regression inputs over the production lexicon with a fresh
//! install's sources: slot 0 is the word the user means
//! (`docs/architecture/unified-word-frequency-roadmap.md` §5). `kau3siu7` is
//! the E1 bug (到受 composed over 教授 before P3), `kausiu` its toneless twin
//! (狗岫 picked for the edge before P5a); the rest pin what the walker already
//! got right.

use crate::common::{
    config, default_sources_bitmask, fetch_cells, production_lexicon_ready, Fetch,
};

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
    let fetch = Fetch {
        enabled_sources_bitmask: default_sources_bitmask(),
        literal_roman_candidate_disabled: true,
        ..Default::default()
    };
    for (raw, hanji, roman) in cases {
        let cells = fetch_cells(&config("tl"), raw, fetch.clone());
        let (slot0_hanji, slot0_roman, _, _) = cells.first().expect("slot 0");
        assert_eq!(
            (slot0_hanji.as_deref(), slot0_roman.as_str()),
            (Some(hanji), roman),
            "{raw}"
        );
    }
}
