//! The rows of the floating Telex key table, spelled for the romanization
//! in use. The window that draws them is `ui/telex_guide.rs` in the TSF
//! crate. Kept here, with no Win32 in sight, so the table is unit-tested
//! where the panel cannot be. macOS keeps a Swift twin of the table:
//! `TelexGuidePanel.swift` `rows`.

use crate::settings::InputMode;
use crate::strings::{StringKey, StringResolver};

/// One row as the window draws it: the key and what it does in the display
/// language. Two columns — no examples (USER 2026-09-09).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TelexGuideRow {
    pub key: &'static str,
    pub meaning: String,
}

/// What a key does — a string key, so the meaning follows the display
/// language. `TelexGuidePanel.swift` `Meaning` is the macOS twin.
#[derive(Clone, Copy)]
enum Meaning {
    Tone(&'static str),
    /// `c` / `ch`, with the initial each spells under TL and under POJ. The
    /// spelling lives in the meaning now that the example column is gone.
    Initial {
        tl: &'static str,
        poj: &'static str,
    },
    Hyphen,
}

/// A row before the display language and the romanization are known.
struct RowSpec {
    key: &'static str,
    meaning: Meaning,
}

impl RowSpec {
    const fn new(key: &'static str, meaning: Meaning) -> Self {
        Self { key, meaning }
    }
}

/// Row order is reading order: the tones by number, then the two consonant
/// keys, then the hyphen. The digit row went 2026-09-09 (USER) — the
/// candidate window already draws the key beside each candidate. `x` and
/// `v` each carry a pair split by the coda (`engine/composing/src/telex.rs`):
/// 1 / 4 and 2 / 8, the open tone on any other syllable, the checked one
/// after `p t k h`.
/// CROSS-PLATFORM INVARIANT — desktop-core owns this table; macOS keeps a
/// Swift twin row for row (`TelexGuidePanel.swift` `rows`).
const ROWS: [RowSpec; 10] = [
    RowSpec::new("x", Meaning::Tone("1 / 4")),
    RowSpec::new("v", Meaning::Tone("2 / 8")),
    RowSpec::new("y", Meaning::Tone("3")),
    RowSpec::new("d", Meaning::Tone("5")),
    RowSpec::new("z", Meaning::Tone("6")),
    RowSpec::new("w", Meaning::Tone("7")),
    RowSpec::new("q", Meaning::Tone("9")),
    RowSpec::new(
        "c",
        Meaning::Initial {
            tl: "ts",
            poj: "ch",
        },
    ),
    RowSpec::new(
        "ch",
        Meaning::Initial {
            tl: "tsh",
            poj: "chh",
        },
    ),
    RowSpec::new("f", Meaning::Hyphen),
];

/// The table for `mode`, with every meaning resolved through `strings`.
pub fn telex_guide_rows(mode: InputMode, strings: &StringResolver) -> Vec<TelexGuideRow> {
    ROWS.iter()
        .map(|row| TelexGuideRow {
            key: row.key,
            meaning: match row.meaning {
                Meaning::Tone(tone) => strings.format(StringKey::DesktopTelexGuideTone, &[&tone]),
                Meaning::Initial { tl, poj } => {
                    // The guide is not shown under TPS (no Telex keys there);
                    // a table asked for anyway spells TL.
                    let initial = match mode {
                        InputMode::Poj => poj,
                        InputMode::Tl | InputMode::Tps => tl,
                    };
                    strings.format(StringKey::DesktopTelexGuideInitial, &[&initial])
                }
                Meaning::Hyphen => strings
                    .resolve(StringKey::DesktopTelexGuideHyphen)
                    .to_owned(),
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strings::DisplayLanguage;

    fn rows(mode: InputMode) -> Vec<TelexGuideRow> {
        telex_guide_rows(mode, &StringResolver::new(DisplayLanguage::Hanji))
    }

    #[test]
    fn the_table_is_the_mac_panels_row_for_row() {
        // trace: `ROWS` — ten rows, keys in reading order of the tone they
        // write; the paired keys lead.
        let keys: Vec<_> = rows(InputMode::Tl).iter().map(|row| row.key).collect();
        assert_eq!(keys, ["x", "v", "y", "d", "z", "w", "q", "c", "ch", "f"]);
    }

    #[test]
    fn meanings_come_from_the_display_language() {
        // trace: Hanji `desktop.telexGuideTone` = "第 {0} 調" (generated.rs),
        // the pair rows feed "1 / 4" and "2 / 8" as the tone;
        // `telexGuideHyphen` = "連字符".
        let tl = rows(InputMode::Tl);
        assert_eq!(tl[0].meaning, "第 1 / 4 調");
        assert_eq!(tl[1].meaning, "第 2 / 8 調");
        assert_eq!(tl[4].meaning, "第 6 調");
        assert_eq!(tl[6].meaning, "第 9 調");
        assert_eq!(tl[9].meaning, "連字符");
        let en = telex_guide_rows(
            InputMode::Tl,
            &StringResolver::new(DisplayLanguage::English),
        );
        assert_eq!(en[0].meaning, "Tone 1 / 4");
        assert_eq!(en[7].meaning, "ts");
    }

    #[test]
    fn the_affricate_rows_follow_the_romanization() {
        // trace: `c` spells ts / ch, `ch` spells tsh / chh — the meaning
        // carries the spelling now that the example column is gone;
        // `desktop.telexGuideInitial` = "{0}" in every language.
        let tl = rows(InputMode::Tl);
        assert_eq!(tl[7].meaning, "ts");
        assert_eq!(tl[8].meaning, "tsh");
        let poj = rows(InputMode::Poj);
        assert_eq!(poj[7].meaning, "ch");
        assert_eq!(poj[8].meaning, "chh");
        // Every other row reads the same under both.
        for index in [0, 4, 6, 9] {
            assert_eq!(tl[index], poj[index], "row {index}");
        }
    }
}
