//! D1 — a `CommitPrediction` has the engine resolve what a NextWord
//! prediction tap writes (`composing::commit_text`, the Continuous pick's
//! resolver) under the request's output settings, report it
//! (`ComposingResponse.commit`) and hand back the usage (`Applied.usage`),
//! leaving the composition untouched. Expected values are the outputs of the
//! removed phone resolvers (D2) — `formatOutputText` / `resolveUnmarkedCommit`
//! (unmarked cells) and `markedCellCommit` / `resolveMarkedCellCommit` (§42
//! split cells) — except the empty-bracket row (parity correction toward the
//! engine and the desktops).

use composing::{Applied, EngineHandle, Usage};
use protos::engine::composing_request::Method;
use protos::engine::{
    AppConfig, CandidateDisplayMode, CommitOutcome, CommitPrediction, CommitResolution,
    CommitScript, Start,
};

use crate::common::{config_tl, ignored, req, resolution};

const TL: &str = "tâi-gí";
const HANJI: &str = "台語";

fn send(handle: &EngineHandle, method: Method, config: &AppConfig) -> Applied {
    handle
        .handle_learning(&req(method), config, 1)
        .expect("composing request")
}

fn tap(script: CommitScript, roman: &str, hanji: &str) -> Method {
    Method::CommitPrediction(CommitPrediction {
        script: script as i32,
        roman: roman.into(),
        hanji: hanji.into(),
        canonical_tl: TL.into(),
    })
}

fn config(swapped: bool, brackets: bool, display: CandidateDisplayMode) -> AppConfig {
    AppConfig {
        is_hanji_first: swapped,
        output_both_scripts: brackets,
        candidate_display_mode: display as i32,
        ..config_tl()
    }
}

fn written(text: &str, roman: bool, space: bool) -> CommitResolution {
    resolution(CommitOutcome::Finalized, text, roman, space)
}

#[test]
fn a_prediction_tap_writes_what_the_phones_wrote() {
    use CandidateDisplayMode::{Combined, RomanOnly, SideBySide};
    use CommitScript::{Hanji, Lead, Roman};
    // (script, swapped, brackets, display, roman, expected)
    #[rustfmt::skip]
    let rows = [
        // trace: formatOutputText / resolveUnmarkedCommit — roman-led writes
        // the romanization and earns the space; Hanji-led writes the Hanji.
        (Lead, false, false, SideBySide, TL, written(TL, true, true)),
        (Lead, true, false, SideBySide, TL, written(HANJI, false, false)),
        // Annotate in Brackets writes the pair either way round (§23).
        (Lead, true, true, SideBySide, TL, written("台語 (tâi-gí)", true, true)),
        (Lead, false, true, SideBySide, TL, written("tâi-gí (台語)", true, true)),
        // Was `台語 ()` + a space on both phones (the bracket arm only checked
        // the Hanji); the engine writes no empty brackets.
        (Lead, true, true, SideBySide, "", written(HANJI, false, false)),
        // A Hanji-only prediction is listed only Hanji-led (`shape_prediction`).
        (Lead, true, false, SideBySide, "", written(HANJI, false, false)),
        // Romanization Only masks the brackets (`commit_text::OutputScripts`).
        (Lead, false, true, RomanOnly, TL, written(TL, true, true)),
        // The Syllable Separator rendering rides `roman` as the strip shows it.
        (Lead, false, false, SideBySide, "tâi gí", written("tâi gí", true, true)),
        // A trailing hyphen earns no space, as `shouldAppendAutoSpace`.
        (Lead, false, false, SideBySide, "tâi-", written("tâi-", true, false)),
        // trace: markedCellCommit / resolveMarkedCellCommit — §42 split cells.
        (Hanji, true, false, Combined, TL, written(HANJI, false, false)),
        (Hanji, true, true, Combined, TL, written("台語 (tâi-gí)", true, true)),
        (Hanji, true, true, Combined, "", written(HANJI, false, false)),
        (Roman, true, true, Combined, TL, written(TL, true, true)),
        // A split cell whose own script is missing commits the lead, as the
        // phones' `defectiveMarkers_failOpenToTheUnmarkedPath` does.
        (Roman, true, false, Combined, "", written(HANJI, false, false)),
    ];
    for (script, swapped, brackets, display, roman, expected) in rows {
        let config = config(swapped, brackets, display);
        let applied = send(&EngineHandle::new(), tap(script, roman, HANJI), &config);
        assert_eq!(
            applied.response.commit,
            Some(expected),
            "{script:?} swapped={swapped} brackets={brackets} {display:?} {roman:?}"
        );
    }
}

#[test]
fn a_poj_prediction_writes_its_poj_romanization() {
    // trace: `nextword::filter::shape_prediction` renders `tsiâ` → POJ
    // `chiâ` in POJ mode; the tap writes that rendering, never the TL.
    let config = AppConfig {
        input_mode: "poj".into(),
        ..config(false, false, CandidateDisplayMode::SideBySide)
    };
    let applied = send(
        &EngineHandle::new(),
        tap(CommitScript::Lead, "chiâ", "誠"),
        &config,
    );
    assert_eq!(applied.response.commit, Some(written("chiâ", true, true)));
}

#[test]
fn a_tps_prediction_brackets_its_bopomofo() {
    // trace: iOS / Android `bracketRoman = tlDisplayToTps(roman)` under the
    // TPS layout, which leads with the Hanji; to_tone_number("tâi-gí") =
    // "tai5-gi2" → ㄉㄞˊ ㆣㄧˋ (commit_text `tps_truth_table`).
    let tps = |brackets| AppConfig {
        input_mode: "tps".into(),
        ..config(false, brackets, CandidateDisplayMode::SideBySide)
    };
    let applied = send(
        &EngineHandle::new(),
        tap(CommitScript::Lead, TL, HANJI),
        &tps(true),
    );
    let expected = written("台語 (ㄉㄞˊ ㆣㄧˋ)", true, true);
    assert_eq!(applied.response.commit, Some(expected));
    let applied = send(
        &EngineHandle::new(),
        tap(CommitScript::Lead, TL, HANJI),
        &tps(false),
    );
    assert_eq!(applied.response.commit, Some(written(HANJI, false, false)));
}

#[test]
fn a_written_prediction_counts_its_usage_without_a_learned_phrase() {
    // trace: the phones' `UsageRecorder.record(Usage(displayText: hanji,
    // canonicalTl: tl))` → `RecordUsage` with no Hanji, so no learned-phrase
    // touch (`userdata` `record_usage`); identity is `(hanji, canonical TL)`.
    let applied = send(
        &EngineHandle::new(),
        tap(CommitScript::Lead, TL, HANJI),
        &config_tl(),
    );
    let expected = Usage {
        display_text: HANJI.into(),
        canonical_tl: TL.into(),
        hanji: None,
    };
    assert_eq!(applied.usage, Some(expected));
    assert_eq!(applied.learned, None);
}

#[test]
fn a_tap_the_engine_cannot_resolve_is_ignored_and_counts_nothing() {
    let roman_led = config(false, false, CandidateDisplayMode::SideBySide);
    let cases = [
        // No script (UNSPECIFIED, or one newer than the engine).
        (tap(CommitScript::Unspecified, TL, HANJI), roman_led.clone()),
        // No Hanji: a prediction always carries one (`filter.rs` drops the rest).
        (tap(CommitScript::Lead, TL, ""), roman_led.clone()),
        // A Hanji-only prediction once the swap is off: nothing to write.
        (tap(CommitScript::Lead, "", HANJI), roman_led),
        // A split cell whose script is missing under Romanization Only.
        (
            tap(CommitScript::Other, TL, HANJI),
            config(true, false, CandidateDisplayMode::RomanOnly),
        ),
    ];
    for (method, config) in cases {
        let applied = send(&EngineHandle::new(), method.clone(), &config);
        assert_eq!(applied.response.commit, Some(ignored()), "{method:?}");
        assert_eq!(applied.usage, None, "{method:?}");
    }
}

#[test]
fn a_prediction_tap_leaves_the_composition_untouched() {
    // A prediction is listed only while Idle; were one tapped mid-composition,
    // the engine still writes nothing into the composition.
    let config = config_tl();
    let handle = EngineHandle::new();
    send(
        &handle,
        Method::Start(Start { text: "tai".into() }),
        &config,
    );
    let applied = send(&handle, tap(CommitScript::Lead, TL, HANJI), &config);
    assert_eq!(applied.response.commit, Some(written(TL, true, true)));
    assert!(applied.response.effect.is_empty());
    assert!(applied.response.is_composing);
    assert_eq!(
        applied.response.preedit.map(|preedit| preedit.raw_input),
        Some("tai".into())
    );
}

#[test]
fn a_prediction_tap_on_a_new_generation_resets_the_composition_first() {
    // trace: `EngineHandle::handle_ranked` drops a composition on a new
    // generation before any mutating intent runs; the tap is then written
    // from Idle, as every request meets a field switch.
    let config = config_tl();
    let handle = EngineHandle::new();
    send(
        &handle,
        Method::Start(Start { text: "tai".into() }),
        &config,
    );
    let applied = handle
        .handle_learning(&req(tap(CommitScript::Lead, TL, HANJI)), &config, 2)
        .expect("composing request");
    assert_eq!(applied.response.commit, Some(written(TL, true, true)));
    assert!(!applied.response.is_composing);
}
