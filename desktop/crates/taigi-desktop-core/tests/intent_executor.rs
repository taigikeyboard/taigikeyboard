//! The shared key-intent executor against the REAL engine and dictionaries,
//! through a surface that records every call — what each intent asks of
//! the document and the list, independent of either shell. The Linux
//! session tests (`taigi-linux-core/tests/session_characterisation.rs`) pin
//! the same paths end to end through that shell's adapter.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use taigi_desktop_core::composing::{
    insert_symbol, pass_through_may_consume, perform_intent, represent_list, CandidateSource,
    ComposingEffectExecutor, ComposingManager, EngineNextWord, IntentSurface, SystemClock,
};
use taigi_desktop_core::dictionary_artifacts::DictionaryArtifacts;
use taigi_desktop_core::engine::{self, Effect};
use taigi_desktop_core::keys::{
    CandidateNavigation, CaretDirection, ComposingKeyIntent, KeyEventSnapshot, KeyModifiers,
    TpsKeyCapIndex,
};
use taigi_desktop_core::settings::{keys, InputMode, SettingsDocument, StaticSettingsProvider};

/// One lock, one lexicon install: the engine is one per process.
fn engine_lock() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    static INSTALLED: OnceLock<()> = OnceLock::new();
    let guard = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    INSTALLED.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../assets/dictionaries");
        let artifacts = DictionaryArtifacts::locate(&dir).expect("repo dictionaries present");
        engine::lexicon_install(&artifacts, 1).expect("lexicon installs");
    });
    guard
}

/// Blocks of 100 so one manager's own session bumps never reach the next.
fn fresh_generation() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(900_000);
    NEXT.fetch_add(100, Ordering::Relaxed)
}

/// Every call the executor makes, in order.
#[derive(Debug, Default)]
struct Surface {
    calls: Vec<String>,
    /// Whether this key may swap (the shell's arm AND its document).
    can_swap: bool,
    has_failed: bool,
    selected: Option<usize>,
    /// The caps of the TPS keys the engine took, in order
    /// (`tps_keyboard_cap_typed`).
    typed_tps_caps: Vec<TpsKeyCapIndex>,
}

impl ComposingEffectExecutor for Surface {
    fn execute(&mut self, effect: &Effect) {
        if let Effect::CommitTextReplacingPreedit(text) = effect {
            self.calls.push(format!("commit {text}"));
        }
    }
}

impl IntentSurface for Surface {
    fn insert_external(&mut self, text: &str) {
        self.calls.push(format!("insert {text:?}"));
    }

    fn swap_preceding_space(&mut self, replacement: &str) -> bool {
        self.calls.push(format!("swap {replacement:?}"));
        self.can_swap
    }

    fn arm_swap(&mut self) {
        self.calls.push("arm".to_owned());
    }

    fn has_write_failed(&self) -> bool {
        self.has_failed
    }

    fn list_changed(&mut self, list: &mut CandidateSource) {
        self.calls.push(format!("list {}", list.len()));
    }

    fn list_closed(&mut self) {
        self.calls.push("list closed".to_owned());
    }

    fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    fn index_for_key_slot(&self, slot: usize) -> Option<usize> {
        // The literal leads the list and takes no key here: slot 0 is cell 1.
        Some(slot + 1)
    }

    fn navigate(&mut self, direction: CandidateNavigation) {
        self.calls.push(format!("navigate {direction:?}"));
    }

    fn tps_keyboard_cap_typed(&mut self, cap: TpsKeyCapIndex) {
        self.typed_tps_caps.push(cap);
    }
}

struct Rig {
    _engine: MutexGuard<'static, ()>,
    settings: SettingsDocument,
    manager: ComposingManager,
    list: CandidateSource,
    surface: Surface,
}

fn new_rig(is_auto_space_enabled: bool) -> Rig {
    let engine = engine_lock();
    let mut settings = SettingsDocument::default();
    settings.set_bool(&keys::IS_AUTO_SPACE_ENABLED, is_auto_space_enabled);
    // The surface's slot mapping assumes the §34 literal at cell 0, so Show
    // Typed Text First is pinned ON (it ships OFF since 2026-10-02).
    settings.set_bool(&keys::IS_LITERAL_ROMAN_CANDIDATE_ENABLED, true);
    let manager = ComposingManager::new(
        Arc::new(StaticSettingsProvider::new(settings.clone())),
        Box::new(EngineNextWord),
        Box::new(SystemClock),
        fresh_generation(),
    );
    Rig {
        _engine: engine,
        settings,
        manager,
        list: CandidateSource::default(),
        surface: Surface::default(),
    }
}

impl Rig {
    fn run(&mut self, intent: ComposingKeyIntent, key: &KeyEventSnapshot) -> bool {
        perform_intent(
            &intent,
            key,
            &self.settings,
            &mut self.manager,
            &mut self.list,
            &mut self.surface,
        )
    }

    fn type_word(&mut self, word: &str) {
        for letter in word.chars() {
            let text = letter.to_string();
            let key = KeyEventSnapshot::text(&text, KeyModifiers::NONE);
            assert!(self.run(ComposingKeyIntent::Input(text), &key));
        }
        self.surface.calls.clear();
    }

    fn calls(&self) -> Vec<&str> {
        self.surface.calls.iter().map(String::as_str).collect()
    }
}

fn no_key() -> KeyEventSnapshot {
    KeyEventSnapshot::default()
}

fn comma() -> KeyEventSnapshot {
    KeyEventSnapshot::text(",", KeyModifiers::NONE)
}

#[test]
fn typing_refetches_the_list_and_hands_it_to_the_surface() {
    let mut rig = new_rig(false);
    let key = KeyEventSnapshot::text("h", KeyModifiers::NONE);
    assert!(rig.run(ComposingKeyIntent::Input("h".to_owned()), &key));
    assert!(!rig.list.is_empty());
    assert_eq!(rig.calls(), [format!("list {}", rig.list.len())]);
}

#[test]
fn commit_writes_the_literal_then_the_auto_space_and_arms_it() {
    let mut rig = new_rig(true);
    rig.type_word("ho");
    assert!(rig.run(ComposingKeyIntent::Commit, &no_key()));
    assert_eq!(
        rig.calls(),
        ["commit ho", "list closed", "insert \" \"", "arm"]
    );
    assert!(rig.list.is_empty());
}

#[test]
fn no_auto_space_when_it_is_off_and_no_arm_after_a_failed_write() {
    let mut rig = new_rig(false);
    rig.type_word("ho");
    rig.run(ComposingKeyIntent::Commit, &no_key());
    assert_eq!(rig.calls(), ["commit ho", "list closed"]);

    drop(rig);
    let mut rig = new_rig(true);
    rig.type_word("ho");
    rig.surface.has_failed = true;
    rig.run(ComposingKeyIntent::Commit, &no_key());
    assert_eq!(
        rig.calls(),
        ["commit ho", "list closed", "insert \" \""],
        "the space is still written; the swap is not armed on it"
    );
}

#[test]
fn cancel_closes_the_list_and_writes_nothing() {
    let mut rig = new_rig(true);
    rig.type_word("ho");
    assert!(rig.run(ComposingKeyIntent::Cancel, &no_key()));
    assert_eq!(rig.calls(), ["list closed"]);
}

#[test]
fn a_slot_key_commits_its_hanji_cell_without_a_space() {
    let mut rig = new_rig(true);
    rig.type_word("ho");
    // The surface's slot 0 is cell 1 because the literal leads the list.
    assert!(rig.list.leads_with_literal_roman());
    let intent = ComposingKeyIntent::SelectCandidateSlot {
        slot: 0,
        flip: false,
    };
    assert!(rig.run(intent, &no_key()));
    assert_eq!(rig.calls(), ["commit 好", "list closed"]);
}

#[test]
fn a_slot_past_the_list_or_no_highlight_is_consumed_and_commits_nothing() {
    let mut rig = new_rig(true);
    rig.type_word("ho");
    let intent = ComposingKeyIntent::SelectCandidateSlot {
        slot: 10_000,
        flip: false,
    };
    assert!(rig.run(intent, &no_key()));
    assert!(rig.calls().is_empty());
    assert!(rig.run(ComposingKeyIntent::CommitHighlightedCandidate, &no_key()));
    assert!(rig.calls().is_empty());
}

#[test]
fn navigate_moves_the_surfaces_highlight_only() {
    let mut rig = new_rig(true);
    rig.type_word("ho");
    assert!(rig.run(
        ComposingKeyIntent::Navigate(CandidateNavigation::Down),
        &no_key()
    ));
    assert_eq!(rig.calls(), ["navigate Down"]);
}

#[test]
fn a_typed_comma_swaps_an_armed_space_or_is_written_full_width() {
    let mut rig = new_rig(true);
    rig.surface.can_swap = true;
    assert!(rig.run(ComposingKeyIntent::PassThrough, &comma()));
    assert_eq!(rig.calls(), ["swap \", \"", "arm"]);

    drop(rig);
    let mut rig = new_rig(true);
    rig.surface.can_swap = false;
    assert!(rig.run(ComposingKeyIntent::PassThrough, &comma()));
    assert_eq!(rig.calls(), ["swap \", \"", "insert \"，\""]);
}

#[test]
fn half_width_punctuation_outside_a_composition_is_the_clients() {
    let mut rig = new_rig(true);
    rig.settings.set_bool(&keys::IS_HANJI_FIRST, false);
    rig.surface.can_swap = false;
    assert!(!rig.run(ComposingKeyIntent::PassThrough, &comma()));
    assert_eq!(rig.calls(), ["swap \", \""]);
}

#[test]
fn a_picked_symbol_swaps_only_when_it_attaches() {
    let mut rig = new_rig(true);
    rig.surface.can_swap = true;
    insert_symbol("·", &rig.settings, &mut rig.manager, &mut rig.surface);
    assert_eq!(
        rig.calls(),
        ["insert \"·\""],
        "not attaching: no swap asked"
    );
    rig.surface.calls.clear();
    insert_symbol("，", &rig.settings, &mut rig.manager, &mut rig.surface);
    assert_eq!(rig.calls(), ["swap \"， \"", "arm"]);
}

/// No chord types the other width (USER 2026-10-07): outside TPS, Ctrl on a
/// punctuation key is the host's in either width mode — not consumed, nothing
/// written, no swap even with one armed.
#[test]
fn outside_tps_ctrl_punctuation_is_never_consumed_or_written() {
    let mut rig = new_rig(true);
    rig.surface.can_swap = true;
    let ctrl_comma = KeyEventSnapshot::chord(None, ",", KeyModifiers::CONTROL);
    for is_hanji_first in [true, false] {
        rig.settings.set_bool(&keys::IS_HANJI_FIRST, is_hanji_first);
        assert!(!pass_through_may_consume(&ctrl_comma, &rig.settings, true));
        assert!(!rig.run(ComposingKeyIntent::PassThrough, &ctrl_comma));
        assert!(rig.calls().is_empty(), "{:?}", rig.calls());
    }
}

#[test]
fn a_pass_through_key_is_consumed_for_punctuation_or_an_armed_swap() {
    let mut settings = SettingsDocument::default();
    settings.set_bool(&keys::IS_AUTO_SPACE_ENABLED, true);
    // Hanji-first: full-width punctuation is written by the input method.
    assert!(pass_through_may_consume(&comma(), &settings, false));
    settings.set_bool(&keys::IS_HANJI_FIRST, false);
    // Roman-first: half width is the client's, unless a swap is armed.
    assert!(!pass_through_may_consume(&comma(), &settings, false));
    assert!(pass_through_may_consume(&comma(), &settings, true));
    settings.set_bool(&keys::IS_AUTO_SPACE_ENABLED, false);
    assert!(!pass_through_may_consume(&comma(), &settings, true));
    // A letter is never punctuation.
    let letter = KeyEventSnapshot::text("a", KeyModifiers::NONE);
    assert!(!pass_through_may_consume(&letter, &settings, true));
    assert!(!pass_through_may_consume(&no_key(), &settings, true));
}

/// A key read in the user's Dvorak / Colemak layout (`is_remapped`): the
/// host would type the QWERTY key's text, so the input method writes what
/// the user's layout types — Dvorak's `,` sits on the US `W`.
fn remapped(key: KeyEventSnapshot) -> KeyEventSnapshot {
    KeyEventSnapshot {
        is_remapped: true,
        ..key
    }
}

#[test]
fn remapped_half_width_text_outside_a_composition_is_written_not_passed() {
    let mut rig = new_rig(true);
    rig.settings.set_bool(&keys::IS_HANJI_FIRST, false);
    rig.surface.can_swap = false;
    let dvorak_comma = remapped(comma());
    assert!(pass_through_may_consume(
        &dvorak_comma,
        &rig.settings,
        false
    ));
    assert!(rig.run(ComposingKeyIntent::PassThrough, &dvorak_comma));
    // trace: Roman-first → no full-width map; the swap is asked first (as
    // for the unremapped comma, `half_width_punctuation_outside_…`), then
    // the remapped text is written.
    assert_eq!(rig.calls(), ["swap \", \"", "insert \",\""]);
}

#[test]
fn a_remapped_mark_takes_an_armed_swap_and_writes_nothing_more() {
    let mut rig = new_rig(true);
    rig.settings.set_bool(&keys::IS_HANJI_FIRST, false);
    rig.surface.can_swap = true;
    // trace: the swap is tried before the write (PassThrough arm), and a
    // swap that lands answers true there — the remapped text never writes.
    assert!(rig.run(ComposingKeyIntent::PassThrough, &remapped(comma())));
    assert_eq!(rig.calls(), ["swap \", \"", "arm"]);
}

#[test]
fn a_remapped_host_chord_stays_the_hosts() {
    let mut rig = new_rig(true);
    // trace: Ctrl held → `document_text` answers None (host chord), so
    // neither the consume check nor the PassThrough arm writes anything.
    let ctrl_comma = remapped(KeyEventSnapshot::chord(None, ",", KeyModifiers::CONTROL));
    assert!(!pass_through_may_consume(&ctrl_comma, &rig.settings, false));
    assert!(!rig.run(ComposingKeyIntent::PassThrough, &ctrl_comma));
    assert!(rig.calls().is_empty(), "{:?}", rig.calls());
}

#[test]
fn enter_commits_the_highlighted_cell_and_space_its_other_script() {
    // trace (read by running): cell 1 is 好 hó; Enter writes 好 (Hanji takes
    // no space), Space writes hó, which earns the space and its arm.
    let mut rig = new_rig(true);
    rig.type_word("ho");
    rig.surface.selected = Some(1);
    assert!(rig.run(ComposingKeyIntent::CommitHighlightedCandidate, &no_key()));
    assert_eq!(rig.calls(), ["commit 好", "list closed"]);

    drop(rig);
    let mut rig = new_rig(true);
    rig.type_word("ho");
    rig.surface.selected = Some(1);
    assert!(rig.run(ComposingKeyIntent::CommitAlternateScript, &no_key()));
    assert_eq!(
        rig.calls(),
        ["commit hó", "list closed", "insert \" \"", "arm"]
    );
}

#[test]
fn enter_on_the_literal_writes_it_spaced_and_space_on_it_writes_nothing() {
    // trace: cell 0 is the §34 literal `ho` (no Hanji). Enter = LEAD → the
    // romanization, which earns the space; Space = OTHER → the engine has no
    // other script to write → IGNORED: nothing written, the list refetched.
    let mut rig = new_rig(true);
    rig.type_word("ho");
    rig.surface.selected = Some(0);
    assert!(rig.run(ComposingKeyIntent::CommitHighlightedCandidate, &no_key()));
    assert_eq!(
        rig.calls(),
        ["commit ho", "list closed", "insert \" \"", "arm"]
    );

    drop(rig);
    let mut rig = new_rig(true);
    rig.type_word("ho");
    rig.surface.selected = Some(0);
    let open = rig.list.len();
    assert!(rig.run(ComposingKeyIntent::CommitAlternateScript, &no_key()));
    assert_eq!(rig.calls(), [format!("list {open}")]);
    assert!(rig.manager.is_composing(), "the composition is untouched");
    assert_eq!(rig.manager.raw_input(), "ho");
}

#[test]
fn punctuation_mid_composition_commits_both_in_one_write_and_arms_the_space() {
    // trace (read by running): Hanji-first maps `?` to `？`; the preedit as
    // typed is romanization, so the one write carries the auto space too.
    let mut rig = new_rig(true);
    rig.type_word("ho");
    let key = KeyEventSnapshot::text("?", KeyModifiers::NONE);
    assert!(rig.run(ComposingKeyIntent::CommitThenInsert("?".to_owned()), &key));
    assert_eq!(rig.calls(), ["commit ho？ ", "list closed", "arm"]);
}

#[test]
fn a_key_that_finishes_the_composition_then_belongs_to_the_client() {
    let mut rig = new_rig(true);
    rig.type_word("ho");
    assert!(!rig.run(ComposingKeyIntent::CommitThenPassThrough, &no_key()));
    assert_eq!(rig.calls(), ["commit ho", "list closed"]);
}

#[test]
fn a_switch_re_presents_an_open_list_and_never_opens_one() {
    let mut rig = new_rig(false);
    // No list open: neither kind of switch opens one.
    assert!(!represent_list(
        &rig.settings,
        &mut rig.manager,
        &mut rig.list,
        true
    ));
    assert!(rig.list.is_empty());

    rig.type_word("ho");
    let open = rig.list.len();
    assert!(open > 0);
    assert!(represent_list(
        &rig.settings,
        &mut rig.manager,
        &mut rig.list,
        false
    ));
    assert_eq!(rig.list.len(), open, "re-rendered in place");
    assert!(represent_list(
        &rig.settings,
        &mut rig.manager,
        &mut rig.list,
        true
    ));
    assert_eq!(rig.list.len(), open, "same composition, same candidates");

    // Show Candidate Window switched off since: a refetch fetches nothing.
    rig.settings
        .set_bool(&keys::IS_CANDIDATE_WINDOW_ENABLED, false);
    assert!(!represent_list(
        &rig.settings,
        &mut rig.manager,
        &mut rig.list,
        true
    ));
    assert!(rig.list.is_empty());
}

// MARK: - TPS (desktop-tps-roadmap.md § D3, O1; Hanji conversion arm B)

fn new_tps_rig() -> Rig {
    let engine = engine_lock();
    let mut settings = SettingsDocument::default();
    settings.set_bool(&keys::IS_AUTO_SPACE_ENABLED, true);
    settings.set_choice(&keys::INPUT_MODE, InputMode::Tps);
    let manager = ComposingManager::new(
        Arc::new(StaticSettingsProvider::new(settings.clone())),
        Box::new(EngineNextWord),
        Box::new(SystemClock),
        fresh_generation(),
    );
    Rig {
        _engine: engine,
        settings,
        manager,
        list: CandidateSource::default(),
        surface: Surface::default(),
    }
}

impl Rig {
    fn type_tps(&mut self, glyphs: &str) {
        for glyph in glyphs.chars() {
            let key = glyph.to_string();
            assert!(self.run(ComposingKeyIntent::TpsKey(key), &no_key()));
        }
        self.surface.calls.clear();
    }

    /// ↓ (or a moving key) while typing: the window comes up (D7).
    fn open_window(&mut self) {
        assert!(self.run(ComposingKeyIntent::OpenCandidates, &no_key()));
        assert!(!self.list.is_empty(), "a window to pick from");
        self.surface.calls.clear();
    }

    /// Opens the window and picks its first cell — a pick that nails the
    /// word and keeps composing (Hanji conversion H4).
    fn pick_first(&mut self) {
        self.open_window();
        self.surface.selected = Some(0);
        assert!(self.run(ComposingKeyIntent::CommitHighlightedCandidate, &no_key()));
        self.surface.calls.clear();
    }
}

/// D7: typing TPS fetches nothing, so no window comes up while the user
/// types; a key typed over an open window takes it down, the glyph kept.
#[test]
fn tps_typing_fetches_nothing_and_shuts_an_open_window() {
    let mut rig = new_tps_rig();
    rig.type_tps("ㄏㄛ");
    assert!(rig.list.is_empty());
    rig.open_window();
    assert!(rig.run(ComposingKeyIntent::TpsKey("ˋ".to_owned()), &no_key()));
    assert_eq!(rig.manager.raw_input(), "ㄏㄛˋ");
    assert!(rig.list.is_empty());
    assert_eq!(rig.calls(), ["list closed"]);
}

/// D7: Escape over the window closes it and keeps the glyphs; Backspace
/// over it closes it and deletes, with no refetch.
#[test]
fn tps_closing_or_backspacing_over_the_window_keeps_composing() {
    let mut rig = new_tps_rig();
    rig.type_tps("ㄏㄛˋ");
    rig.open_window();
    assert!(rig.run(ComposingKeyIntent::CloseCandidates, &no_key()));
    assert_eq!(rig.calls(), ["list closed"]);
    assert_eq!(rig.manager.raw_input(), "ㄏㄛˋ");
    rig.open_window();
    assert!(rig.run(ComposingKeyIntent::DeleteBackward, &no_key()));
    assert_eq!(rig.manager.raw_input(), "ㄏㄛ");
    assert_eq!(rig.calls(), ["list closed"]);
    assert!(rig.run(ComposingKeyIntent::DeleteBackward, &no_key()));
    assert_eq!(rig.calls(), ["list closed"], "no window, nothing more");
}

/// D7: the caret chord over the window takes it down, nothing refetched.
#[test]
fn tps_moving_the_caret_shuts_the_window() {
    let mut rig = new_tps_rig();
    rig.type_tps("ㄏㄛˋ");
    rig.open_window();
    assert!(rig.run(
        ComposingKeyIntent::MoveCaret(CaretDirection::Left),
        &no_key()
    ));
    assert!(rig.list.is_empty());
    assert_eq!(rig.calls(), ["list closed"]);
}

/// The caret jumps (Ctrl+↑ / Ctrl+↓) leave a TL window and its list as they
/// are — no refetch, nothing told to the surface — as a step does; under TPS
/// they take the window down, as a step does (D7).
#[test]
fn the_caret_jumps_keep_a_tl_window_and_shut_a_tps_one() {
    let mut rig = new_rig(false);
    rig.type_word("tsiah");
    let shown = rig.list.len();
    assert!(shown > 0);
    for direction in [CaretDirection::Start, CaretDirection::End] {
        assert!(rig.run(ComposingKeyIntent::MoveCaret(direction), &no_key()));
        assert_eq!(rig.manager.raw_input(), "tsiah", "{direction:?}");
        assert_eq!(rig.list.len(), shown, "{direction:?}");
        assert!(rig.calls().is_empty(), "{direction:?}: {:?}", rig.calls());
    }
    drop(rig);

    let mut rig = new_tps_rig();
    rig.type_tps("ㄏㄛˋ");
    rig.open_window();
    assert!(rig.run(
        ComposingKeyIntent::MoveCaret(CaretDirection::Start),
        &no_key()
    ));
    assert!(rig.list.is_empty());
    assert_eq!(rig.calls(), ["list closed"]);
    assert_eq!(rig.manager.raw_input(), "ㄏㄛˋ");
}

#[test]
fn tps_space_after_an_open_syllable_is_taken_as_the_separator() {
    let mut rig = new_tps_rig();
    rig.type_tps("ㄏㄛ");
    assert!(rig.run(ComposingKeyIntent::TpsKey(" ".to_owned()), &no_key()));
    assert_eq!(rig.manager.raw_input(), "ㄏㄛ ");
    assert!(rig.calls().is_empty(), "no fetch while typing (D7)");
}

#[test]
fn tps_space_on_a_closed_syllable_opens_the_window() {
    // O1 revised by D7: the refused separator puts the window up.
    let mut rig = new_tps_rig();
    rig.type_tps("ㄏㄛˋ");
    assert!(rig.run(ComposingKeyIntent::TpsKey(" ".to_owned()), &no_key()));
    assert!(!rig.list.is_empty());
    assert_eq!(rig.calls(), [format!("list {}", rig.list.len())]);
    assert_eq!(rig.manager.raw_input(), "ㄏㄛˋ");
}

/// The classifier reads Space over the window as the confirm (D7). The pick
/// nails the word and closes the window; nothing reaches the document until
/// a commit (Hanji conversion H4), which writes it as shown with no space.
#[test]
fn tps_with_the_window_up_space_picks_the_highlighted_word_and_keeps_composing() {
    let mut rig = new_tps_rig();
    rig.type_tps("ㄏㄛˋ");
    rig.open_window();
    rig.surface.selected = Some(0);
    let expected = rig
        .list
        .resolve(0, false)
        .expect("a first cell")
        .0
        .hanji
        .clone()
        .expect("the first candidate carries Hanji");
    assert!(rig.run(ComposingKeyIntent::CommitHighlightedCandidate, &no_key()));
    assert_eq!(rig.calls(), ["list closed"]);
    assert!(rig.list.is_empty());
    assert!(rig.manager.is_composing());
    assert_eq!(rig.manager.display_text(), expected);
    rig.surface.calls.clear();
    assert!(rig.run(ComposingKeyIntent::Commit, &no_key()));
    assert_eq!(
        rig.calls(),
        [format!("commit {expected}"), "list closed".to_owned()]
    );
}

#[test]
fn tps_a_flipped_slot_commits_the_cells_own_hanji_never_the_tl() {
    let mut rig = new_tps_rig();
    rig.type_tps("ㄏㄛˋ");
    rig.open_window();
    // The surface maps slot 0 to cell 1.
    let expected = rig
        .list
        .resolve(1, false)
        .expect("a second cell")
        .0
        .hanji
        .clone();
    let intent = ComposingKeyIntent::SelectCandidateSlot {
        slot: 0,
        flip: true,
    };
    assert!(rig.run(intent, &no_key()));
    let expected = expected.expect("the second candidate carries Hanji");
    assert_eq!(rig.calls(), ["list closed"]);
    assert_eq!(rig.manager.display_text(), expected);
}

#[test]
fn tps_ctrl_comma_writes_the_full_width_mark() {
    // trace: under TPS the derived width is full, so the chord's key maps:
    // `policies::full_width_mapped(",")` → `，`; the swap is tried first and
    // the surface refuses it.
    let mut rig = new_tps_rig();
    let ctrl_comma = KeyEventSnapshot::chord(Some(","), ",", KeyModifiers::CONTROL);
    assert!(rig.run(ComposingKeyIntent::PassThrough, &ctrl_comma));
    assert_eq!(rig.calls(), ["swap \"， \"", "insert \"，\""]);
}

#[test]
fn tps_space_on_a_closed_syllable_with_the_window_off_commits_as_shown_unspaced() {
    // trace: with Show Candidate Window off `refresh_list` keeps the list
    // empty, so no cell is highlighted and O1 commits; read by running:
    // `ㄏㄛˋ` shows 好, which the commit writes (Hanji conversion B2).
    let mut rig = new_tps_rig();
    rig.settings
        .set_bool(&keys::IS_CANDIDATE_WINDOW_ENABLED, false);
    rig.type_tps("ㄏㄛˋ");
    assert!(rig.list.is_empty());
    assert!(rig.run(ComposingKeyIntent::TpsKey(" ".to_owned()), &no_key()));
    assert_eq!(rig.calls(), ["commit 好", "list closed"]);
}

/// Hanji conversion H1 + B2: a closed reading shows converted; Enter
/// commits the Hanji as shown, with no auto space even with Auto-Space on.
#[test]
fn tps_a_closed_reading_shows_converted_and_enter_commits_it_as_shown() {
    let mut rig = new_tps_rig();
    rig.type_tps("ㄏㄛˋ");
    // trace, read by running: `ㄏㄛˋ` → 好.
    assert_eq!(rig.manager.display_text(), "好");
    assert!(rig.list.is_empty(), "typing opens no window (D7)");
    assert!(rig.run(ComposingKeyIntent::Commit, &no_key()));
    assert_eq!(rig.calls(), ["commit 好", "list closed"]);
}

/// B2: Shift+Enter commits the glyphs of the whole composition — the pick
/// as typed too, separators dropped.
#[test]
fn tps_commit_as_typed_writes_the_glyphs_of_the_whole_composition() {
    let mut rig = new_tps_rig();
    rig.type_tps("ㄏㄛˋ");
    rig.pick_first();
    rig.type_tps("ㄏㄛ ");
    assert!(rig.run(ComposingKeyIntent::CommitAsTyped, &no_key()));
    assert_eq!(rig.calls(), ["commit ㄏㄛˋㄏㄛ", "list closed"]);
}

/// H6: a key that commits first writes the composition as shown, then
/// itself, in one write. trace: `?` is full width under TPS → `？`.
#[test]
fn tps_punctuation_commits_the_composition_as_shown_then_itself() {
    let mut rig = new_tps_rig();
    rig.type_tps("ㄏㄛˋ");
    let question = KeyEventSnapshot::text("?", KeyModifiers::SHIFT);
    assert!(rig.run(
        ComposingKeyIntent::CommitThenInsert("?".to_owned()),
        &question
    ));
    assert_eq!(rig.calls(), ["commit 好？", "list closed"]);
}

/// H3: Backspace on an empty tail un-nails the pick and the reading is
/// converted again; no window comes up.
#[test]
fn tps_backspace_after_a_pick_reopens_the_word_converted() {
    let mut rig = new_tps_rig();
    rig.type_tps("ㄏㄛˋ");
    rig.pick_first();
    assert!(rig.run(ComposingKeyIntent::DeleteBackward, &no_key()));
    assert_eq!(rig.manager.raw_input(), "ㄏㄛˋ");
    assert_eq!(rig.manager.display_text(), "好");
    assert!(rig.calls().is_empty(), "{:?}", rig.calls());
}

/// H8: a settings change closes an open TPS list instead of re-presenting
/// it; under TL the same change keeps it (see the represent tests above).
#[test]
fn tps_a_settings_change_closes_the_open_list() {
    let mut rig = new_tps_rig();
    rig.type_tps("ㄏㄛˋ");
    rig.open_window();
    for refetch in [false, true] {
        assert!(!represent_list(
            &rig.settings,
            &mut rig.manager,
            &mut rig.list,
            refetch
        ));
        assert!(rig.list.is_empty());
        rig.open_window();
    }
}

/// D6 flash: the surface hears the cap of every TPS glyph the engine took,
/// and nothing else: not the separator (no cap), not a refused Space, not a
/// slot key picking.
#[test]
fn tps_the_surface_hears_the_cap_of_each_glyph_the_engine_took() {
    let mut rig = new_tps_rig();
    for key in ["ㄏ", "ㄛ", " "] {
        assert!(rig.run(ComposingKeyIntent::TpsKey(key.to_owned()), &no_key()));
    }
    // trace: KEYS `c` = ㄏ (row 3 cap 2: z x c), `i` = ㄛ (row 1 cap 7:
    // q w e r t y u i); the separator Space took, but has no cap.
    assert_eq!(
        rig.surface.typed_tps_caps,
        [
            TpsKeyCapIndex { row: 3, cap: 2 },
            TpsKeyCapIndex { row: 1, cap: 7 }
        ]
    );
    drop(rig);

    // trace: `ㄏㄛˋ` closes the syllable, so the Space is refused and opens
    // the window (O1 revised by D7); slot 0 then picks from it.
    let mut rig = new_tps_rig();
    rig.type_tps("ㄏㄛˋ");
    rig.surface.typed_tps_caps.clear();
    assert!(rig.run(ComposingKeyIntent::TpsKey(" ".to_owned()), &no_key()));
    assert!(!rig.list.is_empty(), "the refused Space opened the window");
    assert!(rig.run(
        ComposingKeyIntent::SelectCandidateSlot {
            slot: 0,
            flip: false
        },
        &no_key()
    ));
    assert!(
        rig.surface.typed_tps_caps.is_empty(),
        "{:?}",
        rig.surface.typed_tps_caps
    );
}

#[test]
fn tps_space_refused_inside_the_composition_does_nothing() {
    // trace: caret walked to the start of `ㄏㄛ`; nothing precedes it, so the
    // engine refuses Space away from the end — no commit, no list call.
    let mut rig = new_tps_rig();
    rig.type_tps("ㄏㄛ");
    for _ in 0..2 {
        rig.run(
            ComposingKeyIntent::MoveCaret(CaretDirection::Left),
            &no_key(),
        );
    }
    rig.surface.selected = Some(0);
    rig.surface.calls.clear();
    assert!(rig.run(ComposingKeyIntent::TpsKey(" ".to_owned()), &no_key()));
    assert!(rig.calls().is_empty(), "{:?}", rig.calls());
    assert_eq!(rig.manager.raw_input(), "ㄏㄛ");
}

/// TPS is full width with no way out (USER 2026-10-07): the Ctrl chord on a
/// mark outside the layout flips nothing, idle or composing.
#[test]
fn tps_ctrl_on_a_non_layout_mark_stays_full_width() {
    // trace: under TPS the derived width is full → `policies::full_width_mapped(
    // "[")` = `Some("「")`; `「` attaches to nothing, so no swap is tried.
    // Composing: the commit is TPS (no auto space, H6) → `好「`.
    let mut rig = new_tps_rig();
    let ctrl_bracket = KeyEventSnapshot::chord(Some("["), "[", KeyModifiers::CONTROL);
    assert!(rig.run(ComposingKeyIntent::PassThrough, &ctrl_bracket));
    assert_eq!(rig.calls(), ["insert \"「\""]);

    rig.surface.calls.clear();
    rig.type_tps("ㄏㄛˋ");
    assert!(rig.run(
        ComposingKeyIntent::CommitThenInsert("[".to_owned()),
        &ctrl_bracket
    ));
    assert_eq!(rig.calls(), ["commit 好「", "list closed"]);
}

/// An auto space armed before a switch into TPS: a bare attaching mark
/// swaps with it in full width, the only width TPS writes.
#[test]
fn tps_a_bare_mark_swaps_the_armed_space_in_full_width() {
    // trace: `?` is no TPS layout key → PassThrough; the width is full and
    // the swap attaches what is written: `full_width_mapped("?")` = `？`,
    // attaching (`punctuation.rs` ATTACHING) → swap `？ `.
    let mut rig = new_tps_rig();
    rig.surface.can_swap = true;
    let question = KeyEventSnapshot::text("?", KeyModifiers::SHIFT);
    assert!(rig.run(ComposingKeyIntent::PassThrough, &question));
    assert_eq!(rig.calls(), ["swap \"？ \"", "arm"]);
}

/// A composition a switch across TPS left behind, reached by a commit that
/// runs with no key (the picker's commit-first, a host's Finalize, the
/// Shift-tap English switch): written as shown, with no auto space even
/// though Auto-Space is on and the mode is now TL, and a highlighted cell is
/// not picked instead (Codex P3 post-impl BLOCK 1 and 2). trace, read by
/// running: without the guard in `perform_intent` the `Commit` wrote
/// `ㄍㄚ` then `" "` and armed it, and the highlighted pick wrote 共.
#[test]
fn a_keyless_commit_after_a_switch_across_tps_writes_the_glyphs_unspaced() {
    for (intent, expected) in [
        (
            ComposingKeyIntent::Commit,
            &["commit ㄍㄚ", "list closed", "list closed"][..],
        ),
        (
            ComposingKeyIntent::CommitHighlightedCandidate,
            &["commit ㄍㄚ", "list closed"][..],
        ),
    ] {
        let mut rig = new_tps_rig();
        rig.type_tps("ㄍㄚ");
        rig.surface.selected = Some(0);
        rig.settings.set_choice(&keys::INPUT_MODE, InputMode::Tl);
        assert!(rig.run(intent.clone(), &no_key()));
        assert_eq!(rig.calls(), expected, "{intent:?}");
    }
}
