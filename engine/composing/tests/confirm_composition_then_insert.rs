//! The commit-and-insert wire operation owns the final document write and
//! preserves candidate learning independently of the appended literal.

use composing::{requests, Engine, Intent, Phase};
use protos::engine::{
    composing_request::Method, effect, AppConfig, CommitContinuous, CommitOutcome, CommitScript,
    ComposingResponse, ConfirmCompositionThenInsert,
};

use crate::common::{commit_text, config_tl, req};

fn pick(consumed_bytes: u32) -> CommitContinuous {
    CommitContinuous {
        consumed_bytes,
        syllable_count: 1,
        canonical_text: "台".into(),
        association_tl: "tâi".into(),
        hanji: Some("台".into()),
        script: CommitScript::Lead as i32,
        roman: "tâi".into(),
    }
}

fn insert(
    engine: &mut Engine,
    candidate: Option<CommitContinuous>,
    text: &str,
    auto_space_enabled: bool,
    config: &AppConfig,
) -> composing::Applied {
    let intent = requests::decode_intent(&req(Method::ConfirmCompositionThenInsert(
        ConfirmCompositionThenInsert {
            candidate,
            text: text.into(),
            auto_space_enabled,
        },
    )))
    .unwrap();
    engine.apply_learning(intent, config)
}

fn start(engine: &mut Engine, raw: &str, config: &AppConfig) {
    engine.apply(Intent::Start { text: raw.into() }, config);
}

fn assert_one_write(response: &ComposingResponse, expected: &str) {
    assert_eq!(commit_text(response).as_deref(), Some(expected));
    assert_eq!(
        response
            .effect
            .iter()
            .filter(|effect| matches!(
                effect.kind,
                Some(effect::Kind::CommitTextReplacingPreedit(_))
            ))
            .count(),
        1
    );
    assert!(
        !response.effect.iter().any(|effect| matches!(
            effect.kind,
            Some(effect::Kind::UpdatePreedit(_) | effect::Kind::RefreshCandidates(_))
        )),
        "no intermediate preedit or list refetch escapes the transition"
    );
    assert!(!response.is_composing);
    assert_eq!(
        response.commit.as_ref().unwrap().outcome(),
        CommitOutcome::Finalized
    );
}

#[test]
fn full_pick_keeps_its_identity_and_learning_separate_from_the_literal() {
    for (hanji_first, auto_space, text, expected, leaves_space) in [
        (true, true, "？", "台？", false),
        (false, false, "?", "tâi?", false),
        (false, true, "?", "tâi? ", true),
        (false, true, "(", "tâi (", false),
        (false, true, " ", "tâi ", true),
    ] {
        let config = AppConfig {
            is_hanji_first: hanji_first,
            ..config_tl()
        };
        let mut engine = Engine::new();
        start(&mut engine, "tai", &config);
        let applied = insert(&mut engine, Some(pick(3)), text, auto_space, &config);
        assert_one_write(&applied.response, expected);
        assert_eq!(engine.snapshot_state().phase, Phase::Idle);
        let usage = applied.usage.unwrap();
        assert_eq!(
            (usage.display_text.as_str(), usage.canonical_tl.as_str()),
            ("台", "tâi")
        );
        assert_eq!(usage.hanji.as_deref(), Some("台"));
        assert!(
            applied.learned.is_none(),
            "one picked syllable teaches no phrase"
        );
        let resolution = applied.response.commit.unwrap();
        assert_eq!(resolution.leaves_trailing_auto_space, leaves_space);
        let learned_word = applied
            .response
            .effect
            .iter()
            .find_map(|effect| match &effect.kind {
                Some(effect::Kind::NextWordWordSelected(word)) => Some(word),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            (learned_word.text.as_str(), learned_word.roman.as_str()),
            ("台", "tâi")
        );
    }
}

#[test]
fn partial_pick_finishes_the_tail_without_teaching_an_unpicked_final_word() {
    let config = AppConfig {
        is_hanji_first: true,
        ..config_tl()
    };
    let mut engine = Engine::new();
    start(&mut engine, "taigi", &config);
    let applied = insert(&mut engine, Some(pick(3)), "？", true, &config);
    assert_one_write(&applied.response, "台gi？ ");
    assert_eq!(applied.usage.unwrap().display_text, "台");
    assert!(applied.learned.is_none());
    assert!(!applied
        .response
        .effect
        .iter()
        .any(|effect| matches!(effect.kind, Some(effect::Kind::NextWordWordSelected(_)))));
    assert!(applied.response.effect.iter().any(|effect| matches!(
        effect.kind,
        Some(effect::Kind::NextWordClearForNewComposing(_))
    )));
}

#[test]
fn absent_candidate_commits_the_preedit_without_candidate_usage() {
    let config = config_tl();
    let mut engine = Engine::new();
    start(&mut engine, "tai-", &config);
    let applied = insert(&mut engine, None, "?", true, &config);
    assert_one_write(&applied.response, "tai-?");
    assert!(!applied.response.commit.unwrap().leaves_trailing_auto_space);
    assert!(applied.usage.is_none() && applied.learned.is_none());
}

#[test]
fn invalid_pick_and_empty_literal_leave_the_composition_and_learning_untouched() {
    let config = config_tl();
    let mut engine = Engine::new();
    start(&mut engine, "tai", &config);
    let mut missing_script = pick(3);
    missing_script.script = CommitScript::Unspecified as i32;
    for (candidate, text) in [(pick(1000), "?"), (missing_script, "?"), (pick(3), "")] {
        let before = engine.snapshot_state();
        let applied = insert(&mut engine, Some(candidate), text, true, &config);
        assert!(applied.response.effect.is_empty());
        assert_eq!(
            applied.response.commit.unwrap().outcome(),
            CommitOutcome::Ignored
        );
        assert_eq!(engine.snapshot_state(), before);
        assert!(applied.usage.is_none() && applied.learned.is_none());
    }
}

#[test]
fn idle_rejects_a_stale_candidate_without_inserting_the_literal() {
    let mut engine = Engine::new();
    let applied = insert(&mut engine, Some(pick(3)), "?", true, &config_tl());
    assert!(applied.response.effect.is_empty());
    assert_eq!(
        applied.response.commit.unwrap().outcome(),
        CommitOutcome::Ignored
    );
    assert!(applied.usage.is_none() && applied.learned.is_none());
}

#[test]
fn tps_pick_finishes_the_converted_preedit_without_an_intermediate_update() {
    let _lock = test_support::engine_install_lock();
    crate::tps_hanji_conversion::install_fixture_with(&[("絲", "si", 10)]);
    let config = crate::common::config_converting("tps");
    let (mut engine, _) = crate::tps_hanji_conversion::composing_engine("ㄒㄧˋㄒㄧ ", &config);
    let candidate = CommitContinuous {
        consumed_bytes: 15,
        syllable_count: 1,
        canonical_text: "絲".into(),
        association_tl: "si".into(),
        hanji: Some("絲".into()),
        script: CommitScript::Lead as i32,
        roman: "si".into(),
    };
    let applied = insert(&mut engine, Some(candidate), "？", true, &config);
    assert_one_write(&applied.response, "死絲？");
    assert!(
        !applied
            .response
            .commit
            .as_ref()
            .unwrap()
            .leaves_trailing_auto_space
    );
    assert_eq!(applied.usage.unwrap().display_text, "絲");
    assert!(
        applied.learned.is_none(),
        "the preceding converted word was not picked"
    );
    assert!(selected_words(&applied.response).is_empty());
}

#[test]
fn generation_mismatch_rejects_the_pick_and_literal_together() {
    let engine = composing::EngineHandle::new();
    let config = config_tl();
    engine
        .handle(
            &req(Method::Start(protos::engine::Start { text: "tai".into() })),
            &config,
            1,
        )
        .unwrap();
    let applied = engine
        .handle_learning(
            &req(Method::ConfirmCompositionThenInsert(
                ConfirmCompositionThenInsert {
                    candidate: Some(pick(3)),
                    text: "?".into(),
                    auto_space_enabled: true,
                },
            )),
            &config,
            2,
        )
        .unwrap();
    assert!(applied.response.effect.is_empty());
    assert_eq!(
        applied.response.commit.unwrap().outcome(),
        CommitOutcome::Ignored
    );
    assert!(applied.usage.is_none() && applied.learned.is_none());
}

fn tps_pick() -> CommitContinuous {
    CommitContinuous {
        consumed_bytes: 7,
        syllable_count: 1,
        canonical_text: "詩".into(),
        association_tl: "si".into(),
        hanji: Some("詩".into()),
        script: CommitScript::Lead as i32,
        roman: "si".into(),
    }
}

fn accept_tps_pick(engine: &mut Engine, config: &AppConfig) {
    requests::handle(&req(Method::CommitContinuous(tps_pick())), engine, config).unwrap();
}

fn selected_words(response: &ComposingResponse) -> Vec<&protos::engine::NextWordWordSelected> {
    response
        .effect
        .iter()
        .filter_map(|effect| match &effect.kind {
            Some(effect::Kind::NextWordWordSelected(word)) => Some(word),
            _ => None,
        })
        .collect()
}

#[test]
fn tps_confirmation_preserves_the_final_picks_word_learning() {
    let _lock = test_support::engine_install_lock();
    crate::tps_hanji_conversion::install_fixture_with(&[]);
    let config = crate::common::config_converting("tps");
    for accept_before_confirmation in [false, true] {
        let (mut engine, _) = crate::tps_hanji_conversion::composing_engine("ㄒㄧ ", &config);
        let candidate = if accept_before_confirmation {
            accept_tps_pick(&mut engine, &config);
            None
        } else {
            Some(tps_pick())
        };
        let applied = insert(&mut engine, candidate, "？", true, &config);
        assert_one_write(&applied.response, "詩？");
        assert!(
            !applied
                .response
                .commit
                .as_ref()
                .unwrap()
                .leaves_trailing_auto_space
        );
        assert_eq!(applied.usage.is_some(), !accept_before_confirmation);
        assert!(applied.learned.is_none(), "one word teaches no phrase");
        let words = selected_words(&applied.response);
        assert_eq!(
            words.len(),
            1,
            "the final picked word must be reported once"
        );
        assert_eq!(
            (words[0].text.as_str(), words[0].roman.as_str()),
            ("詩", "si")
        );
        assert!(words[0].preceding.is_empty());
    }
}

#[test]
fn tps_confirmation_preserves_the_fully_picked_phrases_learning() {
    let _lock = test_support::engine_install_lock();
    crate::tps_hanji_conversion::install_fixture_with(&[]);
    let config = crate::common::config_converting("tps");
    let (mut engine, _) = crate::tps_hanji_conversion::composing_engine("ㄒㄧ ㄒㄧ ", &config);
    // Pick the first word explicitly: selecting only the final converted
    // word would nail the first as shown and correctly suppress learning.
    engine.apply(
        Intent::MoveCaret {
            direction: Some(composing::api::CaretDirection::Left),
        },
        &config,
    );
    accept_tps_pick(&mut engine, &config);
    let mut ordinary = engine.clone();
    accept_tps_pick(&mut ordinary, &config);
    let expected = ordinary.apply_learning(Intent::CommitAsShown, &config);
    assert_eq!(
        expected.learned,
        Some(lexicon::LearnedEntry {
            hanji: "詩詩".into(),
            canonical_tl: "si si".into()
        })
    );

    let applied = insert(&mut engine, Some(tps_pick()), "？", true, &config);
    assert_one_write(&applied.response, "詩詩？");
    assert_eq!(applied.learned, expected.learned);
    assert_eq!(
        selected_words(&applied.response),
        selected_words(&expected.response)
    );
    let words = selected_words(&applied.response);
    assert_eq!(words[0].preceding.len(), 1);
    assert_eq!(
        (
            words[0].preceding[0].text.as_str(),
            words[0].preceding[0].roman.as_str()
        ),
        ("詩", "si")
    );
    assert_eq!(applied.usage.unwrap().canonical_tl, "si");
}

#[test]
fn tps_confirmation_does_not_learn_an_unpicked_pending_tail() {
    let _lock = test_support::engine_install_lock();
    crate::tps_hanji_conversion::install_fixture_with(&[]);
    let config = crate::common::config_converting("tps");
    let (mut engine, _) = crate::tps_hanji_conversion::composing_engine("ㄒㄧ ㄒㄧ ", &config);
    engine.apply(
        Intent::MoveCaret {
            direction: Some(composing::api::CaretDirection::Left),
        },
        &config,
    );
    let applied = insert(&mut engine, Some(tps_pick()), "？", true, &config);
    assert_one_write(&applied.response, "詩詩？");
    assert!(applied.learned.is_none());
    assert!(selected_words(&applied.response).is_empty());
    assert_eq!(applied.usage.unwrap().canonical_tl, "si");
}
