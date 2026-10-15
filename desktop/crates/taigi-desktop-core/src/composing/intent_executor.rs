//! What one classified key DOES to a composition — every desktop input
//! method runs it (macOS through `taigi-macos-ffi`): the engine calls per
//! [`ComposingKeyIntent`], the candidate commit, the trailing auto space and
//! its swap (§23), the punctuation this input method writes itself. Each
//! shell keeps its document and its list window behind [`IntentSurface`].

use super::{
    CandidateCommitOutcome, CandidateListChange, CandidateSource, ComposingEffectExecutor,
    ComposingManager, TpsKeyOutcome,
};
use crate::engine;
use crate::keys::{
    tps_keyboard_cap_of, CandidateNavigation, ComposingKeyIntent, KeyEventSnapshot, TpsKeyCapIndex,
};
use crate::policies;
use crate::settings::{keys, InputMode, SettingsDocument};

/// The document and the list window, as the executor reaches them. It is
/// also the executor the engine's effects are replayed on.
pub trait IntentSurface: ComposingEffectExecutor {
    /// Writes `text` outside any composition (a mapped punctuation, a
    /// picked symbol, the auto space).
    fn insert_external(&mut self, text: &str);

    /// Replaces the space the last commit left with `replacement` (the
    /// swap, `guá ` + `，` → `guá，`). `false` when no swap is armed for
    /// this key, or the document cannot take it.
    fn swap_preceding_space(&mut self, replacement: &str) -> bool;

    /// Arms the swap for the next key, at the caret the last write left.
    fn arm_swap(&mut self);

    /// Whether a write of this key has failed so far (a surface keeps its
    /// first failure). A surface that cannot fail answers `false`.
    fn has_write_failed(&self) -> bool;

    /// The list was refetched: show it, update it in place, or hide it.
    fn list_changed(&mut self, list: &mut CandidateSource);

    /// The list closed with the composition.
    fn list_closed(&mut self);

    /// The highlighted cell, if a list shows one.
    fn selected_index(&self) -> Option<usize>;

    /// The cell the `slot`-th selection key addresses on the page shown.
    fn index_for_key_slot(&self, slot: usize) -> Option<usize>;

    fn navigate(&mut self, direction: CandidateNavigation);

    /// The engine took a TPS glyph typed by the key at `cap`: the on-screen
    /// key panel flashes that cap. A click on the panel arrives here too, as
    /// the same key; the separator Space, which has no cap, does not.
    /// Desktop TPS roadmap D6.
    fn tps_keyboard_cap_typed(&mut self, cap: TpsKeyCapIndex);
}

/// Runs `intent` for the key `snapshot`. Answers whether the key was
/// consumed; `false` hands it to the client.
pub fn perform_intent(
    intent: &ComposingKeyIntent,
    snapshot: &KeyEventSnapshot,
    settings: &SettingsDocument,
    manager: &mut ComposingManager,
    list: &mut CandidateSource,
    surface: &mut impl IntentSurface,
) -> bool {
    // A key path reads the key as idle once this ran (the shells call it
    // before classifying); the commits that run with no key — the picker's
    // commit-first, a host's Finalize, the Shift-tap English switch — reach
    // it only here, and a commit or pick after it finds nothing left.
    commit_composition_left_by_mode_change(settings, manager, list, surface);
    match intent {
        ComposingKeyIntent::Input(text) => {
            manager.append(text, surface);
            refresh(settings, manager, list, surface);
            true
        }
        ComposingKeyIntent::TelexKey(key) => {
            manager.telex_key(key, surface);
            refresh(settings, manager, list, surface);
            true
        }
        ComposingKeyIntent::TpsKey(key) => {
            match manager.tps_key(key, surface) {
                // Typing TPS fetches nothing: the window opens on demand
                // (roadmap § D7), and a key typed over it takes it down. The
                // engine's answer already carries the converted preedit.
                TpsKeyOutcome::Taken => {
                    if let Some(cap) = tps_keyboard_cap_of(key) {
                        surface.tps_keyboard_cap_typed(cap);
                    }
                    close_open_list(list, surface);
                }
                // A Space after the closed last syllable (O1, revised by D7):
                // it opens the window for the last word, as Zhuyin input
                // methods do — a window already up took the Space as its
                // confirm in the classifier. With the window switched off it
                // commits the composition as shown — no space after, TPS
                // takes none.
                TpsKeyOutcome::Refused {
                    is_caret_at_end: true,
                } if key == " " => {
                    if settings.bool(&keys::IS_CANDIDATE_WINDOW_ENABLED) {
                        refresh(settings, manager, list, surface);
                    } else {
                        manager.commit_composition(surface);
                        close_list(list, surface);
                    }
                }
                // Refused inside the composition, or the round trip failed:
                // nothing to type and nothing to commit.
                TpsKeyOutcome::Refused { .. } | TpsKeyOutcome::Failed => {}
            }
            true
        }
        ComposingKeyIntent::DeleteBackward => {
            manager.delete_backward(surface);
            if is_tps(settings) {
                close_open_list(list, surface);
            } else {
                refresh(settings, manager, list, surface);
            }
            true
        }
        ComposingKeyIntent::Commit => {
            // The preedit as rendered, whichever script the candidate list
            // led with: the romanization as typed under TL and POJ, the
            // Hanji conversion as shown under TPS.
            let committed = manager.commit_composition(surface);
            close_list(list, surface);
            let earns_auto_space = committed.is_some_and(|text| {
                raw_preedit_wrote_romanization(settings) && policies::should_append_space(&text)
            });
            append_auto_space(earns_auto_space, settings, surface);
            true
        }
        ComposingKeyIntent::CommitAsTyped => {
            // TPS only: glyphs, which earn no space.
            manager.commit_composition_as_typed(surface);
            close_list(list, surface);
            true
        }
        ComposingKeyIntent::Cancel => {
            manager.cancel_composition(surface);
            close_list(list, surface);
            true
        }
        ComposingKeyIntent::CommitThenInsert(text) => {
            // Mapped before the auto-space augmentation so the full-width
            // character rides the same single mutation as the commit. Both
            // rewrites CAN fire: this path commits the preedit as typed —
            // romanization under TL and POJ, whichever script the list led
            // with — while the full-width map still answers to the output
            // MODE, so Hanji-first gets `taigi？ ` (macOS pins the same pair).
            // Under TPS the composition is written as shown and earns no
            // space.
            let document_text =
                document_punctuation(settings, text).unwrap_or_else(|| text.clone());
            let gate = auto_space_gate(settings, raw_preedit_wrote_romanization(settings));
            let insert = policies::augment_insert(&document_text, manager.display_text(), gate);
            let committed = manager.commit_composition_then_insert(&insert.text, surface);
            close_list(list, surface);
            if insert.leaves_trailing_auto_space && committed.is_some() {
                surface.arm_swap();
            }
            true
        }
        ComposingKeyIntent::CommitThenPassThrough => {
            manager.commit_composition(surface);
            close_list(list, surface);
            false
        }
        ComposingKeyIntent::PassThrough => {
            let Some(typed) =
                ComposingKeyIntent::document_text(snapshot, settings.choice(&keys::INPUT_MODE))
            else {
                return false;
            };
            // The swap is read before the width (the word in front of the
            // caret is romanization, which keeps Latin marks); TPS has only
            // the full width, so there the swap attaches the glyph written.
            let punctuation = document_punctuation(settings, &typed);
            let swapping = if is_tps(settings) {
                punctuation.as_deref().unwrap_or(&typed)
            } else {
                &typed
            };
            if swap_auto_space(swapping, settings, manager, surface) {
                return true;
            }
            // Text this input method writes itself: punctuation the client
            // cannot map a key to, and a remapped key's text, which the host
            // would type as the QWERTY key's (mid-composition the classifier
            // already makes printable text `CommitThenInsert`).
            let written = punctuation.or_else(|| snapshot.is_remapped.then(|| typed.clone()));
            if let Some(text) = written {
                surface.insert_external(&text);
                manager.note_character_typed_outside_composition(&text);
                return true;
            }
            manager.note_character_typed_outside_composition(&typed);
            false
        }
        ComposingKeyIntent::CommitHighlightedCandidate => {
            let cell = surface.selected_index();
            commit_candidate(cell, false, settings, manager, list, surface);
            true
        }
        // Output the Other Script: the highlighted cell's OTHER script.
        ComposingKeyIntent::CommitAlternateScript => {
            let cell = surface.selected_index();
            commit_candidate(cell, true, settings, manager, list, surface);
            true
        }
        ComposingKeyIntent::SelectCandidateSlot { slot, flip } => {
            // A chord aimed at an empty slot is consumed all the same. `flip`
            // is Shift on the key: the same cell in its other script.
            let cell = surface.index_for_key_slot(*slot);
            commit_candidate(cell, *flip, settings, manager, list, surface);
            true
        }
        ComposingKeyIntent::Navigate(direction) => {
            surface.navigate(*direction);
            true
        }
        ComposingKeyIntent::OpenCandidates => {
            // An empty fetch leaves the composition up and the window shut.
            refresh(settings, manager, list, surface);
            true
        }
        ComposingKeyIntent::CloseCandidates => {
            close_list(list, surface);
            true
        }
        ComposingKeyIntent::MoveCaret(direction) => {
            // No refetch: the text did not change, so the candidates, the
            // highlight and the page still describe it. Under TPS the window
            // goes: it is opened for the word before the caret, and the
            // caret left it.
            manager.move_caret(*direction, surface);
            if is_tps(settings) {
                close_open_list(list, surface);
            }
            true
        }
    }
}

/// Ends a composition a switch across TPS left behind
/// (`ComposingManager::is_left_by_mode_change`): commits it as shown — the
/// raw commit writes the preedit, whatever the mode now — and closes its
/// list, so a glyph buffer never
/// takes a Latin key, nor a romanization buffer a glyph. `perform_intent`
/// runs it first for whatever reaches it, and every shell sends a keyless
/// `Commit` through it at the start of a key, before the key is classified
/// as idle (`ComposingManager::is_left_by_mode_change`); that covers both
/// switch chords, the menu, the settings window and a restore, without an
/// edit session outside a key. No auto space: the switch, not a word
/// boundary, ended it. Answers whether it committed.
fn commit_composition_left_by_mode_change(
    settings: &SettingsDocument,
    manager: &mut ComposingManager,
    list: &mut CandidateSource,
    surface: &mut impl IntentSurface,
) -> bool {
    if !manager.is_left_by_mode_change(settings.choice(&keys::INPUT_MODE)) {
        return false;
    }
    manager.commit_composition(surface);
    close_list(list, surface);
    true
}

/// A symbol from the symbol picker, written at the caret as one string (so a
/// bracket pair lands as both halves) and not through the full-width map:
/// what the user picked is what they get. An attaching mark swaps with the
/// auto space a commit left, as a typed one would.
pub fn insert_symbol(
    symbol: &str,
    settings: &SettingsDocument,
    manager: &mut ComposingManager,
    surface: &mut impl IntentSurface,
) {
    if !swap_auto_space(symbol, settings, manager, surface) {
        surface.insert_external(symbol);
        manager.note_character_typed_outside_composition(symbol);
    }
}

/// Whether a key the classifier passes through is one this input method
/// consumes all the same: punctuation it writes itself (full width, the TPS
/// punctuation chord included), the text of a remapped key
/// (`KeyEventSnapshot::is_remapped`), or an attaching mark right after an
/// auto space (`is_swap_armed`: the shell's best answer before the key
/// runs — at least the arm; a shell that knows its document cannot take the
/// swap folds that in too).
pub fn pass_through_may_consume(
    snapshot: &KeyEventSnapshot,
    settings: &SettingsDocument,
    is_swap_armed: bool,
) -> bool {
    let Some(typed) =
        ComposingKeyIntent::document_text(snapshot, settings.choice(&keys::INPUT_MODE))
    else {
        return false;
    };
    if snapshot.is_remapped || document_punctuation(settings, &typed).is_some() {
        return true;
    }
    is_swap_armed
        && settings.bool(&keys::IS_AUTO_SPACE_ENABLED)
        && engine::is_attaching_punctuation(&typed)
}

/// Re-reads the candidates for the composition as it now stands. With the
/// Show Candidate Window setting off nothing is fetched, not merely not
/// shown: a list kept behind no window would hand Space and the slot keys to
/// candidates the user cannot see.
pub fn refresh_list(
    settings: &SettingsDocument,
    manager: &mut ComposingManager,
    list: &mut CandidateSource,
) {
    if !settings.bool(&keys::IS_CANDIDATE_WINDOW_ENABLED) {
        list.clear();
        return;
    }
    match manager.fetch_candidates().list_change() {
        CandidateListChange::Replace(candidates) => list.set(candidates, manager),
        CandidateListChange::Clear => list.clear(),
    }
}

/// Re-presents an OPEN list under the settings in force right now, after a
/// switch that changed them: refetched when the change alters which
/// candidates exist (`refetch`), otherwise the same list re-rendered in
/// place. An empty list stays empty — a switch never opens one — and a
/// refetch obeys the Show Candidate Window setting like any other. Under
/// TPS the list closes: it was opened for one word of a conversion the new
/// settings may walk differently (Hanji conversion H8). Answers whether a
/// list is left to show.
pub fn represent_list(
    settings: &SettingsDocument,
    manager: &mut ComposingManager,
    list: &mut CandidateSource,
    refetch: bool,
) -> bool {
    if manager.is_tps_composition() {
        list.clear();
    }
    if list.is_empty() {
        return false;
    }
    if refetch {
        refresh_list(settings, manager, list);
    } else {
        list.refresh_presentation(manager);
    }
    !list.is_empty()
}

fn refresh(
    settings: &SettingsDocument,
    manager: &mut ComposingManager,
    list: &mut CandidateSource,
    surface: &mut impl IntentSurface,
) {
    refresh_list(settings, manager, list);
    surface.list_changed(list);
}

fn close_list(list: &mut CandidateSource, surface: &mut impl IntentSurface) {
    list.clear();
    surface.list_closed();
}

/// `close_list` for a list that may not be up — under TPS typing, where the
/// window is shut on almost every key and a close each time would be noise.
fn close_open_list(list: &mut CandidateSource, surface: &mut impl IntentSurface) {
    if !list.is_empty() {
        close_list(list, surface);
    }
}

fn is_tps(settings: &SettingsDocument) -> bool {
    settings.choice(&keys::INPUT_MODE) == InputMode::Tps
}

/// Commits the candidate behind cell `cell`, in the cell's own script or
/// (`flip`) the other one; the engine resolves what that script writes and
/// whether it earns the auto space. No cell (no list, an index past it)
/// commits nothing. Under TPS there is no other script to flip to — the
/// engine's `Other` would write raw TL — so a flip commits the cell's own.
/// A nail refetches the list when the engine asks for it; a pick under a TPS
/// Hanji conversion does not, and the window closes (H4).
fn commit_candidate(
    cell: Option<usize>,
    flip: bool,
    settings: &SettingsDocument,
    manager: &mut ComposingManager,
    list: &mut CandidateSource,
    surface: &mut impl IntentSurface,
) {
    let flip = flip && !is_tps(settings);
    let Some((candidate, script)) = cell.and_then(|index| list.resolve(index, flip)) else {
        return;
    };
    let outcome = manager.commit_candidate(candidate, script, surface);
    log::debug!("candidate.commit {outcome:?}");
    match outcome {
        CandidateCommitOutcome::Finalized { earns_auto_space } => {
            close_list(list, surface);
            append_auto_space(earns_auto_space, settings, surface);
        }
        CandidateCommitOutcome::Nailed { refetch: false } => close_list(list, surface),
        CandidateCommitOutcome::Nailed { refetch: true }
        | CandidateCommitOutcome::Ignored
        | CandidateCommitOutcome::Unavailable => refresh(settings, manager, list, surface),
    }
}

/// The swap for `text` about to be written outside a composition. The arm's
/// EXISTENCE is the verdict — it is only ever set after a commit that wrote
/// romanization earned its space — so only Auto-Space itself is re-read
/// live here. On success the swap is re-armed (`?!` chains) and the engine
/// hears about the character as the end of a context.
fn swap_auto_space(
    text: &str,
    settings: &SettingsDocument,
    manager: &mut ComposingManager,
    surface: &mut impl IntentSurface,
) -> bool {
    if !settings.bool(&keys::IS_AUTO_SPACE_ENABLED)
        || !engine::is_attaching_punctuation(text)
        || !surface.swap_preceding_space(&format!("{text} "))
    {
        return false;
    }
    manager.note_character_typed_outside_composition(text);
    surface.arm_swap();
    true
}

/// The trailing auto space after an explicit commit that earned one (it
/// wrote romanization with no trailing `-`), and the swap armed on it — with
/// Auto-Space live, and not once a write of this key has failed. Lifecycle
/// commits never come here.
fn append_auto_space(
    earns_auto_space: bool,
    settings: &SettingsDocument,
    surface: &mut impl IntentSurface,
) {
    if !auto_space_gate(settings, earns_auto_space) {
        return;
    }
    surface.insert_external(" ");
    if !surface.has_write_failed() {
        surface.arm_swap();
    }
}

/// The gate every auto-space site reads: Auto-Space live, and the commit's
/// own verdict — the engine's `earns_auto_space` for a candidate, the input
/// mode for the preedit as typed.
fn auto_space_gate(settings: &SettingsDocument, wrote_romanization: bool) -> bool {
    policies::is_gate_active(
        settings.bool(&keys::IS_AUTO_SPACE_ENABLED),
        wrote_romanization,
    )
}

/// Whether committing the preedit AS TYPED writes romanization. One key
/// read, not a whole `engine_settings()` snapshot: this runs per keystroke.
fn raw_preedit_wrote_romanization(settings: &SettingsDocument) -> bool {
    policies::raw_preedit_writes_romanization(settings.choice(&keys::INPUT_MODE))
}

/// The full-width mark this input method writes for `text`, or `None` when
/// the host writes the key: mapped only under the DERIVED width, so
/// roman-only stays half-width, combined follows the stored swap and TPS is
/// always full.
fn document_punctuation(settings: &SettingsDocument, text: &str) -> Option<String> {
    settings
        .engine_settings()
        .is_full_width_punctuation
        .then(|| policies::full_width_mapped(text))
        .flatten()
}
