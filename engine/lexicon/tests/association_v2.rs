//! `association.bin` v2: character keys and word keys (`hanji\u{1}tl`) share
//! one byte-sorted key section; v1 files are rejected with the rebuild hint
//! (`docs/engine/binary-format.md` §2).

use lexicon::association_reader::{word_key, AssociationFilter, AssociationReader};
use lexicon::LexiconError;
use test_support::{build_tkwa, write_temp};

/// Keys in raw UTF-8 byte order: `好` (E5 A5 BD) < `好\u{1}hó` (a prefix
/// sorts before its extension).
fn fixture() -> Vec<u8> {
    let word = word_key("好", "hó");
    build_tkwa(
        2,
        &[
            ("好", &[(0x0001, 100, "伊", "i")]),
            (
                word.as_str(),
                &[(0x0010, 50, "額", "gia̍h"), (0x0001, 40, "食", "tsia̍h")],
            ),
        ],
    )
}

#[test]
fn word_key_joins_hanji_and_tl_with_u1() {
    assert_eq!(word_key("好", "hó"), "好\u{1}hó");
}

#[test]
fn each_namespace_resolves_to_its_own_entries() {
    let path = write_temp("assoc-v2.bin", &fixture());
    let reader = AssociationReader::open(&path).expect("v2 opens");

    let by_char: Vec<_> = reader
        .lookup("好", 10, &AssociationFilter::ALL)
        .into_iter()
        .map(|e| e.next_word)
        .collect();
    assert_eq!(
        by_char,
        ["伊"],
        "character key never returns the word-key rows"
    );

    let by_word: Vec<_> = reader
        .lookup(&word_key("好", "hó"), 10, &AssociationFilter::ALL)
        .into_iter()
        .map(|e| (e.next_word, e.next_tl, e.count, e.bitmask))
        .collect();
    assert_eq!(
        by_word,
        [
            ("額".to_string(), "gia̍h".to_string(), 50, 0x0010),
            ("食".to_string(), "tsia̍h".to_string(), 40, 0x0001),
        ]
    );

    assert!(
        reader
            .lookup(&word_key("好", "hò"), 10, &AssociationFilter::ALL)
            .is_empty(),
        "other reading → no key"
    );
    assert!(
        reader
            .lookup("好\u{1}", 10, &AssociationFilter::ALL)
            .is_empty(),
        "prefix of a word key is not a key"
    );
}

#[test]
fn v1_file_rejected_with_rebuild_hint() {
    let path = write_temp("assoc-v1.bin", &build_tkwa(1, &[]));
    let Err(LexiconError::InvalidBinary(msg)) = AssociationReader::open(&path) else {
        panic!("v1 must be rejected with InvalidBinary");
    };
    assert!(msg.contains("v1→v2"), "marker missing: {msg}");
    assert!(msg.contains("make dict"), "rebuild guidance missing: {msg}");
}

#[test]
fn lookup_associations_strips_khinsiann_from_previous_word() {
    let key = word_key("會記得", "ē-kì--tit");
    let raw = build_tkwa(
        2,
        &[
            ("得", &[(0x0001, 5, "了", "liáu")]),
            (key.as_str(), &[(0x0001, 50, "伊", "i")]),
        ],
    );
    let path = write_temp("assoc-khinsiann-lookup.bin", &raw);
    let reader = AssociationReader::open(&path).expect("v2 opens");

    let hits = lexicon::search::lookup_associations("會記--得", "ē-kì--tit", 10, u32::MAX, &reader)
        .expect("lookup");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].candidate_word, "伊");
    assert_eq!(hits[0].candidate_tl, "i");
    assert_eq!(hits[0].previous_word, "會記--得");
}
