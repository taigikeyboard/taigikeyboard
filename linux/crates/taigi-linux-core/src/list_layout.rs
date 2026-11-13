//! Builds an [`ExpandableListModel`] from real text widths, the same arm as
//! Web's `PageListLayout` (`web/crates/taigi-web/src/layout.rs`) and the
//! Windows / macOS windows.

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex, OnceLock};

use taigi_desktop_core::candidates::{
    CandidateMetrics, CollapsedLayout, ExpandableGeometryInput, ExpandableListModel, FontSpec,
    HorizontalPageLayout, TextMeasurer,
};
use taigi_desktop_core::composing::CandidateCellContent;
use taigi_desktop_core::session::ListLayout;
use taigi_desktop_core::settings::{
    keys, CandidateFontSelection, CandidateLayout, SettingsDocument,
};
use taigikeyboard_popup::text::Fonts;
use taigikeyboard_popup::{horizontal_cell_extent, key_label_width, Cell, CellStyle, Measure};

/// macOS chrome ratio, twin of Web `MACOS_CHROME_RATIO`.
const MACOS_CHROME_RATIO: f32 = 0.7;
/// The chevron column beside a list (`CHEVRON_COLUMN_RATIO` on Web).
const CHEVRON_COLUMN_RATIO: f32 = 1.5;
const LINE_HEIGHT_RATIO: f32 = 1.2;
/// A conservative cap when the display size is unknown (cross-host tests).
const DEFAULT_MAXIMUM_WINDOW_WIDTH: f32 = 640.0;

#[derive(Debug, Default)]
struct Measured {
    maximum_window_width: f32,
    widths: Vec<f32>,
}

trait TextRuler: Debug + Send + Sync {
    fn measure(&self, runs: &[(&str, f32)]) -> Option<Measured>;
    fn size(&self, text: &str, px: f32, hanji: bool) -> Option<(f32, f32)>;
}

struct PopupTextRuler {
    fonts: Mutex<Fonts>,
}

impl std::fmt::Debug for PopupTextRuler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PopupTextRuler")
    }
}

impl PopupTextRuler {
    fn new() -> Self {
        Self {
            fonts: Mutex::new(Fonts::load()),
        }
    }
}

impl TextRuler for PopupTextRuler {
    fn measure(&self, runs: &[(&str, f32)]) -> Option<Measured> {
        let fonts = self.fonts.lock().ok()?;
        let widths: Vec<f32> = runs
            .iter()
            .map(|(text, px)| {
                let hanji = text.chars().any(|ch| ch as u32 > 0x2E7F);
                Measure::size(&*fonts, text, *px, hanji).0
            })
            .collect();
        Some(Measured {
            maximum_window_width: DEFAULT_MAXIMUM_WINDOW_WIDTH,
            widths,
        })
    }

    fn size(&self, text: &str, px: f32, hanji: bool) -> Option<(f32, f32)> {
        let fonts = self.fonts.lock().ok()?;
        Some(Measure::size(&*fonts, text, px, hanji))
    }
}

struct RulerMeasure<'a>(&'a dyn TextRuler);

impl Measure for RulerMeasure<'_> {
    fn size(&self, text: &str, px: f32, hanji: bool) -> (f32, f32) {
        self.0.size(text, px, hanji).unwrap_or((0.0, px))
    }
}

fn line_height(font: FontSpec) -> f32 {
    font.size * LINE_HEIGHT_RATIO
}

#[derive(Default)]
struct RunRecorder {
    runs: std::cell::RefCell<Vec<(String, f32)>>,
}

impl TextMeasurer for RunRecorder {
    fn width(&self, text: &str, font: FontSpec) -> f32 {
        self.runs.borrow_mut().push((text.to_owned(), font.size));
        0.0
    }

    fn line_height(&self, font: FontSpec) -> f32 {
        line_height(font)
    }
}

struct MeasuredRuns(HashMap<u32, HashMap<String, f32>>);

impl TextMeasurer for MeasuredRuns {
    fn width(&self, text: &str, font: FontSpec) -> f32 {
        self.0
            .get(&font.size.to_bits())
            .and_then(|widths| widths.get(text))
            .copied()
            .unwrap_or(0.0)
    }

    fn line_height(&self, font: FontSpec) -> f32 {
        line_height(font)
    }
}

fn drawn_annotation(cell: &CandidateCellContent) -> Option<&String> {
    cell.annotation.as_ref().filter(|text| !text.is_empty())
}

#[derive(Debug)]
pub struct PopupListLayout {
    ruler: Arc<dyn TextRuler>,
}

impl PopupListLayout {
    fn new(ruler: Arc<dyn TextRuler>) -> Self {
        Self { ruler }
    }
}

impl PopupListLayout {
    /// Widths as the popup draws a stacked row, plus the slot a short Hanji
    /// cell occupies — so packing agrees with what the window actually shows.
    /// Whole px, each with the gap after it, floored at the slot: nine short
    /// cells fill a row exactly, and the row packer and the unfolded grid
    /// (one px per column) break at the same cell.
    fn visual_row_widths(
        &self,
        cells: &[CandidateCellContent],
        settings: &SettingsDocument,
    ) -> Option<RowWidths> {
        let hanji_px = settings.i64(&keys::HANJI_FONT_SIZE).clamp(
            *keys::CANDIDATE_FONT_PX_RANGE.start(),
            *keys::CANDIDATE_FONT_PX_RANGE.end(),
        ) as u32;
        let roman_px = settings.i64(&keys::ROMAN_FONT_SIZE).clamp(
            *keys::CANDIDATE_FONT_PX_RANGE.start(),
            *keys::CANDIDATE_FONT_PX_RANGE.end(),
        ) as u32;
        let padding = settings.i64(&keys::CANDIDATE_WINDOW_PADDING).clamp(
            *keys::CANDIDATE_CHROME_PX_RANGE.start(),
            *keys::CANDIDATE_CHROME_PX_RANGE.end(),
        ) as u32;
        let cell_style = CellStyle {
            hanji_px,
            roman_px,
            hanji_leads: settings.engine_settings().is_hanji_first,
            padding,
        };
        let measure = RulerMeasure(&*self.ruler);
        let gap = taigikeyboard_popup::pads(padding, 1.0).gap;
        // The widest key any slot set prints; every cell reserves it.
        let key = KEY_LABEL_SAMPLES
            .iter()
            .map(|label| key_label_width(label, &measure, 1.0))
            .fold(0.0_f32, f32::max);
        let extent = |text: &str, annotation: Option<String>| {
            horizontal_cell_extent(
                &Cell {
                    text: text.to_owned(),
                    annotation,
                },
                "",
                cell_style,
                &measure,
                1.0,
            ) + key
                + gap
                + gap
        };
        let slot = extent("字", Some("lâi".into())).ceil();
        let budget = HorizontalPageLayout::row_budget(slot, PAGE_SIZE_HINT);
        let widths = cells
            .iter()
            .map(|cell| {
                extent(&cell.text, drawn_annotation(cell).cloned())
                    .ceil()
                    .max(slot)
                    .min(budget)
            })
            .collect();
        Some(RowWidths { widths, slot, gap })
    }
}

/// A select key wider than any digit, for the key slot every cell reserves.
const KEY_LABEL_SAMPLES: [&str; 4] = ["8", "w", "m", ";"];
/// The fullest slot set; a row is never wider than this many short cells.
const PAGE_SIZE_HINT: usize = HorizontalPageLayout::PAGE_SIZE;

struct RowWidths {
    /// One per cell, gap included.
    widths: Vec<f32>,
    /// A short one-Hanji cell, gap included.
    slot: f32,
    gap: f32,
}

impl ListLayout for PopupListLayout {
    fn horizontal_page_starts(
        &self,
        cells: &[CandidateCellContent],
        settings: &SettingsDocument,
        page_size: usize,
    ) -> Option<(Vec<usize>, u32)> {
        if settings.choice(&keys::CANDIDATE_LAYOUT) != CandidateLayout::Horizontal
            || cells.is_empty()
        {
            return None;
        }
        let RowWidths { widths, slot, gap } = self.visual_row_widths(cells, settings)?;
        // The same budget a collapsed horizontal-expandable row packs to, so
        // the two modes keep one window width once the list runs past a row.
        let packed = HorizontalPageLayout::pack(
            &widths,
            slot,
            HorizontalPageLayout::row_budget(slot, page_size),
            page_size,
        );
        let mut starts = Vec::new();
        let mut index = 0;
        for page in &packed.pages {
            if page.is_empty() {
                continue;
            }
            starts.push(index);
            index += page.len();
        }
        // The cells' extent only: the window adds the arrows itself.
        let floor = if packed.pages.len() > 1 {
            (packed.page_budget - gap).max(0.0).ceil() as u32
        } else {
            0
        };
        Some((starts, floor))
    }

    fn layout(
        &self,
        cells: &[CandidateCellContent],
        settings: &SettingsDocument,
        page_size: usize,
    ) -> Option<ExpandableListModel> {
        let layout: CandidateLayout = settings.choice(&keys::CANDIDATE_LAYOUT);
        if !matches!(
            layout,
            CandidateLayout::Expandable | CandidateLayout::VerticalExpandable
        ) {
            return None;
        }
        let recorder = RunRecorder::default();
        let sizes = CandidateMetrics::resolve_with_chrome_ratio(
            settings.choice(&keys::CANDIDATE_SIZE),
            CandidateFontSelection::default(),
            layout.cell_arrangement(),
            MACOS_CHROME_RATIO,
            &recorder,
        );
        let mut runs = recorder.runs.into_inner();
        let candidate_size = sizes.candidate_font().size;
        let annotation_size = sizes.annotation_font().size;
        for cell in cells {
            runs.push((cell.text.clone(), candidate_size));
            if let Some(annotation) = drawn_annotation(cell) {
                runs.push((annotation.clone(), annotation_size));
            }
        }
        let borrowed: Vec<(&str, f32)> = runs
            .iter()
            .map(|(text, size)| (text.as_str(), *size))
            .collect();
        let measured = self.ruler.measure(&borrowed)?;
        let mut answers = HashMap::<u32, HashMap<String, f32>>::new();
        for ((text, size), width) in runs.into_iter().zip(measured.widths) {
            answers
                .entry(size.to_bits())
                .or_default()
                .insert(text, width);
        }
        let answers = MeasuredRuns(answers);
        let has_annotations = cells.iter().any(|cell| drawn_annotation(cell).is_some());
        let metrics = sizes.for_content(has_annotations);
        let primary_widths: Vec<f32> = cells
            .iter()
            .map(|cell| metrics.measure_primary_width(&cell.text, &answers))
            .collect();
        let annotation_widths: Vec<Option<f32>> = cells
            .iter()
            .map(|cell| metrics.measure_annotation(cell.annotation.as_deref(), &answers))
            .collect();
        if layout == CandidateLayout::Expandable {
            let RowWidths { widths, slot, .. } = self.visual_row_widths(cells, settings)?;
            let budget = HorizontalPageLayout::row_budget(slot, page_size);
            let chrome = metrics.candidate_font_size() * CHEVRON_COLUMN_RATIO;
            return Some(ExpandableListModel::new(
                &widths,
                ExpandableGeometryInput {
                    collapsed: CollapsedLayout::Row,
                    base_width: slot,
                    // Room for a full row whatever the size, so the window and
                    // not this cap decides how many short cells a row holds.
                    maximum_window_width: measured.maximum_window_width.max(budget + chrome),
                    chrome_width: chrome,
                    scroller_width: 0.0,
                    item_height: metrics.item_height(),
                    page_size,
                    // One column per px of the row: the unfolded grid is the
                    // same width-based packing, each cell keeping its width.
                    row_columns: budget as usize,
                },
            ));
        }
        let (cell_widths, base_width) = {
            let widths: Vec<f32> = primary_widths
                .iter()
                .zip(&annotation_widths)
                .map(|(&primary, &annotation)| metrics.cell_width(primary, annotation))
                .collect();
            (widths, metrics.base_width())
        };
        Some(ExpandableListModel::new(
            &cell_widths,
            ExpandableGeometryInput {
                collapsed: if layout == CandidateLayout::VerticalExpandable {
                    CollapsedLayout::Column {
                        metrics: &metrics,
                        primary_widths: &primary_widths,
                        annotation_widths: &annotation_widths,
                    }
                } else {
                    CollapsedLayout::Row
                },
                base_width,
                maximum_window_width: measured.maximum_window_width,
                chrome_width: metrics.candidate_font_size() * CHEVRON_COLUMN_RATIO,
                scroller_width: 0.0,
                item_height: metrics.item_height(),
                page_size,
                row_columns: page_size,
            },
        ))
    }
}

static LAYOUT: OnceLock<Arc<PopupListLayout>> = OnceLock::new();

/// The process-wide measurer both shells install on every [`EngineState`].
pub fn shared_list_layout() -> Arc<PopupListLayout> {
    LAYOUT
        .get_or_init(|| Arc::new(PopupListLayout::new(Arc::new(PopupTextRuler::new()))))
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use taigi_desktop_core::candidates::ExpandableAxis;
    use taigi_desktop_core::session::PAGE_SIZE;

    #[derive(Debug)]
    struct FixedRuler;

    impl TextRuler for FixedRuler {
        fn measure(&self, runs: &[(&str, f32)]) -> Option<Measured> {
            Some(Measured {
                maximum_window_width: 600.0,
                widths: runs
                    .iter()
                    .map(|(text, size)| text.chars().count() as f32 * size / 2.0)
                    .collect(),
            })
        }

        fn size(&self, text: &str, px: f32, _hanji: bool) -> Option<(f32, f32)> {
            Some((text.chars().count() as f32 * px * 0.5, px))
        }
    }

    fn settings_with(layout: CandidateLayout) -> SettingsDocument {
        let mut settings = SettingsDocument::default();
        settings.set_choice(&keys::CANDIDATE_LAYOUT, layout);
        settings
    }

    #[test]
    fn horizontal_is_not_measured() {
        let layout = PopupListLayout::new(Arc::new(FixedRuler));
        let cells = vec![CandidateCellContent::new("a", None)];
        assert!(layout
            .layout(&cells, &SettingsDocument::default(), PAGE_SIZE)
            .is_none());
    }

    #[test]
    fn horizontal_expandable_builds_a_row_model() {
        let layout = PopupListLayout::new(Arc::new(FixedRuler));
        let cells = vec![CandidateCellContent::new("台", Some("tâi".into())); 20];
        let model = layout
            .layout(
                &cells,
                &settings_with(CandidateLayout::Expandable),
                PAGE_SIZE,
            )
            .expect("model");
        assert_eq!(model.axis(), ExpandableAxis::Row);
        assert!(model.collapsed_cells().len() <= PAGE_SIZE);
    }

    #[test]
    fn nine_short_cells_fill_the_collapsed_row_and_every_unfolded_row() {
        let layout = PopupListLayout::new(Arc::new(FixedRuler));
        let mut cells = vec![CandidateCellContent::new("你", Some("lí".into())); 9];
        cells.push(CandidateCellContent::new(
            "這是很長很長的詞",
            Some("tsit".into()),
        ));
        cells.extend(vec![
            CandidateCellContent::new("里", Some("lí".into()));
            20
        ]);
        let mut model = layout
            .layout(
                &cells,
                &settings_with(CandidateLayout::Expandable),
                PAGE_SIZE,
            )
            .expect("model");
        assert_eq!(model.collapsed_cells().len(), 9);
        model.expand();
        let rows = &model.grid().rows;
        assert_eq!(rows[0].len(), 9);
        // The long word wraps early; the short ones after it fill rows of nine.
        assert!(rows[1].len() < 9);
        assert_eq!(rows[2].len(), 9);
    }

    #[test]
    fn a_long_word_ends_its_own_row_and_leaves_the_short_row_full() {
        let layout = PopupListLayout::new(Arc::new(FixedRuler));
        let mut cells = vec![CandidateCellContent::new("字", None); 9];
        cells.push(CandidateCellContent::new("這是很長很長的詞條", None));
        cells.extend(vec![CandidateCellContent::new("字", None); 3]);
        let (starts, floor) = layout
            .horizontal_page_starts(
                &cells,
                &settings_with(CandidateLayout::Horizontal),
                PAGE_SIZE,
            )
            .expect("horizontal");
        assert_eq!(starts[0], 0);
        assert!(starts.get(1).copied().unwrap_or(0) > 1);
        let other = vec![CandidateCellContent::new("人", Some("lâng".into())); 20];
        let (_, other_floor) = layout
            .horizontal_page_starts(
                &other,
                &settings_with(CandidateLayout::Horizontal),
                PAGE_SIZE,
            )
            .expect("horizontal");
        assert!(floor > 0);
        assert_eq!(floor, other_floor);
    }
}
