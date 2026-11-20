//! `lexicon::classification::CJK_RANGES` against the dictionary pipeline:
//! the Rust table equals `dictionary/common/cjk.py`'s `CJK_RANGES`, and a
//! shipped word beyond Extension E reaches the Hanji search
//! (`INVARIANT_LEX_INPUT_CLASSIFICATION_HANJI_RANGE`).

use lexicon::classification::{is_hanji, CJK_RANGES};
use lexicon::{EngineHandle, LexiconPaths};
use test_support::{dictionary_cjk_py_path, engine_install_lock, ProductionArtifacts};

/// The tuples of `cjk.py`'s top-level `CJK_RANGES = (` assignment, one
/// `(0xLOW, 0xHIGH),` per line (a trailing `#` comment allowed) up to the
/// closing `)`. Anything else panics, so a reshaped table fails the parity
/// test instead of passing it on a misread.
fn python_ranges(source: &str) -> Vec<(u32, u32)> {
    let mut lines = source
        .lines()
        .skip_while(|line| !(line.starts_with("CJK_RANGES") && line.ends_with("= (")));
    assert!(
        lines.next().is_some(),
        "no `CJK_RANGES ... = (` line in cjk.py"
    );
    lines
        .map(|line| line.split('#').next().unwrap_or("").trim())
        .take_while(|line| *line != ")")
        .filter(|line| !line.is_empty())
        .map(|line| {
            line.strip_prefix("(0x")
                .and_then(|rest| rest.strip_suffix("),"))
                .and_then(|inner| inner.split_once(", 0x"))
                .and_then(|(low, high)| {
                    Some((
                        u32::from_str_radix(low, 16).ok()?,
                        u32::from_str_radix(high, 16).ok()?,
                    ))
                })
                .unwrap_or_else(|| panic!("not a `(0xLOW, 0xHIGH),` tuple: {line:?}"))
        })
        .collect()
}

#[test]
fn the_parser_rejects_a_reshaped_table() {
    for source in [
        "\"\"\"CJK_RANGES = (\"\"\"\n", // only a docstring mention
        "CJK_RANGES = (\n    (20000, 0x3134F),\n)\n", // decimal bound
        "CJK_RANGES = (\n    0x4E00, 0x9FFF,\n)\n", // no parentheses
        "CJK_RANGES = (\n    (0x4E00 0x9FFF),\n)\n", // no comma
    ] {
        let parsed = std::panic::catch_unwind(|| python_ranges(source));
        assert!(parsed.is_err(), "accepted {source:?}");
    }
}

#[test]
fn rust_cjk_ranges_equal_the_pipeline_table() {
    let path = dictionary_cjk_py_path();
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    assert_eq!(python_ranges(&source), CJK_RANGES);
}

/// A lone character beyond Extension E is Hanji, so the dictionary search
/// takes its Hanji lookup, which finds the character's `dictionary.csv` row.
#[test]
fn shipped_characters_beyond_extension_e_reach_the_hanji_search() {
    let Some(artifacts) = ProductionArtifacts::locate() else {
        return;
    };
    let _lock = engine_install_lock();
    let paths = LexiconPaths::validated(
        &artifacts.dictionary_fst,
        &artifacts.dictionary_bin,
        &artifacts.association_bin,
        &artifacts.syllables_fst,
        1,
    )
    .expect("production artifacts validate");
    EngineHandle::install(paths).expect("production artifacts install");

    // trace: dictionary.csv rows — 𰣻 U+308FB (Ext G) / ko, 丸 U+2F801
    // (Compatibility Supplement) / huân, 嗀 U+FA0D (Compatibility) / khak.
    for (query, tl) in [
        ("\u{308FB}", "ko"),
        ("\u{2F801}", "huân"),
        ("\u{FA0D}", "khak"),
    ] {
        assert!(is_hanji(query), "{query:?} is Hanji");
        let rows = lexicon::api::dictionary_search(
            protos::engine::DictionarySearchRequest {
                query: query.to_owned(),
                input_mode: "tl".to_owned(),
                toggles: Some(all_sources_on()),
                custom_dictionary_enabled: false,
                limit: 20,
            },
            |_| Vec::new(),
        )
        .expect("dictionary search")
        .rows;
        assert!(
            rows.iter()
                .any(|row| row.hanji.as_deref() == Some(query) && row.lookup_tl == tl),
            "{query:?}/{tl} in {rows:?}"
        );
    }
}

/// Every source and the variant characters on; no kautian subcollection
/// gate (absent sub-message).
fn all_sources_on() -> protos::engine::DictionarySourceToggles {
    protos::engine::DictionarySourceToggles {
        kautian: true,
        taigitv: true,
        itaigi: true,
        sitbut: true,
        taihoa: true,
        taijit: true,
        kungge: true,
        stti: true,
        khpoo: true,
        variant: true,
        khiin: true,
        lkk: true,
        dev: true,
        kautian_subcollections: None,
    }
}
