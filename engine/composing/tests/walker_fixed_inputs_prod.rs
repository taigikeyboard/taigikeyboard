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
fn walker_bigram_keeps_the_reported_words_whole() {
    if !production_lexicon_ready() {
        eprintln!("production artifacts absent — run `make dict`; skipping.");
        return;
    }
    // v3.9.0 fix list row 2 (`bigram-lm-roadmap.md` D6): the unigram walk
    // split both into two frequent single syllables. trace (walker_cost,
    // word_bigrams minus taigi_typing, D 0.75):
    // ue5: 有 4.496 + 的 after 有 4.687 = 9.183 > 鞋 (key min 10.024) / 2^0.2 = 8.726;
    // khikhi: (去 5.461 + 去 after 去 6.135) / 3^0.2 = 9.309 > 起去 11.552 × (2/6)^0.2 = 9.273.
    // POJ `oe5` (ôe, one syllable, no split edge) is the negative control.
    let cases = [
        ("tl", "ue5", "鞋"),
        ("tl", "khikhi", "起去"),
        ("poj", "khikhi", "起去"),
        ("poj", "oe5", "鞋"),
    ];
    for (mode, raw, hanji) in cases {
        let cells = fetch_cells(&config(mode), raw, fresh_install_fetch());
        assert_eq!(
            cells.first().and_then(|(h, ..)| h.as_deref()),
            Some(hanji),
            "{mode} {raw}: {cells:?}"
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

#[test]
fn tps_typed_marks_and_spaces_pin_the_reading() {
    if !production_lexicon_ready() {
        eprintln!("production artifacts absent — run `make dict`; skipping.");
        return;
    }
    // §17 case 4 (2026-10-10 TPS real-text sim). trace (candidate_dump,
    // every source, before the fix): `ㄉㄞ˫ㄨㄢ` → 台灣 tâi-uân (typed 7
    // lost); `ㄌㄧˋㄫㆤ˫` → 靈會 lîng ē (span `ㄌㄧˋㄫ` read §35 `ㄫ`→`ㄥ`
    // through the mark); `ㄍㄚ`␣`ㄍㄚ` → 角仔 kak-á (`ㄍ` after the space
    // read as the coda `ㆻ`). The typed word is at slot 0 now, and no
    // reading contradicting a typed close is listed at all.
    let slot0 = [
        ("ㄉㄞ˫ㄨㄢ", "大員"),
        ("ㄌㄧˋㄫㆤ˫", "你硬"),
        ("ㄏㄛˋㄇㄚˋ", "好馬"),
    ];
    for (raw, hanji) in slot0 {
        let cells = fetch_cells(&config("tps"), raw, fresh_install_fetch());
        assert_eq!(
            cells.first().and_then(|(h, ..)| h.as_deref()),
            Some(hanji),
            "{raw}: {cells:?}"
        );
    }
    let absent = [
        ("ㄉㄞ˫ㄨㄢ", "台灣"),
        ("ㄏㄛˋㄇㄚ", "號碼"),
        ("ㄍㄚˋㄍㄚ", "角仔"),
        ("ㄍㄚ ㄍㄚ", "角仔"),
        ("ㄌㄧˋㄫㆤ˫", "靈會"),
        ("ㄌㄧˋㄫ", "靈"),
        ("ㄌㄧ ㄫㆤ", "靈的"),
        ("ㄌㄧ ㄫ", "靈"),
    ];
    for (raw, hanji) in absent {
        let cells = fetch_cells(&config("tps"), raw, fresh_install_fetch());
        assert!(
            !cells.iter().any(|(h, ..)| h.as_deref() == Some(hanji)),
            "{raw} lists {hanji}: {cells:?}"
        );
    }
    // Controls: no mark and no space keep every reading (§17 case 2), and
    // the interior-space phrases §31 / §35 rely on stay.
    for (raw, hanji) in [
        ("ㄉㄞㄨㄢ", "台灣"),
        ("ㄌㄧㄫㆤ", "靈的"),
        ("ㄍㄠ ㄉㄞ˪", "交代"),
        ("ㄇ ㄒㄧ", "毋是"),
    ] {
        let cells = fetch_cells(&config("tps"), raw, fresh_install_fetch());
        assert_eq!(
            cells.first().and_then(|(h, ..)| h.as_deref()),
            Some(hanji),
            "{raw}: {cells:?}"
        );
    }
}
