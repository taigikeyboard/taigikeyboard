//! What a global shortcut action decides, apart from how a shell carries it
//! out: the settings write, what becomes of the open candidate list, whether
//! the switch is announced, and what only the shell can start. The Linux and
//! web session (`session::perform_global`) turns the outcome into emits; the
//! Windows TSF shell carries it out with its own edit sessions and windows,
//! whose writes can be refused and whose list selection lives in the window
//! — the two reasons it cannot replay emits.

use crate::keys::ShortcutAction;
use crate::runtime::DesktopRuntime;
use crate::settings::{keys, CandidateDisplayMode, InputModeRequest};

/// What the open candidate list does after the action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListEffect {
    Untouched,
    /// Fetched under the old input mode: it goes with the mode that produced
    /// it.
    Clear,
    /// Presented again under the new settings, in place; `refetch` when the
    /// change alters which candidates exist (invariants §44), otherwise only
    /// how they draw.
    Represent {
        refetch: bool,
    },
}

/// What only the shell can start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellRequest {
    OpenSettings,
    /// Runs against the context the chord was pressed in
    /// (`ShortcutAction::needs_key_context`).
    ToggleSymbolPicker,
    /// Linux and web hand it to the settings app; Windows stores the wish
    /// itself and shows its own panel.
    ToggleTpsKeyboard,
    /// `TELEX_GUIDE_SHOWN` was flipped; the shell shows or hides the guide.
    TelexGuideChanged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlobalOutcome {
    pub list: ListEffect,
    /// The switch is announced — each shell in its own words: Linux the whole
    /// mode line (`session::mode_label`), Windows and the web page only what
    /// changed (`ShortcutAction::switched_label_key`).
    pub announces: bool,
    pub shell: Option<ShellRequest>,
}

impl GlobalOutcome {
    fn shell(request: ShellRequest) -> Self {
        Self {
            list: ListEffect::Untouched,
            announces: false,
            shell: Some(request),
        }
    }

    /// The input mode, the output script or the display mode switched: the
    /// one kind of answer that touches the list, so `list` is never
    /// `Untouched` here and the indicator re-reads the mode.
    fn mode_switch(list: ListEffect, announces: bool) -> Self {
        Self {
            list,
            announces,
            shell: None,
        }
    }
}

/// Applies `action`'s settings write and says what follows. `None`: nothing
/// changed — the swap under roman-only (`allows_swap_toggle`), or no store to
/// write to. The caller refuses an action inert under the input mode
/// (`ShortcutAction::is_inert_under`) before calling, and before it takes an
/// open symbol picker down, which every other answer does.
pub fn apply_global_action(
    runtime: &DesktopRuntime,
    action: ShortcutAction,
) -> Option<GlobalOutcome> {
    match action {
        ShortcutAction::OpenLastSettingsPane => {
            Some(GlobalOutcome::shell(ShellRequest::OpenSettings))
        }
        ShortcutAction::ShowSymbolPicker => {
            Some(GlobalOutcome::shell(ShellRequest::ToggleSymbolPicker))
        }
        ShortcutAction::ShowTpsKeyboard => {
            Some(GlobalOutcome::shell(ShellRequest::ToggleTpsKeyboard))
        }
        ShortcutAction::ToggleRomanization => {
            switch_input_mode(runtime, InputModeRequest::ToggleRomanization)
        }
        ShortcutAction::ToggleTps => switch_input_mode(runtime, InputModeRequest::ToggleTps),
        ShortcutAction::ToggleTranslateSwapped => {
            let display_mode = runtime
                .settings
                .current()
                .engine_settings()
                .candidate_display_mode;
            if !display_mode.allows_swap_toggle() {
                return None;
            }
            // The list STAYS: the swap changes how a candidate displays —
            // re-presented in place, selection kept. Under side by side with
            // romanized words as candidates it also changes which exist (the
            // romanization twins are listed only when the Hanji leads,
            // `AppConfig::lists_roman_twins`), so the list is fetched again
            // and the selection follows its word. Announced even over an
            // open list (USER 2026-10-31), side by side only: under combined
            // a commit still writes the Hanji.
            let swap_changes_the_list = swap_changes_the_list(runtime);
            runtime
                .update_settings("toggle_translate_swapped", |document| {
                    let swapped = document.bool(&keys::IS_HANJI_FIRST);
                    document.set_bool(&keys::IS_HANJI_FIRST, !swapped);
                })
                .then_some(GlobalOutcome::mode_switch(
                    ListEffect::Represent {
                        refetch: swap_changes_the_list,
                    },
                    display_mode == CandidateDisplayMode::SideBySide,
                ))
        }
        ShortcutAction::CycleCandidateDisplayMode => runtime
            .update_settings("cycle_candidate_display_mode", |document| {
                let next = document.choice(&keys::CANDIDATE_DISPLAY_MODE).next();
                document.set_choice(&keys::CANDIDATE_DISPLAY_MODE, next);
            })
            .then_some(GlobalOutcome::mode_switch(
                ListEffect::Represent { refetch: true },
                true,
            )),
        ShortcutAction::ShowTelexGuide => runtime
            .update_settings("toggle_telex_guide", |document| {
                let is_shown = document.bool(&keys::TELEX_GUIDE_SHOWN);
                document.set_bool(&keys::TELEX_GUIDE_SHOWN, !is_shown);
            })
            .then_some(GlobalOutcome::shell(ShellRequest::TelexGuideChanged)),
    }
}

/// Whether the swap changes which candidates exist, not only how they draw:
/// side by side, with romanized words as candidates in effect
/// (`SettingsDocument::allows_han_lo_mixed`).
fn swap_changes_the_list(runtime: &DesktopRuntime) -> bool {
    let document = runtime.settings.current();
    document.choice::<CandidateDisplayMode>(&keys::CANDIDATE_DISPLAY_MODE)
        == CandidateDisplayMode::SideBySide
        && document.bool(&keys::IS_HAN_LO_MIXED_ENABLED)
        && document.allows_han_lo_mixed()
}

/// One of the two input-script switches; the candidates on screen were
/// fetched under the old mode and go with it.
fn switch_input_mode(runtime: &DesktopRuntime, request: InputModeRequest) -> Option<GlobalOutcome> {
    runtime
        .update_settings("switch_input_mode", |document| {
            document.switch_input_mode(request);
        })
        .then_some(GlobalOutcome::mode_switch(ListEffect::Clear, true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::test_support::runtime_with_store as runtime;
    use crate::settings::{InputMode, SettingsDocument};

    #[test]
    fn no_store_changes_nothing() {
        let runtime = crate::session::test_support::runtime(SettingsDocument::default());
        assert_eq!(
            apply_global_action(&runtime, ShortcutAction::ToggleRomanization),
            None
        );
    }

    #[test]
    fn shell_only_actions_write_nothing_and_leave_the_list() {
        let runtime = runtime(SettingsDocument::default());
        for (action, request) in [
            (
                ShortcutAction::OpenLastSettingsPane,
                ShellRequest::OpenSettings,
            ),
            (
                ShortcutAction::ShowSymbolPicker,
                ShellRequest::ToggleSymbolPicker,
            ),
            (
                ShortcutAction::ShowTpsKeyboard,
                ShellRequest::ToggleTpsKeyboard,
            ),
        ] {
            let outcome = apply_global_action(&runtime, action).expect("shell request");
            assert_eq!(outcome.shell, Some(request));
            assert_eq!(outcome.list, ListEffect::Untouched);
            assert!(!outcome.announces);
        }
    }

    #[test]
    fn the_romanization_switch_clears_the_list_and_announces() {
        let runtime = runtime(SettingsDocument::default());
        let outcome =
            apply_global_action(&runtime, ShortcutAction::ToggleRomanization).expect("written");
        assert_eq!(outcome.list, ListEffect::Clear);
        assert!(outcome.announces);
        // trace: TL default → the romanization switch's next stop, POJ.
        assert_eq!(
            runtime
                .settings
                .current()
                .choice::<InputMode>(&keys::INPUT_MODE),
            InputMode::Poj
        );
    }

    #[test]
    fn the_swap_is_announced_side_by_side_only_and_refused_under_roman_only() {
        let mut side_by_side = SettingsDocument::default();
        side_by_side.set_choice(
            &keys::CANDIDATE_DISPLAY_MODE,
            CandidateDisplayMode::SideBySide,
        );
        let swapping = runtime(side_by_side);
        let was_hanji_first = swapping.settings.current().bool(&keys::IS_HANJI_FIRST);
        let outcome = apply_global_action(&swapping, ShortcutAction::ToggleTranslateSwapped)
            .expect("written");
        assert_eq!(outcome.list, ListEffect::Represent { refetch: false });
        assert!(outcome.announces);
        assert_eq!(
            swapping.settings.current().bool(&keys::IS_HANJI_FIRST),
            !was_hanji_first
        );

        // Romanized words as candidates: the swap changes which exist.
        let mut han_lo = SettingsDocument::default();
        han_lo.set_choice(
            &keys::CANDIDATE_DISPLAY_MODE,
            CandidateDisplayMode::SideBySide,
        );
        han_lo.set_bool(&keys::IS_HAN_LO_MIXED_ENABLED, true);
        let outcome = apply_global_action(&runtime(han_lo), ShortcutAction::ToggleTranslateSwapped)
            .expect("written");
        assert_eq!(outcome.list, ListEffect::Represent { refetch: true });

        let mut combined = SettingsDocument::default();
        combined.set_choice(
            &keys::CANDIDATE_DISPLAY_MODE,
            CandidateDisplayMode::Combined,
        );
        let outcome =
            apply_global_action(&runtime(combined), ShortcutAction::ToggleTranslateSwapped)
                .expect("written");
        assert!(!outcome.announces);

        let mut roman_only = SettingsDocument::default();
        roman_only.set_choice(
            &keys::CANDIDATE_DISPLAY_MODE,
            CandidateDisplayMode::RomanOnly,
        );
        assert_eq!(
            apply_global_action(&runtime(roman_only), ShortcutAction::ToggleTranslateSwapped),
            None
        );
    }

    #[test]
    fn the_display_mode_cycle_refetches_and_the_guide_is_the_shells() {
        let runtime = runtime(SettingsDocument::default());
        let cycle = apply_global_action(&runtime, ShortcutAction::CycleCandidateDisplayMode)
            .expect("written");
        assert_eq!(cycle.list, ListEffect::Represent { refetch: true });
        assert!(cycle.announces);
        let guide = apply_global_action(&runtime, ShortcutAction::ShowTelexGuide).expect("written");
        assert_eq!(guide.shell, Some(ShellRequest::TelexGuideChanged));
        assert!(runtime.settings.current().bool(&keys::TELEX_GUIDE_SHOWN));
    }
}
