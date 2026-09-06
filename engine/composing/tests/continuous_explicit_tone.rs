//! Explicit-tone filtering integration test — covers the bug where a
//! continuous-input query carrying an explicit numeric tone (e.g. `tsua2`)
//! surfaced candidates for EVERY tone of the syllable, because the
//! continuous path stripped the tone digit and looked up the fused
//! toneless FST key (`tl:tsua`). After the fix a fully-toned span keeps
//! its digits (`tl:tsua2`) so `lookup_exact` / `lookup_prefix` filters to
//! exactly the typed tone — `tsua2` → 紙 (tsuá) only, NOT 蛇 (tsuâ).
//!
//! Toneless input is unchanged: `tsua` still surfaces every tone (the
//! continuous-input no-tone affordance). Pure same-syllable homophones
//! (紙/tsuá tone2 vs 蛇/tsuâ tone5) make the tone axis the ONLY difference,
//! so a leak from the span-local, walker slot-0, OR Step 4b prefix-scan
//! paths would show the wrong-tone row and fail the assertion.
//!
//! Hermetic install of `LexiconHandle` comes from `tests/common/mod.rs`
//! (this binary is its own process with its own
//! singleton; the lock guards in-binary `#[test]` parallelism). The
//! fixture's `dictionary.fst` emits BOTH the toneless `tl:<tl_notone>` and
//! the toned `tl:<tl_num>` key families, matching production
//! `dictionary/build/create_fst.py:127-130` — without the toned keys the
//! tone filter would have nothing to hit.

use protos::engine::FetchAtPos;

mod common;
use common::{
    build_dictionary_fst_tl_toned, build_dictionary_fst_tps, build_syllables_fst_tl,
    build_syllables_fst_tps, build_tkdb_v3, config, empty_association_bin, engine_install_lock,
    fetch_at_pos_response, install_lexicon, write_temp, Row,
};

/// 紙/tsuá (tone2) and 蛇/tsuâ (tone5) share the toneless key `tsua` and
/// differ ONLY by tone — the minimal fixture that makes "explicit tone
/// filters" provable. 珠/tsu is an unrelated control row that must never
/// appear for a `tsua*` query.
fn fixture_rows() -> Vec<Row> {
    vec![
        Row {
            toneless_key: "tsua",
            hanzi: "紙",
            tl: "tsuá", // tone 2 → tl_num tsua2
            syll: 1,
            freq: 100,
        },
        Row {
            toneless_key: "tsua",
            hanzi: "蛇",
            tl: "tsuâ", // tone 5 → tl_num tsua5
            syll: 1,
            freq: 70,
        },
        Row {
            toneless_key: "tsu",
            hanzi: "珠",
            tl: "tsu",
            syll: 1,
            freq: 90,
        },
    ]
}

fn install_fixture() {
    let rows = fixture_rows();
    let dict_path = write_temp("dictionary.bin", &build_tkdb_v3(&rows));
    let fst_path = build_dictionary_fst_tl_toned(&rows);
    let assoc_path = write_temp("association.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst_tl(&["tsua2", "tsua5", "tsu"]);
    install_lexicon(&fst_path, &dict_path, &assoc_path, &syllables_path);
}

/// Drive `raw` through `Start → EnterContinuous → FetchAtPos` in TL mode and
/// return the candidate hanji set.
fn fetch_hanji(raw: &str) -> Vec<String> {
    fetch_hanji_in(raw, "tl")
}

/// Mode-parameterized driver shared by the TL and TPS fetch helpers.
fn fetch_hanji_in(raw: &str, input_mode: &str) -> Vec<String> {
    let cfg = config(input_mode);
    let resp = fetch_at_pos_response(&cfg, raw, FetchAtPos::default());
    resp.continuous
        .map(|c| {
            c.candidates
                .into_iter()
                .filter_map(|cand| cand.hanji)
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn explicit_tone_filters_to_typed_tone() {
    let _lock = engine_install_lock();
    install_fixture();
    // The headline bug: `tsua2` (tone 2) must surface 紙 (tsuá) ONLY —
    // 蛇 (tsuâ, tone 5) shares the toneless key `tsua` but is the wrong
    // tone, so it must NOT appear via span-local, walker slot-0, or the
    // Step 4b prefix scan.
    let hanji = fetch_hanji("tsua2");
    assert!(
        hanji.iter().any(|h| h == "紙"),
        "tsua2 must surface 紙 (tsuá); got {hanji:?}"
    );
    assert!(
        !hanji.iter().any(|h| h == "蛇"),
        "tsua2 must NOT surface 蛇 (tsuâ, wrong tone); got {hanji:?}"
    );

    // Symmetric: tone 5 surfaces 蛇 only.
    let hanji5 = fetch_hanji("tsua5");
    assert!(
        hanji5.iter().any(|h| h == "蛇"),
        "tsua5 must surface 蛇 (tsuâ); got {hanji5:?}"
    );
    assert!(
        !hanji5.iter().any(|h| h == "紙"),
        "tsua5 must NOT surface 紙 (tsuá, wrong tone); got {hanji5:?}"
    );
}

#[test]
fn toneless_input_still_surfaces_all_tones() {
    let _lock = engine_install_lock();
    install_fixture();
    // Regression guard for the no-tone affordance: toneless `tsua` must
    // still surface BOTH 紙 (tsuá) and 蛇 (tsuâ). The fix only changes
    // fully-toned spans; toneless input keeps the all-tone fused-key
    // behavior.
    let hanji = fetch_hanji("tsua");
    assert!(
        hanji.iter().any(|h| h == "紙") && hanji.iter().any(|h| h == "蛇"),
        "toneless tsua must surface both 紙 and 蛇; got {hanji:?}"
    );
}

#[test]
fn longest_match_suppresses_shorter_prefix_syllable() {
    let _lock = engine_install_lock();
    install_fixture();
    // INVARIANT_CONTINUOUS_LONGEST_MATCH_PREFIX (USER 2026-05-31「免調也壓制」).
    // 珠/tsu is a SHORTER single syllable that is a strict prefix of tsua —
    // the reported bug's `ta` ⊂ `tai` / `tai5` shape. The span-local candidate
    // strip surfaces only the LONGEST single syllable at offset 0, so 珠 (tsu,
    // span 0–3) must NOT appear for either a toned or a toneless tsua* query.
    // The fixture's `syllables.fst` carries `tsua2` (build_syllables_fst), so
    // `tsua2` resolves to the toned single syllable exactly as production does.
    //
    // Toned: `tsua2` keeps 紙 (tsuá, tone 2) and drops BOTH 蛇 (wrong tone,
    // §17) AND 珠 (shorter prefix syllable, §18).
    let toned = fetch_hanji("tsua2");
    assert!(
        toned.iter().any(|h| h == "紙"),
        "tsua2 must surface 紙 (tsuá); got {toned:?}"
    );
    assert!(
        !toned.iter().any(|h| h == "珠"),
        "tsua2 must NOT surface 珠 (shorter prefix syllable); got {toned:?}"
    );

    // Toneless: `tsua` surfaces all tones of the longest syllable (紙 + 蛇)
    // but still drops the shorter 珠 — the suppression keys on span length,
    // not tone.
    let toneless = fetch_hanji("tsua");
    assert!(
        toneless.iter().any(|h| h == "紙") && toneless.iter().any(|h| h == "蛇"),
        "toneless tsua must surface both 紙 and 蛇; got {toneless:?}"
    );
    assert!(
        !toneless.iter().any(|h| h == "珠"),
        "toneless tsua must NOT surface 珠 (shorter prefix syllable); got {toneless:?}"
    );
}

// ----- B2 (§17 TPS) — explicit-tone filtering for TPS Bopomofo input -----
//
// TPS tones are Bopomofo diacritic scalars, not ASCII digits, so #367 left
// TPS on the unconditional toneless strip — every tone of a syllable surfaced
// regardless of the typed tone mark. B2 extends the filter to TPS: a span
// closed by a mark-bearing tone (2/3/5/6/7/8/9) keeps its mark and queries the
// `tps:<tps_num>` family. The fixture's TPS keys are DERIVED from the same TL
// readings via `phonetics::tps_num_from_tl` / `tps_notone_from_tl` (the runtime
// mirrors of the build pipeline), so the test cannot drift from production key
// shapes. 紙/tsuá (tone 2) vs 蛇/tsuâ (tone 5) differ ONLY by the tone mark.

fn install_fixture_tps() {
    let rows = fixture_rows();
    let dict_path = write_temp("dictionary-tps.bin", &build_tkdb_v3(&rows));
    let fst_path = build_dictionary_fst_tps(&rows);
    let assoc_path = write_temp("association-tps.bin", &empty_association_bin());
    let syllables_path = build_syllables_fst_tps(&rows);
    install_lexicon(&fst_path, &dict_path, &assoc_path, &syllables_path);
}

fn fetch_hanji_tps(raw: &str) -> Vec<String> {
    fetch_hanji_in(raw, "tps")
}

#[test]
fn explicit_tone_filters_to_typed_tone_tps() {
    let _lock = engine_install_lock();
    install_fixture_tps();
    // tone-2 TPS input (紙/tsuá) must surface 紙 ONLY — 蛇 (tsuâ, tone 5)
    // shares the toneless key but is the wrong tone.
    let tone2 = phonetics::tps_num_from_tl("tsuá");
    let hanji = fetch_hanji_tps(&tone2);
    assert!(
        hanji.iter().any(|h| h == "紙"),
        "TPS {tone2:?} (tsuá) must surface 紙; got {hanji:?}"
    );
    assert!(
        !hanji.iter().any(|h| h == "蛇"),
        "TPS {tone2:?} must NOT surface 蛇 (wrong tone); got {hanji:?}"
    );

    // Symmetric: tone-5 surfaces 蛇 only.
    let tone5 = phonetics::tps_num_from_tl("tsuâ");
    let hanji5 = fetch_hanji_tps(&tone5);
    assert!(
        hanji5.iter().any(|h| h == "蛇"),
        "TPS {tone5:?} (tsuâ) must surface 蛇; got {hanji5:?}"
    );
    assert!(
        !hanji5.iter().any(|h| h == "紙"),
        "TPS {tone5:?} must NOT surface 紙 (wrong tone); got {hanji5:?}"
    );
}

#[test]
fn toneless_input_still_surfaces_all_tones_tps() {
    let _lock = engine_install_lock();
    install_fixture_tps();
    // No-tone affordance preserved for TPS: toneless Bopomofo surfaces every
    // tone (both 紙 and 蛇).
    let toneless = phonetics::tps_notone_from_tl("tsuá");
    let hanji = fetch_hanji_tps(&toneless);
    assert!(
        hanji.iter().any(|h| h == "紙") && hanji.iter().any(|h| h == "蛇"),
        "toneless TPS {toneless:?} must surface both 紙 and 蛇; got {hanji:?}"
    );
}
