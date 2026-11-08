//! What the page reads back: one JSON object per call. A wire type of its own
//! rather than `Serialize` on [`Emit`]: the page needs text a shell elsewhere
//! asks the runtime for, and this is the one place that JSON's shape is named.

use serde::Serialize;
use taigi_desktop_core::candidates::{
    CellLayer, ExpandableAxis, ExpandableDisplayMode, ExpandableListModel, ExpandableModeChange,
    ModeTransition, Rect, Track,
};
use taigi_desktop_core::keys::tps_keyboard_rows;
use taigi_desktop_core::session::{Emit, ListSelection, LookupTableContent};

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
        #[serde(skip_serializing_if = "Option::is_none")]
        layout: Option<WireLayout>,
    },
    HideLookupTable,
    ModeChanged {
        symbol: &'a str,
    },
    AnnounceMode {
        label: &'a str,
    },
    /// The settings are the sidebar, so the page opens or closes it.
    OpenSettings,
    ToggleTpsKeyboard,
    TpsKeyboardCapTyped {
        row: usize,
        cap: usize,
    },
}

/// A collapsed cell.
#[derive(Debug, Serialize)]
pub(crate) struct WireCollapsedCell {
    index: usize,
    width: f32,
}

/// A cell of the unfolded grid, in columns.
#[derive(Debug, Serialize)]
pub(crate) struct WireGridCell {
    index: usize,
    start: usize,
    span: usize,
}

/// What `ExpandableListModel` says about the list, in CSS pixels. A list with
/// no `layout` is paged by the page.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WireLayout {
    is_column: bool,
    is_expanded: bool,
    lead_cell_is_unkeyed: bool,
    has_overflow: bool,
    collapsed: Vec<WireCollapsedCell>,
    rows: Vec<Vec<WireGridCell>>,
    column_width: f32,
    item_height: f32,
    row_height: f32,
    window_height: f32,
    grid_height: f32,
    /// The half row a scrolling grid shows under its last full one.
    peek: f32,
    scroll_y: f32,
    /// The grid's column count: six under a row, a page beside a column.
    columns: usize,
    /// Where an unfolded column's annotations start; `None` for a row.
    primary_column: Option<f32>,
    /// The cells the slot keys number, in key order.
    keyed: Vec<usize>,
    /// The frame tinted behind them, in content coordinates.
    keyed_band: Option<WireRect>,
    /// An unfolded column that pages keeps its chevron column on every page;
    /// the chevron shows while another page follows.
    is_paged: bool,
    has_next_page: bool,
    animation_milliseconds: f32,
    /// Only on the reply whose key changed the mode: what to play over it.
    #[serde(skip_serializing_if = "Option::is_none")]
    transition: Option<WireTransition>,
}

#[derive(Debug, Serialize)]
pub(crate) struct WireRect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl From<Rect> for WireRect {
    fn from(rect: Rect) -> Self {
        Self {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
        }
    }
}

/// `Track`: a drawing's frame (content coordinates) and opacity, start and end.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WireTrack {
    from: WireRect,
    to: WireRect,
    from_alpha: f32,
    to_alpha: f32,
}

impl From<Track> for WireTrack {
    fn from(track: Track) -> Self {
        Self {
            from: track.from.into(),
            to: track.to.into(),
            from_alpha: track.from_alpha,
            to_alpha: track.to_alpha,
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct WireCellTrack {
    index: usize,
    /// `collapsed`, `expanded` or `shared` (`CellLayer`).
    layer: &'static str,
    #[serde(flatten)]
    track: WireTrack,
}

/// `ModeTransition`, oriented the way it plays, less its still cells.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WireTransition {
    is_expanding: bool,
    cells: Vec<WireCellTrack>,
    chevron: WireTrack,
}

impl From<&ModeTransition> for WireTransition {
    fn from(transition: &ModeTransition) -> Self {
        Self {
            is_expanding: transition.change == ExpandableModeChange::Expand,
            // The page draws a still cell where it lays it out: only the moving
            // and fading ones go out, a row's whole list otherwise.
            cells: transition
                .cells
                .iter()
                .filter(|cell| {
                    let track = cell.track;
                    track.from != track.to || track.from_alpha != track.to_alpha
                })
                .map(|cell| WireCellTrack {
                    index: cell.candidate_index,
                    layer: match cell.layer {
                        CellLayer::Collapsed => "collapsed",
                        CellLayer::Expanded => "expanded",
                        CellLayer::Shared => "shared",
                    },
                    track: cell.track.into(),
                })
                .collect(),
            chevron: transition.chevron.into(),
        }
    }
}

pub(crate) fn wire_layout(selection: &ListSelection) -> Option<WireLayout> {
    let model = selection.expandable()?;
    let geometry = model.expanded_geometry();
    let is_expanded = model.mode() == ExpandableDisplayMode::Expanded;
    let grid_height =
        model.grid().rows.len() as f32 * model.row_height() - ExpandableListModel::SEPARATOR_HEIGHT;
    Some(WireLayout {
        is_column: model.axis() == ExpandableAxis::Column,
        is_expanded,
        lead_cell_is_unkeyed: selection.lead_cell_is_unkeyed(),
        has_overflow: model.has_overflow(),
        collapsed: model
            .collapsed_cells()
            .iter()
            .map(|cell| WireCollapsedCell {
                index: cell.candidate_index,
                width: cell.rect.width,
            })
            .collect(),
        // Only an unfolded window draws them, and this goes out on every key.
        rows: if is_expanded {
            model
                .grid()
                .rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|cell| WireGridCell {
                            index: cell.candidate_index,
                            start: cell.column_start,
                            span: cell.column_span,
                        })
                        .collect()
                })
                .collect()
        } else {
            Vec::new()
        },
        column_width: model.column_width(),
        item_height: model.item_height(),
        row_height: model.row_height(),
        window_height: geometry.window_height,
        grid_height,
        peek: geometry.container_height - grid_height,
        scroll_y: model.scroll_y(),
        columns: model.grid().column_count,
        primary_column: model.expanded_primary_column_width(),
        keyed: (0..)
            .map_while(|slot| model.candidate_index_for_slot(slot))
            .collect(),
        keyed_band: model.keyed_band_rect().map(WireRect::from),
        is_paged: model.is_paged(),
        has_next_page: model.has_next_page(),
        animation_milliseconds: ExpandableListModel::ANIMATION_DURATION_SECONDS * 1000.0,
        transition: model.mode_transition().map(WireTransition::from),
    })
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

/// `layout` goes on a `LookupTable`, and nothing else has one.
pub(crate) fn wire_emit<'a>(
    emit: &'a Emit,
    mode: &'a ModeText,
    layout: Option<WireLayout>,
) -> WireEmit<'a> {
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
            layout,
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
        serde_json::to_string(&wire_emit(&emit, &mode, None)).unwrap()
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
