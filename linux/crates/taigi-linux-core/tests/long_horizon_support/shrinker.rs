#![allow(dead_code)]
//! Two-stage delta-debugging shrinker (prior-sentence reduction + intra-sentence
//! keystroke minimization) and `e2e/scenarios/*.json` reproducer emitter.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use taigi_desktop_core::keys::ToneInputScheme;
use taigi_desktop_core::settings::{keys, InputMode, SettingsDocument};

use super::host::{SimStep, SpecialKey};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShrunkReproducer {
    pub original_prior_sentences: usize,
    pub shrunk_prior_sentences: usize,
    pub original_steps: usize,
    pub shrunk_steps: usize,
    pub priming_batches: Vec<Vec<SimStep>>,
    pub minimal_steps: Vec<SimStep>,
    pub e2e_scenario: Value,
}

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
    id: &str,
    source_note: &str,
    settings: &SettingsDocument,
    prior_batches: &[Vec<SimStep>],
    failing_steps: &[SimStep],
    mut reproduces: impl FnMut(&[Vec<SimStep>], &[SimStep]) -> bool,
) -> ShrunkReproducer {
    let stage_budget = std::time::Duration::from_secs(15);

    // Stage 1: Shrink prior sentences (fast-paths `[]` and tail 1/2/4 checked first).
    let shrunk_prior = ddmin_bounded(prior_batches, stage_budget, &mut |candidate_prior| {
        reproduces(candidate_prior, failing_steps)
    });

    // Stage 2: Shrink steps inside the failing sentence.
    let minimal_steps = ddmin_bounded(failing_steps, stage_budget, &mut |candidate_steps| {
        !candidate_steps.is_empty() && reproduces(&shrunk_prior, candidate_steps)
    });

    let e2e_scenario = steps_to_e2e_scenario_json(id, source_note, settings, &minimal_steps);

    ShrunkReproducer {
        original_prior_sentences: prior_batches.len(),
        shrunk_prior_sentences: shrunk_prior.len(),
        original_steps: failing_steps.len(),
        shrunk_steps: minimal_steps.len(),
        priming_batches: shrunk_prior,
        minimal_steps,
        e2e_scenario,
    }
}

/// Translates `SimStep`s into the repo's `e2e/scenarios/*.json` schema where representable.
pub fn steps_to_e2e_scenario_json(
    id: &str,
    source_note: &str,
    settings: &SettingsDocument,
    steps: &[SimStep],
) -> Value {
    let romanization = match settings.choice(&keys::INPUT_MODE) {
        InputMode::Poj => "poj",
        InputMode::Tps => "tps",
        InputMode::Tl => "tl",
    };
    let output = if settings.bool(&keys::IS_HANJI_FIRST) {
        "hanji"
    } else {
        "roman"
    };
    let tone_input = match settings.choice(&keys::TONE_INPUT_SCHEME) {
        ToneInputScheme::Standard => "standard",
        ToneInputScheme::Telex => "telex",
    };

    let mut json_steps: Vec<Value> = Vec::new();
    let mut pending_text = String::new();
    let flush_text = |pending: &mut String, out: &mut Vec<Value>| {
        if !pending.is_empty() {
            out.push(json!({ "type": "text", "value": std::mem::take(pending) }));
        }
    };

    for step in steps {
        match step {
            SimStep::KeyChar { ch, shift: _ } => {
                pending_text.push(*ch);
            }
            SimStep::Special(spec) => {
                flush_text(&mut pending_text, &mut json_steps);
                let key_name = match spec {
                    SpecialKey::Enter | SpecialKey::ShiftEnter => Some("enter"),
                    SpecialKey::Space => Some("space"),
                    SpecialKey::Backspace | SpecialKey::DeleteForward => Some("backspace"),
                    SpecialKey::Escape => Some("escape"),
                    SpecialKey::Tab
                    | SpecialKey::ShiftTab
                    | SpecialKey::PagePrevBracket
                    | SpecialKey::PageNextBracket => None,
                };
                if let Some(k) = key_name {
                    json_steps.push(json!({ "type": "key", "value": k }));
                }
            }
            SimStep::PickSlot { slot, .. } if *slot <= 8 => {
                flush_text(&mut pending_text, &mut json_steps);
                let key_val = ((*slot + 1) % 10).to_string();
                json_steps.push(json!({ "type": "key", "value": key_val }));
            }
            _ => {
                flush_text(&mut pending_text, &mut json_steps);
            }
        }
    }
    flush_text(&mut pending_text, &mut json_steps);

    json!({
        "id": id,
        "source": source_note,
        "settings": {
            "romanization": romanization,
            "continuous_input": true,
            "output": output,
            "show_typed_text_first": settings.bool(&keys::IS_LITERAL_ROMAN_CANDIDATE_ENABLED),
            "tone_input": tone_input,
            "c_affricate": settings.bool(&keys::IS_C_AFFRICATE_ENABLED),
            "f_hyphen": settings.bool(&keys::IS_F_HYPHEN_ENABLED),
            "auto_space": settings.bool(&keys::IS_AUTO_SPACE_ENABLED)
        },
        "steps": json_steps,
        "expect": {
            "committed": ""
        }
    })
}
