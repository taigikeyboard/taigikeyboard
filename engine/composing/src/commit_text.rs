//! What an R5 `CommitContinuous` writes into the composition, or a
//! `CommitPrediction` (D1) into the document — the document text of one pick
//! and whether it carries romanization — resolved by the engine from the
//! pick's scripts and the request's output settings.
//!
//! One port of the platform resolvers (behavioral-invariants §23 / §42): the
//! removed desktop / macOS resolvers (R5 PR-c) and the removed iOS
//! `formatOutputText` / `markedCellCommit` and Android
//! `resolveUnmarkedCommit` / `resolveMarkedCellCommit` prediction resolvers
//! (D2).
//!
//! On the TPS layout the romanization a commit writes is Bopomofo
//! ([`phonetics::api::tl_display_to_tps`]): the bracket half of a Hanji
//! pick, and the whole of a Hanji-less pick (R5 P2, USER 2026-09-30).

use phonetics::api::tl_display_to_tps;
use protos::engine::{AppConfig, CandidateDisplayMode, CommitOutcome, CommitResolution};

use crate::api::CommitScript;

/// One pick's document text, and whether writing it puts romanization in
/// the document — the verdict the auto-space gate reads (§23).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResolvedCommit {
    pub(crate) text: String,
    pub(crate) wrote_romanization: bool,
}

impl ResolvedCommit {
    fn romanization(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            wrote_romanization: true,
        }
    }

    fn hanji(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            wrote_romanization: false,
        }
    }

    /// Bopomofo takes no word spacing, so it earns no auto space.
    fn bopomofo(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            wrote_romanization: false,
        }
    }
}

/// The TPS layout's rendering of a pick's romanization: the Bopomofo its cell
/// shows (iOS `CandidateCellHelper` / Android `CandidateStrip.kt`
/// `tlDisplayToTps(roman)`).
#[derive(Clone, Copy)]
struct Bopomofo {
    or_maps_to_er: bool,
}

impl Bopomofo {
    fn render(self, roman: &str) -> String {
        tl_display_to_tps(roman, self.or_maps_to_er)
    }
}

/// The output settings a commit renders under, after the Candidate Display
/// projections (§42): Romanization Only masks the lead and bracket flags (a
/// §42 split cell, which that mode never shows, still commits its own script);
/// Hanji with Romanization forces only the Hanji lead. The platforms already
/// send the projected flags; applying them here too keeps the resolver right
/// for any caller. The TPS layout ignores Candidate Display: Hanji leads, and
/// the lead's romanization renders as Bopomofo.
struct OutputScripts {
    hanji_leads: bool,
    annotate_in_brackets: bool,
    shows_hanji: bool,
    bopomofo: Option<Bopomofo>,
}

impl OutputScripts {
    fn of(config: &AppConfig) -> Self {
        if config.is_tps_layout() {
            return Self {
                hanji_leads: true,
                annotate_in_brackets: config.output_both_scripts,
                shows_hanji: true,
                bopomofo: Some(Bopomofo {
                    or_maps_to_er: config.tps_or_maps_to_er,
                }),
            };
        }
        if config.is_roman_only_display() {
            return Self {
                hanji_leads: false,
                annotate_in_brackets: false,
                shows_hanji: false,
                bopomofo: None,
            };
        }
        Self {
            hanji_leads: hanji_leads(config),
            annotate_in_brackets: config.output_both_scripts,
            shows_hanji: true,
            bopomofo: None,
        }
    }
}

/// Whether a commit leads with the Hanji: the composition renders Hanji first
/// (the swap, or the TPS layout), or Hanji with Romanization forces the lead.
/// The one place the lead is decided.
fn hanji_leads(config: &AppConfig) -> bool {
    config.renders_hanji_first()
        || config.candidate_display_mode() == CandidateDisplayMode::Combined
}

/// What committing `script` of the pick `(roman, hanji)` writes, or `None`
/// when that script does not exist (`Other` on a one-script pick or under
/// Romanization Only) — the commit is then ignored. An empty `hanji` is no
/// Hanji.
pub(crate) fn resolve_commit_text(
    script: CommitScript,
    roman: &str,
    hanji: Option<&str>,
    config: &AppConfig,
) -> Option<ResolvedCommit> {
    let scripts = OutputScripts::of(config);
    let hanji = hanji.filter(|hanji| !hanji.is_empty());
    match (script, hanji) {
        (CommitScript::Hanji, Some(hanji)) => {
            Some(if scripts.annotate_in_brackets && !roman.is_empty() {
                ResolvedCommit::romanization(format!("{hanji} ({roman})"))
            } else {
                ResolvedCommit::hanji(hanji)
            })
        }
        (CommitScript::Roman, _) if !roman.is_empty() => Some(ResolvedCommit::romanization(roman)),
        (CommitScript::Other, None) => None,
        (CommitScript::Other, Some(hanji)) => scripts.shows_hanji.then(|| {
            if scripts.hanji_leads {
                ResolvedCommit::romanization(roman)
            } else {
                ResolvedCommit::hanji(hanji)
            }
        }),
        // Lead, and a split cell whose own script is missing.
        (_, hanji) => Some(lead(roman, hanji, &scripts)),
    }
}

fn lead(roman: &str, hanji: Option<&str>, scripts: &OutputScripts) -> ResolvedCommit {
    let Some(hanji) = hanji else {
        // §34 literal, an OOV name, a roman-only custom row: romanization
        // under every mode — as Bopomofo on the TPS layout, what the cell
        // shows (R5 P2).
        return match scripts.bopomofo {
            Some(bopomofo) => ResolvedCommit::bopomofo(bopomofo.render(roman)),
            None => ResolvedCommit::romanization(roman),
        };
    };
    if scripts.annotate_in_brackets && !roman.is_empty() {
        // The pair carries the romanization whichever half leads — Bopomofo
        // on the TPS layout (iOS / Android `bracketRoman`). No empty
        // brackets (`台語 ()`): desktop and macOS call them a visible defect,
        // and the split hanji cell above refuses them too.
        let bracket_roman = match scripts.bopomofo {
            Some(bopomofo) => bopomofo.render(roman),
            None => roman.to_owned(),
        };
        let text = if scripts.hanji_leads {
            format!("{hanji} ({bracket_roman})")
        } else {
            format!("{bracket_roman} ({hanji})")
        };
        return ResolvedCommit::romanization(text);
    }
    if scripts.hanji_leads {
        ResolvedCommit::hanji(hanji)
    } else {
        ResolvedCommit::romanization(roman)
    }
}

/// The wire answer for an R5 commit: `resolved` is what the pick wrote,
/// unless `outcome` says nothing changed. The auto-space verdict is the
/// platforms' `shouldAppendAutoSpace` minus the live setting, on a final
/// commit only.
pub(crate) fn commit_resolution(
    outcome: CommitOutcome,
    resolved: Option<ResolvedCommit>,
) -> CommitResolution {
    let Some(resolved) = resolved.filter(|_| outcome != CommitOutcome::Ignored) else {
        return CommitResolution {
            outcome: CommitOutcome::Ignored as i32,
            ..CommitResolution::default()
        };
    };
    let earns_auto_space = outcome == CommitOutcome::Finalized
        && resolved.wrote_romanization
        && !resolved.text.ends_with('-');
    CommitResolution {
        outcome: outcome as i32,
        document_text: resolved.text,
        wrote_romanization: resolved.wrote_romanization,
        earns_auto_space,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(swapped: bool, brackets: bool, display: CandidateDisplayMode) -> AppConfig {
        AppConfig {
            input_mode: "tl".into(),
            is_hanji_first: swapped,
            output_both_scripts: brackets,
            candidate_display_mode: display as i32,
            ..AppConfig::default()
        }
    }

    #[test]
    fn truth_table() {
        use CandidateDisplayMode::{Combined, RomanOnly, SideBySide};
        use CommitScript::{Hanji, Lead, Other, Roman};
        const R: &str = "tâi-gí";
        const H: Option<&str> = Some("台語");
        let roman = |text: &str| Some((text.to_owned(), true));
        let hanji = |text: &str| Some((text.to_owned(), false));
        // (script, swapped, brackets, display, roman, hanji) → (text, wrote_romanization)
        #[rustfmt::skip]
        let rows = [
            // trace: the removed phone resolvers (D2): Android UnmarkedCommitResolverTest / iOS formatOutputText
            // romanLed_… / hanjiLed_… / brackets_writeThePairEitherWayRound_…
            (Lead, false, false, SideBySide, R, H, roman(R)),
            (Lead, true, false, SideBySide, R, H, hanji("台語")),
            (Lead, true, true, SideBySide, R, H, roman("台語 (tâi-gí)")),
            (Lead, false, true, SideBySide, R, H, roman("tâi-gí (台語)")),
            // Android `bracketedCommit` with an empty roman keeps its brackets.
            (Lead, true, true, SideBySide, "", H, hanji("台語")),
            // noHanji_commitsTheRomanizationUnderEveryMode; ported from the
            // removed desktop / macOS resolvers (R5 PR-c)
            // a_candidate_with_no_hanji_always_carries_romanization
            (Lead, true, false, SideBySide, R, None, roman(R)),
            (Lead, true, true, SideBySide, R, None, roman(R)),
            (Lead, false, true, SideBySide, R, Some(""), roman(R)),
            (Lead, true, false, SideBySide, R, Some(""), roman(R)),
            // removed desktop resolver (R5 PR-c)
            // roman_only_mode_shows_and_writes_the_romanization_alone
            (Lead, true, true, RomanOnly, R, H, roman(R)),
            // trace: the removed MarkedCellCommitResolverTest / ActionHandlerMarkedCellCommitTests (D2)
            // hanjiCell_bracketsOff_… / hanjiCell_bracketsOn_… / …_missingRoman_…
            (Hanji, true, false, Combined, R, H, hanji("台語")),
            (Hanji, true, true, Combined, R, H, roman("台語 (tâi-gí)")),
            (Hanji, true, true, Combined, "", H, hanji("台語")),
            // test_INVARIANT_roman_cell_commits_bare_roman_even_with_brackets_on
            (Roman, true, false, Combined, R, H, roman(R)),
            (Roman, true, true, Combined, R, H, roman(R)),
            // defectiveMarkers_failOpenToTheUnmarkedPath: the lead instead
            (Hanji, true, true, Combined, R, None, roman(R)),
            (Roman, true, false, Combined, "", H, hanji("台語")),
            // trace: ported from the removed desktop / macOS resolvers (R5 PR-c)
            // the_alternate_verdict_inverts_the_mode /
            // alternate_text_follows_the_display_mode_not_the_cell
            (Other, true, false, SideBySide, R, H, roman(R)),
            (Other, false, false, SideBySide, R, H, hanji("台語")),
            (Other, false, false, Combined, R, H, roman(R)),
            (Other, true, false, SideBySide, R, None, None),
            (Other, true, false, SideBySide, R, Some(""), None),
            (Other, true, true, RomanOnly, R, H, None),
        ];
        for (script, swapped, brackets, display, roman, hanji, expected) in rows {
            let config = config(swapped, brackets, display);
            let actual = resolve_commit_text(script, roman, hanji, &config)
                .map(|resolved| (resolved.text, resolved.wrote_romanization));
            assert_eq!(
                actual, expected,
                "{script:?} swapped={swapped} brackets={brackets} {display:?} {roman:?} {hanji:?}"
            );
        }
    }

    #[test]
    fn tps_truth_table() {
        use CandidateDisplayMode::{Combined, RomanOnly, SideBySide};
        use CommitScript::{Hanji, Lead, Roman};
        const R: &str = "tâi-gí";
        const H: Option<&str> = Some("台語");
        // trace: to_tone_number("tâi-gí") = "tai5-gi2"; tps.rs t → ㄉ, ai → ㄞ,
        // 5 → ˊ, g → ㆣ, i → ㄧ, 2 → ˋ; syllables joined by one space.
        const T: &str = "ㄉㄞˊ ㆣㄧˋ";
        let roman = |text: &str| Some((text.to_owned(), true));
        let unspaced = |text: &str| Some((text.to_owned(), false));
        // (script, brackets, display, roman, hanji) → (text, wrote_romanization)
        #[rustfmt::skip]
        let rows = [
            // trace: the removed iOS formatOutputText / Android resolveUnmarkedCommit (D2) with
            // effectiveSwapped = isTPSLayout || … and bracketRoman =
            // tlDisplayToTps(roman): the Hanji leads, the bracket half is TPS.
            (Lead, false, SideBySide, R, H, unspaced("台語")),
            (Lead, true, SideBySide, R, H, roman("台語 (ㄉㄞˊ ㆣㄧˋ)")),
            // TPS ignores Candidate Display (the platforms' effectiveSwapped;
            // Annotate in Brackets arrives already masked under Romanization Only).
            (Lead, false, RomanOnly, R, H, unspaced("台語")),
            (Lead, true, Combined, R, H, roman("台語 (ㄉㄞˊ ㆣㄧˋ)")),
            // R5 P2 (USER 2026-09-30): a Hanji-less pick writes what its cell
            // shows, tlDisplayToTps(roman), unspaced. Was: Android `tâi-gí`
            // spaced, iOS tlNumericToTPS of the display romanization.
            (Lead, false, SideBySide, R, None, unspaced(T)),
            (Lead, true, SideBySide, R, None, unspaced(T)),
            (Lead, true, Combined, R, Some(""), unspaced(T)),
            // A §42 split cell never shows under TPS; were one sent, it commits
            // as on TL/POJ (the removed iOS markedCellCommit /
            // Android resolveMarkedCellCommit read no layout).
            (Hanji, true, Combined, R, H, roman("台語 (tâi-gí)")),
            (Roman, true, Combined, R, H, roman(R)),
        ];
        for (script, brackets, display, roman, hanji, expected) in rows {
            let config = AppConfig {
                input_mode: "tps".into(),
                output_both_scripts: brackets,
                candidate_display_mode: display as i32,
                ..AppConfig::default()
            };
            let actual = resolve_commit_text(script, roman, hanji, &config)
                .map(|resolved| (resolved.text, resolved.wrote_romanization));
            assert_eq!(
                actual, expected,
                "{script:?} brackets={brackets} {display:?} {roman:?} {hanji:?}"
            );
        }
    }

    #[test]
    fn tps_bopomofo_follows_the_or_setting() {
        // trace: to_tone_number("kór") = "kor2"; tps.rs `or` → ㄜ only with
        // tps_or_maps_to_er, ㄛ otherwise; k → ㄍ, 2 → ˋ.
        for (or_maps_to_er, expected) in [(true, "ㄍㄜˋ"), (false, "ㄍㄛˋ")] {
            let config = AppConfig {
                input_mode: "tps".into(),
                tps_or_maps_to_er: or_maps_to_er,
                ..AppConfig::default()
            };
            let resolved = resolve_commit_text(CommitScript::Lead, "kór", None, &config);
            assert_eq!(
                resolved.map(|resolved| resolved.text).as_deref(),
                Some(expected),
                "or_maps_to_er={or_maps_to_er}"
            );
        }
    }

    #[test]
    fn resolution_earns_auto_space_only_on_a_romanized_final_commit() {
        // trace: ActionHandler.shouldAppendAutoSpace /
        // AutoSpaceVerdictTest.shouldAppendAutoSpace_requiresSettingRomanizationAndNoHyphenTail,
        // minus the live setting; the caller's final-commit gate folded in.
        let pick = |text: &str, wrote_romanization| {
            Some(ResolvedCommit {
                text: text.to_owned(),
                wrote_romanization,
            })
        };
        let finalized = commit_resolution(CommitOutcome::Finalized, pick("tâi-gí", true));
        assert_eq!(finalized.outcome, CommitOutcome::Finalized as i32);
        assert_eq!(finalized.document_text, "tâi-gí");
        assert!(finalized.wrote_romanization && finalized.earns_auto_space);
        assert!(!commit_resolution(CommitOutcome::Nailed, pick("tâi-gí", true)).earns_auto_space);
        assert!(!commit_resolution(CommitOutcome::Finalized, pick("台語", false)).earns_auto_space);
        assert!(!commit_resolution(CommitOutcome::Finalized, pick("tâi-", true)).earns_auto_space);
        let ignored = commit_resolution(CommitOutcome::Ignored, pick("tâi-gí", true));
        assert_eq!(
            ignored,
            CommitResolution {
                outcome: CommitOutcome::Ignored as i32,
                ..CommitResolution::default()
            }
        );
    }
}
