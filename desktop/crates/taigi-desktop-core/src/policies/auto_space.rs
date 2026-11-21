//! When a committed word earns the auto-space trailing space — the one copy
//! the three desktops run (behavioural invariant §23,
//! `INVARIANT_AUTO_SPACE_PUNCTUATION_SWAP`). Whether a commit wrote
//! spaceable romanization is the engine's verdict
//! (`CommitResolution.earns_auto_space`), as is which punctuation attaches
//! (`engine::is_attaching_punctuation`), as on every platform.

/// True when this commit earns a trailing space at all. `is_spacing_enabled`
/// is the switch the commit reads — Auto-Space for the preedit as rendered,
/// [`spaces_candidate_commits`] for a candidate; `wrote_romanization` is the
/// engine's verdict on the string it resolved
/// (`CommitResolution.earns_auto_space`).
pub fn is_gate_active(is_spacing_enabled: bool, wrote_romanization: bool) -> bool {
    is_spacing_enabled && wrote_romanization
}

/// Whether a candidate commit that earns a space gets one: Auto-Space, or
/// effective Annotate in Brackets, which keeps every bracket unit spaced
/// (`INVARIANT_BRACKETS_SPACE_CANDIDATE_COMMITS`). The swap reads it too: its
/// arm only exists after a commit that wrote the space. Twin of Android
/// `CandidateClickHandler.kt` `spacesCandidateCommits`, iOS
/// `ActionHandler+Suggestions.swift` `spacesCandidateCommits`.
pub fn spaces_candidate_commits(is_auto_space_enabled: bool, is_output_both_scripts: bool) -> bool {
    is_auto_space_enabled || is_output_both_scripts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::is_attaching_punctuation;

    #[test]
    fn the_attaching_set_is_the_engines() {
        // trace: engine `phonetics::punctuation::ATTACHING` holds `？` and
        // `」`, not the opener `「` or a letter; the full roster is pinned
        // there.
        assert!(is_attaching_punctuation("？"));
        assert!(is_attaching_punctuation("」"));
        assert!(!is_attaching_punctuation("「"));
        assert!(!is_attaching_punctuation("a"));
    }

    #[test]
    fn the_gate_follows_whether_the_commit_wrote_romanization() {
        // trace: `is_gate_active` = enabled AND wrote_romanization — on only
        // for (true, true).
        assert!(!is_gate_active(false, true));
        assert!(!is_gate_active(false, false));
        assert!(is_gate_active(true, true));
        assert!(!is_gate_active(true, false));
    }

    #[test]
    fn either_setting_spaces_a_candidate_commit() {
        assert!(!spaces_candidate_commits(false, false));
        assert!(spaces_candidate_commits(true, false));
        assert!(spaces_candidate_commits(false, true));
        assert!(spaces_candidate_commits(true, true));
    }

    #[test]
    fn a_fresh_install_has_the_gate_off_for_romanization() {
        // trace: the shipped default is OFF on every platform
        // (`keys::IS_AUTO_SPACE_ENABLED` says why).
        let document = crate::settings::SettingsDocument::default();
        let enabled = document.bool(&crate::settings::keys::IS_AUTO_SPACE_ENABLED);
        assert!(!is_gate_active(enabled, true));
    }
}
