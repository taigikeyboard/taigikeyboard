//! v3.5.9 B-4 — `canonical_tl_form` mode-aware canonicalization tests.
//! Pins the user-history commit-key contract: `Tl` and `Poj` modes
//! both fold onto canonical TL via `poj_display_to_tl_display`
//! (idempotent on TL input, fold on POJ-form input — PR #310
//! r3278520895); `English` mode is identity (English `hello` must
//! not be reinterpreted as Taigi). Also pins the bounded CapsLock
//! non-idempotence for Tl/Poj modes (was POJ-only pre-r3278520895).

use phonetics::api::{canonical_tl_form, InputMode};

#[test]
fn poj_diacritic_folds_to_canonical_tl() {
    // Custom-entry-shaped POJ display form → canonical TL form. The
    // pe̍h-ōe-jī `ô` / `ⁿ` glyphs fold through `normalize_to_tl`
    // (`oa → ua`) + `to_tl` reassembly into TL `puânn`.
    assert_eq!(
        canonical_tl_form(
            "t\u{00e2}i-g\u{00ed}-kh\u{00ed}-p\u{00f4}a\u{207f}",
            InputMode::Poj
        ),
        "t\u{00e2}i-g\u{00ed}-kh\u{00ed}-pu\u{00e2}nn"
    );
}

#[test]
fn poj_ascii_folds_to_canonical_tl_ascii_toneless() {
    // OOV synth + walker-shaped path: POJ ASCII toneless space-joined
    // syllables (`chiah goa`) → canonical TL ASCII toneless
    // (`tsiah gua`). The `rewrite_token` chain assigns default
    // `tone = "1"` (non-stop) or `"4"` (stop) when the input has no
    // tone mark; `tl_tone_mark("1")` and `tl_tone_mark("4")` both
    // return the empty mark string (`engine/phonetics/src/tables.rs:43-47`),
    // so the assembly is unmarked. This is the load-bearing invariant
    // for §9 #2 cross-mode parity: the TL-mode shadow ASCII goes
    // through the helper as identity, so both modes write the same
    // toneless commit key.
    assert_eq!(canonical_tl_form("chiah goa", InputMode::Poj), "tsiah gua");
}

#[test]
fn cross_mode_oov_shadow_writes_identical_freq_key() {
    // §9 #2 byte-identity guard: the same OOV utterance shadowed in
    // POJ mode (`chiah goa`) and TL mode (`tsiah gua`) must produce
    // the SAME canonical-TL commit key. This is what closes the
    // hanji-absent user-frequency split for the walker greedy-longest
    // OOV synth path (`composing::continuous::fetch_walker_slot0_inner`
    // OOV branch).
    let poj_oov = canonical_tl_form("chiah goa", InputMode::Poj);
    let tl_oov = canonical_tl_form("tsiah gua", InputMode::Tl);
    assert_eq!(poj_oov, tl_oov);
    assert_eq!(poj_oov, "tsiah gua");
}

#[test]
fn cross_mode_custom_display_form_writes_identical_freq_key() {
    // §9 #2 byte-identity guard for the custom dict path: a
    // POJ-display-form custom roman (`góa`) and the TL-display-form
    // equivalent (`guá`) must fold to the same canonical key. Both
    // go through `lexicon::custom_entry_to_candidate` →
    // `canonical_tl_form(&roman, mode)`.
    let poj = canonical_tl_form("g\u{00f3}a", InputMode::Poj);
    let tl = canonical_tl_form("gu\u{00e1}", InputMode::Tl);
    assert_eq!(poj, tl);
    assert_eq!(poj, "gu\u{00e1}");
}

#[test]
fn tl_form_is_idempotent_in_poj_mode() {
    // The function is the freq-key canonicalizer — a TL-shaped input
    // fed in POJ mode must return identically (no double-fold).
    let tl = "t\u{00e2}i-g\u{00ed}-kh\u{00ed}-pu\u{00e2}nn";
    assert_eq!(canonical_tl_form(tl, InputMode::Poj), tl);
}

#[test]
fn tl_mode_observable_identity_for_tl_and_non_taigi_inputs() {
    // PR #310 r3278520895 — `Tl` mode also runs through
    // `poj_display_to_tl_display` now, but the observable behavior
    // for canonical-TL display tokens and non-Taigi strings is still
    // byte-identity: TL form is idempotent through the rewrite
    // chain, and non-Taigi tokens fail `split_initial_final` and
    // pass through unchanged.
    assert_eq!(canonical_tl_form("anything", InputMode::Tl), "anything");
    assert_eq!(
        canonical_tl_form("ts\u{00e1}i", InputMode::Tl),
        "ts\u{00e1}i"
    );
    assert_eq!(
        canonical_tl_form("hello world", InputMode::Tl),
        "hello world"
    );
}

#[test]
fn poj_form_in_tl_mode_folds_canonical_tl() {
    // PR #310 r3278520895 (Codex bot P2): a POJ-form custom entry
    // stored in POJ mode and later accessed while the active input
    // mode is `Tl` must still fold to canonical TL — otherwise the
    // `user_frequency.db` commit key stays POJ-shaped and splits
    // from the same word coming through `dict.bin` (which writes
    // `record.tl` = canonical TL). Closes the cross-mode key-
    // collision contract for mixed-form custom data.
    assert_eq!(canonical_tl_form("g\u{00f3}a", InputMode::Tl), "gu\u{00e1}");
    assert_eq!(
        canonical_tl_form(
            "t\u{00e2}i-g\u{00ed}-kh\u{00ed}-p\u{00f4}a\u{207f}",
            InputMode::Tl
        ),
        "t\u{00e2}i-g\u{00ed}-kh\u{00ed}-pu\u{00e2}nn"
    );
    assert_eq!(canonical_tl_form("chiah goa", InputMode::Tl), "tsiah gua");
}

#[test]
fn tl_mode_preserves_special_final_eng() {
    // 2026-06-05 TL-literal: `canonical_tl_form(Tl)` must NOT fold the TL
    // special nasal final `eng` [ɛŋ] (in `TL_FINALS`, §3.2.6) into `ing`
    // [iŋ] — otherwise a hanji-absent `teng` candidate's committed
    // `display_text` surfaces as `tíng`. The unambiguous POJ→TL folds
    // (`ch`/`oa`/`oe`) stay (see `poj_form_in_tl_mode_folds_canonical_tl`),
    // so cross-mode identity for POJ-form custom entries is preserved.
    assert_eq!(canonical_tl_form("teng", InputMode::Tl), "teng");
    // POJ mode genuinely IS POJ, so `eng`→`ing` is a correct POJ→TL
    // conversion: POJ `teng` → TL `ting`.
    assert_eq!(canonical_tl_form("teng", InputMode::Poj), "ting");
}

#[test]
fn english_mode_is_identity_does_not_misinterpret() {
    // Critical: an English custom entry `hello` must not be re-parsed
    // as Taigi initial+final. English mode short-circuits to identity.
    assert_eq!(canonical_tl_form("hello", InputMode::English), "hello");
    assert_eq!(canonical_tl_form("World", InputMode::English), "World");
}

#[test]
fn non_taigi_passes_through_in_poj_mode() {
    // POJ mode but the token fails `split_initial_final` (no Taigi
    // initial+final) → `rewrite_token` returns the original. Numeric
    // tone-digit tokens also fail the split and pass through.
    assert_eq!(
        canonical_tl_form("hello world", InputMode::Poj),
        "hello world"
    );
    assert_eq!(canonical_tl_form("tai5gi2", InputMode::Poj), "tai5gi2");
    assert_eq!(canonical_tl_form("123", InputMode::Poj), "123");
}

#[test]
fn empty_input_returns_empty() {
    assert_eq!(canonical_tl_form("", InputMode::Tl), "");
    assert_eq!(canonical_tl_form("", InputMode::Poj), "");
    assert_eq!(canonical_tl_form("", InputMode::English), "");
}

#[test]
fn capslock_taigi_is_known_non_idempotent() {
    // KNOWN LIMITATION (Codex pre-impl 2026-05-21 SHOULD; extended
    // by PR #310 r3278520895): the `rewrite_token` chain
    // lowercases for the parse and only title-cases the assembled
    // output via `capitalize_first` per hyphen-split sub-token, so
    // a fully uppercase Taigi-shaped token does NOT preserve
    // CapsLock — `TÂI-GÍ` (POJ) → `Tâi-Gí`, and `TSÁI-GÍ` (TL)
    // also goes through the same chain. After r3278520895 the fold
    // applies to both `Tl` and `Poj` modes, so this asymmetry
    // appears in both. Production touch: bounded to a hanji-absent
    // custom dict entry whose stored roman is all-caps Taigi (an
    // extremely rare user pattern). `English` mode preserves
    // CapsLock identically via the identity branch. Pinned here so
    // a future symmetric fix in `rewrite_token` lands as an
    // intentional behavior change.
    let poj_capslock = "T\u{00c2}I-G\u{00cd}";
    assert_eq!(
        canonical_tl_form(poj_capslock, InputMode::Poj),
        "T\u{00e2}i-G\u{00ed}"
    );
    assert_eq!(
        canonical_tl_form(poj_capslock, InputMode::Tl),
        "T\u{00e2}i-G\u{00ed}"
    );
    // English mode preserves CapsLock — the identity branch leaves
    // the string untouched so an all-caps English entry like
    // `THE-IN` round-trips byte-identically (would otherwise mangle
    // via the `th` initial + `e`/`in` finals path).
    assert_eq!(
        canonical_tl_form(poj_capslock, InputMode::English),
        poj_capslock
    );
}
