//! The desktop "Type - with f" setting (`AppConfig.f_types_hyphen`): a typed
//! `f` writes the syllable hyphen, as the `-` key does. Neither romanization
//! spells a syllable with `f`, so the key is free; idle, the hyphen is the
//! neutral-tone (khinsiann) marker, a document literal (§21).
//!
//! Independent of the tone scheme: an `Append` carries it under Standard and
//! Telex alike. Under Standard `f` is also a candidate slot key; the desktop
//! drops it from the slot row while the setting is on, so it reaches here.

use phonetics::api::InputMode;
use protos::engine::AppConfig;

/// What an `Append` of `ch` writes: `-` when the setting is on, the mode is a
/// romanization and `ch` is exactly `f` / `F`; otherwise `ch` as typed. TPS
/// is excluded by its own mode rather than by `composing_mode`, which reads
/// the TPS layout as TL.
pub(crate) fn expand(ch: String, config: &AppConfig) -> String {
    if !config.f_types_hyphen || config.is_tps_layout() {
        return ch;
    }
    let is_romanization = matches!(
        phonetics::api::parse_input_mode(&config.input_mode),
        InputMode::Tl | InputMode::Poj
    );
    if is_romanization && matches!(ch.as_str(), "f" | "F") {
        return "-".to_owned();
    }
    ch
}

#[cfg(test)]
mod tests {
    // INVARIANT_F_TYPES_HYPHEN (behavioral-invariants.md §62)
    use super::*;

    fn config(mode: &str, on: bool) -> AppConfig {
        AppConfig {
            input_mode: mode.into(),
            f_types_hyphen: on,
            ..Default::default()
        }
    }

    #[test]
    fn f_writes_the_hyphen_in_either_case_when_on() {
        for mode in ["tl", "poj"] {
            for key in ["f", "F"] {
                assert_eq!(expand(key.into(), &config(mode, true)), "-", "{mode} {key}");
            }
        }
    }

    #[test]
    fn f_is_a_plain_letter_when_off() {
        for mode in ["tl", "poj"] {
            for key in ["f", "F"] {
                assert_eq!(
                    expand(key.into(), &config(mode, false)),
                    key,
                    "{mode} {key}"
                );
            }
        }
    }

    #[test]
    fn other_keys_and_other_modes_pass_through() {
        // trace: only exactly `f` / `F` under TL / POJ; TPS and English keep
        // the key.
        for key in ["c", "v", "ff", "-", ""] {
            assert_eq!(expand(key.into(), &config("tl", true)), key, "{key}");
        }
        for mode in ["tps", "english"] {
            assert_eq!(expand("f".into(), &config(mode, true)), "f", "{mode}");
        }
    }
}
