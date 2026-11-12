//! The rows of the floating Telex key table, spelled for the romanization
//! in use. The window that draws them is `ui/telex_guide.rs` in the TSF
//! crate. Kept here, with no Win32 in sight, so the table is unit-tested
//! where the panel cannot be. macOS keeps a Swift twin of the table:
//! `TelexGuidePanel.swift` `rows`.

use crate::strings::{StringKey, StringResolver};

/// One row as the window draws it: the key and what it does in the display
/// language. Two columns — no examples (USER 2026-09-09).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TelexGuideRow {
    pub key: &'static str,
    pub meaning: String,
}

/// A row before the display language is known: the key and the tone it
/// writes.
struct RowSpec {
    key: &'static str,
    tone: &'static str,
}

impl RowSpec {
    const fn new(key: &'static str, tone: &'static str) -> Self {
        Self { key, tone }
    }
}

/// Row order is reading order: the tones by number. The digit row went 2026-09-09 (USER) — the
/// candidate window already draws the key beside each candidate. `x` and
/// `v` each carry a pair split by the coda (`engine/composing/src/telex.rs`):
/// 1 / 4 and 2 / 8, the open tone on any other syllable, the checked one
/// after `p t k h`.
/// CROSS-PLATFORM INVARIANT — desktop-core owns this table; macOS keeps a
/// Swift twin row for row (`TelexGuidePanel.swift` `rows`).
const ROWS: [RowSpec; 7] = [
    RowSpec::new("x", "1 / 4"),
    RowSpec::new("v", "2 / 8"),
    RowSpec::new("y", "3"),
    RowSpec::new("d", "5"),
    RowSpec::new("z", "6"),
    RowSpec::new("w", "7"),
    RowSpec::new("q", "9"),
];

/// The table, with every meaning resolved through `strings`. The same under
/// TL and POJ, and only the tones: `c` → `ts` / `ch` and `f` → `-` are their
/// own settings, not Telex keys.
pub fn telex_guide_rows(strings: &StringResolver) -> Vec<TelexGuideRow> {
    ROWS.iter()
        .map(|row| TelexGuideRow {
            key: row.key,
            meaning: strings.format(StringKey::DesktopTelexGuideTone, &[&row.tone]),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strings::DisplayLanguage;

    fn rows() -> Vec<TelexGuideRow> {
        telex_guide_rows(&StringResolver::new(DisplayLanguage::Hanji))
    }

    #[test]
    fn the_table_is_the_mac_panels_row_for_row() {
        // trace: `ROWS` — seven rows, keys in reading order of the tone they
        // write; the paired keys lead; no `c` / `ch` / `f` rows.
        let keys: Vec<_> = rows().iter().map(|row| row.key).collect();
        assert_eq!(keys, ["x", "v", "y", "d", "z", "w", "q"]);
    }

    #[test]
    fn meanings_come_from_the_display_language() {
        // trace: Hanji `desktop.telexGuideTone` = "第 {0} 調" (generated.rs),
        // the pair rows feed "1 / 4" and "2 / 8" as the tone.
        let hanji = rows();
        assert_eq!(hanji[0].meaning, "第 1 / 4 調");
        assert_eq!(hanji[1].meaning, "第 2 / 8 調");
        assert_eq!(hanji[4].meaning, "第 6 調");
        assert_eq!(hanji[6].meaning, "第 9 調");
        let en = telex_guide_rows(&StringResolver::new(DisplayLanguage::English));
        assert_eq!(en[0].meaning, "Tone 1 / 4");
    }
}
