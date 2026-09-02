//! The composing orchestration against the REAL engine and dictionaries, with
//! in-memory stores and a recording executor — the port of macOS's
//! `ComposingManagerTests` / `ComposingManagerCandidateTests` /
//! `ComposingManagerLearningTests` / `ComposingSessionCoordinatorTests`.
//!
//! Same singleton discipline as `engine_roundtrip.rs`: one lock, one fresh
//! generation block per test.

// 中文: 組字管理者對真實引擎的整合測試;記憶體儲存 + 錄影執行器;與 macOS 測試逐案對應。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use taigi_windows_core::composing::{
    AssociationSink, CandidateCommitOutcome, CandidateFetchOutcome, CandidateScript, Clock,
    ComposingEffectExecutor, ComposingManager, ComposingSessionCoordinator, CustomDictionarySource,
    FrequencySource, NextWordLearner,
};
use taigi_windows_core::dictionary_artifacts::DictionaryArtifacts;
use taigi_windows_core::engine::{
    self, AssociationPair, ContinuousCandidate, CustomEntry, Effect, FrequencyRow,
};
use taigi_windows_core::settings::{
    keys, CandidateDisplayMode, SettingsDocument, SettingsProvider,
};

// MARK: - Fixtures

fn engine_lock() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    static INSTALLED: OnceLock<()> = OnceLock::new();
    let guard = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    INSTALLED.get_or_init(|| {
        let dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../ios/Resources/Dictionaries");
        let artifacts = DictionaryArtifacts::locate(&dir).expect("repo dictionaries present");
        engine::lexicon_install(&artifacts, 1).expect("lexicon installs");
    });
    guard
}

/// Blocks of 100 so a manager's own `start_new_session` bumps never reach
/// the next test's block.
fn fresh_generation() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(100_000);
    NEXT.fetch_add(100, Ordering::Relaxed)
}

/// Settings a test can change mid-way; every read sees the latest document.
#[derive(Clone, Default)]
struct MutableSettings(Arc<Mutex<SettingsDocument>>);

impl MutableSettings {
    fn edit(&self, edit: impl FnOnce(&mut SettingsDocument)) {
        edit(&mut self.0.lock().unwrap());
    }
}

impl SettingsProvider for MutableSettings {
    fn current(&self) -> Arc<SettingsDocument> {
        Arc::new(self.0.lock().unwrap().clone())
    }
}

#[derive(Default)]
struct Memory {
    frequency: Mutex<HashMap<(String, String), i64>>,
    associations: Mutex<Vec<AssociationPair>>,
    custom: Mutex<Vec<CustomEntry>>,
    now_ms: Mutex<i64>,
}

#[derive(Clone)]
struct Handle(Arc<Memory>);

impl FrequencySource for Handle {
    fn rows_for_words(&self, words: &[String]) -> Option<Vec<FrequencyRow>> {
        let store = self.0.frequency.lock().unwrap();
        Some(
            store
                .iter()
                .filter(|((word, _), _)| words.contains(word))
                .map(|((word, tl), count)| FrequencyRow {
                    word: word.clone(),
                    tl: tl.clone(),
                    count: *count,
                    last_used_ms: 1,
                })
                .collect(),
        )
    }
    fn record(&self, word: &str, tl: &str) {
        *self
            .0
            .frequency
            .lock()
            .unwrap()
            .entry((word.to_owned(), tl.to_owned()))
            .or_insert(0) += 1;
    }
}

impl CustomDictionarySource for Handle {
    fn rows_matching(&self, _family: &str, _form: &str, key: &str) -> Vec<CustomEntry> {
        self.0
            .custom
            .lock()
            .unwrap()
            .iter()
            .filter(|entry| {
                entry
                    .roman
                    .to_ascii_lowercase()
                    .starts_with(&key[..key.len().min(2)])
            })
            .cloned()
            .collect()
    }
}

impl AssociationSink for Handle {
    fn record(&self, pairs: &[AssociationPair]) {
        self.0.associations.lock().unwrap().extend_from_slice(pairs);
    }
}

impl Clock for Handle {
    fn now_ms(&self) -> i64 {
        *self.0.now_ms.lock().unwrap()
    }
}

#[derive(Default)]
struct Recorder {
    effects: Vec<Effect>,
}

impl ComposingEffectExecutor for Recorder {
    fn execute(&mut self, effect: &Effect) {
        self.effects.push(effect.clone());
    }
}

impl Recorder {
    fn preedits(&self) -> Vec<&str> {
        self.effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::UpdatePreedit(text) => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }
    fn committed(&self) -> Vec<&str> {
        self.effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::CommitTextReplacingPreedit(text) => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }
    fn cleared(&self) -> bool {
        self.effects.contains(&Effect::ClearPreeditWithoutCommit)
    }
}

struct Rig {
    manager: ComposingManager,
    settings: MutableSettings,
    memory: Arc<Memory>,
    recorder: Recorder,
}

fn rig() -> Rig {
    let memory = Arc::new(Memory::default());
    *memory.now_ms.lock().unwrap() = 1_000;
    let handle = Handle(Arc::clone(&memory));
    let settings = MutableSettings::default();
    let manager = ComposingManager::new(
        Arc::new(settings.clone()),
        Box::new(handle.clone()),
        Box::new(handle.clone()),
        NextWordLearner::new(Box::new(handle.clone()), Box::new(handle.clone())),
        Box::new(handle),
        fresh_generation(),
    );
    Rig {
        manager,
        settings,
        memory,
        recorder: Recorder::default(),
    }
}

impl Rig {
    fn type_text(&mut self, text: &str) {
        for character in text.chars() {
            self.manager
                .append(&character.to_string(), &mut self.recorder);
        }
    }

    fn candidates(&mut self) -> Vec<ContinuousCandidate> {
        match self.manager.fetch_candidates() {
            CandidateFetchOutcome::Found(candidates) => candidates,
            other => panic!("expected candidates, got {other:?}"),
        }
    }

    fn candidate(&mut self, hanji: &str) -> ContinuousCandidate {
        let candidates = self.candidates();
        candidates
            .iter()
            .find(|candidate| candidate.hanji.as_deref() == Some(hanji))
            .cloned()
            .unwrap_or_else(|| {
                panic!(
                    "{hanji} among {:?}",
                    candidates
                        .iter()
                        .map(|c| &c.display_text)
                        .collect::<Vec<_>>()
                )
            })
    }

    fn commit(
        &mut self,
        candidate: &ContinuousCandidate,
        script: CandidateScript,
    ) -> (CandidateCommitOutcome, Option<String>) {
        let (outcome, committed) =
            self.manager
                .commit_candidate(candidate, script, &mut self.recorder);
        (outcome, committed.map(|commit| commit.text))
    }

    /// The auto-space verdict the same commit resolves — asserted apart from
    /// the text because the two travel together through one return.
    fn commit_verdict(&mut self, candidate: &ContinuousCandidate, script: CandidateScript) -> bool {
        self.manager
            .commit_candidate(candidate, script, &mut self.recorder)
            .1
            .expect("a finalized commit carries its text and verdict")
            .wrote_romanization
    }

    fn advance_clock(&self, ms: i64) {
        *self.memory.now_ms.lock().unwrap() += ms;
    }
}

// MARK: - ComposingManagerTests

#[test]
fn append_shows_the_preedit_and_mirrors_the_engine() {
    let _lock = engine_lock();
    let mut rig = rig();
    rig.type_text("tai5");
    assert!(rig.manager.is_composing());
    assert_eq!(rig.manager.raw_input(), "tai5");
    assert_eq!(rig.manager.display_text(), "tâi");
    assert_eq!(rig.recorder.preedits().last(), Some(&"tâi"));
}

#[test]
fn commit_composition_writes_the_composition_and_ends_it() {
    let _lock = engine_lock();
    let mut rig = rig();
    rig.type_text("gua2");
    let committed = rig.manager.commit_composition(&mut rig.recorder);
    assert_eq!(committed.as_deref(), Some("guá"));
    assert!(!rig.manager.is_composing());
    assert_eq!(rig.manager.raw_input(), "");
    assert_eq!(rig.recorder.committed(), ["guá"]);
}

#[test]
fn commit_then_insert_reaches_the_host_as_one_write_and_drops_the_context() {
    let _lock = engine_lock();
    let mut rig = rig();
    rig.type_text("gua2");
    let committed = rig
        .manager
        .commit_composition_then_insert(" ", &mut rig.recorder);
    assert_eq!(committed.as_deref(), Some("guá "));
    assert_eq!(
        rig.recorder.committed(),
        ["guá "],
        "one mutation for preedit + space"
    );
    assert!(!rig.manager.is_composing());
}

#[test]
fn cancel_clears_without_writing_and_delete_to_empty_ends() {
    let _lock = engine_lock();
    let mut rig = rig();
    rig.type_text("tai");
    rig.manager.cancel_composition(&mut rig.recorder);
    assert!(rig.recorder.cleared());
    assert!(rig.recorder.committed().is_empty());
    assert!(!rig.manager.is_composing());

    rig.type_text("t");
    assert!(rig.manager.is_composing());
    rig.manager.delete_backward(&mut rig.recorder);
    assert!(!rig.manager.is_composing());
    assert_eq!(rig.manager.raw_input(), "");
}

#[test]
fn start_new_session_drops_the_composition_without_touching_the_client() {
    let _lock = engine_lock();
    let mut rig = rig();
    rig.type_text("tai");
    let before = rig.recorder.effects.len();
    rig.manager.start_new_session();
    assert!(!rig.manager.is_composing());
    assert_eq!(rig.manager.display_text(), "");
    assert_eq!(
        rig.recorder.effects.len(),
        before,
        "nothing is written into the old client"
    );
    // The engine dropped the old composition: a fresh append starts clean.
    rig.type_text("g");
    assert_eq!(rig.manager.raw_input(), "g");
}

// MARK: - ComposingManagerCandidateTests

#[test]
fn fetch_candidates_finds_candidates_while_composing_and_reports_idle_otherwise() {
    let _lock = engine_lock();
    let mut rig = rig();
    assert_eq!(
        rig.manager.fetch_candidates(),
        CandidateFetchOutcome::NotComposing
    );
    rig.type_text("taigi");
    let candidates = rig.candidates();
    assert!(candidates
        .iter()
        .any(|c| c.hanji.as_deref() == Some("台語")));
    assert!(
        rig.manager.is_composing(),
        "a fetch leaves the composition intact"
    );
    rig.type_text("k");
    assert_eq!(rig.manager.raw_input(), "taigik");
}

#[test]
fn commit_candidate_consuming_the_whole_buffer_writes_the_document_and_ends() {
    let _lock = engine_lock();
    let mut rig = rig();
    rig.type_text("taigi");
    let taigi = rig.candidate("台語");
    let (outcome, committed) = rig.commit(&taigi, CandidateScript::Primary);
    assert_eq!(outcome, CandidateCommitOutcome::Finalized);
    assert_eq!(
        committed.as_deref(),
        Some("tâi-gí"),
        "roman-first output writes the romanization"
    );
    assert!(!rig.manager.is_composing());
    assert_eq!(rig.recorder.committed(), ["tâi-gí"]);
}

#[test]
fn commit_candidate_swapped_output_writes_the_hanji_and_alternate_writes_the_other() {
    let _lock = engine_lock();
    let mut rig = rig();
    rig.settings
        .edit(|doc| doc.set_bool(&keys::IS_TRANSLATE_SWAPPED, true));
    rig.type_text("taigi");
    let taigi = rig.candidate("台語");
    let (outcome, committed) = rig.commit(&taigi, CandidateScript::Primary);
    assert_eq!(outcome, CandidateCommitOutcome::Finalized);
    assert_eq!(committed.as_deref(), Some("台語"));

    rig.type_text("taigi");
    let taigi = rig.candidate("台語");
    let (outcome, committed) = rig.commit(&taigi, CandidateScript::Alternate);
    assert_eq!(outcome, CandidateCommitOutcome::Finalized);
    assert_eq!(
        committed.as_deref(),
        Some("tâi-gí"),
        "Space writes the other script"
    );
}

/// trace: `resolved_commit` — the hanji-absent arm, through the real engine.
/// §34's literal is a one-script candidate, so it earns the auto space under
/// every mode; the old gate read `(script, swap)` and called it a hanji
/// commit in 漢字優先 and 漢羅濫, the two modes that force the swap on.
#[test]
fn commit_candidate_with_no_hanji_wrote_romanization_under_every_mode() {
    let _lock = engine_lock();
    for (swapped, display_mode) in [
        (false, CandidateDisplayMode::SideBySide),
        (true, CandidateDisplayMode::SideBySide),
        (true, CandidateDisplayMode::Combined),
    ] {
        let mut rig = rig();
        rig.settings.edit(|doc| {
            doc.set_bool(&keys::IS_TRANSLATE_SWAPPED, swapped);
            doc.set_choice(&keys::CANDIDATE_DISPLAY_MODE, display_mode);
        });
        rig.type_text("taigi");
        let literal = rig
            .candidates()
            .into_iter()
            .find(|candidate| candidate.nonempty_hanji().is_none())
            .expect("§34 literal leads the desktop list");
        assert_eq!(literal.roman, "taigi");

        assert!(
            rig.commit_verdict(&literal, CandidateScript::Primary),
            "swapped={swapped} mode={display_mode:?}"
        );
    }
}

/// And the other direction is untouched: a 漢字 commit earns nothing.
#[test]
fn commit_candidate_of_a_hanji_wrote_no_romanization_when_the_mode_leads_with_it() {
    let _lock = engine_lock();
    let mut rig = rig();
    rig.settings
        .edit(|doc| doc.set_bool(&keys::IS_TRANSLATE_SWAPPED, true));
    rig.type_text("taigi");
    let taigi = rig.candidate("台語");

    assert!(!rig.commit_verdict(&taigi, CandidateScript::Primary));
}

#[test]
fn commit_candidate_consuming_part_of_the_buffer_nails_it_and_keeps_composing() {
    let _lock = engine_lock();
    let mut rig = rig();
    rig.type_text("taigi");
    let candidates = rig.candidates();
    let tai = candidates
        .iter()
        .find(|c| c.hanji.as_deref() == Some("台") && c.consumed_span_end == 3)
        .cloned()
        .expect("台 over `tai`");
    let (outcome, committed) = rig.commit(&tai, CandidateScript::Primary);
    assert_eq!(outcome, CandidateCommitOutcome::Nailed);
    assert_eq!(
        committed, None,
        "Model B writes nothing until the final commit"
    );
    assert!(rig.manager.is_composing());
    assert_eq!(
        rig.manager.raw_input(),
        "gi",
        "the mirror holds only the pending tail"
    );
    assert!(
        rig.manager.display_text().starts_with("tâi"),
        "{}",
        rig.manager.display_text()
    );
    assert!(
        rig.manager.display_text().len() > 3,
        "the display holds the nailed prefix too"
    );
}

#[test]
fn commit_candidate_after_the_composition_ended_is_ignored() {
    let _lock = engine_lock();
    let mut rig = rig();
    rig.type_text("taigi");
    let taigi = rig.candidate("台語");
    rig.manager.cancel_composition(&mut rig.recorder);
    let (outcome, committed) = rig.commit(&taigi, CandidateScript::Primary);
    assert_eq!(outcome, CandidateCommitOutcome::Ignored);
    assert_eq!(committed, None);
    assert!(
        rig.memory.frequency.lock().unwrap().is_empty(),
        "an ignored commit learns nothing"
    );
}

/// §34 under the shipped defaults (顯示當咧拍的字 ON): a fresh bar has the
/// typed literal in slot 0 — one script — so the highlighted-candidate commit
/// that Enter routes to (`keys/intent.rs` `return_commits_the_candidate_and_shift_return_the_literal`;
/// the window opens on slot 0) writes exactly what was typed in either output
/// mode, and learns it under its canonical reading. The dictionary's first
/// candidate is one slot along. ⇧Enter's raw path is
/// `commit_composition_writes_the_composition_and_ends_it`.
#[test]
fn enter_on_a_fresh_bar_commits_the_typed_literal_in_either_mode() {
    let _lock = engine_lock();
    for swapped in [false, true] {
        let mut rig = rig();
        rig.settings
            .edit(|doc| doc.set_bool(&keys::IS_TRANSLATE_SWAPPED, swapped));
        rig.type_text("taigi");
        let candidates = rig.candidates();
        assert_eq!(candidates[0].display_text, "taigi", "swapped={swapped}");
        assert!(candidates[0].is_roman_only(), "swapped={swapped}");
        assert!(
            candidates[1].hanji.is_some(),
            "the dictionary's first candidate is next — swapped={swapped}"
        );
        let (outcome, committed) = rig.commit(&candidates[0], CandidateScript::Primary);
        assert_eq!(
            outcome,
            CandidateCommitOutcome::Finalized,
            "swapped={swapped}"
        );
        assert_eq!(committed.as_deref(), Some("taigi"), "swapped={swapped}");
        assert!(!rig.manager.is_composing(), "swapped={swapped}");
        assert_eq!(rig.recorder.committed(), ["taigi"], "swapped={swapped}");
        let learned = rig.memory.frequency.lock().unwrap().clone();
        assert_eq!(
            learned.keys().collect::<Vec<_>>(),
            [&("taigi".to_string(), candidates[0].canonical_tl.clone())],
            "learned under the literal's canonical reading — swapped={swapped}"
        );
    }
}

/// 顯示當咧拍的字 OFF, read through the real settings document: the forced §34
/// row is gone, so the bar opens on a two-script dictionary candidate and Enter
/// writes that word rather than the typed letters. Asserted against the cell the
/// bar actually offered, not against a fixed dictionary word — which reading
/// ranks first is the lattice's business, not this switch's.
#[test]
fn enter_on_a_fresh_bar_commits_the_dictionary_word_when_the_literal_row_is_off() {
    let _lock = engine_lock();
    let mut rig = rig();
    rig.settings
        .edit(|doc| doc.set_bool(&keys::IS_LITERAL_ROMAN_CANDIDATE_ENABLED, false));
    rig.type_text("taigi");
    let candidates = rig.candidates();
    assert!(
        candidates[0].hanji.is_some(),
        "nothing forces a one-script row to the front: {:?}",
        candidates[0]
    );
    let (outcome, committed) = rig.commit(&candidates[0], CandidateScript::Primary);
    assert_eq!(outcome, CandidateCommitOutcome::Finalized);
    assert_eq!(committed.as_deref(), Some(candidates[0].roman.as_str()));
    assert_ne!(
        committed.as_deref(),
        Some("taigi"),
        "the dictionary word, not the typed literal"
    );
}

#[test]
fn alternate_on_a_single_script_candidate_commits_nothing() {
    let _lock = engine_lock();
    let mut rig = rig();
    rig.type_text("tai");
    let candidates = rig.candidates();
    let literal = candidates[0].clone();
    assert!(literal.is_roman_only());
    let (outcome, committed) = rig.commit(&literal, CandidateScript::Alternate);
    assert_eq!(outcome, CandidateCommitOutcome::Ignored);
    assert_eq!(committed, None);
    assert!(rig.manager.is_composing(), "the composition is untouched");
}

// MARK: - ComposingManagerLearningTests

#[test]
fn commit_candidate_counts_the_word_under_its_reading_in_either_script() {
    let _lock = engine_lock();
    let mut rig = rig();
    rig.type_text("taigi");
    let taigi = rig.candidate("台語");
    rig.commit(&taigi, CandidateScript::Primary);
    rig.type_text("taigi");
    let taigi = rig.candidate("台語");
    rig.commit(&taigi, CandidateScript::Alternate);
    let store = rig.memory.frequency.lock().unwrap();
    assert_eq!(
        store.get(&("台語".to_owned(), "tâi-gí".to_owned())),
        Some(&2),
        "{store:?}"
    );
    assert_eq!(store.len(), 1, "identity is the pair, not the rendering");
}

#[test]
fn commit_candidate_with_recording_off_learns_nothing() {
    let _lock = engine_lock();
    let mut rig = rig();
    rig.settings
        .edit(|doc| doc.set_bool(&keys::IS_FREQUENCY_RECORDING_ENABLED, false));
    rig.type_text("taigi");
    let taigi = rig.candidate("台語");
    let (outcome, _) = rig.commit(&taigi, CandidateScript::Primary);
    assert_eq!(outcome, CandidateCommitOutcome::Finalized);
    assert!(rig.memory.frequency.lock().unwrap().is_empty());
}

#[test]
fn a_repeatedly_committed_candidate_overtakes_the_one_above_it() {
    let _lock = engine_lock();
    let mut rig = rig();
    rig.type_text("tai");
    let neutral = rig.candidates();
    let underdog = neutral.last().cloned().expect("candidates");
    assert_ne!(underdog.display_text, neutral[0].display_text);
    rig.memory.frequency.lock().unwrap().insert(
        (underdog.display_text.clone(), underdog.canonical_tl.clone()),
        500,
    );
    let boosted = rig.candidates();
    let position = boosted
        .iter()
        .position(|c| {
            c.display_text == underdog.display_text && c.canonical_tl == underdog.canonical_tl
        })
        .expect("still listed");
    assert!(position < neutral.len() - 1, "{position}");
}

#[test]
fn two_commits_in_a_row_learn_the_bigram_and_a_full_stop_breaks_it() {
    let _lock = engine_lock();
    let mut rig = rig();
    rig.type_text("tai5");
    let tai = rig.candidate("台");
    rig.commit(&tai, CandidateScript::Primary);
    rig.advance_clock(500);
    rig.type_text("gi2");
    let gi = rig.candidate("語");
    rig.commit(&gi, CandidateScript::Primary);
    {
        let pairs = rig.memory.associations.lock().unwrap();
        assert!(
            pairs.iter().any(|p| p.previous == "台"
                && p.next == "語"
                && p.previous_tl == "tâi"
                && p.next_tl == "gí"),
            "{pairs:?}"
        );
    }
    rig.memory.associations.lock().unwrap().clear();

    // A full stop typed outside a composition ends the context.
    rig.manager.note_character_typed_outside_composition("。");
    rig.advance_clock(500);
    rig.type_text("bun5");
    let bun = rig.candidate("文");
    rig.commit(&bun, CandidateScript::Primary);
    assert!(
        !rig.memory
            .associations
            .lock()
            .unwrap()
            .iter()
            .any(|p| p.next == "文"),
        "{:?}",
        rig.memory.associations.lock().unwrap()
    );
}

#[test]
fn a_comma_leaves_the_bigram_intact_and_a_letter_outside_is_not_a_word() {
    let _lock = engine_lock();
    let mut rig = rig();
    rig.type_text("tai5");
    let tai = rig.candidate("台");
    rig.commit(&tai, CandidateScript::Primary);
    rig.manager.note_character_typed_outside_composition(",");
    rig.manager.note_character_typed_outside_composition("x");
    rig.advance_clock(500);
    rig.type_text("gi2");
    let gi = rig.candidate("語");
    rig.commit(&gi, CandidateScript::Primary);
    let pairs = rig.memory.associations.lock().unwrap();
    assert!(
        pairs.iter().any(|p| p.previous == "台" && p.next == "語"),
        "{pairs:?}"
    );
    assert!(
        !pairs.iter().any(|p| p.previous == "x" || p.previous == ","),
        "{pairs:?}"
    );
}

#[test]
fn with_association_recording_off_two_commits_learn_nothing() {
    let _lock = engine_lock();
    let mut rig = rig();
    rig.settings
        .edit(|doc| doc.set_bool(&keys::IS_ASSOCIATION_RECORDING_ENABLED, false));
    rig.type_text("tai5");
    let tai = rig.candidate("台");
    rig.commit(&tai, CandidateScript::Primary);
    rig.type_text("gi2");
    let gi = rig.candidate("語");
    rig.commit(&gi, CandidateScript::Primary);
    assert!(rig.memory.associations.lock().unwrap().is_empty());
}

#[test]
fn a_new_session_and_a_mid_composition_punctuation_both_forget_the_context() {
    let _lock = engine_lock();
    let mut rig = rig();
    rig.type_text("tai5");
    let tai = rig.candidate("台");
    rig.commit(&tai, CandidateScript::Primary);
    rig.manager.start_new_session();
    rig.type_text("gi2");
    let gi = rig.candidate("語");
    rig.commit(&gi, CandidateScript::Primary);
    assert!(
        rig.memory.associations.lock().unwrap().is_empty(),
        "{:?}",
        rig.memory.associations.lock().unwrap()
    );

    // Punctuation committed mid-composition drops the context rather than
    // letting the next commit skip a word.
    rig.type_text("gua2");
    rig.manager
        .commit_composition_then_insert(".", &mut rig.recorder);
    rig.type_text("bun5");
    let bun = rig.candidate("文");
    rig.commit(&bun, CandidateScript::Primary);
    assert!(!rig
        .memory
        .associations
        .lock()
        .unwrap()
        .iter()
        .any(|p| p.next == "文"));
}

#[test]
fn custom_entries_reach_the_fetch_only_while_the_setting_is_on() {
    let _lock = engine_lock();
    let mut rig = rig();
    // A pair the bundled dictionary does not carry, so its presence can only
    // come from the custom store (𠢕早 / gâu-tsá is a real dictionary word
    // and would surface with the setting off too).
    rig.memory.custom.lock().unwrap().push(CustomEntry {
        roman: "khiam-tsi".into(),
        hanzi: "測試自訂".into(),
    });
    rig.type_text("khiamtsi");
    assert!(rig
        .candidates()
        .iter()
        .any(|c| c.hanji.as_deref() == Some("測試自訂")));
    rig.settings
        .edit(|doc| doc.set_bool(&keys::IS_CUSTOM_DICT_ENABLED, false));
    assert!(!rig
        .candidates()
        .iter()
        .any(|c| c.hanji.as_deref() == Some("測試自訂")));
}

// MARK: - ComposingSessionCoordinatorTests

#[test]
fn only_the_claiming_context_can_drive_the_engine_and_handover_starts_idle() {
    let _lock = engine_lock();
    let rig = rig();
    let mut coordinator = ComposingSessionCoordinator::new(rig.manager);
    let a = coordinator.allocate_token();
    let b = coordinator.allocate_token();
    let mut recorder = Recorder::default();

    coordinator.claim(a).append("t", &mut recorder);
    assert!(
        coordinator.manager(b).is_none(),
        "b does not own the engine"
    );
    assert!(coordinator.manager(a).is_some());

    // Re-claiming an owned context leaves the composition running.
    assert!(coordinator.claim(a).is_composing());

    // Handover: b takes over from an idle engine.
    let manager = coordinator.claim(b);
    assert!(!manager.is_composing());
    assert_eq!(manager.raw_input(), "");
    assert!(coordinator.manager(a).is_none(), "a was superseded");

    // Releasing the superseded context leaves the live one alone.
    coordinator.claim(b).append("g", &mut recorder);
    coordinator.release(a);
    assert!(coordinator.manager(b).expect("b still owns").is_composing());

    // Releasing the live one frees the engine.
    coordinator.release(b);
    assert!(coordinator.manager(b).is_none());
    assert_eq!(coordinator.current_owner(), None);
}
