#![allow(dead_code)]
//! Six realistic user-interaction personas that compile real `CorpusSentence`s
//! into lifelike `SimStep` streams with typos, prefix nailing, caret edits,
//! inline English, punctuation attachment, overlays, and focus bounces.

use serde::{Deserialize, Serialize};
use taigi_desktop_core::keys::ToneInputScheme;
use taigi_desktop_core::settings::{
    keys, CandidateDisplayMode, CandidateLayout, InputMode, SettingsDocument, SyllableSeparator,
};

use super::corpus::{telex_key_for_tone, CorpusSentence, EncodedWord, SentenceSegment};
use super::host::{CaretStep, NavDir, SafeGlobalShortcut, SimStep, SpecialKey};
use super::rng::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PersonaKind {
    EverydayContinuousTl,
    CarefulPrefixNailer,
    TelexPowerTypist,
    PojMixedScriptEditor,
    TpsPhoneticTypist,
    ChaosBilingualMonkey,
}

impl PersonaKind {
    pub const ALL: [Self; 6] = [
        Self::EverydayContinuousTl,
        Self::CarefulPrefixNailer,
        Self::TelexPowerTypist,
        Self::PojMixedScriptEditor,
        Self::TpsPhoneticTypist,
        Self::ChaosBilingualMonkey,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::EverydayContinuousTl => "everyday-continuous-tl",
            Self::CarefulPrefixNailer => "careful-prefix-nailer",
            Self::TelexPowerTypist => "telex-power-typist",
            Self::PojMixedScriptEditor => "poj-mixed-script-editor",
            Self::TpsPhoneticTypist => "tps-phonetic-typist",
            Self::ChaosBilingualMonkey => "chaos-bilingual-monkey",
        }
    }
}

pub fn build_episode_settings(persona: PersonaKind, rng: &mut Rng) -> SettingsDocument {
    let mut doc = SettingsDocument::default();
    let layout = if rng.chance(1, 2) {
        CandidateLayout::Vertical
    } else {
        CandidateLayout::Horizontal
    };
    doc.set_choice(&keys::CANDIDATE_LAYOUT, layout);
    doc.set_bool(&keys::IS_AUTO_SPACE_ENABLED, rng.chance(1, 2));
    doc.set_bool(&keys::IS_LITERAL_ROMAN_CANDIDATE_ENABLED, rng.chance(1, 3));

    match persona {
        PersonaKind::EverydayContinuousTl => {
            doc.set_choice(&keys::INPUT_MODE, InputMode::Tl);
            doc.set_choice(&keys::TONE_INPUT_SCHEME, ToneInputScheme::Standard);
            doc.set_bool(&keys::IS_HANJI_FIRST, true);
        }
        PersonaKind::CarefulPrefixNailer => {
            doc.set_choice(&keys::INPUT_MODE, InputMode::Tl);
            doc.set_choice(&keys::TONE_INPUT_SCHEME, ToneInputScheme::Standard);
            doc.set_bool(&keys::IS_HANJI_FIRST, true);
            doc.set_bool(&keys::IS_F_HYPHEN_ENABLED, rng.chance(1, 3));
        }
        PersonaKind::TelexPowerTypist => {
            let mode = if rng.chance(3, 4) {
                InputMode::Tl
            } else {
                InputMode::Poj
            };
            doc.set_choice(&keys::INPUT_MODE, mode);
            doc.set_choice(&keys::TONE_INPUT_SCHEME, ToneInputScheme::Telex);
            doc.set_bool(&keys::IS_C_AFFRICATE_ENABLED, rng.chance(2, 3));
            doc.set_bool(&keys::IS_F_HYPHEN_ENABLED, rng.chance(2, 3));
            doc.set_bool(&keys::TELEX_GUIDE_SHOWN, rng.chance(1, 4));
        }
        PersonaKind::PojMixedScriptEditor => {
            doc.set_choice(&keys::INPUT_MODE, InputMode::Poj);
            doc.set_choice(
                &keys::TONE_INPUT_SCHEME,
                if rng.chance(2, 3) {
                    ToneInputScheme::Standard
                } else {
                    ToneInputScheme::Telex
                },
            );
            doc.set_bool(&keys::IS_HANJI_FIRST, rng.chance(1, 2));
            let display = *rng.choose(&[
                CandidateDisplayMode::SideBySide,
                CandidateDisplayMode::Combined,
                CandidateDisplayMode::RomanOnly,
            ]);
            doc.set_choice(&keys::CANDIDATE_DISPLAY_MODE, display);
            let sep = *rng.choose(&[
                SyllableSeparator::Hyphen,
                SyllableSeparator::Space,
                SyllableSeparator::None,
            ]);
            doc.set_choice(&keys::SYLLABLE_SEPARATOR, sep);
            doc.set_bool(&keys::IS_NASAL_MARKER_UPPERCASE_ENABLED, rng.chance(1, 4));
        }
        PersonaKind::TpsPhoneticTypist => {
            doc.set_choice(&keys::INPUT_MODE, InputMode::Tps);
            doc.set_bool(&keys::IS_HANJI_FIRST, true);
        }
        PersonaKind::ChaosBilingualMonkey => {
            let mode = *rng.choose(&[InputMode::Tl, InputMode::Poj, InputMode::Tps]);
            doc.set_choice(&keys::INPUT_MODE, mode);
            doc.set_choice(
                &keys::TONE_INPUT_SCHEME,
                if rng.chance(1, 2) {
                    ToneInputScheme::Standard
                } else {
                    ToneInputScheme::Telex
                },
            );
            doc.set_bool(&keys::IS_C_AFFRICATE_ENABLED, rng.chance(1, 2));
            doc.set_bool(&keys::IS_F_HYPHEN_ENABLED, rng.chance(1, 2));
            doc.set_bool(&keys::IS_HANJI_FIRST, rng.chance(2, 3));
            doc.set_bool(&keys::IS_CANDIDATE_WINDOW_ENABLED, !rng.chance(1, 10));
        }
    }
    doc
}

/// Compiles one `CorpusSentence` into a realistic sequence of `SimStep`s for `persona`.
pub fn compile_sentence_steps(
    persona: PersonaKind,
    sentence: &CorpusSentence,
    settings: &SettingsDocument,
    rng: &mut Rng,
) -> Vec<SimStep> {
    let mut steps = Vec::with_capacity(64);
    let mode = settings.choice(&keys::INPUT_MODE);
    let scheme = settings.choice(&keys::TONE_INPUT_SCHEME);
    let use_c_affricate = settings.bool(&keys::IS_C_AFFRICATE_ENABLED);
    let use_f_hyphen = settings.bool(&keys::IS_F_HYPHEN_ENABLED);

    if persona == PersonaKind::ChaosBilingualMonkey && rng.chance(1, 12) {
        steps.push(SimStep::SaveCustomWord {
            roman: "tâi-gí-lâng".to_owned(),
            hanji: "台語人".to_owned(),
        });
    }

    let mut words_in_buffer = 0usize;

    for seg in &sentence.segments {
        match seg {
            SentenceSegment::Word(word) => {
                if mode == InputMode::Tps {
                    append_tps_word(&mut steps, word, persona, rng);
                } else {
                    append_roman_word(
                        &mut steps,
                        word,
                        mode,
                        scheme,
                        use_c_affricate,
                        use_f_hyphen,
                        words_in_buffer > 0,
                        persona,
                        rng,
                    );
                }
                words_in_buffer += 1;

                // Decide whether to keep typing continuously, nail a prefix, or commit the word.
                match persona {
                    PersonaKind::EverydayContinuousTl => {
                        if words_in_buffer >= rng.range_inclusive(2, 5) {
                            finish_active_composition(&mut steps, mode, rng);
                            words_in_buffer = 0;
                        }
                    }
                    PersonaKind::CarefulPrefixNailer => {
                        if words_in_buffer >= 2 && rng.chance(2, 3) {
                            if rng.chance(1, 4) {
                                steps.push(SimStep::Navigate(NavDir::Down));
                            }
                            let slot = rng.next_usize(3);
                            let alt = rng.chance(1, 10);
                            steps.push(SimStep::PickSlot {
                                slot,
                                alternate_script: alt,
                            });
                            // A partial pick leaves the rest composing; occasionally edit tail.
                            if rng.chance(1, 5) {
                                steps.push(SimStep::Special(SpecialKey::Backspace));
                            }
                        }
                        if words_in_buffer >= rng.range_inclusive(3, 5) {
                            finish_active_composition(&mut steps, mode, rng);
                            words_in_buffer = 0;
                        }
                    }
                    PersonaKind::TelexPowerTypist | PersonaKind::PojMixedScriptEditor => {
                        if rng.chance(1, 4) {
                            let slot = rng.next_usize(4);
                            steps.push(SimStep::PickSlot {
                                slot,
                                alternate_script: rng.chance(1, 6),
                            });
                        } else if words_in_buffer >= rng.range_inclusive(2, 4) {
                            finish_active_composition(&mut steps, mode, rng);
                            words_in_buffer = 0;
                        }
                    }
                    PersonaKind::TpsPhoneticTypist => {
                        if words_in_buffer >= rng.range_inclusive(1, 3) {
                            steps.push(SimStep::Navigate(NavDir::Down));
                            if rng.chance(1, 4) {
                                steps.push(SimStep::Special(SpecialKey::Escape));
                                steps.push(SimStep::Navigate(NavDir::Down));
                            }
                            if rng.chance(1, 2) {
                                steps.push(SimStep::PickSlot {
                                    slot: rng.next_usize(3),
                                    alternate_script: false,
                                });
                            } else {
                                steps.push(SimStep::Special(SpecialKey::Enter));
                            }
                            words_in_buffer = 0;
                        }
                    }
                    PersonaKind::ChaosBilingualMonkey => {
                        inject_chaos_mid_sentence(&mut steps, rng);
                        if words_in_buffer >= rng.range_inclusive(1, 4) {
                            finish_active_composition(&mut steps, mode, rng);
                            words_in_buffer = 0;
                        }
                    }
                }
            }
            SentenceSegment::Punct(punct) => {
                // Sometimes type punctuation straight into an active composition to exercise
                // `CommitThenInsert`, and sometimes commit first then type punctuation to
                // exercise `auto_space` `DeleteSurrounding` swaps!
                if words_in_buffer > 0 && rng.chance(1, 2) {
                    finish_active_composition(&mut steps, mode, rng);
                    words_in_buffer = 0;
                } else {
                    words_in_buffer = 0;
                }
                append_punctuation(&mut steps, punct, rng);
            }
        }
    }

    if words_in_buffer > 0 {
        finish_active_composition(&mut steps, mode, rng);
    }
    // Ensure we never end a sentence stuck inside English mode or an open symbol picker.
    steps.push(SimStep::Special(SpecialKey::Escape));
    let pause_ms = if rng.chance(1, 15) {
        45_000 // Cross next-word association expiry window
    } else {
        rng.range_inclusive(800, 4_500) as i64
    };
    steps.push(SimStep::AdvanceClockMs(pause_ms));
    steps
}

#[allow(clippy::too_many_arguments)]
fn append_roman_word(
    steps: &mut Vec<SimStep>,
    word: &EncodedWord,
    mode: InputMode,
    scheme: ToneInputScheme,
    use_c_affricate: bool,
    use_f_hyphen: bool,
    has_prior_word_in_buffer: bool,
    persona: PersonaKind,
    rng: &mut Rng,
) {
    if has_prior_word_in_buffer && rng.chance(1, 3) {
        push_hyphen(steps, use_f_hyphen && rng.chance(2, 3));
    }

    for (idx, syl) in word.syllables.iter().enumerate() {
        if syl.khinsiann_before {
            if scheme == ToneInputScheme::Telex && use_f_hyphen && idx == 0 && rng.chance(1, 2) {
                steps.push(SimStep::KeyChar {
                    ch: 'f',
                    shift: false,
                });
                steps.push(SimStep::KeyChar {
                    ch: 'f',
                    shift: false,
                });
            } else {
                push_hyphen(steps, false);
                push_hyphen(steps, false);
            }
        } else if idx > 0 && rng.chance(1, 2) {
            push_hyphen(steps, use_f_hyphen && rng.chance(2, 3));
        }

        let mut base = match mode {
            InputMode::Poj => syl.toneless_poj.clone(),
            _ => syl.toneless_tl.clone(),
        };
        if use_c_affricate && rng.chance(2, 3) {
            if let Some(rest) = base.strip_prefix("ts") {
                base = format!("c{rest}");
            } else if let Some(rest) = base.strip_prefix("ch") {
                base = format!("c{rest}");
            }
        }

        // Occasional capital initial at word start.
        let capitalize = idx == 0 && !has_prior_word_in_buffer && rng.chance(1, 12);
        for (c_idx, ch) in base.chars().enumerate() {
            // Realistic typo + immediate Backspace!
            if should_inject_typo(persona, rng) {
                let wrong = *rng.choose(&['a', 'e', 'i', 'o', 'u', 'k', 'p', 't', 'n']);
                steps.push(SimStep::KeyChar {
                    ch: wrong,
                    shift: false,
                });
                steps.push(SimStep::Special(SpecialKey::Backspace));
            }
            if c_idx == 0 && capitalize {
                steps.push(SimStep::KeyChar {
                    ch: ch.to_ascii_uppercase(),
                    shift: true,
                });
            } else {
                steps.push(SimStep::KeyChar { ch, shift: false });
            }
        }

        // Tone entry according to scheme and persona habits.
        match scheme {
            ToneInputScheme::Standard => {
                let include_tone = match persona {
                    PersonaKind::EverydayContinuousTl => rng.chance(1, 3),
                    _ => rng.chance(3, 4),
                };
                if include_tone {
                    if let Some(d) = syl.tone_digit {
                        steps.push(SimStep::KeyChar {
                            ch: (b'0' + d) as char,
                            shift: false,
                        });
                    } else if rng.chance(1, 6) {
                        // Explicit tone 1 or 4 digit.
                        let d = if ends_with_stop_coda(&base) { '4' } else { '1' };
                        steps.push(SimStep::KeyChar {
                            ch: d,
                            shift: false,
                        });
                    }
                }
            }
            ToneInputScheme::Telex => {
                if let Some(tk) = telex_key_for_tone(syl.tone_digit, rng.chance(1, 5)) {
                    // Occasionally double-tap Telex tone to toggle off then re-apply!
                    if persona == PersonaKind::TelexPowerTypist && rng.chance(1, 12) {
                        steps.push(SimStep::KeyChar {
                            ch: tk,
                            shift: false,
                        });
                        steps.push(SimStep::KeyChar {
                            ch: tk,
                            shift: false,
                        });
                    }
                    steps.push(SimStep::KeyChar {
                        ch: tk,
                        shift: false,
                    });
                }
            }
        }
    }
}

fn append_tps_word(
    steps: &mut Vec<SimStep>,
    word: &EncodedWord,
    persona: PersonaKind,
    rng: &mut Rng,
) {
    for stroke in &word.tps_keys {
        if persona == PersonaKind::TpsPhoneticTypist && stroke.ch == '=' && rng.chance(1, 6) {
            steps.push(SimStep::TpsPanelGlyph("-".to_owned()));
            continue;
        }
        steps.push(SimStep::KeyChar {
            ch: stroke.ch,
            shift: stroke.shift,
        });
        if should_inject_typo(persona, rng) {
            steps.push(SimStep::KeyChar {
                ch: '1',
                shift: false,
            });
            steps.push(SimStep::Special(SpecialKey::Backspace));
        }
    }
}

fn append_punctuation(steps: &mut Vec<SimStep>, punct: &str, rng: &mut Rng) {
    for ch in punct.chars() {
        let mapped = match ch {
            '。' | '.' => {
                if rng.chance(1, 2) {
                    '.'
                } else {
                    ','
                }
            }
            '，' | '、' | ',' => ',',
            '？' | '?' => '?',
            '！' | '!' => '!',
            '：' | ':' => ':',
            '；' | ';' => ';',
            '「' | '“' | '"' => '[',
            '」' | '”' => ']',
            _ if ch.is_ascii_graphic() => ch,
            _ => continue,
        };
        let shift = matches!(mapped, '?' | '!' | ':' | '"' | '(' | ')');
        steps.push(SimStep::KeyChar { ch: mapped, shift });
    }
}

fn finish_active_composition(steps: &mut Vec<SimStep>, mode: InputMode, rng: &mut Rng) {
    if mode == InputMode::Tps && rng.chance(2, 3) {
        steps.push(SimStep::Navigate(NavDir::Down));
    }
    let roll = rng.next_usize(10);
    match roll {
        0 => {
            steps.push(SimStep::Special(SpecialKey::Tab));
            steps.push(SimStep::Special(SpecialKey::Enter));
        }
        1 => {
            steps.push(SimStep::PanelClick { position: 0 });
            steps.push(SimStep::Special(SpecialKey::Enter));
        }
        2 => {
            steps.push(SimStep::Special(SpecialKey::ShiftEnter));
        }
        3..=5 => {
            steps.push(SimStep::Special(SpecialKey::Space));
        }
        6..=7 => {
            steps.push(SimStep::PickSlot {
                slot: 0,
                alternate_script: false,
            });
            // In case PickSlot nailed only a prefix, finish the rest with Enter.
            steps.push(SimStep::Special(SpecialKey::Enter));
        }
        _ => {
            steps.push(SimStep::Special(SpecialKey::Enter));
        }
    }
}

fn inject_chaos_mid_sentence(steps: &mut Vec<SimStep>, rng: &mut Rng) {
    match rng.next_usize(14) {
        0 => {
            // Caret walk + mid-buffer edit + return to end.
            steps.push(SimStep::MoveCaret(CaretStep::Left));
            steps.push(SimStep::MoveCaret(CaretStep::Left));
            steps.push(SimStep::KeyChar {
                ch: 'a',
                shift: false,
            });
            steps.push(SimStep::Special(SpecialKey::Backspace));
            steps.push(SimStep::MoveCaret(CaretStep::End));
        }
        1 => {
            // Home -> DeleteForward -> End.
            steps.push(SimStep::MoveCaret(CaretStep::Home));
            steps.push(SimStep::Special(SpecialKey::DeleteForward));
            steps.push(SimStep::MoveCaret(CaretStep::CtrlDown));
        }
        2 => {
            // Inline English burst via Shift tap toggle, then toggle back to Taigi!
            steps.push(SimStep::ShiftTapLanguageToggle);
            for ch in rng.choose(&["OK ", "LINE ", "2026 ", "App "]).chars() {
                steps.push(SimStep::KeyChar {
                    ch,
                    shift: ch.is_ascii_uppercase(),
                });
            }
            steps.push(SimStep::ShiftTapLanguageToggle);
        }
        3 => {
            // Open symbol picker, navigate, pick slot 0.
            steps.push(SimStep::GlobalShortcut(
                SafeGlobalShortcut::ShowSymbolPicker,
            ));
            steps.push(SimStep::Navigate(NavDir::Right));
            steps.push(SimStep::PickSlot {
                slot: 0,
                alternate_script: false,
            });
        }
        4 => {
            // Mid-composition mode or display switch!
            let sc = *rng.choose(&[
                SafeGlobalShortcut::ToggleRomanization,
                SafeGlobalShortcut::ToggleTranslateSwapped,
                SafeGlobalShortcut::CycleCandidateDisplayMode,
                SafeGlobalShortcut::ShowTelexGuide,
            ]);
            steps.push(SimStep::GlobalShortcut(sc));
        }
        5 => {
            steps.push(SimStep::FocusBounceLinux);
        }
        6 => {
            steps.push(SimStep::FocusBounceCommitComposition);
        }
        7 => {
            steps.push(SimStep::SetSurroundingCap(rng.chance(1, 2)));
        }
        8 => {
            // Multi-page candidate walk + PanelNavigate.
            steps.push(SimStep::Special(SpecialKey::PageNextBracket));
            steps.push(SimStep::PanelNavigate(NavDir::Down));
            steps.push(SimStep::Special(SpecialKey::PagePrevBracket));
        }
        9 => {
            // Multi-char backspace across boundary.
            for _ in 0..rng.range_inclusive(1, 4) {
                steps.push(SimStep::Special(SpecialKey::Backspace));
            }
        }
        10 => {
            steps.push(SimStep::FlipLiveSettings);
        }
        _ => {}
    }
}

fn push_hyphen(steps: &mut Vec<SimStep>, as_f: bool) {
    steps.push(SimStep::KeyChar {
        ch: if as_f { 'f' } else { '-' },
        shift: false,
    });
}

fn should_inject_typo(persona: PersonaKind, rng: &mut Rng) -> bool {
    match persona {
        PersonaKind::ChaosBilingualMonkey => rng.chance(1, 10),
        PersonaKind::EverydayContinuousTl | PersonaKind::CarefulPrefixNailer => rng.chance(1, 35),
        _ => rng.chance(1, 45),
    }
}

fn ends_with_stop_coda(s: &str) -> bool {
    s.ends_with('p') || s.ends_with('t') || s.ends_with('k') || s.ends_with('h')
}
