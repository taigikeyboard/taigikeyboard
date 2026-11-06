//! What the page reads back: one JSON object per call. A wire type of its own
//! rather than `Serialize` on [`Emit`]: the page needs text a shell elsewhere
//! asks the runtime for, and this is the one place that JSON's shape is named.

use serde::Serialize;
use taigi_desktop_core::keys::tps_keyboard_rows;
use taigi_desktop_core::session::{Emit, LookupTableContent};

/// One candidate: the two scripts apart, because the panel sets them apart.
#[derive(Debug, Serialize)]
pub(crate) struct WireCell<'a> {
    text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    annotation: Option<&'a str>,
}

/// One call's answer.
#[derive(Debug, Serialize)]
pub(crate) struct Reply<'a> {
    /// `true`: the page calls `preventDefault`.
    pub handled: bool,
    pub emits: Vec<WireEmit<'a>>,
    /// The settings document, when this call wrote it: the page persists it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<String>,
}

/// [`Emit`] for the page. `caret` counts Unicode scalars, as `Emit` does:
/// `Array.from(text)` on the page's side.
#[derive(Debug, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum WireEmit<'a> {
    Preedit {
        text: &'a str,
        caret: u32,
    },
    ClearPreedit,
    Commit {
        text: &'a str,
    },
    DeleteSurrounding {
        offset: i32,
        count: u32,
    },
    LookupTable {
        cells: Vec<WireCell<'a>>,
        labels: &'a [String],
        cursor: u32,
        cursor_visible: bool,
        page_size: u32,
        vertical: bool,
    },
    HideLookupTable,
    ModeChanged {
        symbol: &'a str,
    },
    AnnounceMode {
        label: &'a str,
    },
    OpenSettings,
    ToggleTpsKeyboard,
    TpsKeyboardCapTyped {
        row: usize,
        cap: usize,
    },
}

/// One row of the TPS key panel for the page: `indent` in key widths.
#[derive(Debug, Serialize)]
pub(crate) struct WireTpsRow {
    indent: f32,
    caps: Vec<WireTpsCap>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WireTpsCap {
    label: char,
    glyph: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    shift_glyph: Option<&'static str>,
}

/// The panel's rows, as JSON.
pub(crate) fn tps_rows_json() -> String {
    let rows: Vec<WireTpsRow> = tps_keyboard_rows()
        .into_iter()
        .map(|row| WireTpsRow {
            indent: row.indent,
            caps: row
                .caps
                .into_iter()
                .map(|cap| WireTpsCap {
                    label: cap.label,
                    glyph: cap.glyph,
                    shift_glyph: cap.shift_glyph,
                })
                .collect(),
        })
        .collect();
    // Strings and numbers only: this cannot fail.
    serde_json::to_string(&rows).unwrap_or_else(|_| String::from("[]"))
}

/// What the two mode emits carry.
pub(crate) struct ModeText {
    pub symbol: &'static str,
    pub label: String,
}

pub(crate) fn wire_emit<'a>(emit: &'a Emit, mode: &'a ModeText) -> WireEmit<'a> {
    match emit {
        Emit::Preedit { text, caret } => WireEmit::Preedit {
            text,
            caret: *caret,
        },
        Emit::ClearPreedit => WireEmit::ClearPreedit,
        Emit::Commit(text) => WireEmit::Commit { text },
        Emit::DeleteSurrounding { offset, count } => WireEmit::DeleteSurrounding {
            offset: *offset,
            count: *count,
        },
        Emit::LookupTable(LookupTableContent {
            cells,
            labels,
            cursor,
            cursor_visible,
            page_size,
            vertical,
        }) => WireEmit::LookupTable {
            cells: cells
                .iter()
                .map(|cell| WireCell {
                    text: &cell.text,
                    annotation: cell.annotation.as_deref(),
                })
                .collect(),
            labels,
            cursor: *cursor,
            cursor_visible: *cursor_visible,
            page_size: *page_size,
            vertical: *vertical,
        },
        Emit::HideLookupTable => WireEmit::HideLookupTable,
        Emit::ModeChanged => WireEmit::ModeChanged {
            symbol: mode.symbol,
        },
        Emit::AnnounceMode => WireEmit::AnnounceMode { label: &mode.label },
        Emit::OpenSettings => WireEmit::OpenSettings,
        Emit::ToggleTpsKeyboard => WireEmit::ToggleTpsKeyboard,
        Emit::TpsKeyboardCapTyped(index) => WireEmit::TpsKeyboardCapTyped {
            row: index.row,
            cap: index.cap,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taigi_desktop_core::composing::CandidateCellContent;
    use taigi_desktop_core::keys::TpsKeyCapIndex;

    fn json(emit: Emit) -> String {
        let mode = ModeText {
            symbol: "台",
            label: "label".to_owned(),
        };
        serde_json::to_string(&wire_emit(&emit, &mode)).unwrap()
    }

    #[test]
    fn the_field_names_are_the_pages() {
        assert_eq!(
            json(Emit::Commit("台".to_owned())),
            r#"{"type":"commit","text":"台"}"#
        );
        assert_eq!(
            json(Emit::Preedit {
                text: "a".to_owned(),
                caret: 1
            }),
            r#"{"type":"preedit","text":"a","caret":1}"#
        );
        assert_eq!(json(Emit::ClearPreedit), r#"{"type":"clearPreedit"}"#);
        assert_eq!(
            json(Emit::DeleteSurrounding {
                offset: -1,
                count: 1
            }),
            r#"{"type":"deleteSurrounding","offset":-1,"count":1}"#
        );
        assert_eq!(
            json(Emit::LookupTable(LookupTableContent {
                cells: vec![CandidateCellContent::new("台", Some("tâi".to_owned()))],
                labels: vec!["1".to_owned()],
                cursor: 0,
                cursor_visible: true,
                page_size: 9,
                vertical: false,
            })),
            r#"{"type":"lookupTable","cells":[{"text":"台","annotation":"tâi"}],"labels":["1"],"cursor":0,"cursorVisible":true,"pageSize":9,"vertical":false}"#
        );
        assert_eq!(
            json(Emit::ModeChanged),
            r#"{"type":"modeChanged","symbol":"台"}"#
        );
        assert_eq!(
            json(Emit::AnnounceMode),
            r#"{"type":"announceMode","label":"label"}"#
        );
        assert_eq!(
            json(Emit::TpsKeyboardCapTyped(TpsKeyCapIndex { row: 1, cap: 2 })),
            r#"{"type":"tpsKeyboardCapTyped","row":1,"cap":2}"#
        );
    }

    #[test]
    fn the_key_panel_rows_are_the_cores() {
        let rows: serde_json::Value = serde_json::from_str(&tps_rows_json()).unwrap();
        let shared = tps_keyboard_rows();
        assert_eq!(rows.as_array().unwrap().len(), shared.len());
        let first = &shared[0].caps[0];
        assert_eq!(rows[0]["caps"][0]["label"], first.label.to_string());
        assert_eq!(rows[0]["caps"][0]["glyph"], first.glyph);
        assert_eq!(rows[1]["indent"], 0.5);
    }
}
