//! v3.5.8 Phase 9 Item 9 — POJ-display canonicalization integration matrix.
//!
//! **2026-06-05 TL-literal update**: TL mode no longer applies the POJ→TL
//! SPELLING fold (`ch→ts`/`oa→ua`/`oe→ue`/`eng→ing`/`ek→ik`) to user input,
//! so the Item-9 *TL* POJ-display recovery (e.g. `ōe`→`ue` to reach 白話字)
//! is intentionally removed — TL input is literal. The ENCODING folds
//! (`o͘→oo`, `ⁿ→nn`, tone-mark strip, hyphen-shadow) still apply. The
//! spelling-fold cases below are now LITERAL-behavior guards (they assert
//! the fold does NOT fire); the encoding cases are unchanged.
//!
//! Pins `build_keys_tl_with_inventory` against a hermetic
//! `SyllableInventory` for the shapes Codex's pre-impl + post-impl
//! consults (2026-05-15) called out as the user-visible coverage gap
//! and the offset-map atomic-fold regression hot-spot:
//!
//! - `pe̍h-ōe-jī` (NFC) — POJ display with combining tone marks and the
//!   `oe→ue` substitution, the showcase scenario for 白話字 recovery.
//! - `pe̍h-ōe-jī` (NFD canary) — one NFD form to lock parity per
//!   F5C; full NFC/NFD duplication is over-built per YAGNI.
//! - `chóa` — POJ initial `ch→ts` plus `oa→ua` plus combining tone-2
//!   acute on the base vowel.
//! - `pe\u{207f}` — POJ nasal superscript `ⁿ` → `nn`.
//! - `so\u{0358}` — POJ `o + U+0358 (combining dot above right)` → `oo`.
//! - `so\u{0358}` with `so` + `soo` sibling inventory — Codex post-impl
//!   P1 regression guard: the partial-prefix `tl:so` candidate must
//!   consume the whole source spelling, not just `so`, so the
//!   combining dot never dangles in the pending buffer.
//! - `tâi-ōe` — mixed: combining circumflex on first syllable, ASCII
//!   hyphen, combining macron on second syllable.
//! - `tâi5-ban3` — combining circumflex AND trailing ASCII tone digit
//!   on the same first syllable (regression for the dual-marker edge
//!   case Codex highlighted).
//! - `taibak` (regression) — pure-ASCII input must remain byte-for-byte
//!   identical to the Item 8 pipeline; the F3C identity-fast-path is
//!   what keeps `tó-uī` (`dictionary/output/dictionary.csv:1984`,
//!   `tl_notone=toui`) from being garbled by `ou→oo`.
//! - `oe-ji` — ASCII-only F3C sanity: `oe→ue` substitution must NOT
//!   fire on pure ASCII input.
//!
//! The hermetic inventory builder mirrors
//! `engine/composing/tests/build_keys_tl_hyphen.rs:177-214`.

// 中文: Phase 9 Item 9 — POJ-display canonicalize + offset map 的 end-to-end 測試。
// 中文: 九組案例:NFC 白話字 / NFD canary / chóa / peⁿ / so͘ / so͘ + so/soo 兄弟 / tâi-ōe / 混合 combining + digit / ASCII regression / ASCII-only 防呆。

use std::path::PathBuf;

use composing::dispatch::build_keys_tl_with_inventory;
use fst::SetBuilder;
use lexicon::SyllableInventory;
use phonetics::canonicalize_syllable;
use unicode_normalization::UnicodeNormalization;

#[test]
fn nfc_peh_oe_ji_tl_literal_no_oe_ue_recovery() {
    // TL-literal (2026-06-05): the Item-9 POJ-display recovery is removed
    // for TL mode. `pe̍h-ōe-jī` (NFC) is taken literally — the `oe→ue`
    // SPELLING fold does NOT fire, so the POJ `ōe` does not match the TL
    // `ue` inventory and the fused `tl:pehueji` (白話字) is NOT produced.
    // Encoding still applies: `pe̍h` tone-strips to the valid `tl:peh`,
    // consuming the whole `pe̍h` (5 bytes, dropped `\u{030d}` folds into
    // the preceding `e`'s raw_end so commit leaves no dangling mark).
    let inv = build_inventory(&["peh8", "ue7", "ji7"]);
    let keys = build_keys_tl_with_inventory(
        "pe\u{030d}h-\u{014d}e-j\u{012b}",
        &inv,
        phonetics::InputMode::Tl,
    );
    let mapped: Vec<((u32, u32), &str)> = keys
        .iter()
        .map(|(span, key)| (*span, key.as_str()))
        .collect();
    assert!(
        !mapped.iter().any(|(_, k)| *k == "tl:pehueji"),
        "TL literal must NOT fold `oe→ue` to recover 白話字, got {mapped:?}",
    );
    assert!(
        mapped.contains(&((0, 5), "tl:peh")),
        "encoding still strips the tone mark → `tl:peh` at raw_end=5, got {mapped:?}",
    );
}

#[test]
fn nfd_peh_oe_ji_tl_literal_matches_nfc_canary() {
    // F5C single NFD canary under TL-literal. `pe̍h-ōe-jī` as NFD doubles
    // the combining marks; the literal result must match the NFC case —
    // no `oe→ue` fold either way, so `tl:pehueji` is absent and the
    // tone-stripped `tl:peh` still surfaces. The `tl:peh` raw_end tracks
    // the NFD byte count (offset map faithful to NFD bytes).
    let nfc = "pe\u{030d}h-\u{014d}e-j\u{012b}";
    let nfd: String = nfc.nfd().collect();
    assert!(nfd.len() > nfc.len(), "NFD canary assumption violated");
    let inv = build_inventory(&["peh8", "ue7", "ji7"]);
    let keys = build_keys_tl_with_inventory(&nfd, &inv, phonetics::InputMode::Tl);
    let key_strs: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(
        !key_strs.contains(&"tl:pehueji"),
        "TL literal NFD must NOT fold `oe→ue`, got {key_strs:?}",
    );
    let peh = keys
        .iter()
        .find(|(_, k)| k == "tl:peh")
        .expect("`tl:peh` present (tone-strip encoding)");
    assert!(
        peh.0 .1 as usize > "peh".len(),
        "raw_end must track the longer NFD bytes for `pe̍h`, got {:?}",
        peh.0,
    );
}

#[test]
fn tl_literal_choa_does_not_fold_to_tsua() {
    // TL-literal (2026-06-05): `chóa` → drop combining → literal `choa` —
    // the `ch→ts` / `oa→ua` SPELLING fold does NOT fire, so `choa` (POJ
    // shape, `ch` is not a TL initial) never matches the TL `tsua`
    // inventory. The pre-2026-06-05 `tl:tsua` recovery is gone.
    let inv = build_inventory(&["tsua7"]);
    let keys = build_keys_tl_with_inventory("ch\u{00f3}a", &inv, phonetics::InputMode::Tl);
    let key_strs: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(
        !key_strs.contains(&"tl:tsua"),
        "TL literal must NOT fold POJ `chóa` into `tl:tsua`, got {key_strs:?}",
    );
}

#[test]
fn poj_superscript_nasal_marker_becomes_nn() {
    // `pe\u{207f}` (5 bytes) → Phase 1 NFD walk emits `pe\u{207f}` →
    // Phase 2 `\u{207f}→nn` → `penn` → matches dict canonical for
    // 平 (`pee` + nasal) etc.
    let inv = build_inventory(&["penn1"]);
    let keys = build_keys_tl_with_inventory("pe\u{207f}", &inv, phonetics::InputMode::Tl);
    let key_strs: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(
        key_strs.contains(&"tl:penn"),
        "expected `tl:penn` after POJ superscript ⁿ canonicalize, got {key_strs:?}",
    );
    // Raw end for the full-buffer match equals the raw byte length (5)
    // — both ASCII bytes emitted by `\u{207f}→nn` collapse onto the
    // post-superscript raw_end.
    let full = keys
        .iter()
        .find(|(_, k)| k == "tl:penn")
        .expect("`tl:penn` present");
    assert_eq!(full.0 .1, 5, "raw_end after `pe\\u207f` must be 5");
}

#[test]
fn poj_o_with_dot_above_right_becomes_oo() {
    // `so\u{0358}` (4 bytes) → Phase 1 keeps `\u{0358}` → Phase 2
    // `o\u{0358}→oo` → `soo` → matches dict canonical for 數 etc.
    let inv = build_inventory(&["soo3"]);
    let keys = build_keys_tl_with_inventory("so\u{0358}", &inv, phonetics::InputMode::Tl);
    let key_strs: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(
        key_strs.contains(&"tl:soo"),
        "expected `tl:soo` after POJ `o\\u0358` canonicalize, got {key_strs:?}",
    );
}

#[test]
fn poj_o_dot_atomic_longest_match_suppresses_shorter_so() {
    // §18 longest-match prefix suppression (`INVARIANT_CONTINUOUS_LONGEST_MATCH_PREFIX`,
    // USER 2026-05-31「免調也壓制」). Against the live `dictionary.csv`
    // sibling pair where both `so` (no tone, e.g. 蓑) and `soo` (no tone,
    // e.g. 數) are valid syllables, the shadow `soo` (from POJ `so\u{0358}`)
    // has single-syllable ends {2 (`so`), 3 (`soo`)}. `so` is a strict
    // prefix of the longer `soo`, so it is SUPPRESSED — only the longest
    // single syllable `tl:soo` is emitted.
    //
    // This subsumes the prior Codex post-impl P1 (2026-05-15) concern: that
    // round worried the shorter `tl:so` candidate might commit raw_end=2 and
    // strand the combining dot `\u{0358}`. Under longest-match `tl:so` no
    // longer surfaces at all, so there is no shorter candidate to strand the
    // mark. `tl:soo` still consumes the whole `so\u{0358}` source (raw_end=4).
    let inv = build_inventory(&["soo3", "so7"]);
    let keys = build_keys_tl_with_inventory("so\u{0358}", &inv, phonetics::InputMode::Tl);
    let mapped: Vec<((u32, u32), &str)> = keys
        .iter()
        .map(|(span, key)| (*span, key.as_str()))
        .collect();
    assert!(
        mapped.iter().all(|(_, k)| *k != "tl:so"),
        "shorter prefix syllable `tl:so` must be suppressed under longest-match, got {mapped:?}",
    );
    let soo_candidate = mapped.iter().find(|(_, k)| *k == "tl:soo");
    assert!(
        soo_candidate.is_some(),
        "expected the longest single syllable `tl:soo` to be emitted, got {mapped:?}",
    );
    assert_eq!(
        soo_candidate.unwrap().0,
        (0, 4),
        "`tl:soo` must consume the whole `so\\u0358` (raw_end=4), got {mapped:?}",
    );
}

#[test]
fn mixed_combining_with_hyphen_tl_literal_no_oe_fold() {
    // `tâi-ōe` — combining circumflex + ASCII hyphen + combining macron.
    // Encoding (tone-strip) + Item-8 hyphen-shadow still compose without
    // shifting the offset map, so the first syllable yields `tl:tai`. But
    // TL-literal (2026-06-05) does NOT fold `oe→ue`, so the POJ `ōe` does
    // not match the TL `ue` inventory and the fused `tl:taiue` is absent.
    let inv = build_inventory(&["tai5", "ue7"]);
    let keys = build_keys_tl_with_inventory("t\u{00e2}i-\u{014d}e", &inv, phonetics::InputMode::Tl);
    let key_strs: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(
        key_strs.contains(&"tl:tai"),
        "expected `tl:tai` single-syllable hit (tone-strip encoding), got {key_strs:?}",
    );
    assert!(
        !key_strs.contains(&"tl:taiue"),
        "TL literal must NOT fold `oe→ue` into fused `tl:taiue`, got {key_strs:?}",
    );
}

#[test]
fn dual_marker_combining_and_trailing_digit_canonicalizes() {
    // `tâi5-ban3` — combining circumflex on `â` AND trailing ASCII
    // tone digit `5` on the same first syllable. After the explicit-tone
    // fix this is a FULLY toned reading, so the combining circumflex is
    // dropped (Phase 1) and the surviving ASCII tone digits are KEPT
    // (verbatim `tl_num` form), not stripped: `tâi5-ban3` → `tai5-ban3`
    // → hyphen-strip → `tai5ban3` → key `tl:tai5ban3` (was `tl:taiban`
    // pre-fix). The single-syllable prefix span keeps its tone too:
    // `tl:tai5`.
    let inv = build_inventory(&["tai5", "ban3"]);
    let keys = build_keys_tl_with_inventory("t\u{00e2}i5-ban3", &inv, phonetics::InputMode::Tl);
    let key_strs: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(
        key_strs.contains(&"tl:tai5ban3"),
        "expected toned `tl:tai5ban3` after dual-marker canonicalize, got {key_strs:?}",
    );
    assert!(
        key_strs.contains(&"tl:tai5"),
        "expected toned single-syllable prefix `tl:tai5`, got {key_strs:?}",
    );
}

#[test]
fn pure_ascii_input_takes_identity_fast_path() {
    // `taibak` — F3C pure-ASCII identity guard, **TL mode**
    // (`mode = InputMode::Tl`). Output must be byte-identical to the Item 8
    // pipeline (consumed_span values unchanged). Any drift here means
    // `canonicalize_poj_shadow` is running the substitution chain on
    // ASCII TL input, which would risk garbling the `tó-uī` class of
    // real dictionary entries. (POJ mode deliberately DOES run the
    // chain on ASCII — see `poj_mode_ascii_*` tests below.)
    let inv = build_inventory(&["tai5", "bak4"]);
    let keys = build_keys_tl_with_inventory("taibak", &inv, phonetics::InputMode::Tl);
    let mapped: Vec<((u32, u32), &str)> = keys
        .iter()
        .map(|(span, key)| (*span, key.as_str()))
        .collect();
    assert_eq!(
        mapped,
        vec![((0, 3), "tl:tai"), ((0, 6), "tl:taibak")],
        "pure-ASCII input must take the F3C identity fast path",
    );
}

#[test]
fn ascii_only_poj_spellings_skip_canonicalize() {
    // F3C guard end-to-end, **TL mode** (`mode = InputMode::Tl`): `oe-ji` is
    // pure ASCII so canonicalize is a no-op; the `oe→ue` substitution
    // does NOT fire, and the FST lookup against `oeji` misses. Without
    // this guard, ASCII-only dictionary entries whose `tl_notone`
    // legitimately contain `oa`, `oe`, `ou`, etc. (e.g. `toui` for
    // 佗位) would be mis-rewritten into `uai`, `uei`, `oo`. Test
    // asserts the negative: no `tl:ueji` candidate emerges from pure
    // ASCII in TL mode. (POJ mode is the opposite — see
    // `poj_mode_ascii_oe_substitution_fires`.)
    let inv = build_inventory(&["ue7", "ji7"]);
    let keys = build_keys_tl_with_inventory("oe-ji", &inv, phonetics::InputMode::Tl);
    let key_strs: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(
        !key_strs.iter().any(|k| k.contains("ue")),
        "ASCII-only `oe-ji` must NOT trigger `oe→ue` substitution, got {key_strs:?}",
    );
}

// ---- POJ-mode ASCII canonicalize (B-2 first-class) ------------------
// v3.5.9 B-2 promoted POJ to a first-class FST key family. POJ
// continuous typing of toneless pure-ASCII POJ (`chiah`, `chhia`,
// `goa`, `che`) now reaches the `poj:` family of `dictionary.fst` /
// `syllables.fst` directly — `chiah` stays `chiah`, no POJ→TL fold.
// PR #309 Codex P1 refinement: the legacy `ou→oo` alias was further
// removed from the shadow canonicalize so hyphenless multi-syllable
// POJ input like `toui` (intended `tó-uī`) is boundary-preserving.

#[test]
fn poj_mode_ascii_chiah_emits_poj_family() {
    // v3.5.9 B-2 — 食 — POJ `chia̍h`, toneless ASCII `chiah`. POJ mode
    // now preserves POJ ASCII and emits the `poj:` family key against
    // the `poj:` family of the tagged-single-FST. Pre-B-2 this folded
    // to `tl:tsiah`; B-2 makes POJ first-class so `chiah` stays
    // `poj:chiah`.
    let inv = build_poj_inventory(&["chiah8"]);
    let keys = build_keys_tl_with_inventory("chiah", &inv, phonetics::InputMode::Poj);
    let key_strs: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(
        key_strs.contains(&"poj:chiah"),
        "POJ-mode ASCII `chiah` must emit `poj:chiah`, got {key_strs:?}",
    );
    assert!(
        !key_strs.iter().any(|k| k.starts_with("tl:")),
        "POJ-mode keys must not leak into `tl:` family, got {key_strs:?}",
    );
}

#[test]
fn poj_mode_ascii_chhia_emits_poj_family() {
    // v3.5.9 B-2 — 車 — POJ `chhia`. `chh→tsh` chain rule belongs to
    // NORMALIZE_TO_TL_RULES (TL fold), not POJ — POJ keeps POJ shape.
    let inv = build_poj_inventory(&["chhia1"]);
    let keys = build_keys_tl_with_inventory("chhia", &inv, phonetics::InputMode::Poj);
    let key_strs: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(
        key_strs.contains(&"poj:chhia"),
        "POJ-mode ASCII `chhia` must emit `poj:chhia`, got {key_strs:?}",
    );
}

#[test]
fn poj_mode_ascii_goa_emits_poj_family() {
    // v3.5.9 B-2 — 我 — POJ `góa`. `oa→ua` is TL-only; POJ keeps `goa`.
    let inv = build_poj_inventory(&["goa2"]);
    let keys = build_keys_tl_with_inventory("goa", &inv, phonetics::InputMode::Poj);
    let key_strs: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(
        key_strs.contains(&"poj:goa"),
        "POJ-mode ASCII `goa` must emit `poj:goa`, got {key_strs:?}",
    );
}

#[test]
fn poj_mode_ascii_oe_no_tl_chain_fold() {
    // v3.5.9 B-2 — `oe→ue` is in NORMALIZE_TO_TL_RULES, not POJ. POJ
    // mode keeps `oe-ji` → `oeji` (hyphen-shadow strips `-`; POJ rule
    // list is encoding-only and does NOT fold `oe → ue`).
    let inv = build_poj_inventory(&["oe7", "ji7"]);
    let keys = build_keys_tl_with_inventory("oe-ji", &inv, phonetics::InputMode::Poj);
    let key_strs: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(
        key_strs.iter().any(|k| k.contains("poj:oe")),
        "POJ-mode ASCII `oe-ji` MUST stay POJ shape (no `oe→ue` fold), got {key_strs:?}",
    );
    assert!(
        !key_strs.iter().any(|k| k.contains("ue")),
        "POJ-mode must NOT fire the TL-only `oe→ue` chain, got {key_strs:?}",
    );
}

#[test]
fn poj_mode_ascii_toui_preserves_token_boundary() {
    // v3.5.9 B-2 PR #309 Codex P1 (`r3276402303`) — hermetic
    // reproduction of the multi-syllable POJ boundary-preservation
    // case. User types `toui` (no hyphen) meaning POJ `tó-uī` (佗位),
    // indexed `poj_notone=toui`. Pre-fix the shadow canonicalize ran
    // `ou→oo` whole-buffer → `tooi` → `poj:tooi` lookup missed → 佗位
    // candidate dropped. Post-fix the shadow stays `toui` and the
    // lattice emits the `poj:toui` full-span phrase key.
    let inv = build_poj_inventory(&["to2", "ui7"]);
    let keys = build_keys_tl_with_inventory("toui", &inv, phonetics::InputMode::Poj);
    let key_strs: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(
        key_strs.iter().any(|k| k == &"poj:toui"),
        "POJ-mode `toui` MUST emit full-span `poj:toui` key, got {key_strs:?}",
    );
    assert!(
        !key_strs.iter().any(|k| k.contains("tooi")),
        "POJ-mode `toui` MUST NOT fold `ou→oo` whole-buffer to `tooi`, got {key_strs:?}",
    );
}

#[test]
fn poj_mode_non_ascii_toui_preserves_token_boundary() {
    // v3.5.9 B-2 PR #309 Codex post-impl SHOULD — non-ASCII parallel of
    // the boundary-preservation test. Input `t\u{f3}u\u{12b}` (POJ
    // `tóuī` typed without hyphen — same buffer as `toui` but with
    // precomposed NFC diacritics). After Phase 1 NFD-walk + tone-mark
    // drop, the intermediate is `toui`; Phase 2 (POJ mode) must use the
    // glyph-only rule subset so `ou→oo` does NOT fire whole-buffer.
    // Shadow stays `toui`, lattice emits `poj:toui` phrase key.
    let inv = build_poj_inventory(&["to2", "ui7"]);
    let keys = build_keys_tl_with_inventory("t\u{f3}u\u{12b}", &inv, phonetics::InputMode::Poj);
    let key_strs: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(
        key_strs.iter().any(|k| k == &"poj:toui"),
        "POJ-mode non-ASCII `tóuī` MUST emit full-span `poj:toui` key, got {key_strs:?}",
    );
    assert!(
        !key_strs.iter().any(|k| k.contains("tooi")),
        "POJ-mode non-ASCII MUST NOT fold `ou→oo` whole-buffer after NFD tone-mark drop, got {key_strs:?}",
    );
}

#[test]
fn tl_mode_ascii_chiah_stays_identity() {
    // Regression guard for the F3C gate: the SAME ASCII `chiah`, in TL
    // mode (`mode = InputMode::Tl`), must NOT be rewritten — it stays
    // `tl:chiah` (no `tl:tsiah`), so a real TL entry whose toneless
    // form legitimately contains `ch`/`oa`/`oe`/`ou` is never garbled.
    let inv = build_inventory(&["tsiah8"]);
    let keys = build_keys_tl_with_inventory("chiah", &inv, phonetics::InputMode::Tl);
    let key_strs: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(
        !key_strs.contains(&"tl:tsiah"),
        "TL-mode ASCII `chiah` must NOT canonicalize to `tl:tsiah`, got {key_strs:?}",
    );
}

#[test]
fn config_input_mode_string_drives_mode_through_key_construction() {
    // Codex post-impl P3 — close the production-plumbing gap: prove the
    // raw `AppConfig.input_mode` STRING ("poj" / "tl") → the same
    // `parse_input_mode` derivation `handle_fetch_at_pos` does → the
    // `mode: InputMode` value that gates key construction. This is the
    // exact chain at the dispatch entry point, reproduced here against
    // the real `phonetics` parser (a full lexicon-installed
    // `FetchAtPos` integration harness does not exist on the composing
    // side, and the bug locus is the key construction, not dict-row
    // resolution).
    //
    // v3.5.9 B-0c: `is_poj: bool` parameter retired in favor of
    // `mode: phonetics::InputMode`; this test now threads `mode`
    // directly (was: `parse_input_mode(...) == InputMode::Poj`).
    let inv = build_inventory(&["tsiah8"]);

    // Mirror of `handle_fetch_at_pos`: parse the config string, thread
    // the resulting `mode` into the production key builder.
    let poj_mode = phonetics::api::parse_input_mode("poj");
    assert_eq!(
        poj_mode,
        phonetics::InputMode::Poj,
        "config `input_mode=\"poj\"` must parse to InputMode::Poj"
    );
    let poj_inv = build_poj_inventory(&["chiah8"]);
    let poj_keys: Vec<String> = build_keys_tl_with_inventory("chiah", &poj_inv, poj_mode)
        .into_iter()
        .map(|(_, k)| k)
        .collect();
    assert!(
        poj_keys.iter().any(|k| k == "poj:chiah"),
        "v3.5.9 B-2 — config `poj` must thread through to `poj:chiah`, got {poj_keys:?}",
    );

    let tl_mode = phonetics::api::parse_input_mode("tl");
    assert_eq!(
        tl_mode,
        phonetics::InputMode::Tl,
        "config `input_mode=\"tl\"` must parse to InputMode::Tl"
    );
    let tl_keys: Vec<String> = build_keys_tl_with_inventory("chiah", &inv, tl_mode)
        .into_iter()
        .map(|(_, k)| k)
        .collect();
    assert!(
        !tl_keys.iter().any(|k| k == "tl:tsiah"),
        "config `tl` must keep the F3C identity (no `tl:tsiah`), got {tl_keys:?}",
    );
}

// ---- Hermetic SyllableInventory builder -----------------------------
// Pattern mirrors `engine/composing/tests/build_keys_tl_hyphen.rs:177-214`.

fn build_inventory(samples: &[&str]) -> SyllableInventory {
    let pairs: Vec<(String, String)> = samples
        .iter()
        .map(|s| {
            canonicalize_syllable(s)
                .unwrap_or_else(|| panic!("sample {s:?} failed canonicalize_syllable"))
        })
        .collect();

    // v3.5.9 B-1: tagged-single-FST — emit keys with `tl:` prefix.
    let mut keys: Vec<String> = Vec::new();
    for (canonical, tone) in &pairs {
        if tone.is_empty() {
            keys.push(format!("tl:{canonical}"));
        } else {
            keys.push(format!("tl:{canonical}{tone}"));
            keys.push(format!("tl:{canonical}"));
        }
    }
    keys.sort();
    keys.dedup();

    let path = unique_temp_path();
    let file = std::fs::File::create(&path).expect("create fst");
    let mut builder = SetBuilder::new(std::io::BufWriter::new(file)).expect("builder");
    for key in &keys {
        builder.insert(key.as_bytes()).expect("insert");
    }
    builder.finish().expect("finish");
    SyllableInventory::open(&path).expect("open inventory")
}

fn unique_temp_path() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("composing-build-keys-tl-poj-{pid}-{n}.fst"))
}

/// v3.5.9 B-2 — POJ family inventory builder. Uses
/// `phonetics::canonicalize_poj_syllable` so emitted keys preserve POJ
/// ASCII (e.g. `poj:chiah` not `poj:tsiah`) — distinct from the TL
/// helper above which folds through the TL chain.
fn build_poj_inventory(samples: &[&str]) -> SyllableInventory {
    use phonetics::canonicalize_poj_syllable;

    let pairs: Vec<(String, String)> = samples
        .iter()
        .map(|s| {
            canonicalize_poj_syllable(s)
                .unwrap_or_else(|| panic!("sample {s:?} failed canonicalize_poj_syllable"))
        })
        .collect();

    let mut keys: Vec<String> = Vec::new();
    for (canonical, tone) in &pairs {
        if tone.is_empty() {
            keys.push(format!("poj:{canonical}"));
        } else {
            keys.push(format!("poj:{canonical}{tone}"));
            keys.push(format!("poj:{canonical}"));
        }
    }
    keys.sort();
    keys.dedup();

    let path = unique_temp_path();
    let file = std::fs::File::create(&path).expect("create fst");
    let mut builder = SetBuilder::new(std::io::BufWriter::new(file)).expect("builder");
    for key in &keys {
        builder.insert(key.as_bytes()).expect("insert");
    }
    builder.finish().expect("finish");
    SyllableInventory::open(&path).expect("open inventory")
}
