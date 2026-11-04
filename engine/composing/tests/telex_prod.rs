//! The buffers the Telex keys write, over the production lexicon: tone 6 and
//! a spelled-out aspirated affricate reach real words. What each key writes:
//! `engine/composing/src/telex.rs`.

use crate::common::{fetch_hanji, production_lexicon_ready, Fetch};

#[test]
fn telex_buffers_reach_the_dictionary() {
    if !production_lexicon_ready() {
        eprintln!("production artifacts absent — run `make dict`; skipping.");
        return;
    }
    // trace: `z` on `huattoo` writes `huattoo6`, the 法度 huat4too6 row of
    // dictionary.csv; `c` `h` `h` `i` `a` writes `tshia` / `chhia`, 車.
    for (raw, mode, word) in [
        ("huattoo6", "tl", "法度"),
        ("hoattoo6", "poj", "法度"),
        ("tshia", "tl", "車"),
        ("chhia", "poj", "車"),
    ] {
        let hanji = fetch_hanji(raw, mode, Fetch::default());
        assert!(
            hanji.iter().any(|h| h == word),
            "{raw} ({mode}): {word} missing; got {hanji:?}"
        );
    }
}
