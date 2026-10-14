//! R5 — a `CommitContinuous` has the engine resolve the document text
//! (`composing::commit_text`) from the script it names, report what the
//! commit did (`ComposingResponse.commit`) and hand back the pick's usage
//! (`Applied.usage`); one naming no script is ignored.
//! Through `EngineHandle`, so a stale generation is exercised as the
//! platforms meet it.

use composing::{Applied, EngineHandle, Usage};
use protos::engine::composing_request::Method;
use protos::engine::{AppConfig, CommitContinuous, CommitOutcome, CommitScript, Start};

use crate::common::{commit_text, config_tl, ignored, req, resolution};

fn send(handle: &EngineHandle, generation: u64, method: Method, config: &AppConfig) -> Applied {
    handle
        .handle_learning(&req(method), config, generation)
        .expect("composing request")
}

/// A handle in `Phase::Continuous` over `raw` at `generation`.
fn continuous(raw: &str, generation: u64, config: &AppConfig) -> EngineHandle {
    let handle = EngineHandle::new();
    let start = Method::Start(Start { text: raw.into() });
    send(&handle, generation, start, config);
    handle
}

/// A pick of `(roman, hanji)` whose identity is `hanji ?? roman` / `tl`.
fn pick(script: CommitScript, roman: &str, hanji: Option<&str>, tl: &str, bytes: u32) -> Method {
    Method::CommitContinuous(CommitContinuous {
        canonical_text: hanji.unwrap_or(roman).into(),
        association_tl: tl.into(),
        hanji: hanji.map(str::to_owned),
        consumed_bytes: bytes,
        syllable_count: 1,
        script: script as i32,
        roman: roman.into(),
    })
}

fn usage(display_text: &str, canonical_tl: &str, hanji: Option<&str>) -> Option<Usage> {
    Some(Usage {
        display_text: display_text.into(),
        canonical_tl: canonical_tl.into(),
        hanji: hanji.map(str::to_owned),
    })
}

#[test]
fn a_nail_then_a_final_pick_report_their_outcomes() {
    // trace: roman-led TL; `taigi` = 台 `tâi` (3 bytes) + 語 `gí` (2 bytes).
    // The nail writes nothing to the document; the final commit writes the
    // composition `tâi gí` (roman-ish word space, no lexicon compound) and
    // earns the space, as iOS / Android `didFinalCommit` +
    // `shouldAppendAutoSpace` do.
    let config = config_tl();
    let handle = continuous("taigi", 1, &config);

    let nailed = send(
        &handle,
        1,
        pick(CommitScript::Lead, "tâi", Some("台"), "tâi", 3),
        &config,
    );
    let expected = resolution(CommitOutcome::Nailed, "tâi", true, false);
    assert_eq!(nailed.response.commit, Some(expected));
    assert_eq!(nailed.usage, usage("台", "tâi", Some("台")));
    assert!(nailed.response.is_composing);
    assert_eq!(commit_text(&nailed.response), None);

    let finalized = send(
        &handle,
        1,
        pick(CommitScript::Lead, "gí", Some("語"), "gí", 2),
        &config,
    );
    let expected = resolution(CommitOutcome::Finalized, "gí", true, true);
    assert_eq!(finalized.response.commit, Some(expected));
    assert_eq!(finalized.usage, usage("語", "gí", Some("語")));
    assert!(!finalized.response.is_composing);
    assert_eq!(commit_text(&finalized.response).as_deref(), Some("tâi gí"));
}

#[test]
fn a_hanji_led_final_pick_writes_the_hanji_and_earns_no_space() {
    // trace: §23 row "Hanji candidate, Hanji-led, brackets OFF".
    let config = AppConfig {
        is_hanji_first: true,
        ..config_tl()
    };
    let handle = continuous("tai", 1, &config);
    let applied = send(
        &handle,
        1,
        pick(CommitScript::Lead, "tâi", Some("台"), "tâi", 3),
        &config,
    );
    let expected = resolution(CommitOutcome::Finalized, "台", false, false);
    assert_eq!(applied.response.commit, Some(expected));
    assert_eq!(commit_text(&applied.response).as_deref(), Some("台"));
}

#[test]
fn a_hyphen_tail_earns_no_space() {
    // trace: the §34 literal `tâi-` (no Hanji) consumes `tai-` whole; the
    // document string ends in `-`, which a platform never spaces after.
    let config = config_tl();
    let handle = continuous("tai-", 1, &config);
    let applied = send(
        &handle,
        1,
        pick(CommitScript::Lead, "tâi-", None, "tâi-", 4),
        &config,
    );
    let expected = resolution(CommitOutcome::Finalized, "tâi-", true, false);
    assert_eq!(applied.response.commit, Some(expected));
    assert_eq!(applied.usage, usage("tâi-", "tâi-", None));
}

#[test]
fn a_stale_generation_is_ignored_and_counts_nothing() {
    // trace: `EngineHandle::handle_learning` resets to Idle on a new
    // generation before the commit runs — the noop iOS reads as
    // `didCommit == false`, desktop as `CandidateCommitOutcome::Ignored`.
    let config = config_tl();
    let handle = continuous("tai", 1, &config);
    let applied = send(
        &handle,
        2,
        pick(CommitScript::Lead, "tâi", Some("台"), "tâi", 3),
        &config,
    );
    assert_eq!(applied.response.commit, Some(ignored()));
    assert_eq!(applied.usage, None);
    assert!(applied.response.effect.is_empty());
}

#[test]
fn a_rejected_pick_is_ignored_and_changes_nothing() {
    // trace: Space on a one-script cell (desktop `resolved_alternate` →
    // None → `Ignored` without reaching the engine), and a pick without the
    // identity key (no fallback on the R5 path).
    let config = config_tl();
    let handle = continuous("tai", 1, &config);
    let no_other_script = pick(CommitScript::Other, "tâi", None, "tâi", 3);
    let mut no_identity = pick(CommitScript::Lead, "tâi", Some("台"), "tâi", 3);
    if let Method::CommitContinuous(commit) = &mut no_identity {
        commit.canonical_text.clear();
    }
    for method in [no_other_script, no_identity] {
        let applied = send(&handle, 1, method, &config);
        assert_eq!(applied.response.commit, Some(ignored()));
        assert_eq!(applied.usage, None);
        assert!(applied.response.effect.is_empty());
        assert!(
            applied.response.is_composing,
            "the composition is untouched"
        );
    }
    let applied = send(
        &handle,
        1,
        pick(CommitScript::Other, "tâi", Some("台"), "tâi", 3),
        &config,
    );
    let expected = resolution(CommitOutcome::Finalized, "台", false, false);
    assert_eq!(applied.response.commit, Some(expected));
    // The other script counts under the same `(hanji ?? roman, TL)` pair as
    // the lead: identity never follows the rendering (Core Principle #6).
    assert_eq!(applied.usage, usage("台", "tâi", Some("台")));
}

#[test]
fn a_tps_pick_without_hanji_writes_its_bopomofo_and_earns_no_space() {
    // trace: R5 P2 (USER 2026-09-30) — the TPS layout writes what the cell
    // shows, `tl_display_to_tps("tâi")` = ㄉ + ㄞ + ˊ, which takes no word
    // spacing; the pick still counts under its TL identity.
    let config = AppConfig {
        input_mode: "tps".into(),
        ..config_tl()
    };
    let handle = continuous("tai", 1, &config);
    let applied = send(
        &handle,
        1,
        pick(CommitScript::Lead, "tâi", None, "tâi", 3),
        &config,
    );
    let expected = resolution(CommitOutcome::Finalized, "ㄉㄞˊ", false, false);
    assert_eq!(applied.response.commit, Some(expected));
    assert_eq!(commit_text(&applied.response).as_deref(), Some("ㄉㄞˊ"));
    assert_eq!(applied.usage, usage("tâi", "tâi", None));
}

#[test]
fn a_commit_naming_no_script_is_ignored() {
    // trace: `requests::commit_script` reads UNSPECIFIED (and an unknown,
    // newer script) as `None`; `transition::commit_continuous` then resolves
    // nothing — the legacy platform-written path is gone (R5 PR-b2).
    let config = config_tl();
    let handle = continuous("tai", 1, &config);
    let applied = send(
        &handle,
        1,
        pick(CommitScript::Unspecified, "tâi", Some("台"), "tâi", 3),
        &config,
    );
    assert_eq!(applied.response.commit, Some(ignored()));
    assert_eq!(applied.usage, None);
    assert!(applied.response.effect.is_empty());
    assert!(
        applied.response.is_composing,
        "the composition is untouched"
    );
}
