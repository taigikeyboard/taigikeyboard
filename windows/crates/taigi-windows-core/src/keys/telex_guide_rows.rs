//! The rows of the floating Telex key table, spelled for the romanization
//! in use. Port of the `rows` table in `TelexGuidePanel.swift`; the window
//! that draws them is `ui/telex_guide.rs` in the TSF crate. Kept here, with
//! no Win32 in sight, so the table is unit-tested where the panel cannot be.

use crate::settings::InputMode;
use crate::strings::{StringKey, StringResolver};

/// One row as the window draws it: the key, what it does in the display
/// language, and the example spelled for the romanization in use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TelexGuideRow {
    pub key: &'static str,
    pub meaning: String,
    pub example: &'static str,
}

/// What a key does — a string key, so the meaning follows the display
/// language (`TelexGuidePanel.swift` `Meaning`).
#[derive(Clone, Copy)]
enum Meaning {
    Tone(&'static str),
    Initial,
    Hyphen,
    Pick,
}

/// A row before the display language and the romanization are known.
///
/// The examples are romanization, not prose, so they are spelled here
/// rather than translated: `z` is `ts` under TL and `ch` under POJ, and
/// tone 9 is a double acute (U+030B) in TL but a breve (U+0306) in POJ —
/// the row has to show the spelling the user will actually see.
struct RowSpec {
    key: &'static str,
    meaning: Meaning,
    tl_example: &'static str,
    poj_example: &'static str,
}

impl RowSpec {
    /// The same example under both romanizations — every tone key but 9.
    const fn same(key: &'static str, meaning: Meaning, example: &'static str) -> Self {
        Self {
            key,
            meaning,
            tl_example: example,
            poj_example: example,
        }
    }

    const fn split(
        key: &'static str,
        meaning: Meaning,
        tl_example: &'static str,
        poj_example: &'static str,
    ) -> Self {
        Self {
            key,
            meaning,
            tl_example,
            poj_example,
        }
    }
}

/// Row order is reading order: the tones by number, then the two consonant
/// keys, then the hyphen, then the digits. CROSS-PLATFORM INVARIANT —
/// mirrors `TelexGuidePanel.swift` `rows`, row for row.
const ROWS: [RowSpec; 10] = [
    RowSpec::same("v", Meaning::Tone("2"), "tev → té"),
    RowSpec::same("y", Meaning::Tone("3"), "pay → pà"),
    RowSpec::same("d", Meaning::Tone("5"), "langd → lâng"),
    RowSpec::same("w", Meaning::Tone("7"), "kangw → kāng"),
    RowSpec::same("x", Meaning::Tone("8"), "titx → ti̍t"),
    RowSpec::split("q", Meaning::Tone("9"), "tsangq → tsa̋ng", "zangq → chăng"),
    RowSpec::split("z", Meaning::Initial, "zo → tso", "zit → chit"),
    RowSpec::split("zh", Meaning::Initial, "zhi → tshi", "zhit → chhit"),
    RowSpec::same("f", Meaning::Hyphen, "taidfgiv → tâi-gí"),
    RowSpec::same("1–9", Meaning::Pick, ""),
];

/// The table for `mode`, with every meaning resolved through `strings`.
pub fn telex_guide_rows(mode: InputMode, strings: &StringResolver) -> Vec<TelexGuideRow> {
    ROWS.iter()
        .map(|row| TelexGuideRow {
            key: row.key,
            meaning: match row.meaning {
                Meaning::Tone(tone) => strings.format(StringKey::DesktopTelexGuideTone, &[&tone]),
                Meaning::Initial => strings
                    .resolve(StringKey::DesktopTelexGuideInitial)
                    .to_owned(),
                Meaning::Hyphen => strings
                    .resolve(StringKey::DesktopTelexGuideHyphen)
                    .to_owned(),
                Meaning::Pick => strings.resolve(StringKey::DesktopTelexGuidePick).to_owned(),
            },
            example: match mode {
                InputMode::Tl => row.tl_example,
                InputMode::Poj => row.poj_example,
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
        // trace: TelexGuidePanel.swift `rows` — ten rows, keys in reading
        // order, and `1–9` is the only row with no example.
        let tl = rows(InputMode::Tl);
        let keys: Vec<_> = tl.iter().map(|row| row.key).collect();
        assert_eq!(keys, ["v", "y", "d", "w", "x", "q", "z", "zh", "f", "1–9"]);
        assert_eq!(
            tl.iter().filter(|row| row.example.is_empty()).count(),
            1,
            "only the pick row has no example"
        );
        assert_eq!(tl[9].example, "");
    }

    #[test]
    fn meanings_come_from_the_display_language() {
        // trace: Hanji `desktop.telexGuideTone` = "第 {0} 聲",
        // `telexGuideInitial` = "聲母", `telexGuideHyphen` = "連字號",
        // `telexGuidePick` = "候選窗顯示時選詞" (`strings/generated.rs`).
        let tl = rows(InputMode::Tl);
        assert_eq!(tl[0].meaning, "第 2 聲");
        assert_eq!(tl[5].meaning, "第 9 聲");
        assert_eq!(tl[6].meaning, "聲母");
        assert_eq!(tl[8].meaning, "連字號");
        assert_eq!(tl[9].meaning, "候選窗顯示時選詞");
        let en = telex_guide_rows(
            InputMode::Tl,
            &StringResolver::new(DisplayLanguage::English),
        );
        assert_eq!(en[0].meaning, "Tone 2");
        assert_eq!(en[6].meaning, "Initial");
    }

    #[test]
    fn examples_follow_the_romanization() {
        // trace: TelexGuidePanel.swift — `q`, `z`, `zh` are the three rows
        // spelled differently under POJ (breve for tone 9, `ch` / `chh` for
        // the affricate); every other row reads the same under both.
        let tl = rows(InputMode::Tl);
        let poj = rows(InputMode::Poj);
        assert_eq!(tl[5].example, "tsangq → tsa̋ng");
        assert_eq!(poj[5].example, "zangq → chăng");
        assert_eq!(tl[6].example, "zo → tso");
        assert_eq!(poj[6].example, "zit → chit");
        assert_eq!(tl[7].example, "zhi → tshi");
        assert_eq!(poj[7].example, "zhit → chhit");
        for index in [0, 1, 2, 3, 4, 8, 9] {
            assert_eq!(tl[index].example, poj[index].example, "row {index}");
        }
        assert_eq!(tl[8].example, "taidfgiv → tâi-gí");
        // The meanings do not depend on the romanization.
        for (left, right) in tl.iter().zip(&poj) {
            assert_eq!(left.meaning, right.meaning);
        }
    }
}
