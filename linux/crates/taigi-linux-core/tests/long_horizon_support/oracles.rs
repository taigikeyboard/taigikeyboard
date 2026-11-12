//! Metamorphic equivalence oracles, long-horizon learning drift probes,
//! learning monotonicity checks, and corpus reachability / behavioral surprise
//! detectors.

use std::collections::BTreeSet;
use std::path::Path;

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use taigi_desktop_core::engine::{
    dictionary_filters, search_by_hanji, ALL_SOURCES_ENABLED_SEARCH_BITMASK,
};
use taigi_desktop_core::keys::ToneInputScheme;
use taigi_desktop_core::settings::{
    keys, CandidateDisplayMode, DictionarySourceToggles, InputMode, SettingsDocument,
};

use super::corpus::{CorpusSentence, EncodedWord, SentenceSegment};
use super::host::{CaretStep, HostSimulator, NavDir, SimStep, SpecialKey};
use super::{reset_transient_engine_state, sync_user_data_workers};

#[derive(Clone, Debug, Serialize)]
pub struct OracleFailure {
    pub oracle: &'static str,
    pub sentence_index: usize,
    pub sentence_tailo: String,
    pub detail: String,
    pub clean_steps: Vec<SimStep>,
    pub dirty_steps: Vec<SimStep>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SurpriseFinding {
    pub kind: &'static str,
    pub key: String,
    pub sentence_index: usize,
    pub sentence_hanji: String,
    pub sentence_tailo: String,
    pub detail: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReachabilityStats {
    pub words_checked: u64,
    pub digit_tl_rank1: u64,
    pub digit_tl_top9: u64,
    pub digit_tl_any: u64,
    pub digit_tl_missing: u64,
    pub toneless_rank1: u64,
    pub toneless_top9: u64,
    pub toneless_any: u64,
    pub toneless_missing: u64,
    pub metamorphic_checks: u64,
    pub monotonicity_checks: u64,
}

const GOLDEN_PROBES: &[(&str, &str)] = &[
    ("tai5-uan5", "台灣/臺灣"),
    ("tai5-gi2", "台語/臺語"),
    ("tsiah8-pa2-bue7", "食飽未"),
    ("kin-a2-jit8", "今仔日"),
    ("to-sia7", "多謝"),
    ("bo5-bun7-te5", "無問題"),
    ("a-bu2", "阿母"),
    ("ho2-se3", "好勢"),
    ("an2-ne", "按呢"),
    ("sann2-mih8", "啥物"),
    ("ka-ki7", "家己"),
    ("sing-li2", "生理"),
    ("huann-hi2", "歡喜"),
    ("penn7-inn7", "病院"),
    ("hak8-hau7", "學校"),
    ("ping5-iu2", "朋友"),
    ("thinn-khi3", "天氣"),
    ("hue2-tshia", "火車"),
    ("ka-phi", "咖啡"),
    ("am3-tng3", "暗頓"),
];

#[derive(Clone, Debug)]
pub struct ProbeSnapshot {
    pub rank1_per_probe: Vec<(String, Option<String>)>,
}

fn standard_tl_settings() -> SettingsDocument {
    let mut doc = SettingsDocument::default();
    doc.set_choice(&keys::INPUT_MODE, InputMode::Tl);
    doc.set_choice(&keys::TONE_INPUT_SCHEME, ToneInputScheme::Standard);
    doc.set_choice(
        &keys::CANDIDATE_DISPLAY_MODE,
        CandidateDisplayMode::SideBySide,
    );
    doc.set_bool(&keys::IS_HANJI_FIRST, true);
    doc.set_bool(&keys::IS_LITERAL_ROMAN_CANDIDATE_ENABLED, false);
    doc.set_bool(&keys::IS_C_AFFRICATE_ENABLED, false);
    doc.set_bool(&keys::IS_F_HYPHEN_ENABLED, false);
    doc
}

pub fn capture_probe_snapshot(user_data_dir: &Path, dictionaries_dir: &Path) -> ProbeSnapshot {
    let _ = sync_user_data_workers();
    reset_transient_engine_state();
    let settings = standard_tl_settings();
    let mut sim = HostSimulator::new(user_data_dir, dictionaries_dir, &settings, true);
    let mut out = Vec::with_capacity(GOLDEN_PROBES.len());
    for &(reading, _) in GOLDEN_PROBES {
        for ch in reading.chars() {
            let _ = sim.step(&SimStep::KeyChar { ch, shift: false });
        }
        let top1 = sim
            .buffer
            .lookup
            .as_ref()
            .and_then(|t| t.cells.first().map(|c| c.text.clone()));
        let _ = sim.step(&SimStep::Special(SpecialKey::Escape));
        out.push((reading.to_owned(), top1));
    }
    drop(sim);
    reset_transient_engine_state();
    ProbeSnapshot {
        rank1_per_probe: out,
    }
}

pub fn diff_probe_snapshots(
    baseline: &ProbeSnapshot,
    after: &ProbeSnapshot,
    episode_id: usize,
    seen_keys: &mut BTreeSet<String>,
    surprises: &mut Vec<SurpriseFinding>,
) {
    for ((reading, base_top), (_, new_top)) in baseline
        .rank1_per_probe
        .iter()
        .zip(after.rank1_per_probe.iter())
    {
        if base_top != new_top {
            let dedup_key = format!("drift:{reading}:{base_top:?}->{new_top:?}");
            if seen_keys.insert(dedup_key.clone()) {
                surprises.push(SurpriseFinding {
                    kind: "LEARNING_RANK1_DRIFT",
                    key: dedup_key,
                    sentence_index: episode_id,
                    sentence_hanji: String::new(),
                    sentence_tailo: reading.clone(),
                    detail: format!(
                        "probe {reading:?} rank-1 shifted from {base_top:?} to {new_top:?} after long-horizon episode #{episode_id}"
                    ),
                });
            }
        }
    }
}

/// Runs four exact metamorphic equivalence checks on a clause extracted from `sentence`.
pub fn run_metamorphic_suite(
    user_data_dir: &Path,
    dictionaries_dir: &Path,
    sentence: &CorpusSentence,
    stats: &mut ReachabilityStats,
) -> Result<(), Box<OracleFailure>> {
    let words: Vec<&EncodedWord> = sentence
        .segments
        .iter()
        .filter_map(|s| match s {
            SentenceSegment::Word(w) => Some(w),
            SentenceSegment::Punct(_) => None,
        })
        .take(3)
        .collect();
    if words.is_empty() {
        return Ok(());
    }

    let raw_clause: String = words
        .iter()
        .map(|w| w.digit_tl.as_str())
        .collect::<Vec<_>>()
        .join("-");
    if raw_clause.len() < 4 {
        return Ok(());
    }

    let _ = sync_user_data_workers();
    let settings = standard_tl_settings();

    let clean_steps: Vec<SimStep> = raw_clause
        .chars()
        .map(|ch| SimStep::KeyChar { ch, shift: false })
        .collect();
    let split = (clean_steps.len() / 2).max(2);

    // Reference clean observation.
    let (clean_preedit, clean_cells) =
        observe_composing_state(user_data_dir, dictionaries_dir, &settings, &clean_steps)?;

    // 1. Caret walk roundtrip: Ctrl+Left x2 -> Ctrl+Right x2 -> Home -> End.
    let mut caret_steps = clean_steps[..split].to_vec();
    caret_steps.push(SimStep::MoveCaret(CaretStep::Left));
    caret_steps.push(SimStep::MoveCaret(CaretStep::Left));
    caret_steps.push(SimStep::MoveCaret(CaretStep::Right));
    caret_steps.push(SimStep::MoveCaret(CaretStep::Right));
    caret_steps.push(SimStep::MoveCaret(CaretStep::Home));
    caret_steps.push(SimStep::MoveCaret(CaretStep::End));
    caret_steps.extend_from_slice(&clean_steps[split..]);
    stats.metamorphic_checks += 1;
    let (caret_preedit, caret_cells) =
        observe_composing_state(user_data_dir, dictionaries_dir, &settings, &caret_steps)?;
    if (clean_preedit.as_ref(), &clean_cells) != (caret_preedit.as_ref(), &caret_cells) {
        return Err(Box::new(OracleFailure {
            oracle: "METAMORPHIC_CARET_ROUNDTRIP",
            sentence_index: sentence.index,
            sentence_tailo: sentence.tailo.clone(),
            detail: format!(
                "caret walk diverged: clean=({clean_preedit:?}, top={:?}) vs dirty=({caret_preedit:?}, top={:?})",
                &clean_cells[..clean_cells.len().min(5)],
                &caret_cells[..caret_cells.len().min(5)],
            ),
            clean_steps,
            dirty_steps: caret_steps,
        }));
    }

    // 2. Tail typo `'b'` + immediate Backspace confluence.
    let mut typo_steps = clean_steps[..split].to_vec();
    typo_steps.push(SimStep::KeyChar {
        ch: 'b',
        shift: false,
    });
    typo_steps.push(SimStep::Special(SpecialKey::Backspace));
    typo_steps.extend_from_slice(&clean_steps[split..]);
    stats.metamorphic_checks += 1;
    let (typo_preedit, typo_cells) =
        observe_composing_state(user_data_dir, dictionaries_dir, &settings, &typo_steps)?;
    if (clean_preedit.as_ref(), &clean_cells) != (typo_preedit.as_ref(), &typo_cells) {
        return Err(Box::new(OracleFailure {
            oracle: "METAMORPHIC_TYPO_BACKSPACE_CONFLUENCE",
            sentence_index: sentence.index,
            sentence_tailo: sentence.tailo.clone(),
            detail: format!(
                "tail typo+Backspace diverged: clean=({clean_preedit:?}, top={:?}) vs dirty=({typo_preedit:?}, top={:?})",
                &clean_cells[..clean_cells.len().min(5)],
                &typo_cells[..typo_cells.len().min(5)],
            ),
            clean_steps,
            dirty_steps: typo_steps,
        }));
    }

    // 3. Mid-word caret step + `'b'` + Backspace + return to End.
    let mut mid_steps = clean_steps[..split].to_vec();
    mid_steps.push(SimStep::MoveCaret(CaretStep::Left));
    mid_steps.push(SimStep::KeyChar {
        ch: 'b',
        shift: false,
    });
    mid_steps.push(SimStep::Special(SpecialKey::Backspace));
    mid_steps.push(SimStep::MoveCaret(CaretStep::End));
    mid_steps.extend_from_slice(&clean_steps[split..]);
    stats.metamorphic_checks += 1;
    let (mid_preedit, mid_cells) =
        observe_composing_state(user_data_dir, dictionaries_dir, &settings, &mid_steps)?;
    if (clean_preedit.as_ref(), &clean_cells) != (mid_preedit.as_ref(), &mid_cells) {
        return Err(Box::new(OracleFailure {
            oracle: "METAMORPHIC_MID_INSERT_DELETE_CONFLUENCE",
            sentence_index: sentence.index,
            sentence_tailo: sentence.tailo.clone(),
            detail: format!(
                "mid-word insert+Backspace diverged: clean=({clean_preedit:?}, top={:?}) vs dirty=({mid_preedit:?}, top={:?})",
                &clean_cells[..clean_cells.len().min(5)],
                &mid_cells[..mid_cells.len().min(5)],
            ),
            clean_steps,
            dirty_steps: mid_steps,
        }));
    }

    // 4. Aborted composition (`Escape`) leaves zero residue for the following sentence.
    let mut abort_steps = vec![
        SimStep::KeyChar {
            ch: 't',
            shift: false,
        },
        SimStep::KeyChar {
            ch: 's',
            shift: false,
        },
        SimStep::KeyChar {
            ch: 'i',
            shift: false,
        },
        SimStep::KeyChar {
            ch: 'a',
            shift: false,
        },
        SimStep::KeyChar {
            ch: 'h',
            shift: false,
        },
        SimStep::KeyChar {
            ch: '8',
            shift: false,
        },
        SimStep::Navigate(NavDir::Down),
        SimStep::Special(SpecialKey::Escape),
    ];
    abort_steps.extend_from_slice(&clean_steps);
    stats.metamorphic_checks += 1;
    let (abort_preedit, abort_cells) =
        observe_composing_state(user_data_dir, dictionaries_dir, &settings, &abort_steps)?;
    if (clean_preedit.as_ref(), &clean_cells) != (abort_preedit.as_ref(), &abort_cells) {
        return Err(Box::new(OracleFailure {
            oracle: "METAMORPHIC_ESCAPE_CLEAN_SLATE",
            sentence_index: sentence.index,
            sentence_tailo: sentence.tailo.clone(),
            detail: format!(
                "Escape residue diverged: clean=({clean_preedit:?}, top={:?}) vs post-Escape=({abort_preedit:?}, top={:?})",
                &clean_cells[..clean_cells.len().min(5)],
                &abort_cells[..abort_cells.len().min(5)],
            ),
            clean_steps,
            dirty_steps: abort_steps,
        }));
    }

    Ok(())
}

type ObservedComposingState = (Option<(String, u32)>, Vec<String>);

fn observe_composing_state(
    user_data_dir: &Path,
    dictionaries_dir: &Path,
    settings: &SettingsDocument,
    steps: &[SimStep],
) -> Result<ObservedComposingState, Box<OracleFailure>> {
    reset_transient_engine_state();
    let mut sim = HostSimulator::new(user_data_dir, dictionaries_dir, settings, true);
    for s in steps {
        sim.step(s).map_err(|v| {
            Box::new(OracleFailure {
                oracle: v.code,
                sentence_index: 0,
                sentence_tailo: String::new(),
                detail: v.message,
                clean_steps: Vec::new(),
                dirty_steps: steps.to_vec(),
            })
        })?;
    }
    let cells = sim
        .buffer
        .lookup
        .as_ref()
        .map(|t| {
            t.cells
                .iter()
                .map(|c| match &c.annotation {
                    Some(a) => format!("{}:{a}", c.text),
                    None => c.text.clone(),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok((sim.buffer.preedit.clone(), cells))
}

/// Checks explicit pick promotion monotonicity: picking a candidate at slot `1`
/// four times in a row should never worsen its rank (and normally promotes it).
pub fn check_learning_monotonicity(
    user_data_dir: &Path,
    dictionaries_dir: &Path,
    reading: &str,
    sentence_idx: usize,
    stats: &mut ReachabilityStats,
    seen_keys: &mut BTreeSet<String>,
    surprises: &mut Vec<SurpriseFinding>,
) {
    let settings = standard_tl_settings();
    let mut sim = HostSimulator::new(user_data_dir, dictionaries_dir, &settings, true);

    for ch in reading.chars() {
        if sim.step(&SimStep::KeyChar { ch, shift: false }).is_err() {
            return;
        }
    }
    let Some(initial_cells) = sim.buffer.lookup.as_ref().map(|t| t.cells.clone()) else {
        return;
    };
    if initial_cells.len() < 2 {
        let _ = sim.step(&SimStep::Special(SpecialKey::Escape));
        return;
    }
    let target_text = initial_cells[1].text.clone();
    let target_ann = initial_cells[1].annotation.clone();
    let _ = sim.step(&SimStep::Special(SpecialKey::Escape));

    // Pick that exact `(text, annotation)` 4 times.
    for _ in 0..4 {
        for ch in reading.chars() {
            let _ = sim.step(&SimStep::KeyChar { ch, shift: false });
        }
        let pos = sim.buffer.lookup.as_ref().and_then(|t| {
            t.cells
                .iter()
                .position(|c| c.text == target_text && c.annotation == target_ann)
        });
        let Some(idx) = pos else {
            let _ = sim.step(&SimStep::Special(SpecialKey::Escape));
            return;
        };
        if idx < 9 {
            let _ = sim.step(&SimStep::PickSlot {
                slot: idx,
                alternate_script: false,
            });
        } else {
            let _ = sim.step(&SimStep::Special(SpecialKey::Escape));
            return;
        }
        if sim.buffer.preedit.is_some() {
            // Partial prefix pick rather than full word; cancel and skip.
            let _ = sim.step(&SimStep::Special(SpecialKey::Escape));
            return;
        }
    }

    let _ = sync_user_data_workers();
    reset_transient_engine_state();
    stats.monotonicity_checks += 1;

    for ch in reading.chars() {
        let _ = sim.step(&SimStep::KeyChar { ch, shift: false });
    }
    let after_pos = sim.buffer.lookup.as_ref().and_then(|t| {
        t.cells
            .iter()
            .position(|c| c.text == target_text && c.annotation == target_ann)
    });
    let _ = sim.step(&SimStep::Special(SpecialKey::Escape));

    match after_pos {
        Some(new_idx) if new_idx <= 1 => {
            if new_idx == 1 {
                let key = format!("mono-unpromoted:{reading}:{target_text}");
                if seen_keys.insert(key.clone()) {
                    surprises.push(SurpriseFinding {
                        kind: "LEARNING_PICK4X_STILL_RANK2",
                        key,
                        sentence_index: sentence_idx,
                        sentence_hanji: target_text,
                        sentence_tailo: reading.to_owned(),
                        detail: format!(
                            "picking slot-1 candidate 4x for {reading:?} left it at index 1 instead of promoting to rank 0"
                        ),
                    });
                }
            }
        }
        other => {
            let key = format!("mono-demoted:{reading}:{target_text}:{other:?}");
            if seen_keys.insert(key.clone()) {
                surprises.push(SurpriseFinding {
                    kind: "LEARNING_NON_MONOTONIC",
                    key,
                    sentence_index: sentence_idx,
                    sentence_hanji: target_text,
                    sentence_tailo: reading.to_owned(),
                    detail: format!(
                        "picking slot-1 candidate 4x for {reading:?} moved it from index 1 to {other:?}"
                    ),
                });
            }
        }
    }
}

/// Evaluates corpus word reachability under both `digit_tl` and `toneless_tl`
/// and checks committed text cleanliness for soft behavioral surprises.
pub fn evaluate_corpus_reachability(
    user_data_dir: &Path,
    dictionaries_dir: &Path,
    sentence: &CorpusSentence,
    stats: &mut ReachabilityStats,
    seen_keys: &mut BTreeSet<String>,
    surprises: &mut Vec<SurpriseFinding>,
) {
    let settings = standard_tl_settings();
    let mut sim = HostSimulator::new(user_data_dir, dictionaries_dir, &settings, true);

    // Extract pure CJK runs from `sentence.hanji` and match by syllable count against words.
    let hanji_clauses = extract_cjk_runs(&sentence.hanji);
    let words: Vec<&EncodedWord> = sentence
        .segments
        .iter()
        .filter_map(|s| match s {
            SentenceSegment::Word(w) => Some(w),
            SentenceSegment::Punct(_) => None,
        })
        .collect();

    // Check individual words whose Hanji substring is present in `sentence.hanji` AND in lexicon.
    for word in words.iter().take(6) {
        if word.syllables.is_empty() || word.syllables.len() > 4 {
            continue;
        }
        let syl_len = word.syllables.len();
        // Find any substring of `syl_len` characters inside `hanji_clauses` that `search_by_hanji` knows!
        let Some(target_hanji) = find_matching_lexicon_hanji(&hanji_clauses, syl_len, word) else {
            continue;
        };

        stats.words_checked += 1;

        // 1. Query with `digit_tl`.
        for ch in word.digit_tl.chars() {
            let _ = sim.step(&SimStep::KeyChar { ch, shift: false });
        }
        let digit_pos = sim
            .buffer
            .lookup
            .as_ref()
            .and_then(|t| t.cells.iter().position(|c| c.text == target_hanji));
        let top_preview: Vec<String> = sim
            .buffer
            .lookup
            .as_ref()
            .map(|t| t.cells.iter().take(5).map(|c| c.text.clone()).collect())
            .unwrap_or_default();
        let _ = sim.step(&SimStep::Special(SpecialKey::Escape));

        match digit_pos {
            Some(0) => {
                stats.digit_tl_rank1 += 1;
                stats.digit_tl_top9 += 1;
                stats.digit_tl_any += 1;
            }
            Some(idx) if idx < 9 => {
                stats.digit_tl_top9 += 1;
                stats.digit_tl_any += 1;
            }
            Some(idx) => {
                stats.digit_tl_any += 1;
                if idx >= 27 {
                    let key = format!("crowded:{}:{target_hanji}", word.digit_tl);
                    if seen_keys.insert(key.clone()) {
                        surprises.push(SurpriseFinding {
                            kind: "LEXICON_WORD_BURIED_PAST_PAGE3",
                            key,
                            sentence_index: sentence.index,
                            sentence_hanji: sentence.hanji.clone(),
                            sentence_tailo: sentence.tailo.clone(),
                            detail: format!(
                                "in-lexicon word {target_hanji:?} ({:?} -> {:?}) ranked at index {idx} (top5={top_preview:?})",
                                word.display_tl, word.digit_tl
                            ),
                        });
                    }
                }
            }
            None => {
                stats.digit_tl_missing += 1;
                let key = format!("missing-digit:{}:{target_hanji}", word.digit_tl);
                if seen_keys.insert(key.clone()) {
                    surprises.push(SurpriseFinding {
                        kind: "LEXICON_WORD_UNREACHABLE_BY_DIGIT_TL",
                        key,
                        sentence_index: sentence.index,
                        sentence_hanji: sentence.hanji.clone(),
                        sentence_tailo: sentence.tailo.clone(),
                        detail: format!(
                            "in-lexicon word {target_hanji:?} ({:?} -> {:?}) missing from candidates (top5={top_preview:?}, url={})",
                            word.display_tl, word.digit_tl, sentence.dic_url
                        ),
                    });
                }
            }
        }

        // 2. Query with continuous toneless TL (no hyphens, no digits).
        let toneless: String = word
            .syllables
            .iter()
            .map(|s| s.toneless_tl.as_str())
            .collect();
        for ch in toneless.chars() {
            let _ = sim.step(&SimStep::KeyChar { ch, shift: false });
        }
        let toneless_pos = sim
            .buffer
            .lookup
            .as_ref()
            .and_then(|t| t.cells.iter().position(|c| c.text == target_hanji));
        let _ = sim.step(&SimStep::Special(SpecialKey::Escape));

        match toneless_pos {
            Some(0) => {
                stats.toneless_rank1 += 1;
                stats.toneless_top9 += 1;
                stats.toneless_any += 1;
            }
            Some(idx) if idx < 9 => {
                stats.toneless_top9 += 1;
                stats.toneless_any += 1;
            }
            Some(_) => {
                stats.toneless_any += 1;
            }
            None => {
                stats.toneless_missing += 1;
            }
        }
    }
}

pub fn check_committed_output_surprises(
    committed: &str,
    persona_name: &str,
    sentence: &CorpusSentence,
    seen_keys: &mut BTreeSet<String>,
    surprises: &mut Vec<SurpriseFinding>,
) {
    if committed.contains('\u{FFFD}') {
        let key = format!(" replacement-char:{persona_name}:{}", sentence.index);
        if seen_keys.insert(key.clone()) {
            surprises.push(SurpriseFinding {
                kind: "REPLACEMENT_CHAR_IN_OUTPUT",
                key,
                sentence_index: sentence.index,
                sentence_hanji: sentence.hanji.clone(),
                sentence_tailo: sentence.tailo.clone(),
                detail: format!("U+FFFD appeared in committed text: {committed:?}"),
            });
        }
    }
    if committed.contains("  ") {
        let key = format!("double-space:{persona_name}");
        if seen_keys.insert(key.clone()) {
            surprises.push(SurpriseFinding {
                kind: "DOUBLE_SPACE_IN_COMMITTED_TEXT",
                key,
                sentence_index: sentence.index,
                sentence_hanji: sentence.hanji.clone(),
                sentence_tailo: sentence.tailo.clone(),
                detail: format!(
                    "persona {persona_name} produced consecutive spaces in committed text: {committed:?}"
                ),
            });
        }
    }
}

fn extract_cjk_runs(text: &str) -> Vec<Vec<char>> {
    let mut runs = Vec::new();
    let mut cur = Vec::new();
    for ch in text.chars() {
        if is_cjk(ch) {
            cur.push(ch);
        } else if !cur.is_empty() {
            runs.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        runs.push(cur);
    }
    runs
}

fn is_cjk(ch: char) -> bool {
    let code = ch as u32;
    (0x3400..=0x9FFF).contains(&code) || (0x20000..=0x323AF).contains(&code)
}

fn default_enabled_bitmask() -> u32 {
    static MASK: OnceLock<u32> = OnceLock::new();
    *MASK.get_or_init(|| {
        dictionary_filters(&DictionarySourceToggles::default())
            .map(|f| f.dictionary_filter_bitmask)
            .unwrap_or(ALL_SOURCES_ENABLED_SEARCH_BITMASK)
    })
}

fn find_matching_lexicon_hanji(
    runs: &[Vec<char>],
    syl_len: usize,
    word: &EncodedWord,
) -> Option<String> {
    let bitmask = default_enabled_bitmask();
    let target_tl_lower = word.display_tl.to_lowercase();
    for run in runs {
        if run.len() < syl_len {
            continue;
        }
        for win in run.windows(syl_len) {
            let candidate: String = win.iter().collect();
            let rows = search_by_hanji(&candidate, InputMode::Tl, 16, bitmask);
            if rows.iter().any(|r| {
                r.hanji.as_deref() == Some(candidate.as_str())
                    && r.roman.to_lowercase() == target_tl_lower
            }) {
                return Some(candidate);
            }
        }
    }
    None
}
