//! Where each line of a page sits. A horizontal page stacks a two-script
//! cell: the two lines share one center, with the script gap between them.
//! The leading line takes the primary size and the other line the secondary
//! size; a one-script cell takes the primary size. The typeface stays with
//! the script. A vertical page keeps them on one line, the annotation after
//! the text, and puts every select key in one column, centered, so the text
//! of each row starts at the same x. The window multiplies every px by its
//! scale and then asks a [`Measure`].

use std::borrow::Cow;

use crate::protocol::{Cell, Show};

/// What one line occupies. `px` is the size it was measured at.
#[derive(Clone, Debug, PartialEq)]
pub struct Glyph {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub px: f32,
    pub role: Role,
    /// The line is Hanji script, so it takes the Hanji typeface. The size is
    /// separate: `px` is the primary or secondary size. Labels and arrows
    /// are not Hanji.
    pub hanji: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Text,
    Annotation,
    Label,
    Arrow,
}

/// A clickable rectangle, in the same coordinates as the glyphs.
#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub target: Target,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Cell(u32),
    Page { forward: bool },
}

/// The page, ready to paint. `width` and `height` include the outer pad.
#[derive(Clone, Debug, PartialEq)]
pub struct Arrangement {
    pub width: f32,
    pub height: f32,
    pub highlight: Option<Rect>,
    pub glyphs: Vec<Glyph>,
    pub hits: Vec<Hit>,
    /// The first row a vertical list shows when the whole page is taller
    /// than [`Fit::height`]; 0 otherwise. Hand it back as the next
    /// [`Fit::first_row`] so the rows scroll only when the cursor leaves them.
    pub first_row: usize,
}

/// The room the window has on screen, in px after scale: the caret's monitor
/// less a margin, and for the height the roomier side of the caret. A cell
/// wider than the room ends in an ellipsis; a vertical list taller than it
/// shows the rows around the cursor, from `first_row` when the cursor is
/// among them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fit {
    pub width: f32,
    pub height: f32,
    pub first_row: usize,
}

impl Fit {
    pub const UNBOUNDED: Self = Self {
        width: f32::INFINITY,
        height: f32::INFINITY,
        first_row: 0,
    };
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// The width and height of `text` at `px`. `hanji` picks the typeface.
pub trait Measure {
    fn size(&self, text: &str, px: f32, hanji: bool) -> (f32, f32);

    /// Distance from the top of [`size`](Measure::size)'s box down to the
    /// baseline. Side-by-side scripts share this line; the default is the
    /// bottom of the box.
    fn baseline(&self, text: &str, px: f32, hanji: bool) -> f32 {
        self.size(text, px, hanji).1
    }
}

/// The air around the text for a window pad of `padding_px`. At 8 the five
/// numbers are 8 / 6 / 8 / 2 / 8. `script` is the gap between the two scripts
/// of a horizontal cell; it used to be `line`, which left them almost flush.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pads {
    pub window: f32,
    pub cell: f32,
    pub gap: f32,
    pub line: f32,
    pub script: f32,
}

pub fn pads(padding_px: u32, scale: f32) -> Pads {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let window = padding_px as f32 * scale;
    Pads {
        window,
        cell: window * 0.75,
        gap: window,
        line: window * 0.25,
        script: window,
    }
}

const LABEL_PX: f32 = 13.0;

/// `value` in whole pixels, at least one. The window and the settings
/// sample both size the buffer with this, so the two stay the same size.
pub fn px(value: f32) -> u32 {
    value.round().clamp(1.0, 16_000.0) as u32
}

/// The page settings a cell's width depends on, in px before scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellStyle {
    pub hanji_px: u32,
    pub roman_px: u32,
    pub hanji_leads: bool,
    pub padding: u32,
}

/// The widths a horizontal row draws its cells at, in px before scale. The
/// row packer (`taigi-linux-core` `PopupListLayout`) packs these, and
/// [`arrange`] draws every row cell at its width less the gap — one number on
/// both sides, so a packed row fills the window it was packed to.
#[derive(Clone, Debug, PartialEq)]
pub struct RowWidths {
    /// One per cell, the gap after it included.
    pub widths: Vec<f32>,
    /// A short one-Hanji cell, gap included: no cell is narrower.
    pub slot: f32,
    pub gap: f32,
}

/// Measures `cells` as a horizontal row draws them: each reserves the widest
/// select key (`KEY_LABEL_SAMPLES`), is floored at the slot, and is whole px.
pub fn row_widths(cells: &[Cell], cell_style: CellStyle, measure: &impl Measure) -> RowWidths {
    let row = RowCell::new(cell_style, measure, 1.0);
    RowWidths {
        widths: cells.iter().map(|cell| row.width(cell, measure)).collect(),
        slot: row.slot,
        gap: row.style.gap,
    }
}

/// What a horizontal row's window adds around its cells, in px before scale:
/// the pad on both sides and the page-arrow column. The session subtracts it
/// from the screen to know how wide a row of cells may pack.
pub fn row_chrome_width(padding_px: u32, measure: &impl Measure) -> f32 {
    let Pads { window, gap, .. } = pads(padding_px, 1.0);
    window * 2.0 + gap + arrow_span(LABEL_PX, measure)
}

/// A select key wider than any digit: every row cell reserves the widest key
/// any slot set prints, so a cell keeps its width whatever key it carries.
const KEY_LABEL_SAMPLES: [&str; 4] = ["8", "w", "m", ";"];

fn key_column(label_px: f32, measure: &impl Measure) -> f32 {
    KEY_LABEL_SAMPLES
        .iter()
        .map(|label| measure.size(label, label_px, false).0)
        .fold(0.0_f32, f32::max)
}

/// How wide a horizontal row cell is drawn. Counted in whole px before scale,
/// as the packer counts it, then scaled — so the window's scale never makes
/// the drawn row narrower than the width the session packed it to.
struct RowCell {
    /// The page's sizes before scale.
    style: Scale,
    /// The key column every row cell reserves, before scale.
    key: f32,
    /// A short one-Hanji cell, gap included, before scale.
    slot: f32,
    scale: f32,
}

impl RowCell {
    fn new(cell_style: CellStyle, measure: &impl Measure, scale: f32) -> Self {
        let style = Scale::of(cell_style, 1.0);
        let key = key_column(style.label_px, measure);
        let short = Cell {
            text: "字".into(),
            annotation: Some("lâi".into()),
        };
        let short_w = layout_cell(&short, "", 0, false, key, &style, measure).w;
        Self {
            slot: (short_w + style.gap).ceil(),
            style,
            key,
            scale,
        }
    }

    /// `cell`'s width before scale, the gap after it included.
    fn width(&self, cell: &Cell, measure: &impl Measure) -> f32 {
        let laid = layout_cell(cell, "", 0, false, self.key, &self.style, measure);
        (laid.w + self.style.gap).ceil().max(self.slot)
    }

    /// Widens `laid` (`cell` laid out at scale) to its row width, less the
    /// gap, never past `max_w`. The key stays left; the scripts stay centered
    /// in what it leaves.
    fn widen(&self, laid: &mut LaidCell, cell: &Cell, measure: &impl Measure, max_w: f32) {
        let target = ((self.width(cell, measure) - self.style.gap) * self.scale).min(max_w);
        let extra = target - laid.w;
        if extra <= 0.0 {
            return;
        }
        for glyph in &mut laid.glyphs {
            if glyph.role != Role::Label {
                glyph.x += extra / 2.0;
            }
        }
        laid.w = target;
        laid.hit.w = target;
    }
}

/// Lays `page` out at `scale`, inside `fit`. Sizes on the page are px before
/// scale.
pub fn arrange(page: &Show, measure: &impl Measure, scale: f32, fit: Fit) -> Arrangement {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    if page.expandable.as_ref().is_some_and(|view| view.mode == 1) {
        return arrange_expandable(page, measure, scale);
    }
    let Pads {
        window: pad,
        cell: cell_pad,
        gap,
        line: line_gap,
        ..
    } = pads(page.chrome.padding, scale);
    let mut style = Scale::of(cell_style(page), scale);
    let label_px = style.label_px;
    // The column a page arrow takes beside the cells: a paged plain row keeps
    // it on every page (so the first and last pages are as wide as the middle
    // ones), a collapsed vertical-expandable list shows its `›` there.
    let min_w = page.min_content_width as f32 * scale;
    let arrow_column = if page.vertical {
        page.expandable.is_some() && page.has_next
    } else {
        page.has_prev || page.has_next || (min_w > 0.0 && page.expandable.is_none())
    };
    let arrow_w = if arrow_column {
        gap + arrow_span(label_px, measure)
    } else {
        0.0
    };
    style.max_cell_w = (fit.width - pad * 2.0 - arrow_w).max(0.0);
    // Vertical rows share one key column. A horizontal cell is drawn at the
    // width the row was packed to (`row_widths`), so the cells fill the
    // window the session sized from them.
    let row = (!page.vertical).then(|| RowCell::new(cell_style(page), measure, scale));
    let label_slot = if row.is_some() {
        key_column(label_px, measure)
    } else {
        label_column(page, label_px, measure)
    };
    let mut cells = Vec::with_capacity(page.cells.len());
    for (index, cell) in page.cells.iter().enumerate() {
        let label = page.labels.get(index).map(String::as_str).unwrap_or("");
        let mut laid = layout_cell(
            cell,
            label,
            index as u32,
            page.vertical,
            label_slot,
            &style,
            measure,
        );
        if let Some(row) = &row {
            row.widen(&mut laid, cell, measure, style.max_cell_w);
        }
        cells.push(laid);
    }

    let mut glyphs = Vec::new();
    let mut hits = Vec::new();
    let mut highlight = None;
    let content_h = cells.iter().map(|cell| cell.h).fold(0.0_f32, f32::max);
    let mut x = pad;
    let mut y = pad;
    let row_h = content_h;

    if !page.vertical {
        // Like the macOS window, the only paging chrome is one trailing
        // up/down column — the Up / Down keys turn the page.
        let arrow_h = row_h.max(label_px + cell_pad * 2.0);
        let cells_left = x;
        for cell in &cells {
            let top = y + (row_h - cell.h).max(0.0) / 2.0;
            shift_cell(cell, x, top, &mut glyphs, &mut hits);
            if page.cursor == Some(cell.index) {
                // The full row height, so a romanization-only cell lights
                // up as tall as its two-line neighbours.
                highlight = Some(Rect {
                    x,
                    y,
                    w: cell.w,
                    h: row_h,
                });
            }
            x += cell.w + gap;
        }
        if x > cells_left {
            x -= gap;
        }
        // `min_content_width` is the cells' own extent; the arrows sit after.
        if x - cells_left < min_w {
            x = cells_left + min_w.min(style.max_cell_w);
        }
        if arrow_column {
            x += gap;
            place_page_column(
                &mut glyphs,
                &mut hits,
                (page.has_prev, page.has_next),
                (x, y),
                arrow_h,
                &style,
                measure,
            );
            x += arrow_span(label_px, measure);
        }
        let width = x + pad;
        let height = y + row_h.max(label_px) + pad;
        let mut arrangement = Arrangement {
            width,
            height,
            highlight: highlight.map(|cell| {
                grow_highlight(
                    cell,
                    horizontal_limits(cell, width, height, pad, gap),
                    page.chrome.highlight_fill,
                )
            }),
            glyphs,
            hits,
            first_row: 0,
        };
        remap_expandable_hits(page, &mut arrangement.hits);
        arrangement
    } else {
        // Twice the stacked-script line gap, so vertical rows are not flush.
        let row_gap = line_gap * 2.0;
        let mut width = cells.iter().map(|cell| cell.w).fold(0.0_f32, f32::max);
        // A plain paged list keeps its arrow row under the cells.
        let arrow_row = page.expandable.is_none() && (page.has_prev || page.has_next);
        let arrow_row_h = if arrow_row { label_px + cell_pad } else { 0.0 };
        let heights: Vec<f32> = cells.iter().map(|cell| cell.h).collect();
        let cursor_row = page
            .cursor
            .and_then(|cursor| cells.iter().position(|cell| cell.index == cursor));
        let (first_row, row_count) = visible_rows(
            &heights,
            row_gap,
            fit.height - pad * 2.0 - arrow_row_h,
            cursor_row,
            fit.first_row,
        );
        for cell in &cells[first_row..first_row + row_count] {
            shift_cell(cell, x, y, &mut glyphs, &mut hits);
            if page.cursor == Some(cell.index) {
                highlight = Some(Rect {
                    x,
                    y,
                    w: width.max(cell.w),
                    h: cell.h,
                });
            }
            y += cell.h + row_gap;
        }
        // A collapsed vertical-expandable list shows its overflow as a `›`
        // column right of the cells — the unfolded grid's slot, and the
        // macOS / Windows chevron column — never as a row under them, so the
        // window keeps its height when it unfolds.
        let collapsed_expandable = page.expandable.is_some();
        let mut fill_right = None;
        if collapsed_expandable {
            if y > pad {
                y -= row_gap;
            }
            if page.has_next {
                let arrow_h = (y - pad).max(label_px);
                place_arrow(
                    &mut glyphs,
                    &mut hits,
                    true,
                    (x + width + gap, pad),
                    arrow_h,
                    &style,
                    measure,
                );
                // Halfway across the gap, as a collapsed row stops before
                // its arrow column.
                fill_right = Some(x + width + gap * 0.5);
                width += gap + arrow_span(label_px, measure);
                y = pad + arrow_h;
            }
        } else if page.has_prev || page.has_next {
            let mut ax = x;
            if page.has_prev {
                place_arrow(
                    &mut glyphs,
                    &mut hits,
                    false,
                    (ax, y),
                    label_px + cell_pad,
                    &style,
                    measure,
                );
                ax += arrow_span(label_px, measure) + gap;
            }
            if page.has_next {
                place_arrow(
                    &mut glyphs,
                    &mut hits,
                    true,
                    (ax, y),
                    label_px + cell_pad,
                    &style,
                    measure,
                );
                ax += arrow_span(label_px, measure);
            }
            width = width.max(ax - x);
            y += label_px + cell_pad;
        } else if y > pad {
            y -= row_gap;
        }
        let window_w = x + width + pad;
        let window_h = y + pad;
        let mut arrangement = Arrangement {
            width: window_w,
            height: window_h,
            highlight: highlight.map(|cell| {
                grow_highlight(
                    cell,
                    vertical_limits(cell, fill_right.unwrap_or(window_w), window_h, pad, row_gap),
                    page.chrome.highlight_fill,
                )
            }),
            glyphs,
            hits,
            first_row,
        };
        remap_expandable_hits(page, &mut arrangement.hits);
        arrangement
    }
}

/// Which rows of a vertical list fit `room`: `(first, count)`. All of them
/// when they fit; else from `previous` while the cursor stays among the rows
/// shown, scrolled just far enough to bring it back when it leaves them.
fn visible_rows(
    heights: &[f32],
    row_gap: f32,
    room: f32,
    cursor: Option<usize>,
    previous: usize,
) -> (usize, usize) {
    let fitting_from = |first: usize| {
        let mut used = 0.0_f32;
        let mut count = 0;
        for height in &heights[first..] {
            let next = used
                + if count == 0 {
                    *height
                } else {
                    row_gap + height
                };
            if count > 0 && next > room {
                break;
            }
            used = next;
            count += 1;
        }
        count
    };
    if heights.is_empty() || fitting_from(0) == heights.len() {
        return (0, heights.len());
    }
    let mut first = previous.min(heights.len() - 1);
    if let Some(cursor) = cursor {
        if cursor < first {
            first = cursor;
        }
        while first < cursor && first + fitting_from(first) <= cursor {
            first += 1;
        }
    }
    (first, fitting_from(first))
}

fn remap_expandable_hits(page: &Show, hits: &mut [Hit]) {
    let Some(view) = &page.expandable else {
        return;
    };
    for hit in hits {
        if let Target::Cell(index) = hit.target {
            if let Some(&absolute) = view.absolute_indices.get(index as usize) {
                hit.target = Target::Cell(absolute);
            }
        }
    }
}

fn arrange_expandable(page: &Show, measure: &impl Measure, scale: f32) -> Arrangement {
    let view = page.expandable.as_ref().expect("expanded");
    let Pads {
        window: pad, gap, ..
    } = pads(page.chrome.padding, scale);
    let style = Scale::of(cell_style(page), scale);
    let label_px = style.label_px;
    let primary = style.primary;
    // A column keeps the two scripts on one line. A row stacks them.
    let inline = view.axis == 1;
    // Only the columns a cell reaches: a trailing empty column (the last
    // columns of the list) would leave a strip past the highlight's reach.
    let columns = (0..page.cells.len())
        .map(|index| {
            let start = view
                .column_starts
                .get(index)
                .copied()
                .unwrap_or(index as u32 % view.grid_columns.max(1));
            start + view.column_spans.get(index).copied().unwrap_or(1).max(1)
        })
        .max()
        .unwrap_or(1)
        .clamp(1, view.grid_columns.max(1)) as usize;
    if !inline {
        return arrange_expanded_row(page, &style, measure, scale);
    }
    // Every vertical-expandable cell reserves the key column, including rows
    // that are not numbered yet, so a column does not jump when its numbers
    // appear.
    let label_slot = label_column(page, label_px, measure);
    let mut cells = Vec::new();
    for (index, cell) in page.cells.iter().enumerate() {
        let label = page.labels.get(index).map(String::as_str).unwrap_or("");
        cells.push(layout_cell(
            cell,
            label,
            index as u32,
            inline,
            label_slot,
            &style,
            measure,
        ));
    }
    let span_of = |index: usize| view.column_spans.get(index).copied().unwrap_or(1).max(1) as usize;
    let start_of = |index: usize| {
        view.column_starts
            .get(index)
            .copied()
            .unwrap_or((index % columns) as u32) as usize
    };
    // Each vertical-expandable column is as wide as its own widest cell.
    let mut column_widths = vec![0.0_f32; columns];
    for (index, cell) in cells.iter().enumerate() {
        let start = start_of(index).min(columns.saturating_sub(1));
        let span = span_of(index).min(columns - start).max(1);
        if span == 1 {
            column_widths[start] = column_widths[start].max(cell.w);
        }
    }
    for (index, cell) in cells.iter().enumerate() {
        let start = start_of(index).min(columns.saturating_sub(1));
        let span = span_of(index).min(columns - start).max(1);
        if span == 1 {
            continue;
        }
        let covered: f32 = column_widths[start..start + span].iter().sum();
        if cell.w > covered {
            let extra = (cell.w - covered) / span as f32;
            for width in &mut column_widths[start..start + span] {
                *width += extra;
            }
        }
    }
    for width in &mut column_widths {
        *width = width.max(1.0);
    }
    let mut column_x = Vec::with_capacity(columns + 1);
    column_x.push(0.0);
    for width in &column_widths {
        column_x.push(column_x.last().copied().unwrap_or(0.0) + width);
    }
    let row_body = cells.iter().map(|cell| cell.h).fold(primary, f32::max);
    // The collapsed vertical list's row gap, so rows keep their spacing when
    // the column unfolds.
    let separator = pads(page.chrome.padding, scale).line * 2.0;
    let pitch = row_body + separator;
    let row_count = cells
        .iter()
        .enumerate()
        .map(|(index, _)| {
            view.row_indexes
                .get(index)
                .copied()
                .unwrap_or((index / columns) as u32)
        })
        .max()
        .map(|row| row as usize + 1)
        .unwrap_or(0)
        .max(1);
    let visible = if view.visible_rows == 0 {
        row_count
    } else {
        (view.visible_rows as usize).clamp(1, row_count)
    };
    let scroll_rows = if view.row_height > 0 {
        (view.scroll_y as f32 / view.row_height as f32).round() as usize
    } else {
        0
    };
    let scroll = scroll_rows as f32 * pitch;
    // A row peeks the next line so the list reads as scrollable. A column
    // keeps the height it had before it unfolded.
    let peek = if view.axis == 0 && row_count > visible {
        row_body * 0.5
    } else {
        0.0
    };
    let mut glyphs = Vec::new();
    let mut hits = Vec::new();
    let mut highlight = None;
    let grid_bottom =
        pad + visible as f32 * row_body + visible.saturating_sub(1) as f32 * separator + peek;
    for (index, cell) in cells.iter().enumerate() {
        let start = start_of(index).min(columns.saturating_sub(1));
        let span = span_of(index).min(columns - start).max(1);
        let row = view
            .row_indexes
            .get(index)
            .copied()
            .unwrap_or((index / columns) as u32) as f32;
        let x = pad + column_x[start];
        let slot_w = column_x[start + span] - column_x[start];
        let row_top = pad + row * pitch - scroll;
        let top = row_top + (row_body - cell.h).max(0.0) / 2.0;
        // Whole rows: at zero padding a centred-cell test would let the row
        // just outside the viewport draw (and take clicks) at its edge.
        if row_top + row_body <= pad || row_top >= grid_bottom {
            continue;
        }
        shift_cell(cell, x, top, &mut glyphs, &mut hits);
        // The whole row, not the cell's own height, so a romanization-only
        // cell lights up (and takes clicks) as tall as its Hanji neighbours —
        // the unfolded row's rule.
        let band = Rect {
            x,
            y: row_top,
            w: slot_w.max(cell.w),
            h: row_body,
        };
        if let Some(hit) = hits.last_mut() {
            if hit.target == Target::Cell(index as u32) {
                *hit = Hit {
                    x: band.x,
                    y: band.y,
                    w: band.w,
                    h: band.h,
                    target: hit.target,
                };
            }
        }
        if page.cursor == Some(index as u32) {
            highlight = Some(band);
        }
    }
    let height = grid_bottom + pad;
    let mut width = pad * 2.0 + column_x[columns];
    if page.has_next {
        let arrow_w = arrow_span(label_px, measure);
        place_arrow(
            &mut glyphs,
            &mut hits,
            true,
            (width - pad + gap, pad),
            (height - pad * 2.0).max(label_px),
            &style,
            measure,
        );
        width += gap + arrow_w;
    }
    let mut arrangement = Arrangement {
        width,
        height,
        highlight: highlight.map(|cell| {
            let content = Rect {
                x: pad,
                y: pad,
                w: column_x[columns],
                h: grid_bottom - pad,
            };
            grow_highlight(
                cell,
                grid_limits(cell, content, pad, 0.0, separator),
                page.chrome.highlight_fill,
            )
        }),
        glyphs,
        hits,
        first_row: 0,
    };
    remap_expandable_hits(page, &mut arrangement.hits);
    arrangement
}

/// An unfolded row keeps each cell's own width — the width the grid packed
/// (`row_widths`) — and wraps left. Stretching to a column grid is what made
/// some candidates look twice as wide as they are.
fn arrange_expanded_row(
    page: &Show,
    style: &Scale,
    measure: &impl Measure,
    scale: f32,
) -> Arrangement {
    let view = page.expandable.as_ref().expect("expanded");
    let Pads {
        window: pad, gap, ..
    } = pads(page.chrome.padding, scale);
    // Every cell reserves the key column, numbered or not, so a row does not
    // shift right when the keys move onto it.
    let row = RowCell::new(cell_style(page), measure, scale);
    let key = key_column(style.label_px, measure);
    let mut cells = Vec::new();
    for (index, cell) in page.cells.iter().enumerate() {
        let label = page.labels.get(index).map(String::as_str).unwrap_or("");
        let mut laid = layout_cell(cell, label, index as u32, false, key, style, measure);
        row.widen(&mut laid, cell, measure, f32::INFINITY);
        cells.push(laid);
    }
    let row_body = cells
        .iter()
        .map(|cell| cell.h)
        .fold(style.primary, f32::max);
    let separator = scale;
    let pitch = row_body + separator;
    // The model decides the rows (navigation and keys read them); each row
    // here only lays its cells end to end from the left.
    let row_of = |index: usize| view.row_indexes.get(index).copied().unwrap_or(0) as usize;
    let scroll_rows = if view.row_height > 0 {
        (view.scroll_y as f32 / view.row_height as f32).round() as usize
    } else {
        0
    };
    let scroll = scroll_rows as f32 * pitch;
    let mut glyphs = Vec::new();
    let mut hits = Vec::new();
    let mut highlight = None;
    let mut widest_row = 0.0_f32;
    let mut x = pad;
    let mut row = usize::MAX;
    for (index, cell) in cells.iter().enumerate() {
        if row_of(index) != row {
            row = row_of(index);
            x = pad;
        }
        let top = pad + row as f32 * pitch - scroll + (row_body - cell.h).max(0.0) / 2.0;
        shift_cell(cell, x, top, &mut glyphs, &mut hits);
        if page.cursor == Some(index as u32) {
            // The row's full body, as in the collapsed row.
            highlight = Some(Rect {
                x,
                y: pad + row as f32 * pitch - scroll,
                w: cell.w,
                h: row_body,
            });
        }
        widest_row = widest_row.max(x + cell.w - pad);
        x += cell.w + gap;
    }
    let wrap = widest_row.max(page.min_content_width as f32 * scale);
    let row_count = cells
        .iter()
        .enumerate()
        .map(|(index, _)| row_of(index) + 1)
        .max()
        .unwrap_or(1);
    let visible = if view.visible_rows == 0 {
        row_count
    } else {
        (view.visible_rows as usize).clamp(1, row_count)
    };
    let peek = if row_count > visible {
        row_body * 0.5
    } else {
        0.0
    };
    let height =
        pad + visible as f32 * row_body + visible.saturating_sub(1) as f32 * separator + peek + pad;
    let width = pad * 2.0 + wrap;
    let mut arrangement = Arrangement {
        width,
        height,
        highlight: highlight.map(|cell| {
            let content = Rect {
                x: pad,
                y: pad,
                w: wrap,
                h: height - pad * 2.0,
            };
            grow_highlight(
                cell,
                grid_limits(cell, content, pad, gap, separator),
                page.chrome.highlight_fill,
            )
        }),
        glyphs,
        hits,
        first_row: 0,
    };
    remap_expandable_hits(page, &mut arrangement.hits);
    arrangement
}

/// `fill` is 0–100. `0` leaves `cell`. `100` is `limits`, which meets the
/// window edge on the outer sides and stops halfway to a neighbour.
fn grow_highlight(cell: Rect, limits: Rect, fill: u32) -> Rect {
    let t = (fill.min(100) as f32) / 100.0;
    if t == 0.0 {
        return cell;
    }
    let right = cell.x + cell.w;
    let bottom = cell.y + cell.h;
    let x = cell.x + (limits.x - cell.x) * t;
    let y = cell.y + (limits.y - cell.y) * t;
    let x2 = right + ((limits.x + limits.w) - right) * t;
    let y2 = bottom + ((limits.y + limits.h) - bottom) * t;
    Rect {
        x,
        y,
        w: (x2 - x).max(0.0),
        h: (y2 - y).max(0.0),
    }
}

fn horizontal_limits(cell: Rect, width: f32, height: f32, pad: f32, gap: f32) -> Rect {
    let right_edge = cell.x + cell.w;
    let left = if cell.x <= pad + 0.5 {
        0.0
    } else {
        (cell.x - gap * 0.5).max(0.0)
    };
    let right = if right_edge >= width - pad - 0.5 {
        width
    } else {
        (right_edge + gap * 0.5).min(width)
    };
    Rect {
        x: left,
        y: 0.0,
        w: (right - left).max(0.0),
        h: height,
    }
}

/// An expanded cell's fill limits: a side on the edge of `content` meets the
/// window edge (`pad` beyond it), any other side stops halfway across the gap
/// to its neighbour (`col_gap` across, `row_gap` down) — the collapsed rules
/// applied on both axes.
fn grid_limits(cell: Rect, content: Rect, pad: f32, col_gap: f32, row_gap: f32) -> Rect {
    let near = |a: f32, b: f32| (a - b).abs() <= 0.5;
    let left = if near(cell.x, content.x) || cell.x < content.x {
        content.x - pad
    } else {
        cell.x - col_gap * 0.5
    };
    let right_edge = cell.x + cell.w;
    let right = if right_edge >= content.x + content.w - 0.5 {
        content.x + content.w + pad
    } else {
        right_edge + col_gap * 0.5
    };
    let top = if cell.y <= content.y + 0.5 {
        content.y - pad
    } else {
        cell.y - row_gap * 0.5
    };
    let bottom_edge = cell.y + cell.h;
    let bottom = if bottom_edge >= content.y + content.h - 0.5 {
        content.y + content.h + pad
    } else {
        bottom_edge + row_gap * 0.5
    };
    Rect {
        x: left.max(0.0),
        y: top.max(0.0),
        w: (right - left.max(0.0)).max(0.0),
        h: (bottom - top.max(0.0)).max(0.0),
    }
}

/// A vertical row spans from the window's left edge to `right` (the window
/// edge, or halfway to an arrow column). The top and bottom meet the outer
/// edge, or stop halfway across `line_gap` when another row is there.
fn vertical_limits(cell: Rect, right: f32, height: f32, pad: f32, line_gap: f32) -> Rect {
    let bottom_edge = cell.y + cell.h;
    let top = if cell.y <= pad + 0.5 {
        0.0
    } else {
        (cell.y - line_gap * 0.5).max(0.0)
    };
    let bottom = if bottom_edge >= height - pad - 0.5 {
        height
    } else {
        (bottom_edge + line_gap * 0.5).min(height)
    };
    Rect {
        x: 0.0,
        y: top,
        w: right,
        h: (bottom - top).max(0.0),
    }
}

#[derive(Clone, Copy)]
struct Scale {
    primary: f32,
    secondary: f32,
    hanji_leads: bool,
    label_px: f32,
    cell_pad: f32,
    gap: f32,
    /// Between the two scripts of a stacked (horizontal) cell.
    script_gap: f32,
    /// Between the text and the annotation of a one-line (vertical) cell.
    annotation_gap: f32,
    /// The widest a cell may be drawn; past it the scripts end in an
    /// ellipsis. Unbounded unless the window's [`Fit`] says otherwise.
    max_cell_w: f32,
}

/// The candidate size the window pads are tuned at (`hanjiFontSize`'s
/// default). Above it the gaps between the two scripts grow with the text,
/// as `candidate_annotation_gap` does on macOS and Windows.
const BASE_TEXT_PX: f32 = 16.0;

impl Scale {
    fn of(cell_style: CellStyle, scale: f32) -> Self {
        let Pads {
            cell: cell_pad,
            gap,
            script: script_gap,
            ..
        } = pads(cell_style.padding, scale);
        let text_ratio =
            (cell_style.hanji_px.max(cell_style.roman_px) as f32 / BASE_TEXT_PX).max(1.0);
        Self {
            primary: cell_style.hanji_px.max(1) as f32 * scale,
            secondary: cell_style.roman_px.max(1) as f32 * scale,
            hanji_leads: cell_style.hanji_leads,
            label_px: LABEL_PX * scale,
            cell_pad,
            gap,
            script_gap: script_gap * text_ratio,
            annotation_gap: gap * text_ratio,
            max_cell_w: f32::INFINITY,
        }
    }
}

/// The settings `page`'s cells are sized by.
fn cell_style(page: &Show) -> CellStyle {
    CellStyle {
        hanji_px: page.hanji_px,
        roman_px: page.roman_px,
        hanji_leads: page.hanji_leads,
        padding: page.chrome.padding,
    }
}

struct LaidCell {
    index: u32,
    w: f32,
    h: f32,
    glyphs: Vec<Glyph>,
    hit: Hit,
}

/// The shared select-key column on a vertical page: the widest key on the
/// page. An empty key still reserves this width, so the text column lines up.
fn label_column(page: &Show, label_px: f32, measure: &impl Measure) -> f32 {
    page.labels
        .iter()
        .take(page.cells.len())
        .filter(|label| !label.is_empty())
        .map(|label| measure.size(label, label_px, false).0)
        .fold(0.0_f32, f32::max)
}

/// Lays one cell out. `slot` is the key column the cell reserves
/// ([`key_column`] in a row, [`label_column`] in a column); the key is
/// centered inside it.
fn layout_cell(
    cell: &Cell,
    label: &str,
    index: u32,
    vertical: bool,
    slot: f32,
    style: &Scale,
    measure: &impl Measure,
) -> LaidCell {
    let Scale {
        primary,
        secondary,
        hanji_leads,
        label_px,
        cell_pad,
        gap,
        script_gap,
        annotation_gap,
        max_cell_w,
    } = *style;
    let (label_w, label_h) = if label.is_empty() {
        (0.0, 0.0)
    } else {
        measure.size(label, label_px, false)
    };
    let annotation = cell.annotation.as_deref().filter(|text| !text.is_empty());
    let paired = annotation.is_some();
    let text_hanji = hanji_script(&cell.text, true, paired, hanji_leads);
    let text_px = if primary_line(true, paired) {
        primary
    } else {
        secondary
    };
    let (text_w, text_h) = measure.size(&cell.text, text_px, text_hanji);
    let (ann_px, ann_hanji, ann_w, ann_h) = match annotation {
        Some(text) => {
            let hanji = hanji_script(text, false, true, hanji_leads);
            let px = if primary_line(false, true) {
                primary
            } else {
                secondary
            };
            let (w, h) = measure.size(text, px, hanji);
            (px, hanji, w, h)
        }
        None => (0.0, false, 0.0, 0.0),
    };
    let label_gap = if slot > 0.0 { gap } else { 0.0 };
    // A cell wider than `max_cell_w` keeps one line per script and ends it in
    // an ellipsis (macOS `byTruncatingTail`): a stacked pair cuts each line to
    // the room, a one-line row lets the annotation give way first.
    let room = max_cell_w - (cell_pad * 2.0 + slot + label_gap);
    let (text, text_w, annotation, ann_w) = match annotation {
        Some(ann) if !vertical => {
            let (text, text_w) = truncate(&cell.text, text_px, text_hanji, text_w, room, measure);
            let (ann, ann_w) = truncate(ann, ann_px, ann_hanji, ann_w, room, measure);
            (text, text_w, Some(ann), ann_w)
        }
        Some(ann) => {
            let ann_room = room - text_w - annotation_gap;
            if ann_room >= measure.size(ELLIPSIS, ann_px, ann_hanji).0 {
                let (ann, ann_w) = truncate(ann, ann_px, ann_hanji, ann_w, ann_room, measure);
                (Cow::Borrowed(cell.text.as_str()), text_w, Some(ann), ann_w)
            } else {
                let (text, text_w) =
                    truncate(&cell.text, text_px, text_hanji, text_w, room, measure);
                (text, text_w, None, 0.0)
            }
        }
        None => {
            let (text, text_w) = truncate(&cell.text, text_px, text_hanji, text_w, room, measure);
            (text, text_w, None, 0.0)
        }
    };
    let stacked = !vertical && annotation.is_some();
    let text_base = measure.baseline(&text, text_px, text_hanji);
    let ann_base = match annotation.as_deref() {
        Some(text) => measure.baseline(text, ann_px, ann_hanji),
        None => 0.0,
    };
    let label_base = if label.is_empty() {
        0.0
    } else {
        measure.baseline(label, label_px, false)
    };
    // One line shares a baseline. A stacked pair keeps each line's own box.
    let (ascent, descent) = if stacked {
        (0.0, 0.0)
    } else {
        let mut ascent = text_base;
        let mut descent = (text_h - text_base).max(0.0);
        if annotation.is_some() {
            ascent = ascent.max(ann_base);
            descent = descent.max((ann_h - ann_base).max(0.0));
        }
        if label_w > 0.0 {
            ascent = ascent.max(label_base);
            descent = descent.max((label_h - label_base).max(0.0));
        }
        (ascent, descent)
    };
    let (block_w, block_h) = if stacked {
        (text_w.max(ann_w), text_h + script_gap + ann_h)
    } else if annotation.is_some() {
        (text_w + annotation_gap + ann_w, ascent + descent)
    } else {
        (text_w, ascent + descent)
    };
    let inner_h = if stacked {
        block_h.max(label_h)
    } else {
        ascent + descent
    };
    let w = cell_pad * 2.0 + slot + label_gap + block_w;
    let h = cell_pad * 2.0 + inner_h;
    let mut glyphs = Vec::new();
    if label_w > 0.0 {
        glyphs.push(Glyph {
            text: label.to_owned(),
            x: cell_pad + (slot - label_w) / 2.0,
            y: if stacked {
                cell_pad + (inner_h - label_h) / 2.0
            } else {
                cell_pad + ascent - label_base
            },
            w: label_w,
            h: label_h,
            px: label_px,
            role: Role::Label,
            hanji: false,
        });
    }
    let block_x = cell_pad + slot + label_gap;
    let block_y = cell_pad + (inner_h - block_h) / 2.0;
    if stacked {
        glyphs.push(Glyph {
            text: text.to_string(),
            x: block_x + (block_w - text_w) / 2.0,
            y: block_y,
            w: text_w,
            h: text_h,
            px: text_px,
            role: Role::Text,
            hanji: text_hanji,
        });
        if let Some(text) = annotation.as_deref() {
            glyphs.push(Glyph {
                text: text.to_owned(),
                x: block_x + (block_w - ann_w) / 2.0,
                y: block_y + text_h + script_gap,
                w: ann_w,
                h: ann_h,
                px: ann_px,
                role: Role::Annotation,
                hanji: ann_hanji,
            });
        }
    } else {
        glyphs.push(Glyph {
            text: text.to_string(),
            x: block_x,
            y: cell_pad + ascent - text_base,
            w: text_w,
            h: text_h,
            px: text_px,
            role: Role::Text,
            hanji: text_hanji,
        });
        if let Some(text) = annotation.as_deref() {
            glyphs.push(Glyph {
                text: text.to_owned(),
                x: block_x + text_w + annotation_gap,
                y: cell_pad + ascent - ann_base,
                w: ann_w,
                h: ann_h,
                px: ann_px,
                role: Role::Annotation,
                hanji: ann_hanji,
            });
        }
    }
    LaidCell {
        index,
        w,
        h,
        glyphs,
        hit: Hit {
            x: 0.0,
            y: 0.0,
            w,
            h,
            target: Target::Cell(index),
        },
    }
}

const ELLIPSIS: &str = "…";

/// `text` (`width` wide at `px`) cut at the end to fit `room`, an ellipsis
/// after what is left, and its new width. A cut never separates a letter from
/// its combining mark (POJ o͘, a decomposed tone mark), and drops the space
/// it would leave before the ellipsis. Nothing fits: the ellipsis alone.
fn truncate<'a>(
    text: &'a str,
    px: f32,
    hanji: bool,
    width: f32,
    room: f32,
    measure: &impl Measure,
) -> (Cow<'a, str>, f32) {
    if width <= room {
        return (Cow::Borrowed(text), width);
    }
    let cuts: Vec<usize> = text
        .char_indices()
        .filter(|&(index, scalar)| index > 0 && !is_combining(scalar))
        .map(|(index, _)| index)
        .collect();
    let shortened = |cut: usize| {
        let candidate = format!("{}{ELLIPSIS}", text[..cut].trim_end());
        let width = measure.size(&candidate, px, hanji).0;
        (candidate, width)
    };
    // The longest prefix that fits: widths grow with the prefix.
    let (mut low, mut high) = (0, cuts.len());
    while low < high {
        let middle = (low + high).div_ceil(2);
        if shortened(cuts[middle - 1]).1 <= room {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    let (candidate, width) = if low == 0 {
        (ELLIPSIS.to_owned(), measure.size(ELLIPSIS, px, hanji).0)
    } else {
        shortened(cuts[low - 1])
    };
    (Cow::Owned(candidate), width)
}

fn is_combining(scalar: char) -> bool {
    matches!(
        scalar as u32,
        0x0300..=0x036F | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x20D0..=0x20FF | 0xFE20..=0xFE2F
    )
}

/// Whether one line is Hanji script, and so takes the Hanji typeface. A
/// two-script cell follows `hanji_leads` (the leading script is the cell's
/// `text`). A one-script cell is Hanji when it contains a CJK scalar.
fn hanji_script(text: &str, is_leading: bool, paired: bool, hanji_leads: bool) -> bool {
    if paired {
        is_leading == hanji_leads
    } else {
        is_cjk(text)
    }
}

/// Whether one line takes the primary size. The size stays with the line's
/// role: a paired cell's leading line is primary whatever script it is, and
/// a one-script cell is the only line, so it is primary too. Switching the
/// output script moves which script sits on that line; it does not resize a
/// line that kept its role, and it does not resize a cell that has one line.
fn primary_line(is_leading: bool, paired: bool) -> bool {
    !paired || is_leading
}

fn is_cjk(text: &str) -> bool {
    text.chars().any(|scalar| (scalar as u32) >= 0x2E80)
}

fn shift_cell(cell: &LaidCell, x: f32, y: f32, glyphs: &mut Vec<Glyph>, hits: &mut Vec<Hit>) {
    for glyph in &cell.glyphs {
        let mut placed = glyph.clone();
        placed.x += x;
        placed.y += y;
        glyphs.push(placed);
    }
    let mut hit = cell.hit.clone();
    hit.x += x;
    hit.y += y;
    hits.push(hit);
}

fn arrow_span(label_px: f32, measure: &impl Measure) -> f32 {
    let (w, _) = measure.size("‹", label_px, false);
    w + label_px
}

/// The horizontal row's paging edge: ▲ over ▼ in one column, each half a
/// hit target. A direction with no page left draws nothing but keeps its slot.
fn place_page_column(
    glyphs: &mut Vec<Glyph>,
    hits: &mut Vec<Hit>,
    (has_prev, has_next): (bool, bool),
    at: (f32, f32),
    row_h: f32,
    style: &Scale,
    measure: &impl Measure,
) {
    let (x, y) = at;
    let px = style.label_px * 0.7;
    let span = arrow_span(style.label_px, measure);
    let half = row_h / 2.0;
    for (forward, shown, top, text) in [(false, has_prev, y, "▲"), (true, has_next, y + half, "▼")]
    {
        if !shown {
            continue;
        }
        let (w, h) = measure.size(text, px, false);
        glyphs.push(Glyph {
            text: text.to_owned(),
            x: x + (span - w) / 2.0,
            y: top + (half - h) / 2.0,
            w,
            h,
            px,
            role: Role::Arrow,
            hanji: false,
        });
        hits.push(Hit {
            x,
            y: top,
            w: span,
            h: half,
            target: Target::Page { forward },
        });
    }
}

fn place_arrow(
    glyphs: &mut Vec<Glyph>,
    hits: &mut Vec<Hit>,
    forward: bool,
    at: (f32, f32),
    row_h: f32,
    style: &Scale,
    measure: &impl Measure,
) {
    let (x, y) = at;
    let label_px = style.label_px;
    let text = if forward { "›" } else { "‹" };
    let (w, h) = measure.size(text, label_px, false);
    let span = arrow_span(label_px, measure);
    glyphs.push(Glyph {
        text: text.to_owned(),
        x: x + (span - w) / 2.0,
        y: y + (row_h - h) / 2.0,
        w,
        h,
        px: label_px,
        role: Role::Arrow,
        hanji: false,
    });
    hits.push(Hit {
        x,
        y,
        w: span,
        h: row_h,
        target: Target::Page { forward },
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Caret, Cell, ExpandableView, Show};

    struct Mono;

    impl Measure for Mono {
        fn size(&self, text: &str, px: f32, _hanji: bool) -> (f32, f32) {
            (text.chars().count() as f32 * px * 0.5, px)
        }
    }

    fn page(vertical: bool, hanji_leads: bool, cells: Vec<Cell>) -> Show {
        let labels = (1..=cells.len()).map(|index| index.to_string()).collect();
        Show {
            cells,
            labels,
            cursor: Some(0),
            vertical,
            has_prev: false,
            has_next: false,
            hanji_px: 20,
            roman_px: 10,
            hanji_leads,
            chrome: crate::protocol::Chrome::default(),
            caret: Caret {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            },
            min_content_width: 0,
            expandable: None,
        }
    }

    fn center(glyph: &Glyph) -> f32 {
        glyph.x + glyph.w / 2.0
    }

    fn role(layout: &Arrangement, role: Role) -> &Glyph {
        layout
            .glyphs
            .iter()
            .find(|glyph| glyph.role == role)
            .expect("glyph")
    }

    #[test]
    fn a_horizontal_cell_centers_the_two_scripts() {
        let layout = arrange(
            &page(
                false,
                true,
                vec![Cell {
                    text: "好".into(),
                    annotation: Some("hóo".into()),
                }],
            ),
            &Mono,
            1.0,
            Fit::UNBOUNDED,
        );
        let text = role(&layout, Role::Text);
        let annotation = role(&layout, Role::Annotation);
        assert!((center(text) - center(annotation)).abs() < 0.01);
        assert_eq!(text.px, 20.0);
        assert_eq!(annotation.px, 10.0);
        assert!(text.hanji);
        assert!(!annotation.hanji);
        assert!(annotation.y > text.y);
        // trace: default window pad 4 = the script gap at 16 px; the page's
        // larger size is 20 px, so the gap grows to 4 × 20 / 16 = 5.
        assert!((annotation.y - (text.y + text.h) - 5.0).abs() < 0.01);
    }

    /// L6: at 48 px the Hanji and the romanization of a vertical row sat the
    /// fixed 4 px pad apart. The gap now grows with the text: 4 px up to the
    /// 16 px default, 4 × 48 / 16 = 12 px at 48.
    #[test]
    fn the_gap_between_the_scripts_grows_with_the_text() {
        let gap_at = |px: u32, vertical: bool| {
            let mut shown = page(
                vertical,
                true,
                vec![Cell {
                    text: "一".into(),
                    annotation: Some("tsit".into()),
                }],
            );
            shown.hanji_px = px;
            shown.roman_px = px;
            let layout = arrange(&shown, &Mono, 1.0, Fit::UNBOUNDED);
            let text = role(&layout, Role::Text);
            let annotation = role(&layout, Role::Annotation);
            if vertical {
                annotation.x - (text.x + text.w)
            } else {
                annotation.y - (text.y + text.h)
            }
        };
        assert!((gap_at(13, true) - 4.0).abs() < 0.01);
        assert!((gap_at(16, true) - 4.0).abs() < 0.01);
        assert!((gap_at(48, true) - 12.0).abs() < 0.01);
        assert!((gap_at(16, false) - 4.0).abs() < 0.01);
        assert!((gap_at(48, false) - 12.0).abs() < 0.01);
    }

    #[test]
    fn a_vertical_page_centers_keys_and_left_aligns_text() {
        let mut shown = page(
            true,
            true,
            vec![
                Cell {
                    text: "母".into(),
                    annotation: Some("m".into()),
                },
                Cell {
                    text: "媽".into(),
                    annotation: Some("ma".into()),
                },
                Cell {
                    text: "嘛".into(),
                    annotation: None,
                },
            ],
        );
        // "10" is twice as wide as "1" under Mono, the way "w" is wider than "1".
        shown.labels = vec!["1".into(), "10".into(), String::new()];
        let layout = arrange(&shown, &Mono, 1.0, Fit::UNBOUNDED);
        let labels: Vec<_> = layout
            .glyphs
            .iter()
            .filter(|glyph| glyph.role == Role::Label)
            .collect();
        let texts: Vec<_> = layout
            .glyphs
            .iter()
            .filter(|glyph| glyph.role == Role::Text)
            .collect();
        assert_eq!(labels.len(), 2);
        assert!((center(labels[0]) - center(labels[1])).abs() < 0.01);
        assert!(labels[0].w < labels[1].w);
        assert!((texts[0].x - texts[1].x).abs() < 0.01);
        assert!((texts[1].x - texts[2].x).abs() < 0.01);
        assert!(texts[0].x >= labels[1].x + labels[1].w);
    }

    #[test]
    fn a_vertical_cell_keeps_the_scripts_on_one_line() {
        let layout = arrange(
            &page(
                true,
                true,
                vec![Cell {
                    text: "好".into(),
                    annotation: Some("hó".into()),
                }],
            ),
            &Mono,
            1.0,
            Fit::UNBOUNDED,
        );
        let text = role(&layout, Role::Text);
        let annotation = role(&layout, Role::Annotation);
        assert!(annotation.x > text.x);
        let baseline = |glyph: &Glyph| glyph.y + glyph.h;
        assert!((baseline(text) - baseline(annotation)).abs() < 0.01);
    }

    #[test]
    fn a_one_script_cell_takes_the_primary_size() {
        let hanji = arrange(
            &page(
                false,
                true,
                vec![Cell {
                    text: "好".into(),
                    annotation: None,
                }],
            ),
            &Mono,
            1.0,
            Fit::UNBOUNDED,
        );
        let roman = arrange(
            &page(
                false,
                true,
                vec![Cell {
                    text: "hó".into(),
                    annotation: None,
                }],
            ),
            &Mono,
            1.0,
            Fit::UNBOUNDED,
        );
        assert_eq!(role(&hanji, Role::Text).px, 20.0);
        assert!(role(&hanji, Role::Text).hanji);
        assert_eq!(role(&roman, Role::Text).px, 20.0);
        assert!(!role(&roman, Role::Text).hanji);
        let swapped = arrange(
            &page(
                false,
                false,
                vec![
                    Cell {
                        text: "好".into(),
                        annotation: None,
                    },
                    Cell {
                        text: "hó".into(),
                        annotation: None,
                    },
                ],
            ),
            &Mono,
            1.0,
            Fit::UNBOUNDED,
        );
        let lines: Vec<_> = swapped
            .glyphs
            .iter()
            .filter(|glyph| glyph.role == Role::Text)
            .collect();
        assert_eq!(lines[0].px, 20.0);
        assert!(lines[0].hanji);
        assert_eq!(lines[1].px, 20.0);
        assert!(!lines[1].hanji);
    }

    #[test]
    fn the_leading_line_is_primary_when_romanization_leads() {
        let layout = arrange(
            &page(
                false,
                false,
                vec![Cell {
                    text: "hó".into(),
                    annotation: Some("好".into()),
                }],
            ),
            &Mono,
            1.0,
            Fit::UNBOUNDED,
        );
        let text = role(&layout, Role::Text);
        let annotation = role(&layout, Role::Annotation);
        assert_eq!(text.px, 20.0);
        assert!(!text.hanji);
        assert_eq!(annotation.px, 10.0);
        assert!(annotation.hanji);
    }

    #[test]
    fn a_full_highlight_meets_the_window_edge() {
        let mut shown = page(
            false,
            true,
            vec![Cell {
                text: "好".into(),
                annotation: Some("hó".into()),
            }],
        );
        shown.chrome.highlight_fill = 100;
        let layout = arrange(&shown, &Mono, 1.0, Fit::UNBOUNDED);
        let highlight = layout.highlight.expect("highlight");
        assert_eq!(highlight.x, 0.0);
        assert_eq!(highlight.y, 0.0);
        assert!((highlight.w - layout.width).abs() < 0.01);
        assert!((highlight.h - layout.height).abs() < 0.01);
    }

    fn short_cells() -> Vec<Cell> {
        ["矮", "啞"]
            .into_iter()
            .map(|text| Cell {
                text: text.into(),
                annotation: Some("é".into()),
            })
            .collect()
    }

    /// The packed extent the session sends as `min_content_width`.
    fn packed_extent(shown: &Show) -> u32 {
        let packed = row_widths(&shown.cells, cell_style(shown), &Mono);
        (packed.widths.iter().sum::<f32>() - packed.gap).ceil() as u32
    }

    #[test]
    fn a_row_draws_its_cells_at_the_packed_widths() {
        let shown = page(false, true, short_cells());
        let pad = pads(shown.chrome.padding, 1.0).window;
        let layout = arrange(&shown, &Mono, 1.0, Fit::UNBOUNDED);
        assert_eq!(layout.width, pad * 2.0 + packed_extent(&shown) as f32);
    }

    #[test]
    fn a_collapsed_row_of_short_cells_leaves_no_space_after_the_last() {
        let mut shown = page(false, true, short_cells());
        shown.cursor = Some(1);
        shown.chrome.highlight_fill = 100;
        shown.min_content_width = packed_extent(&shown);
        shown.expandable = Some(ExpandableView {
            absolute_indices: vec![0, 1],
            ..ExpandableView::default()
        });
        for scale in [1.0, 1.25, 2.0] {
            let layout = arrange(&shown, &Mono, scale, Fit::UNBOUNDED);
            let highlight = layout.highlight.expect("highlight");
            assert!(
                (highlight.x + highlight.w - layout.width).abs() < 0.01,
                "at {scale}x the last cell ends {} short of the window",
                layout.width - highlight.x - highlight.w,
            );
        }
    }

    #[test]
    fn a_full_highlight_stops_halfway_to_the_next_cell() {
        let mut shown = page(
            false,
            true,
            vec![
                Cell {
                    text: "好".into(),
                    annotation: Some("hó".into()),
                },
                Cell {
                    text: "號".into(),
                    annotation: Some("hō".into()),
                },
            ],
        );
        shown.chrome.highlight_fill = 100;
        let layout = arrange(&shown, &Mono, 1.0, Fit::UNBOUNDED);
        let highlight = layout.highlight.expect("highlight");
        assert_eq!(highlight.x, 0.0);
        assert_eq!(highlight.y, 0.0);
        assert!((highlight.h - layout.height).abs() < 0.01);
        assert!(highlight.x + highlight.w < layout.width - 1.0);
        let second = layout
            .glyphs
            .iter()
            .filter(|glyph| glyph.role == Role::Text)
            .nth(1)
            .expect("second");
        assert!(second.x + second.w / 2.0 > highlight.x + highlight.w);
    }

    #[test]
    fn an_expanded_row_keeps_natural_widths_on_the_models_rows_from_the_left() {
        use crate::protocol::ExpandableView;
        let mut shown = page(
            false,
            true,
            vec![
                Cell {
                    text: "好".into(),
                    annotation: Some("hó".into()),
                },
                Cell {
                    text: "台灣".into(),
                    annotation: Some("tâi-uân".into()),
                },
                Cell {
                    text: "人".into(),
                    annotation: Some("lâng".into()),
                },
            ],
        );
        shown.labels = vec![String::new(), String::new(), "1".into()];
        shown.min_content_width = 200;
        shown.expandable = Some(ExpandableView {
            absolute_indices: vec![0, 1, 2],
            mode: 1,
            axis: 0,
            column_width: 80,
            grid_columns: 2,
            scroll_y: 0,
            row_indexes: vec![0, 0, 1],
            column_starts: vec![0, 1, 0],
            column_spans: vec![1, 1, 2],
            visible_rows: 2,
            row_height: 40,
        });
        let layout = arrange(&shown, &Mono, 1.0, Fit::UNBOUNDED);
        let texts: Vec<_> = layout
            .glyphs
            .iter()
            .filter(|glyph| glyph.role == Role::Text)
            .collect();
        assert_eq!(texts.len(), 3);
        assert!(texts[0].y + texts[0].h <= texts[2].y);
        assert!(texts[0].x + texts[0].w <= texts[1].x + 1.0);
        // Both rows start at the left edge whatever the model's column grid,
        // and neither row's cells are stretched to it.
        let starts: Vec<_> = layout
            .hits
            .iter()
            .filter(|hit| matches!(hit.target, Target::Cell(_)))
            .map(|hit| (hit.x, hit.w))
            .collect();
        assert_eq!(starts[0].0, starts[2].0);
        assert!(starts[0].1 < 80.0);
        let pad = pads(shown.chrome.padding, 1.0).window;
        assert!((layout.width - (pad * 2.0 + 200.0)).abs() < 1.0);
    }

    #[test]
    fn an_expanded_highlight_honours_the_fill_and_rows_keep_their_gap() {
        use crate::protocol::ExpandableView;
        let cells = vec![
            Cell {
                text: "好".into(),
                annotation: Some("hó".into()),
            },
            Cell {
                text: "人".into(),
                annotation: Some("lâng".into()),
            },
        ];
        for axis in [0, 1] {
            let vertical = axis == 1;
            let mut shown = page(vertical, true, cells.clone());
            shown.cursor = Some(0);
            shown.expandable = Some(ExpandableView {
                absolute_indices: vec![0, 1],
                mode: 1,
                axis,
                column_width: 80,
                grid_columns: 1,
                scroll_y: 0,
                row_indexes: vec![0, 1],
                column_starts: vec![0, 0],
                column_spans: vec![1, 1],
                visible_rows: 2,
                row_height: 40,
            });
            shown.chrome.highlight_fill = 0;
            let bare = arrange(&shown, &Mono, 1.0, Fit::UNBOUNDED)
                .highlight
                .expect("highlight");
            shown.chrome.highlight_fill = 100;
            let layout = arrange(&shown, &Mono, 1.0, Fit::UNBOUNDED);
            let full = layout.highlight.expect("highlight");
            // The top-left cell's fill reaches the window edge, as collapsed.
            assert_eq!((full.x, full.y), (0.0, 0.0), "axis {axis}");
            assert!(full.h > bare.h, "axis {axis}");
            if vertical {
                // Rows are spaced like the collapsed vertical list.
                let mut collapsed = page(true, true, cells.clone());
                collapsed.cursor = Some(0);
                let pitch = |l: &Arrangement| {
                    let hits: Vec<_> = l
                        .hits
                        .iter()
                        .filter(|hit| matches!(hit.target, Target::Cell(_)))
                        .collect();
                    hits[1].y - hits[0].y
                };
                let unfolded = arrange(&shown, &Mono, 1.0, Fit::UNBOUNDED);
                let folded = arrange(&collapsed, &Mono, 1.0, Fit::UNBOUNDED);
                assert!((pitch(&unfolded) - pitch(&folded)).abs() < 0.5);
            }
        }
    }

    #[test]
    fn a_vertical_expandable_list_keeps_its_height_and_fills_to_the_right_edge() {
        use crate::protocol::ExpandableView;
        let cells: Vec<Cell> = ["好", "人"]
            .iter()
            .map(|text| Cell {
                text: (*text).into(),
                annotation: Some("hó".into()),
            })
            .collect();
        let mut folded = page(true, true, cells.clone());
        folded.has_next = true;
        folded.expandable = Some(ExpandableView {
            absolute_indices: vec![0, 1],
            mode: 0,
            axis: 1,
            ..ExpandableView::default()
        });
        let mut unfolded = page(false, true, cells);
        unfolded.cursor = Some(1);
        unfolded.chrome.highlight_fill = 100;
        // A three-column grid whose last view holds one column.
        unfolded.expandable = Some(ExpandableView {
            absolute_indices: vec![0, 1],
            mode: 1,
            axis: 1,
            column_width: 80,
            grid_columns: 3,
            scroll_y: 0,
            row_indexes: vec![0, 1],
            column_starts: vec![0, 0],
            column_spans: vec![1, 1],
            visible_rows: 2,
            row_height: 40,
        });
        let before = arrange(&folded, &Mono, 1.0, Fit::UNBOUNDED);
        let after = arrange(&unfolded, &Mono, 1.0, Fit::UNBOUNDED);
        assert!((before.height - after.height).abs() < 0.5);
        let full = after.highlight.expect("highlight");
        assert!((full.x + full.w - after.width).abs() < 0.01);
    }

    /// trace: folded column, two cells, overflow (`has_next`) → a `›` column
    /// right of the cells (Page hit, x past every cell), as tall as the
    /// cells; the full highlight stops halfway across the gap before it.
    #[test]
    fn a_folded_vertical_expandable_list_shows_its_arrow_column_on_the_right() {
        use crate::protocol::ExpandableView;
        let cells: Vec<Cell> = ["好", "人"]
            .iter()
            .map(|text| Cell {
                text: (*text).into(),
                annotation: Some("hó".into()),
            })
            .collect();
        let mut folded = page(true, true, cells);
        folded.has_next = true;
        folded.chrome.highlight_fill = 100;
        folded.expandable = Some(ExpandableView {
            absolute_indices: vec![0, 1],
            mode: 0,
            axis: 1,
            ..ExpandableView::default()
        });
        let layout = arrange(&folded, &Mono, 1.0, Fit::UNBOUNDED);
        let cells_right = layout
            .hits
            .iter()
            .filter(|hit| matches!(hit.target, Target::Cell(_)))
            .map(|hit| hit.x + hit.w)
            .fold(0.0_f32, f32::max);
        let arrow = layout
            .hits
            .iter()
            .find(|hit| hit.target == Target::Page { forward: true })
            .expect("the folded list's arrow");
        assert!(arrow.x > cells_right, "the arrow sits right of the cells");
        let pad = pads(folded.chrome.padding, 1.0).window;
        assert!((arrow.y - pad).abs() < 0.01);
        assert!((arrow.y + arrow.h - (layout.height - pad)).abs() < 0.01);
        let full = layout.highlight.expect("highlight");
        assert!(full.x + full.w <= arrow.x, "the fill leaves the arrow be");
        // Without overflow there is no arrow and the fill meets the edge.
        folded.has_next = false;
        let plain = arrange(&folded, &Mono, 1.0, Fit::UNBOUNDED);
        assert!(plain
            .hits
            .iter()
            .all(|hit| !matches!(hit.target, Target::Page { .. })));
        let full = plain.highlight.expect("highlight");
        assert!((full.x + full.w - plain.width).abs() < 0.01);
    }

    /// Hanji glyphs a fifth taller than romanization, as real typefaces'
    /// ascent + descent differ.
    struct TallHanji;

    impl Measure for TallHanji {
        fn size(&self, text: &str, px: f32, hanji: bool) -> (f32, f32) {
            let h = if hanji { px * 1.2 } else { px };
            (text.chars().count() as f32 * px * 0.5, h)
        }
    }

    /// trace: unfolded column, row 0 a romanization-only cell (h = 20 plus
    /// 2·cell_pad), row 1 a Hanji cell (h = 24 plus 2·cell_pad) → row_body is
    /// the Hanji height; the short cell's bare highlight and hit start at the row
    /// top (pad) and are row_body tall, as its Hanji neighbour's.
    #[test]
    fn an_unfolded_column_highlights_a_short_cell_at_its_row_height() {
        use crate::protocol::ExpandableView;
        let cells = vec![
            Cell {
                text: "siáng".into(),
                annotation: None,
            },
            Cell {
                text: "是".into(),
                annotation: Some("sī".into()),
            },
        ];
        let mut shown = page(false, true, cells);
        shown.chrome.highlight_fill = 0;
        shown.expandable = Some(ExpandableView {
            absolute_indices: vec![0, 1],
            mode: 1,
            axis: 1,
            column_width: 80,
            grid_columns: 1,
            scroll_y: 0,
            row_indexes: vec![0, 1],
            column_starts: vec![0, 0],
            column_spans: vec![1, 1],
            visible_rows: 2,
            row_height: 40,
        });
        shown.cursor = Some(0);
        let layout = arrange(&shown, &TallHanji, 1.0, Fit::UNBOUNDED);
        let short = layout.highlight.expect("cursor");
        shown.cursor = Some(1);
        let tall = arrange(&shown, &TallHanji, 1.0, Fit::UNBOUNDED)
            .highlight
            .expect("cursor");
        let pad = pads(shown.chrome.padding, 1.0).window;
        assert!(
            (short.y - pad).abs() < 0.01,
            "the band starts at the row top"
        );
        assert!(
            (short.h - tall.h).abs() < 0.01,
            "the band is the row's height"
        );
        let hit = layout
            .hits
            .iter()
            .find(|hit| hit.target == Target::Cell(0))
            .expect("the short cell's hit");
        assert!((hit.y - short.y).abs() < 0.01 && (hit.h - short.h).abs() < 0.01);
    }

    #[test]
    fn a_paged_plain_row_keeps_one_width_on_every_page() {
        let cells = |text: &str| {
            vec![
                Cell {
                    text: text.into(),
                    annotation: Some("lí".into()),
                };
                3
            ]
        };
        let mut first = page(false, true, cells("你"));
        first.has_next = true;
        first.min_content_width = 300;
        let mut middle = page(false, true, cells("離"));
        middle.has_prev = true;
        middle.has_next = true;
        middle.min_content_width = 300;
        let mut last = page(false, true, cells("里"));
        last.has_prev = true;
        last.min_content_width = 300;
        let widths: Vec<f32> = [first, middle, last]
            .iter()
            .map(|shown| arrange(shown, &Mono, 1.0, Fit::UNBOUNDED).width)
            .collect();
        assert_eq!(widths[0], widths[1]);
        assert_eq!(widths[1], widths[2]);
    }

    #[test]
    fn a_romanization_only_cell_highlights_the_full_row_height() {
        let two_line = Cell {
            text: "是".into(),
            annotation: Some("sī".into()),
        };
        let roman_only = Cell {
            text: "siáng".into(),
            annotation: None,
        };
        let mut shown = page(false, true, vec![two_line, roman_only]);
        shown.chrome.highlight_fill = 0;
        let tall = arrange(&shown, &Mono, 1.0, Fit::UNBOUNDED)
            .highlight
            .expect("cursor");
        shown.cursor = Some(1);
        let short = arrange(&shown, &Mono, 1.0, Fit::UNBOUNDED)
            .highlight
            .expect("cursor");
        assert_eq!(short.y, tall.y);
        assert_eq!(short.h, tall.h);
    }

    #[test]
    fn padding_widens_the_window_and_keeps_the_old_ratio() {
        assert_eq!(
            pads(8, 1.0),
            Pads {
                window: 8.0,
                cell: 6.0,
                gap: 8.0,
                line: 2.0,
                script: 8.0,
            }
        );
        let tight = page(
            false,
            true,
            vec![Cell {
                text: "好".into(),
                annotation: Some("hó".into()),
            }],
        );
        let mut loose = tight.clone();
        loose.chrome.padding = 16;
        let tight = arrange(&tight, &Mono, 1.0, Fit::UNBOUNDED);
        let loose = arrange(&loose, &Mono, 1.0, Fit::UNBOUNDED);
        assert!(loose.width > tight.width);
        assert!(loose.height > tight.height);
    }

    fn fit(width: f32, height: f32) -> Fit {
        Fit {
            width,
            height,
            first_row: 0,
        }
    }

    fn long_cell() -> Cell {
        Cell {
            text: "一上台語的聲袂大米款黃接相見入".into(),
            annotation: Some("tsit siōng tâi-gí ê siann bē tāi bí khuán n̂g tsiap sio-kìnn".into()),
        }
    }

    /// Every text and annotation line ends inside the window's pad.
    fn assert_inside(layout: &Arrangement, pad: f32) {
        for glyph in &layout.glyphs {
            assert!(
                glyph.x + glyph.w <= layout.width - pad + 0.01,
                "{:?} ends at {} in a {} window",
                glyph.text,
                glyph.x + glyph.w,
                layout.width
            );
        }
    }

    /// L1: a horizontal cell wider than the screen was drawn whole and ran off
    /// its right edge. Both lines now end in an ellipsis inside the window.
    #[test]
    fn a_cell_wider_than_the_screen_ends_in_an_ellipsis() {
        let shown = page(false, true, vec![long_cell()]);
        let whole = arrange(&shown, &Mono, 1.0, Fit::UNBOUNDED);
        // trace: Mono = chars × px / 2: text 15 × 20 / 2 = 150, past a 140 window.
        assert!(whole.width > 140.0);
        let layout = arrange(&shown, &Mono, 1.0, fit(140.0, 600.0));
        assert!(layout.width <= 140.0, "window {}", layout.width);
        assert_inside(&layout, pads(4, 1.0).window);
        assert!(role(&layout, Role::Text).text.ends_with('…'));
        assert!(role(&layout, Role::Annotation).text.ends_with('…'));
        // The highlight stays inside the window too.
        let highlight = layout.highlight.expect("highlight");
        assert!(highlight.x + highlight.w <= layout.width);
    }

    /// L2 (the first row at tsf 1.0): a vertical row wider than the screen.
    /// The annotation gives way first; when even the text does not fit, the
    /// annotation goes and the text ends in an ellipsis.
    #[test]
    fn a_vertical_row_lets_the_annotation_give_way_first() {
        let shown = page(true, true, vec![long_cell()]);
        // trace: Mono = chars × px / 2. Text 15 × 20 / 2 = 150; the room
        // leaves it whole and cuts the annotation.
        let layout = arrange(&shown, &Mono, 1.0, fit(260.0, 600.0));
        assert!(layout.width <= 260.0);
        assert_inside(&layout, pads(4, 1.0).window);
        assert_eq!(role(&layout, Role::Text).text, long_cell().text);
        assert!(role(&layout, Role::Annotation).text.ends_with('…'));

        let narrow = arrange(&shown, &Mono, 1.0, fit(120.0, 600.0));
        assert!(narrow.width <= 120.0);
        assert_inside(&narrow, pads(4, 1.0).window);
        assert!(role(&narrow, Role::Text).text.ends_with('…'));
        assert!(narrow
            .glyphs
            .iter()
            .all(|glyph| glyph.role != Role::Annotation));
    }

    #[test]
    fn a_cut_keeps_a_letter_with_its_combining_mark() {
        // POJ "o͘" is o + U+0358: no cut may leave the o without its dot.
        let text = "ho͘ho͘ho͘ho͘ho͘ho͘";
        for room in 1..40 {
            let (cut, _) = truncate(text, 10.0, false, 1000.0, room as f32 * 5.0, &Mono);
            let kept = cut.trim_end_matches('…');
            assert!(text.starts_with(kept));
            assert!(
                !text[kept.len()..].starts_with('\u{0358}'),
                "{cut:?} split o͘"
            );
        }
    }

    fn rows_page(count: usize, cursor: u32) -> Show {
        let cells = (0..count)
            .map(|index| Cell {
                text: format!("字{index}"),
                annotation: Some("lâi".into()),
            })
            .collect();
        let mut shown = page(true, true, cells);
        shown.cursor = Some(cursor);
        shown
    }

    fn shown_rows(layout: &Arrangement) -> Vec<String> {
        layout
            .glyphs
            .iter()
            .filter(|glyph| glyph.role == Role::Text)
            .map(|glyph| glyph.text.clone())
            .collect()
    }

    /// L2: a vertical list taller than the screen only moved and ran past the
    /// bottom. It now shows the rows that fit, around the cursor, and scrolls
    /// only when the cursor leaves them.
    #[test]
    fn a_vertical_list_taller_than_its_room_shows_the_rows_around_the_cursor() {
        let whole = arrange(&rows_page(9, 0), &Mono, 1.0, Fit::UNBOUNDED);
        let room = whole.height * 0.5;
        let top = arrange(&rows_page(9, 0), &Mono, 1.0, fit(600.0, room));
        assert!(top.height <= room, "{} > {room}", top.height);
        let visible = shown_rows(&top).len();
        assert!((2..9).contains(&visible), "{visible} rows");
        assert_eq!(top.first_row, 0);
        assert_eq!(shown_rows(&top)[0], "字0");

        // The cursor on the last row scrolls it into view, at the bottom.
        let bottom = arrange(&rows_page(9, 8), &Mono, 1.0, fit(600.0, room));
        assert_eq!(bottom.first_row, 9 - visible);
        assert_eq!(shown_rows(&bottom).last().map(String::as_str), Some("字8"));
        let highlight = bottom.highlight.expect("highlight");
        assert!(highlight.y + highlight.h <= bottom.height);
        assert!(bottom.hits.iter().all(|hit| hit.y + hit.h <= bottom.height));

        // Back up one row: still on screen, so the rows stay put.
        let mut held = fit(600.0, room);
        held.first_row = bottom.first_row;
        let up = arrange(&rows_page(9, 7), &Mono, 1.0, held);
        assert_eq!(up.first_row, bottom.first_row);
    }
}
