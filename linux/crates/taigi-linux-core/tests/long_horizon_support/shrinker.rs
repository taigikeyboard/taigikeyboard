//! Bounded delta debugging with lossless native replay artifacts. The portable
//! e2e schema cannot express host invariants, paired observations, every live
//! setting, or all simulator actions, so it must not receive a lossy scenario.

use serde::{Deserialize, Serialize};
use taigi_desktop_core::settings::SettingsDocument;

use super::host::SimStep;
use super::oracles::MetamorphicCase;
use super::UserDataSnapshot;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReproSetup {
    pub settings: SettingsDocument,
    pub can_delete_surrounding: bool,
    pub user_data_snapshot: Option<UserDataSnapshot>,
}

impl ReproSetup {
    pub fn new(settings: &SettingsDocument, can_delete_surrounding: bool) -> Self {
        Self {
            settings: settings.clone(),
            can_delete_surrounding,
            user_data_snapshot: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShrunkReproducer {
    pub setup: ReproSetup,
    pub original_prior_sentences: usize,
    pub shrunk_prior_sentences: usize,
    pub original_steps: usize,
    pub shrunk_steps: usize,
    pub priming_batches: Vec<Vec<SimStep>>,
    pub minimal_steps: Vec<SimStep>,
    pub metamorphic_case: Option<MetamorphicCase>,
    pub replay_verified: bool,
    pub e2e_scenario_unavailable: String,
}

const E2E_UNAVAILABLE: &str = "The portable e2e schema cannot express this failure predicate and full simulator setup/actions. Use the lossless .repro.json with long_horizon_replay.";

/// Generic `ddmin` over a slice, preserving element order, bounded by `budget`.
fn ddmin_bounded<T: Clone>(
    items: &[T],
    budget: std::time::Duration,
    predicate: &mut impl FnMut(&[T]) -> bool,
) -> Vec<T> {
    if items.is_empty() {
        return Vec::new();
    }
    let deadline = std::time::Instant::now() + budget;
    if predicate(&[]) {
        return Vec::new();
    }
    // Fast-path: check recent tail of 1 or 2 items (common for stateful session bugs).
    for tail_len in [1usize, 2, 4] {
        if items.len() > tail_len && std::time::Instant::now() < deadline {
            let tail = &items[items.len() - tail_len..];
            if predicate(tail) {
                return ddmin_bounded(tail, budget, predicate);
            }
        }
    }

    let mut current = items.to_vec();
    let mut n = 2usize;

    while current.len() >= 2 && std::time::Instant::now() < deadline {
        let len = current.len();
        let chunk_size = len.div_ceil(n);
        let mut reduced = false;

        let chunks = n;

        // First try keeping just one chunk (fast jump when failure is localized).
        if chunks > 2 {
            for i in 0..chunks {
                if std::time::Instant::now() >= deadline {
                    break;
                }
                let start = i * chunk_size;
                if start >= len {
                    break;
                }
                let end = ((i + 1) * chunk_size).min(len);
                let subset = current[start..end].to_vec();
                if subset.len() < len && predicate(&subset) {
                    current = subset;
                    n = 2;
                    reduced = true;
                    break;
                }
            }
        }
        if reduced {
            continue;
        }

        // Try removing each chunk (`complement`).
        for i in 0..chunks {
            if std::time::Instant::now() >= deadline {
                break;
            }
            let start = i * chunk_size;
            if start >= len {
                break;
            }
            let end = ((i + 1) * chunk_size).min(len);
            let mut complement = Vec::with_capacity(len - (end - start));
            complement.extend_from_slice(&current[..start]);
            complement.extend_from_slice(&current[end..]);
            if !complement.is_empty() && predicate(&complement) {
                current = complement;
                n = (chunks - 1).max(2);
                reduced = true;
                break;
            }
        }

        if !reduced {
            if n >= len {
                break;
            }
            n = (n * 2).min(len);
        }
    }
    current
}

pub fn shrink_episode_failure(
    setup: &ReproSetup,
    prior_batches: &[Vec<SimStep>],
    failing_steps: &[SimStep],
    mut reproduces: impl FnMut(&[Vec<SimStep>], &[SimStep]) -> bool,
) -> ShrunkReproducer {
    let budget = std::time::Duration::from_secs(15);
    // Do not claim an unreproducible original was minimized successfully.
    let (priming_batches, minimal_steps) = if reproduces(prior_batches, failing_steps) {
        let prior = ddmin_bounded(prior_batches, budget, &mut |candidate| {
            reproduces(candidate, failing_steps)
        });
        let steps = ddmin_bounded(failing_steps, budget, &mut |candidate| {
            !candidate.is_empty() && reproduces(&prior, candidate)
        });
        (prior, steps)
    } else {
        (prior_batches.to_vec(), failing_steps.to_vec())
    };
    let replay_verified = reproduces(&priming_batches, &minimal_steps);
    ShrunkReproducer {
        setup: setup.clone(),
        original_prior_sentences: prior_batches.len(),
        shrunk_prior_sentences: priming_batches.len(),
        original_steps: failing_steps.len(),
        shrunk_steps: minimal_steps.len(),
        priming_batches,
        minimal_steps,
        metamorphic_case: None,
        replay_verified,
        e2e_scenario_unavailable: E2E_UNAVAILABLE.to_owned(),
    }
}

pub fn shrink_metamorphic_failure(
    setup: &ReproSetup,
    case: &MetamorphicCase,
    mut reproduces: impl FnMut(&MetamorphicCase) -> bool,
) -> ShrunkReproducer {
    let input = if reproduces(case) {
        ddmin_bounded(
            &case.input,
            std::time::Duration::from_secs(15),
            &mut |input| {
                let candidate = MetamorphicCase {
                    oracle: case.oracle,
                    input: input.to_vec(),
                };
                candidate.is_valid() && reproduces(&candidate)
            },
        )
    } else {
        case.input.clone()
    };
    let minimized = MetamorphicCase {
        oracle: case.oracle,
        input,
    };
    let replay_verified = reproduces(&minimized);
    let minimal_steps = minimized.dirty_steps();
    ShrunkReproducer {
        setup: setup.clone(),
        original_prior_sentences: 0,
        shrunk_prior_sentences: 0,
        original_steps: case.dirty_steps().len(),
        shrunk_steps: minimal_steps.len(),
        priming_batches: Vec::new(),
        minimal_steps,
        metamorphic_case: Some(minimized),
        replay_verified,
        e2e_scenario_unavailable: E2E_UNAVAILABLE.to_owned(),
    }
}

#[cfg(test)]
mod regression_tests {
    use super::super::host::{CaretStep, SpecialKey};
    use super::super::oracles::MetamorphicOracle;
    use super::*;

    #[test]
    fn metamorphic_shrinking_keeps_the_equivalence_transformation() {
        let case = MetamorphicCase {
            oracle: MetamorphicOracle::MidInsertDelete,
            input: "tai5-uan5"
                .chars()
                .map(|ch| SimStep::KeyChar { ch, shift: false })
                .collect(),
        };
        let shrunk = shrink_metamorphic_failure(
            &ReproSetup::new(&SettingsDocument::default(), true),
            &case,
            |candidate| {
                // Model an oracle failure tied to an input token, while every probe
                // still has the same reversible edit and corresponding clean input.
                let dirty = candidate.dirty_steps();
                assert!(dirty.contains(&SimStep::MoveCaret(CaretStep::Left)));
                assert!(dirty.contains(&SimStep::Special(SpecialKey::Backspace)));
                candidate.input.contains(&SimStep::KeyChar {
                    ch: 'u',
                    shift: false,
                })
            },
        );
        let minimized = shrunk.metamorphic_case.expect("paired replay");
        assert!(minimized.is_valid());
        assert_eq!(minimized.oracle, case.oracle);
        assert_eq!(minimized.input.len(), 4);
        assert_eq!(shrunk.minimal_steps, minimized.dirty_steps());
        assert!(shrunk.replay_verified);
    }

    #[test]
    fn exported_reproducer_retains_priming_and_exact_actions() {
        let priming = vec![vec![SimStep::KeyChar {
            ch: 'a',
            shift: false,
        }]];
        let actions = vec![
            SimStep::MoveCaret(CaretStep::Home),
            SimStep::Special(SpecialKey::ShiftEnter),
            SimStep::Special(SpecialKey::DeleteForward),
            SimStep::FlipLiveSettings,
        ];
        let setup = ReproSetup::new(&SettingsDocument::default(), false);
        let shrunk = shrink_episode_failure(&setup, &priming, &actions, |p, s| {
            !p.is_empty() && s == actions
        });
        let json = serde_json::to_vec(&shrunk).expect("serialize replay");
        let restored: ShrunkReproducer = serde_json::from_slice(&json).expect("read replay");
        assert_eq!(restored.setup.settings, setup.settings);
        assert!(!restored.setup.can_delete_surrounding);
        assert!(!restored.e2e_scenario_unavailable.is_empty());
        assert_eq!(restored.priming_batches, priming);
        assert_eq!(restored.minimal_steps, actions);
        assert!(restored.replay_verified);
    }
}
