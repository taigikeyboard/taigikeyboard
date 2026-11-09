//! The desktop "Type ts / ch with c" setting (`AppConfig.c_types_affricate`):
//! a typed `c` writes the unaspirated affricate initial the input mode spells.
//! Neither romanization spells a syllable with a bare `c`, so the key is free;
//! `h` stays an ordinary letter, so `c` `h` spells the aspirated one.
//!
//! Independent of the tone scheme: an `Append` carries it under Standard and
//! Telex alike. POJ's spelled-out `ch` costs the same as it did under Telex:
//! `c` `h` is `chh`, so typing `cha` letter by letter reaches `chha`.

use phonetics::api::InputMode;
use protos::engine::AppConfig;

/// What an `Append` of `ch` writes: the initial when the setting is on, the
/// mode is a romanization and `ch` is exactly `c` / `C`; otherwise `ch` as
/// typed. TPS is excluded by its own mode rather than by `composing_mode`,
/// which reads the TPS layout as TL.
pub(crate) fn expand(ch: String, config: &AppConfig) -> String {
    if !config.c_types_affricate || config.is_tps_layout() {
        return ch;
    }
    let initial = match (
        phonetics::api::parse_input_mode(&config.input_mode),
        ch.as_str(),
    ) {
        (InputMode::Tl, "c") => "ts",
        (InputMode::Tl, "C") => "Ts",
        (InputMode::Poj, "c") => "ch",
        (InputMode::Poj, "C") => "Ch",
        _ => return ch,
    };
    initial.to_owned()
}

#[cfg(test)]
mod tests {
    // INVARIANT_C_TYPES_AFFRICATE (behavioral-invariants.md §62)
    use super::*;

    fn config(mode: &str, on: bool) -> AppConfig {
        AppConfig {
            input_mode: mode.into(),
            c_types_affricate: on,
            ..Default::default()
        }
    }

    #[test]
    fn c_expands_by_mode_and_case_when_on() {
        for (mode, key, initial) in [
            ("tl", "c", "ts"),
            ("tl", "C", "Ts"),
            ("poj", "c", "ch"),
            ("poj", "C", "Ch"),
        ] {
            assert_eq!(
                expand(key.into(), &config(mode, true)),
                initial,
                "{mode} {key}"
            );
        }
    }

    #[test]
    fn c_is_a_plain_letter_when_off() {
        for mode in ["tl", "poj"] {
            for key in ["c", "C"] {
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
        // trace: only exactly `c` / `C` under TL / POJ; `h` writes itself
        // (no slip rule after `tsh` / `chh`), TPS and English keep the key.
        for key in ["h", "H", "t", "cc", "ch", ""] {
            assert_eq!(expand(key.into(), &config("tl", true)), key, "{key}");
        }
        for mode in ["tps", "english"] {
            assert_eq!(expand("c".into(), &config(mode, true)), "c", "{mode}");
        }
    }
}
