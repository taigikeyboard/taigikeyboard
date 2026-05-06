//! Property-based round-trip tests. Generates well-formed TL syllables and
//! confirms TL→POJ→TL is lossless.

// 中文: 屬性測試:亂數產出合法 TL 音節,驗證 TL→POJ→TL 來回轉換不會掉資訊。

use phonetics::api::{poj_display_to_tl_display, tl_display_to_poj_display};
use proptest::prelude::*;

fn well_formed_tl_syllable() -> impl Strategy<Value = String> {
    let initials = prop::sample::select(vec!["", "k", "p", "t", "h", "tsh", "ng", "g", "b"]);
    let finals = prop::sample::select(vec![
        "a", "ai", "ang", "iong", "ua", "ng", "m", "iang", "oo",
    ]);
    let tones = prop::sample::select(vec!["1", "2", "3", "5", "7"]);
    (initials, finals, tones).prop_map(|(i, f, t)| phonetics::to_tl(i, f, t))
}

proptest! {
    #[test]
    fn tl_to_poj_roundtrip(tl in well_formed_tl_syllable()) {
        let poj = tl_display_to_poj_display(&tl);
        let back = poj_display_to_tl_display(&poj);
        prop_assert_eq!(back, tl);
    }
}
