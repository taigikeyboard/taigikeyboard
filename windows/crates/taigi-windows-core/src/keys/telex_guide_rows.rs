//! The rows of the floating Telex key table, spelled for the romanization
//! in use. Port of the `rows` table in `TelexGuidePanel.swift`; the window
//! that draws them is `ui/telex_guide.rs` in the TSF crate. Kept here, with
//! no Win32 in sight, so the table is unit-tested where the panel cannot be.

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
/// language (`TelexGuidePanel.swift` `Meaning`).
#[derive(Clone, Copy)]
enum Meaning {
    Tone(&'static str),
    /// `z` / `zh`, with the initial each spells under TL and under POJ. The
    /// spelling lives in the meaning now that the example column is gone.
    Initial {
        tl: &'static str,
        poj: &'static str,
    },
    Hyphen,
    Pick,
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
/// keys, then the hyphen, then the digits. CROSS-PLATFORM INVARIANT —
/// mirrors `TelexGuidePanel.swift` `rows`, row for row.
const ROWS: [RowSpec; 10] = [
    RowSpec::new("v", Meaning::Tone("2")),
    RowSpec::new("y", Meaning::Tone("3")),
    RowSpec::new("d", Meaning::Tone("5")),
    RowSpec::new("w", Meaning::Tone("7")),
    RowSpec::new("x", Meaning::Tone("8")),
    RowSpec::new("q", Meaning::Tone("9")),
    RowSpec::new(
        "z",
        Meaning::Initial {
            tl: "ts",
            poj: "ch",
        },
    ),
    RowSpec::new(
        "zh",
        Meaning::Initial {
            tl: "tsh",
            poj: "chh",
        },
    ),
    RowSpec::new("f", Meaning::Hyphen),
    RowSpec::new("1–9", Meaning::Pick),
];

/// The table for `mode`, with every meaning resolved through `strings`.
pub fn telex_guide_rows(mode: InputMode, strings: &StringResolver) -> Vec<TelexGuideRow> {
    ROWS.iter()
        .map(|row| TelexGuideRow {
            key: row.key,
            meaning: match row.meaning {
                Meaning::Tone(tone) => strings.format(StringKey::DesktopTelexGuideTone, &[&tone]),
                Meaning::Initial { tl, poj } => {
                    let initial = match mode {
                        InputMode::Poj => poj,
                        _ => tl,
                    };
                    strings.format(StringKey::DesktopTelexGuideInitial, &[&initial])
                }
                Meaning::Hyphen => strings
                    .resolve(StringKey::DesktopTelexGuideHyphen)
                    .to_owned(),
                Meaning::Pick => strings.resolve(StringKey::DesktopTelexGuidePick).to_owned(),
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
        // trace: TelexGuidePanel.swift `rows` — ten rows, keys in reading order.
        let keys: Vec<_> = rows(InputMode::Tl).iter().map(|row| row.key).collect();
        assert_eq!(keys, ["v", "y", "d", "w", "x", "q", "z", "zh", "f", "1–9"]);
    }

    #[test]
    fn meanings_come_from_the_display_language() {
        // trace: Hanji `desktop.telexGuideTone` = "第 {0} 聲",
        // `telexGuideHyphen` = "連字號", `telexGuidePick` = "選詞".
        let tl = rows(InputMode::Tl);
        assert_eq!(tl[0].meaning, "第 2 聲");
        assert_eq!(tl[5].meaning, "第 9 聲");
        assert_eq!(tl[8].meaning, "連字號");
        assert_eq!(tl[9].meaning, "選詞");
        let en = telex_guide_rows(
            InputMode::Tl,
            &StringResolver::new(DisplayLanguage::English),
        );
        assert_eq!(en[0].meaning, "Tone 2");
        assert_eq!(en[6].meaning, "Initial ts");
    }

    #[test]
    fn the_affricate_rows_follow_the_romanization() {
        // trace: `z` spells ts / ch, `zh` spells tsh / chh — the meaning
        // carries the spelling now that the example column is gone.
        let tl = rows(InputMode::Tl);
        assert_eq!(tl[6].meaning, "聲母 ts");
        assert_eq!(tl[7].meaning, "聲母 tsh");
        let poj = rows(InputMode::Poj);
        assert_eq!(poj[6].meaning, "聲母 ch");
        assert_eq!(poj[7].meaning, "聲母 chh");
        // Every other row reads the same under both.
        for index in [0, 5, 8, 9] {
            assert_eq!(tl[index], poj[index], "row {index}");
        }
    }
}
