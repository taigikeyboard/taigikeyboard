//! Lays out a list for the page's own candidate window: the expandable arm of
//! the Windows window's `build_layout`, with the page as the ruler. The session
//! needs the model before its reply (the labels read it), so the page measures
//! during the key, in one batch: `docs/architecture/web-roadmap.md` § Candidate
//! window width source.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use taigi_desktop_core::candidates::{
    CandidateCellArrangement, CandidateChrome, CandidateMetrics, CollapsedLayout,
    ExpandableGeometryInput, ExpandableListModel, FontSpec, HorizontalPageLayout, TextMeasurer,
    MAX_DISPLAY_CANDIDATES,
};
use taigi_desktop_core::composing::CandidateCellContent;
use taigi_desktop_core::session::ListLayout;
use taigi_desktop_core::settings::{
    keys, CandidateFontSelection, CandidateLayout, CandidateSizeChoice, SettingsDocument,
};

/// `#page-arrows`' width against the candidate size.
const CHEVRON_COLUMN_RATIO: f32 = 1.5;
/// The panel's `line-height`.
const LINE_HEIGHT_RATIO: f32 = 1.2;

/// What the page answers.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Measured {
    pub maximum_window_width: f32,
    pub widths: Vec<f32>,
}

/// The widths of `runs` of `(text, font size)` as the page draws them, in
/// order. `None`: the page could not say, and the list stays paged.
pub(crate) trait TextRuler: Debug + Send + Sync {
    fn measure(&self, runs: &[(&str, f32)]) -> Option<Measured>;
}

/// The line box of the panel's text.
fn line_height(font: FontSpec) -> f32 {
    font.size * LINE_HEIGHT_RATIO
}

/// An annotation that draws: `CandidateMetrics::measure_annotation`'s rule.
fn drawn_annotation(cell: &CandidateCellContent) -> Option<&String> {
    cell.annotation.as_ref().filter(|text| !text.is_empty())
}

/// The metrics the page lays a list out with and draws it at: macOS's air.
fn page_metrics(
    settings: &SettingsDocument,
    arrangement: CandidateCellArrangement,
    measurer: &dyn TextMeasurer,
) -> CandidateMetrics {
    CandidateMetrics::resolve_with_chrome(
        settings.choice(&keys::CANDIDATE_SIZE),
        CandidateFontSelection::default(),
        arrangement,
        CandidateChrome::MACOS,
        measurer,
    )
}

/// The lengths the panel draws every list at (`WireLengths`): the font sizes,
/// gaps and paddings, which no measurement and no arrangement moves.
pub(crate) fn page_lengths(settings: &SettingsDocument) -> CandidateMetrics {
    page_metrics(
        settings,
        CandidateCellArrangement::Inline,
        &RunRecorder::default(),
    )
}

/// Writes down what `CandidateMetrics` wants measured. The font sizes do not
/// depend on any answer, so one pass with no answers names every run.
#[derive(Default)]
struct RunRecorder {
    runs: RefCell<Vec<(String, f32)>>,
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

/// The ruler's answers, by font size then text, for `CandidateMetrics` to read.
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

/// A list measured for the panel: what both the expandable model and the
/// horizontal packing read.
struct MeasuredCells {
    metrics: CandidateMetrics,
    primary_widths: Vec<f32>,
    annotation_widths: Vec<Option<f32>>,
    cell_widths: Vec<f32>,
    maximum_window_width: f32,
}

impl MeasuredCells {
    /// `#page-arrows`' width, whole pixels as the stylesheet rounds it: a
    /// fractional column left the row's budget a fraction the page floor
    /// (`page_starts_and_floor`) rounds up past the window.
    fn chrome_width(&self) -> f32 {
        (self.metrics.candidate_font_size() * CHEVRON_COLUMN_RATIO).ceil()
    }
}

/// What a horizontal row's cell widths depend on besides the window.
#[derive(Debug, PartialEq)]
struct RowKey {
    cells: Vec<CandidateCellContent>,
    size: CandidateSizeChoice,
    /// `fontType`, which picks the face the panel draws in (`settings.js`).
    font: Option<String>,
}

/// A horizontal row's cells as measured: what the packing needs except the
/// window's width, which is read again at every key.
#[derive(Debug)]
struct MeasuredRow {
    cell_widths: Vec<f32>,
    slot_width: f32,
    chrome_width: f32,
}

/// The last horizontal list's measurement. The session asks for the pages on
/// every key and the page measures in the DOM, so the text is measured once
/// per list, as the expandable layouts are; `measured` is `None` when the
/// page could not say.
#[derive(Debug)]
struct RowCache {
    key: RowKey,
    measured: Option<MeasuredRow>,
}

#[derive(Debug)]
pub(crate) struct PageListLayout {
    ruler: Arc<dyn TextRuler>,
    row_cache: Mutex<Option<RowCache>>,
}

impl PageListLayout {
    pub(crate) fn new(ruler: Arc<dyn TextRuler>) -> Self {
        Self {
            ruler,
            row_cache: Mutex::default(),
        }
    }

    fn row_cache(&self) -> std::sync::MutexGuard<'_, Option<RowCache>> {
        self.row_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Every cell of `cells` as `layout` draws it, in one batch from the
    /// ruler. `None`: the page could not say.
    fn measure_cells(
        &self,
        cells: &[CandidateCellContent],
        settings: &SettingsDocument,
        layout: CandidateLayout,
    ) -> Option<MeasuredCells> {
        let resolve = |measurer: &dyn TextMeasurer| {
            page_metrics(settings, layout.cell_arrangement(), measurer)
        };
        let recorder = RunRecorder::default();
        let sizes = resolve(&recorder);
        let mut runs = recorder.runs.into_inner();
        let candidate_size = sizes.candidate_font().size;
        let annotation_size = sizes.annotation_font().size;
        for cell in cells {
            runs.push((cell.text.clone(), candidate_size));
            if let Some(annotation) = drawn_annotation(cell) {
                runs.push((annotation.clone(), annotation_size));
            }
        }
        let measured = {
            let borrowed: Vec<(&str, f32)> = runs
                .iter()
                .map(|(text, size)| (text.as_str(), *size))
                .collect();
            self.ruler.measure(&borrowed)?
        };
        let mut answers = HashMap::<u32, HashMap<String, f32>>::new();
        for ((text, size), width) in runs.into_iter().zip(measured.widths) {
            answers
                .entry(size.to_bits())
                .or_default()
                .insert(text, width);
        }
        let answers = MeasuredRuns(answers);
        let has_annotations = cells.iter().any(|cell| drawn_annotation(cell).is_some());
        let metrics = resolve(&answers).for_content(has_annotations);
        let primary_widths: Vec<f32> = cells
            .iter()
            .map(|cell| metrics.measure_primary_width(&cell.text, &answers))
            .collect();
        let annotation_widths: Vec<Option<f32>> = cells
            .iter()
            .map(|cell| metrics.measure_annotation(cell.annotation.as_deref(), &answers))
            .collect();
        let cell_widths: Vec<f32> = primary_widths
            .iter()
            .zip(&annotation_widths)
            .map(|(&primary, &annotation)| metrics.cell_width(primary, annotation))
            .collect();
        Some(MeasuredCells {
            metrics,
            primary_widths,
            annotation_widths,
            cell_widths,
            maximum_window_width: measured.maximum_window_width,
        })
    }

    /// Pages `cells` for a window `maximum_window_width` wide.
    fn pack_row(
        row: &MeasuredRow,
        maximum_window_width: f32,
        count: usize,
        page_size: usize,
    ) -> (Vec<usize>, u32) {
        // The collapsed horizontal-expandable row's packing
        // (`ExpandableListModel::collapsed_row`), so the two layouts break at
        // the same cell and keep one window width.
        // Every page keeps the packing budget as its width once the list runs
        // past one (`HorizontalCandidatePanel.rebuildVisiblePage` on macOS).
        // The cell widths count no gap, so there is none to leave out.
        HorizontalPageLayout::pack_with_chrome(
            &row.cell_widths,
            row.slot_width,
            maximum_window_width,
            row.chrome_width,
            page_size,
        )
        .page_starts_and_floor(count, page_size, 0.0)
    }
}

impl ListLayout for PageListLayout {
    fn layout(
        &self,
        cells: &[CandidateCellContent],
        settings: &SettingsDocument,
        page_size: usize,
    ) -> Option<ExpandableListModel> {
        // A new list, or cells changed in place: the row is measured again,
        // as an expandable list is — a font that loaded meanwhile included.
        *self.row_cache() = None;
        let layout: CandidateLayout = settings.choice(&keys::CANDIDATE_LAYOUT);
        if !matches!(
            layout,
            CandidateLayout::Expandable | CandidateLayout::VerticalExpandable
        ) {
            return None;
        }
        let measured = self.measure_cells(cells, settings, layout)?;
        Some(ExpandableListModel::new(
            &measured.cell_widths,
            ExpandableGeometryInput {
                collapsed: if layout == CandidateLayout::VerticalExpandable {
                    CollapsedLayout::Column {
                        metrics: &measured.metrics,
                        primary_widths: &measured.primary_widths,
                        annotation_widths: &measured.annotation_widths,
                    }
                } else {
                    CollapsedLayout::Row
                },
                base_width: measured.metrics.base_width(),
                maximum_window_width: measured.maximum_window_width,
                chrome_width: measured.chrome_width(),
                // Hidden: a classic scrollbar takes a width no constant here
                // would match.
                scroller_width: 0.0,
                item_height: measured.metrics.item_height(),
                page_size,
                row_columns: 0,
            },
        ))
    }

    /// Pages a horizontal list where its measured cells fill a row, and
    /// gives every page of a list that runs past one the same width — the
    /// Linux popup's rule (`taigi-linux-core` `PopupListLayout`).
    fn horizontal_page_starts(
        &self,
        cells: &[CandidateCellContent],
        settings: &SettingsDocument,
        page_size: usize,
    ) -> Option<(Vec<usize>, u32)> {
        if settings.choice::<CandidateLayout>(&keys::CANDIDATE_LAYOUT)
            != CandidateLayout::Horizontal
            || cells.is_empty()
        {
            return None;
        }
        let key = RowKey {
            cells: cells.to_vec(),
            size: settings.choice(&keys::CANDIDATE_SIZE),
            font: settings.raw_string("fontType").map(str::to_owned),
        };
        let mut cache = self.row_cache();
        let (row, maximum_window_width) = match cache.as_ref() {
            Some(cached) if cached.key == key => {
                // The text is as measured; the window may have changed width
                // (a resize, the sidebar), so read only that.
                let row = cached.measured.as_ref()?;
                (row, self.ruler.measure(&[])?.maximum_window_width)
            }
            _ => {
                let shown = &cells[..cells.len().min(MAX_DISPLAY_CANDIDATES)];
                let measured = self.measure_cells(shown, settings, CandidateLayout::Horizontal);
                let maximum_window_width = measured
                    .as_ref()
                    .map_or(0.0, |measured| measured.maximum_window_width);
                let cached = cache.insert(RowCache {
                    key,
                    measured: measured.map(|measured| MeasuredRow {
                        slot_width: measured.metrics.base_width(),
                        chrome_width: measured.chrome_width(),
                        cell_widths: measured.cell_widths,
                    }),
                });
                (cached.measured.as_ref()?, maximum_window_width)
            }
        };
        Some(Self::pack_row(
            row,
            maximum_window_width,
            cells.len(),
            page_size,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use taigi_desktop_core::candidates::ExpandableAxis;
    use taigi_desktop_core::session::PAGE_SIZE;

    /// Every character `size / 2` wide, and it keeps what it was asked.
    #[derive(Debug)]
    struct FixedRuler {
        maximum_window_width: Mutex<f32>,
        asked: Mutex<Vec<(String, f32)>>,
        /// Every call, a window-only read (no runs) included.
        calls: Mutex<usize>,
    }

    impl FixedRuler {
        fn new() -> Self {
            Self {
                maximum_window_width: Mutex::new(600.0),
                asked: Mutex::default(),
                calls: Mutex::default(),
            }
        }

        fn asked(&self) -> usize {
            self.asked.lock().unwrap().len()
        }
    }

    impl TextRuler for FixedRuler {
        fn measure(&self, runs: &[(&str, f32)]) -> Option<Measured> {
            *self.calls.lock().unwrap() += 1;
            let mut asked = self.asked.lock().unwrap();
            asked.extend(runs.iter().map(|(text, size)| ((*text).to_owned(), *size)));
            Some(Measured {
                maximum_window_width: *self.maximum_window_width.lock().unwrap(),
                widths: runs
                    .iter()
                    .map(|(text, size)| text.chars().count() as f32 * size / 2.0)
                    .collect(),
            })
        }
    }

    #[derive(Debug)]
    struct RefusingRuler;

    impl TextRuler for RefusingRuler {
        fn measure(&self, _runs: &[(&str, f32)]) -> Option<Measured> {
            None
        }
    }

    fn settings_with(layout: CandidateLayout) -> SettingsDocument {
        let mut settings = SettingsDocument::default();
        settings.set_choice(&keys::CANDIDATE_LAYOUT, layout);
        settings
    }

    fn annotated(count: usize) -> Vec<CandidateCellContent> {
        (0..count)
            .map(|_| CandidateCellContent::new("台語", Some("tâi-gí".to_owned())))
            .collect()
    }

    fn lay_out(
        layout: CandidateLayout,
        cells: &[CandidateCellContent],
    ) -> Option<ExpandableListModel> {
        PageListLayout::new(Arc::new(FixedRuler::new())).layout(
            cells,
            &settings_with(layout),
            PAGE_SIZE,
        )
    }

    #[test]
    fn a_paged_layout_is_never_measured() {
        let ruler = Arc::new(FixedRuler::new());
        let layout = PageListLayout::new(ruler.clone());
        for choice in [CandidateLayout::Horizontal, CandidateLayout::Vertical] {
            assert!(layout
                .layout(&annotated(3), &settings_with(choice), PAGE_SIZE)
                .is_none());
        }
        assert!(ruler.asked.lock().unwrap().is_empty());
    }

    #[test]
    fn each_layout_takes_its_axis() {
        let row = lay_out(CandidateLayout::Expandable, &annotated(30)).expect("a row");
        assert_eq!(row.axis(), ExpandableAxis::Row);
        let column =
            lay_out(CandidateLayout::VerticalExpandable, &annotated(30)).expect("a column");
        assert_eq!(column.axis(), ExpandableAxis::Column);
        assert_eq!(column.collapsed_cells().len(), 9);
        assert!(column.has_overflow());
    }

    #[test]
    fn scripts_are_measured_at_their_own_sizes() {
        let ruler = Arc::new(FixedRuler::new());
        PageListLayout::new(ruler.clone())
            .layout(
                &annotated(1),
                &settings_with(CandidateLayout::Expandable),
                PAGE_SIZE,
            )
            .expect("a model");
        let asked = ruler.asked.lock().unwrap();
        // trace: Standard is 17 pt; annotation 14 × 17 / 16 = 14.875, rounds to 15.
        assert!(asked.contains(&("台語".to_owned(), 17.0)));
        assert!(asked.contains(&("tâi-gí".to_owned(), 15.0)));
        // What `CandidateMetrics` asks for itself goes in the same batch.
        assert!(asked.contains(&("永".to_owned(), 17.0)));
    }

    #[test]
    fn a_list_with_no_second_script_has_one_line_rows() {
        let plain = vec![CandidateCellContent::new("tâi", None); 30];
        let one_line = lay_out(CandidateLayout::Expandable, &plain).expect("a model");
        let two_lines = lay_out(CandidateLayout::Expandable, &annotated(30)).expect("a model");
        assert!(one_line.row_height() < two_lines.row_height());
    }

    fn horizontal_pages(
        ruler: &Arc<FixedRuler>,
        cells: &[CandidateCellContent],
        layout: CandidateLayout,
    ) -> Option<(Vec<usize>, u32)> {
        PageListLayout::new(ruler.clone()).horizontal_page_starts(
            cells,
            &settings_with(layout),
            PAGE_SIZE,
        )
    }

    #[test]
    fn only_a_horizontal_list_is_paged_by_width() {
        let ruler = Arc::new(FixedRuler::new());
        for choice in [
            CandidateLayout::Vertical,
            CandidateLayout::Expandable,
            CandidateLayout::VerticalExpandable,
        ] {
            assert!(horizontal_pages(&ruler, &annotated(30), choice).is_none());
        }
        assert!(ruler.asked.lock().unwrap().is_empty());
    }

    #[test]
    fn a_horizontal_list_that_runs_past_a_page_keeps_the_collapsed_rows_width() {
        let ruler = Arc::new(FixedRuler::new());
        let cells = annotated(30);
        let (starts, floor) =
            horizontal_pages(&ruler, &cells, CandidateLayout::Horizontal).expect("pages");
        assert_eq!(starts[0], 0);
        assert!(starts.len() > 1);
        assert!(starts.windows(2).all(|pair| pair[1] - pair[0] <= PAGE_SIZE));
        // The two horizontal layouts break at the same cell and keep one width.
        let row = lay_out(CandidateLayout::Expandable, &cells).expect("a row");
        assert_eq!(row.collapsed_cells().len(), starts[1]);
        assert_eq!(floor, row.collapsed_content_width().ceil() as u32);
    }

    #[test]
    fn a_horizontal_list_that_fits_one_page_stays_short() {
        let ruler = Arc::new(FixedRuler::new());
        let (starts, floor) =
            horizontal_pages(&ruler, &annotated(3), CandidateLayout::Horizontal).expect("pages");
        assert_eq!(starts, vec![0]);
        assert_eq!(floor, 0);
    }

    #[test]
    fn longer_words_take_fewer_cells_to_a_page() {
        let ruler = Arc::new(FixedRuler::new());
        let short = annotated(30);
        let long: Vec<_> = (0..30)
            .map(|_| CandidateCellContent::new("台語文學", Some("tâi-gí-bûn-ha̍k".to_owned())))
            .collect();
        let (short_starts, short_floor) =
            horizontal_pages(&ruler, &short, CandidateLayout::Horizontal).expect("pages");
        let (long_starts, long_floor) =
            horizontal_pages(&ruler, &long, CandidateLayout::Horizontal).expect("pages");
        assert!(long_starts[1] < short_starts[1]);
        // The width is the budget's, whatever the words.
        assert_eq!(short_floor, long_floor);
    }

    #[test]
    fn the_same_list_is_measured_once_and_the_window_read_at_every_key() {
        let ruler = Arc::new(FixedRuler::new());
        let layout = PageListLayout::new(ruler.clone());
        let settings = settings_with(CandidateLayout::Horizontal);
        let cells = annotated(30);
        let first = layout.horizontal_page_starts(&cells, &settings, PAGE_SIZE);
        let asked = ruler.asked();
        let calls = *ruler.calls.lock().unwrap();
        let again = layout.horizontal_page_starts(&cells, &settings, PAGE_SIZE);
        assert_eq!(first, again);
        assert_eq!(ruler.asked(), asked);
        assert_eq!(*ruler.calls.lock().unwrap(), calls + 1);
        // A narrower window (a resize, the sidebar) repacks the same widths.
        *ruler.maximum_window_width.lock().unwrap() = 200.0;
        let narrow = layout
            .horizontal_page_starts(&cells, &settings, PAGE_SIZE)
            .expect("pages");
        assert_eq!(ruler.asked(), asked);
        let (first_starts, first_floor) = first.expect("pages");
        assert!(narrow.0[1] < first_starts[1]);
        assert!(narrow.1 < first_floor);
    }

    #[test]
    fn a_new_list_or_another_font_measures_again() {
        let ruler = Arc::new(FixedRuler::new());
        let layout = PageListLayout::new(ruler.clone());
        let mut settings = settings_with(CandidateLayout::Horizontal);
        let cells = annotated(30);
        layout.horizontal_page_starts(&cells, &settings, PAGE_SIZE);
        let asked = ruler.asked();
        // The session lays out every new list first, the same cells too.
        assert!(layout.layout(&cells, &settings, PAGE_SIZE).is_none());
        layout.horizontal_page_starts(&cells, &settings, PAGE_SIZE);
        let relaid = ruler.asked();
        assert!(relaid > asked);
        settings.set_raw_string("fontType", "iansui");
        layout.horizontal_page_starts(&cells, &settings, PAGE_SIZE);
        assert!(ruler.asked() > relaid);
    }

    #[test]
    fn cells_past_the_packer_take_equal_pages() {
        let ruler = Arc::new(FixedRuler::new());
        let count = MAX_DISPLAY_CANDIDATES + 20;
        let (starts, _) = horizontal_pages(&ruler, &annotated(count), CandidateLayout::Horizontal)
            .expect("pages");
        let tail: Vec<usize> = starts
            .iter()
            .copied()
            .filter(|&start| start >= MAX_DISPLAY_CANDIDATES)
            .collect();
        assert_eq!(tail.first(), Some(&MAX_DISPLAY_CANDIDATES));
        assert!(tail.windows(2).all(|pair| pair[1] - pair[0] == PAGE_SIZE));
        assert!(count - tail.last().unwrap() <= PAGE_SIZE);
    }

    #[test]
    fn a_ruler_that_cannot_answer_leaves_the_list_paged() {
        let layout = PageListLayout::new(Arc::new(RefusingRuler));
        assert!(layout
            .layout(
                &annotated(3),
                &settings_with(CandidateLayout::Expandable),
                PAGE_SIZE
            )
            .is_none());
    }
}
