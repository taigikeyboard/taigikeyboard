//! Every point value the candidate window's geometry is built from, resolved
//! once from the size choice and the typeface. Port of
//! `CandidateMetrics.swift`; the AppKit measurements go through the
//! [`TextMeasurer`] the renderer supplies (DirectWrite in PR6, a stub in tests).

use super::index_label::CandidateIndexLabel;
use crate::composing::CandidateCellContent;
use crate::settings::{CandidateFontChoice, CandidateFontSelection, CandidateSizeChoice};

/// Where a cell puts the candidate's second script (`CandidateCellArrangement.swift`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CandidateCellArrangement {
    /// Annotation beside the candidate, sharing its baseline.
    Inline,
    /// Annotation under the candidate, both centred.
    Stacked,
}

/// A face and size to measure in. `System` is the platform UI font — the
/// index hint always uses it, whatever the candidate typeface
/// (`CandidateMetrics.swift:126`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontSpec {
    pub selection: CandidateFontSelection,
    pub size: f32,
}

/// What the renderer knows and the geometry needs: how wide a string is in
/// a face, and how tall that face's line box is.
pub trait TextMeasurer {
    /// Advance width of `text` set in `font`, in points, rounded up.
    fn width(&self, text: &str, font: FontSpec) -> f32;
    /// The line box `font` draws in.
    fn line_height(&self, font: FontSpec) -> f32;
}

/// Reference values at 16 pt (`CandidateMetrics.swift:192-211`).
const BASE_CANDIDATE_FONT_SIZE: f32 = 16.0;
const BASE_ANNOTATION_FONT_SIZE: f32 = 14.0;
const BASE_CANDIDATE_ANNOTATION_GAP: f32 = 7.0;
const BASE_INDEX_FONT_SIZE: f32 = 10.0;
const BASE_INDEX_CANDIDATE_GAP: f32 = 2.0;
/// What the slot adds to the digit's own width — enough for a digit's
/// bearing on either side. Never scaled.
const INDEX_SLOT_PADDING: f32 = 2.0;
const BASE_HORIZONTAL_PADDING: f32 = 9.0;
const BASE_VERTICAL_PADDING: f32 = 12.0;
const BASE_STACKED_LINE_GAP: f32 = 2.0;
const BASE_TAHOE_SEPARATOR_INSET: f32 = 8.0;
/// What a window of two-line cells rounds to — fixed, not scaled.
const STACKED_CONTAINER_CORNER_RADIUS: f32 = 16.0;
const INLINE_HIGHLIGHT_INSET: f32 = 2.0;
const STACKED_HIGHLIGHT_INSET: f32 = 4.0;
/// How much of the reference air the window keeps: the paddings are the base
/// values times this, times the text scale, so the air grows and shrinks with
/// the glyphs at one fixed ratio (USER 2026-09-23).
///
/// NAMED DIVERGENCE from `CandidateMetrics.swift`'s `chromeRatio` 0.7 (USER
/// 2026-09-01, first real-Windows dogfood): the point values are read as DIPs
/// here and as points on the Mac, so the same number lands differently
/// against the platform's own chrome, and the window carried too much air on
/// Windows. Each platform's ratio is its tightest step of the former
/// window-size ladder (Mac 0.7 / 0.85 / 1.0, Windows 0.6 / 0.72 / 0.85).
const CHROME_RATIO: f32 = 0.6;

/// The metrics, resolved. Immutable: a size change rebuilds the window.
#[derive(Clone, Debug, PartialEq)]
pub struct CandidateMetrics {
    font_selection: CandidateFontSelection,
    cell_arrangement: CandidateCellArrangement,
    candidate_font_size: f32,
    annotation_font_size: f32,
    candidate_annotation_gap: f32,
    index_font_size: f32,
    index_candidate_gap: f32,
    horizontal_padding: f32,
    vertical_padding: f32,
    stacked_line_gap: f32,
    tahoe_separator_inset: f32,
    /// How tall one cell renders: inline = one line of candidate; stacked =
    /// two line boxes plus the air between, kept for an unannotated cell in
    /// annotated content so a page's rows line up — and one line when NO
    /// cell in the content carries an annotation (`for_content`).
    item_height: f32,
    /// The two-line stacked box, kept so `for_content` can go back to it
    /// from a single-line variant. Mirrors `item_height` under Inline.
    stacked_item_height: f32,
    /// The stacked candidate and annotation line boxes as measured, `None`
    /// under Inline (never measured there) — what a stacked cell's paint
    /// centres by.
    stacked_line_heights: Option<(f32, f32)>,
    /// The slot the key is centred in — wide enough for every form it can
    /// take, so the column keeps one width as the live key set changes.
    index_width: f32,
    /// The candidate column never renders narrower than one full-width glyph.
    primary_column_floor: f32,
}

impl CandidateMetrics {
    /// Resolves every value. Scaled values round to whole points; the size
    /// choice scales the fonts, the text-anchored gaps and — through
    /// `CHROME_RATIO` — the paddings, all by the same text scale.
    pub fn resolve(
        size: CandidateSizeChoice,
        font_selection: CandidateFontSelection,
        cell_arrangement: CandidateCellArrangement,
        measurer: &dyn TextMeasurer,
    ) -> Self {
        let candidate_font_size = size.font_size();
        let text_scale = candidate_font_size / BASE_CANDIDATE_FONT_SIZE;
        let chrome_scale = CHROME_RATIO * text_scale;
        let annotation_font_size = (BASE_ANNOTATION_FONT_SIZE * text_scale).round();
        let index_font_size = (BASE_INDEX_FONT_SIZE * text_scale).round();
        let stacked_line_gap = (BASE_STACKED_LINE_GAP * text_scale).round();
        let vertical_padding = (BASE_VERTICAL_PADDING * chrome_scale).round();
        let candidate_font = FontSpec {
            selection: font_selection,
            size: candidate_font_size,
        };
        let annotation_font = FontSpec {
            selection: font_selection,
            size: annotation_font_size,
        };
        let index_font = FontSpec {
            selection: CandidateFontSelection::BuiltIn(CandidateFontChoice::System),
            size: index_font_size,
        };
        let stacked_line_heights = matches!(cell_arrangement, CandidateCellArrangement::Stacked)
            .then(|| {
                (
                    measurer.line_height(candidate_font),
                    measurer.line_height(annotation_font),
                )
            });
        let item_height = match stacked_line_heights {
            // The point size is what the bundled faces need: their line boxes
            // fit the row it gives. A face the user brought, or one the OS
            // supplies, has no such guarantee — tall ascenders, stacked
            // diacritics and a fallback glyph all draw outside it — so those
            // are given their own measured line box instead. Bundled
            // selections keep the arithmetic they shipped with, exactly
            // (`CandidateMetrics.swift`'s inline arm).
            None if font_selection.requires_line_box_measurement() => {
                (candidate_font_size.max(measurer.line_height(candidate_font)) + vertical_padding)
                    .ceil()
            }
            None => candidate_font_size + vertical_padding,
            Some((candidate_line, annotation_line)) => {
                (candidate_line + annotation_line + stacked_line_gap + vertical_padding).ceil()
            }
        };
        let index_width = CandidateIndexLabel::widest_label_forms()
            .iter()
            .map(|form| measurer.width(form, index_font).ceil())
            .fold(index_font_size, f32::max)
            + INDEX_SLOT_PADDING;
        let primary_column_floor =
            candidate_font_size.max(measurer.width("永", candidate_font).ceil());
        Self {
            font_selection,
            cell_arrangement,
            candidate_font_size,
            annotation_font_size,
            candidate_annotation_gap: (BASE_CANDIDATE_ANNOTATION_GAP * text_scale).round(),
            index_font_size,
            index_candidate_gap: (BASE_INDEX_CANDIDATE_GAP * text_scale).round(),
            horizontal_padding: (BASE_HORIZONTAL_PADDING * chrome_scale).round(),
            vertical_padding,
            stacked_line_gap,
            tahoe_separator_inset: (BASE_TAHOE_SEPARATOR_INSET * chrome_scale).round(),
            item_height,
            stacked_item_height: item_height,
            stacked_line_heights,
            index_width,
            primary_column_floor,
        }
    }

    /// These metrics for content that does or does not carry an annotated
    /// cell: a stacked cell is one line tall (the inline height) when nothing
    /// in the list has an annotation — Romanization Only, or Combined's one-script cells —
    /// and the two-line box otherwise, so Pairing's mixed lists keep lining up.
    /// Inline is one line either way. Idempotent: resolved once per list,
    /// on `show` and on `update_cells`.
    pub fn for_content(&self, has_annotations: bool) -> Self {
        let item_height = match self.cell_arrangement {
            CandidateCellArrangement::Inline => self.item_height,
            CandidateCellArrangement::Stacked if has_annotations => self.stacked_item_height,
            CandidateCellArrangement::Stacked => self.candidate_font_size + self.vertical_padding,
        };
        Self {
            item_height,
            ..self.clone()
        }
    }

    /// The two stacked line boxes `resolve` measured, `None` under Inline —
    /// what a stacked cell's paint centres by.
    pub fn stacked_line_heights(&self) -> Option<(f32, f32)> {
        self.stacked_line_heights
    }

    // Read-only: a size change rebuilds the window (`resolve`), so no two
    // values here can disagree.
    pub fn font_selection(&self) -> CandidateFontSelection {
        self.font_selection
    }

    pub fn cell_arrangement(&self) -> CandidateCellArrangement {
        self.cell_arrangement
    }

    pub fn candidate_font_size(&self) -> f32 {
        self.candidate_font_size
    }

    pub fn annotation_font_size(&self) -> f32 {
        self.annotation_font_size
    }

    pub fn candidate_annotation_gap(&self) -> f32 {
        self.candidate_annotation_gap
    }

    pub fn index_font_size(&self) -> f32 {
        self.index_font_size
    }

    pub fn index_candidate_gap(&self) -> f32 {
        self.index_candidate_gap
    }

    pub fn horizontal_padding(&self) -> f32 {
        self.horizontal_padding
    }

    pub fn vertical_padding(&self) -> f32 {
        self.vertical_padding
    }

    pub fn stacked_line_gap(&self) -> f32 {
        self.stacked_line_gap
    }

    pub fn tahoe_separator_inset(&self) -> f32 {
        self.tahoe_separator_inset
    }

    pub fn item_height(&self) -> f32 {
        self.item_height
    }

    pub fn index_width(&self) -> f32 {
        self.index_width
    }

    pub fn primary_column_floor(&self) -> f32 {
        self.primary_column_floor
    }

    pub fn candidate_font(&self) -> FontSpec {
        FontSpec {
            selection: self.font_selection,
            size: self.candidate_font_size,
        }
    }

    pub fn annotation_font(&self) -> FontSpec {
        FontSpec {
            selection: self.font_selection,
            size: self.annotation_font_size,
        }
    }

    /// The system font, never the user's candidate typeface.
    pub fn index_font(&self) -> FontSpec {
        FontSpec {
            selection: CandidateFontSelection::BuiltIn(CandidateFontChoice::System),
            size: self.index_font_size,
        }
    }

    /// The radius the window rounds to under the newer chrome: a capsule for
    /// inline cells, a fixed rounded rectangle for stacked ones.
    pub fn tahoe_container_corner_radius(&self) -> f32 {
        match self.cell_arrangement {
            CandidateCellArrangement::Inline => self.item_height / 2.0,
            CandidateCellArrangement::Stacked => STACKED_CONTAINER_CORNER_RADIUS,
        }
    }

    pub fn tahoe_highlight_inset(&self) -> f32 {
        match self.cell_arrangement {
            CandidateCellArrangement::Inline => INLINE_HIGHLIGHT_INSET,
            CandidateCellArrangement::Stacked => STACKED_HIGHLIGHT_INSET,
        }
    }

    /// Concentric with the window: the outer radius less the padding between.
    pub fn tahoe_highlight_corner_radius(&self) -> f32 {
        self.tahoe_container_corner_radius() - self.tahoe_highlight_inset()
    }

    /// `radius` held to what a box of `width × height` can round — the one
    /// clamp both the window and the selection go through.
    pub fn corner_radius(radius: f32, width: f32, height: f32) -> f32 {
        radius.min(width.min(height) / 2.0)
    }

    /// `base`, stated at 16 pt, scaled to this text size and rounded — what
    /// the chevron and page-arrow views size their symbols by.
    pub fn scaled_symbol_metric(&self, base: f32) -> f32 {
        (base * self.candidate_font_size / BASE_CANDIDATE_FONT_SIZE).round()
    }

    /// The narrowest a cell renders: one full-width glyph and no annotation.
    /// The packing budget must agree with `measure_width` about minimums or a
    /// page drops a column.
    pub fn base_width(&self) -> f32 {
        2.0 * self.horizontal_padding + self.index_column_width() + self.primary_column_floor
    }

    /// What the key column costs a cell: its slot plus the gap after it.
    /// Charged to every cell, including ones whose position carries no key.
    pub fn index_column_width(&self) -> f32 {
        self.index_width + self.index_candidate_gap
    }

    /// The width a cell wants for `cell`.
    pub fn measure_width(&self, cell: &CandidateCellContent, measurer: &dyn TextMeasurer) -> f32 {
        self.cell_width(
            self.measure_primary_width(&cell.text, measurer),
            self.measure_annotation(cell.annotation.as_deref(), measurer),
        )
    }

    /// `measure_width` for a cell whose two texts are already measured:
    /// `primary_width` at the candidate font, `annotation_width` at the
    /// annotation font, or `None` for an absent or empty annotation. The
    /// window measures every cell's texts once per list
    /// (`CandidateWindow::measure_cells`) and lays out from those, rather than
    /// building a second DirectWrite layout per text just to sum the same
    /// numbers with the chrome.
    pub fn cell_width(&self, primary_width: f32, annotation_width: Option<f32>) -> f32 {
        let text = self.primary_column_floor.max(primary_width);
        match self.cell_arrangement {
            // The gap is charged beside an annotation that is there, whatever
            // it measured: an absent second script must cost the cell no width.
            CandidateCellArrangement::Inline => {
                self.horizontal_padding
                    + self.index_column_width()
                    + text
                    + annotation_width.map_or(0.0, |width| self.candidate_annotation_gap + width)
                    + self.horizontal_padding
            }
            // The two scripts are on top of each other, so the cell is as wide
            // as the WIDER of them — not both plus a gap.
            CandidateCellArrangement::Stacked => {
                self.horizontal_padding
                    + self.index_column_width()
                    + text.max(annotation_width.unwrap_or(0.0))
                    + self.horizontal_padding
            }
        }
    }

    /// The candidate column's width for `text` alone, at the candidate font.
    pub fn measure_primary_width(&self, text: &str, measurer: &dyn TextMeasurer) -> f32 {
        measurer.width(text, self.candidate_font()).ceil()
    }

    /// The widest the candidate column can be in a cell `cell_width` across.
    pub fn maximum_primary_column_width(&self, cell_width: f32, trailing_inset: f32) -> f32 {
        self.candidate_font_size
            .max(cell_width - self.horizontal_padding - self.index_column_width() - trailing_inset)
    }

    /// The annotation's own width at the annotation font, or `None` for an
    /// absent or empty one — the shape `cell_width` takes.
    pub fn measure_annotation(
        &self,
        annotation: Option<&str>,
        measurer: &dyn TextMeasurer,
    ) -> Option<f32> {
        annotation
            .filter(|a| !a.is_empty())
            .map(|annotation| measurer.width(annotation, self.annotation_font()).ceil())
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    /// A measurer with arithmetic answers: every char is one em wide (CJK) or
    /// half an em (ASCII), the line box is 1.2 em. Enough to pin every rule
    /// that does not depend on a real face.
    pub struct EmMeasurer;

    impl TextMeasurer for EmMeasurer {
        fn width(&self, text: &str, font: FontSpec) -> f32 {
            text.chars()
                .map(|c| {
                    if c.is_ascii() {
                        font.size * 0.5
                    } else {
                        font.size
                    }
                })
                .sum::<f32>()
                .ceil()
        }
        fn line_height(&self, font: FontSpec) -> f32 {
            (font.size * 1.2).ceil()
        }
    }

    pub fn metrics(
        size: CandidateSizeChoice,
        arrangement: CandidateCellArrangement,
    ) -> CandidateMetrics {
        CandidateMetrics::resolve(
            size,
            CandidateFontSelection::default(),
            arrangement,
            &EmMeasurer,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;
    use crate::settings::{CandidateFontChoice, CustomFontId, SettingChoice};
    use CandidateCellArrangement::{Inline, Stacked};
    use CandidateSizeChoice as S;

    #[test]
    fn every_step_resolves_the_traced_values() {
        // The text values follow the macOS trace (CandidateMetricsTests.swift);
        // the chrome is the tighter Windows ratio (`CHROME_RATIO` 0.6) times
        // the text scale. trace, scale = size / 16, chrome = 0.6 * scale:
        // 13 — scale .8125: ann 11.375→11, gap 5.6875→6, h 9*.4875=4.39→4,
        //      v 12*.4875=5.85→6, tahoe 8*.4875=3.9→4, item 13+6=19;
        // 15 — scale .9375: ann 13.125→13, gap 6.5625→7, h 5.06→5, v 6.75→7,
        //      tahoe 4.5→5, item 22;
        // 17 — scale 1.0625: ann 14.875→15, gap 7.4375→7, h 5.74→6, v 7.65→8,
        //      tahoe 5.1→5, item 25;
        // 20 — scale 1.25: ann 17.5→18, gap 8.75→9, h 6.75→7, v 9, tahoe 6,
        //      item 29;
        // 23 — scale 1.4375: ann 20.125→20, gap 10.06→10, h 7.76→8,
        //      v 10.35→10, tahoe 6.9→7, item 33.
        let resolved: Vec<_> = S::ALL
            .iter()
            .map(|size| {
                let m = metrics(*size, Inline);
                (
                    m.candidate_font_size(),
                    m.annotation_font_size(),
                    m.candidate_annotation_gap(),
                    m.horizontal_padding(),
                    m.vertical_padding(),
                    m.tahoe_separator_inset(),
                    m.item_height(),
                )
            })
            .collect();
        assert_eq!(
            resolved,
            [
                (13.0, 11.0, 6.0, 4.0, 6.0, 4.0, 19.0),
                (15.0, 13.0, 7.0, 5.0, 7.0, 5.0, 22.0),
                (17.0, 15.0, 7.0, 6.0, 8.0, 5.0, 25.0),
                (20.0, 18.0, 9.0, 7.0, 9.0, 6.0, 29.0),
                (23.0, 20.0, 10.0, 8.0, 10.0, 7.0, 33.0),
            ]
        );
        // The symbol scaler is anchored at the 16 pt reference.
        let m = metrics(S::Large, Inline);
        assert_eq!(m.scaled_symbol_metric(8.0), 10.0);
    }

    #[test]
    fn every_step_is_whole_distinct_and_larger_than_the_last() {
        let mut previous: Option<CandidateMetrics> = None;
        for size in S::ALL {
            let m = metrics(*size, Inline);
            for value in [
                m.annotation_font_size(),
                m.candidate_annotation_gap(),
                m.index_font_size(),
                m.index_candidate_gap(),
                m.horizontal_padding(),
                m.vertical_padding(),
                m.stacked_line_gap(),
                m.tahoe_separator_inset(),
            ] {
                assert_eq!(value, value.round(), "{size:?}: {value}");
            }
            if let Some(previous) = previous {
                // One knob: the text AND the air grow with every step.
                assert!(m.candidate_font_size() > previous.candidate_font_size());
                assert!(m.item_height() > previous.item_height(), "{size:?}");
                assert!(m.horizontal_padding() >= previous.horizontal_padding());
                assert!(m.vertical_padding() >= previous.vertical_padding());
            }
            previous = Some(m);
        }
    }

    #[test]
    fn corner_geometry_matches_macos() {
        // trace: CandidateMetricsTests.swift:132-203.
        let inline = metrics(S::Standard, Inline);
        assert_eq!(
            inline.tahoe_container_corner_radius(),
            inline.item_height / 2.0
        );
        assert_eq!(inline.tahoe_highlight_inset(), 2.0);
        for size in S::ALL {
            let stacked = metrics(*size, Stacked);
            assert_eq!(stacked.tahoe_container_corner_radius(), 16.0);
            assert_eq!(stacked.tahoe_highlight_inset(), 4.0);
            assert_eq!(stacked.tahoe_highlight_corner_radius(), 12.0);
            assert!(
                16.0 < stacked.item_height() / 2.0,
                "{size:?} {}",
                stacked.item_height()
            );
        }
        assert_eq!(CandidateMetrics::corner_radius(16.0, 200.0, 57.0), 16.0);
        assert_eq!(CandidateMetrics::corner_radius(16.0, 10.0, 57.0), 5.0);
        assert_eq!(CandidateMetrics::corner_radius(16.0, 200.0, 24.0), 12.0);
    }

    #[test]
    fn stacked_content_without_annotations_is_one_line_tall() {
        // trace: Standard — inline item 17+8=25; stacked resolve =
        // ceil(21+18+2+8)=49 (lines ceil(17*1.2)=21 / ceil(15*1.2)=18, gap
        // 2*1.0625=2.125→2). No annotated cell → 25 (the inline height); any
        // annotated cell → 49; Inline is 25 either way and measures no
        // stacked lines; the variant round-trips back to the resolved box.
        for size in S::ALL {
            let stacked = metrics(*size, Stacked);
            let inline = metrics(*size, Inline);
            let single = stacked.for_content(false);
            assert_eq!(single.item_height(), inline.item_height(), "{size:?}");
            assert_eq!(
                single.stacked_line_heights(),
                stacked.stacked_line_heights()
            );
            assert!(stacked.stacked_line_heights().is_some());
            assert!(inline.stacked_line_heights().is_none());
            assert_eq!(stacked.for_content(true), stacked);
            assert_eq!(single.for_content(true), stacked, "round-trips");
            assert_eq!(single.for_content(false), single, "idempotent");
            assert_eq!(
                inline.for_content(false).item_height(),
                inline.item_height()
            );
            assert_eq!(inline.for_content(true).item_height(), inline.item_height());
        }
        let stacked = metrics(S::Standard, Stacked);
        assert_eq!(stacked.item_height(), 49.0);
        assert_eq!(stacked.for_content(false).item_height(), 25.0);
    }

    #[test]
    fn width_arithmetic_matches_macos() {
        // trace: CandidateMetricsTests.swift:220-353.
        let m = metrics(S::Standard, Inline);
        let measurer = EmMeasurer;
        assert_eq!(m.measure_annotation(None, &measurer), None);
        assert_eq!(m.measure_annotation(Some(""), &measurer), None);
        let cell = CandidateCellContent::new("tâi", Some("台".into()));
        let expected = m.horizontal_padding()
            + m.index_width()
            + m.index_candidate_gap()
            + m.measure_primary_width("tâi", &measurer)
            + m.candidate_annotation_gap()
            + measurer.width("台", m.annotation_font()).ceil()
            + m.horizontal_padding();
        assert!((m.measure_width(&cell, &measurer) - expected).abs() < 0.01);
        assert!(
            (m.base_width() - m.measure_width(&CandidateCellContent::new("永", None), &measurer))
                .abs()
                < 0.01
        );
        assert_eq!(
            m.base_width(),
            2.0 * m.horizontal_padding() + m.index_column_width() + m.primary_column_floor()
        );
        assert!(m.index_width() > measurer.width("9", m.index_font()) + 2.0);
        assert_eq!(
            m.index_column_width(),
            m.index_width() + m.index_candidate_gap()
        );
        assert_eq!(
            m.maximum_primary_column_width(300.0, m.horizontal_padding()),
            300.0 - (m.base_width() - m.primary_column_floor())
        );
        assert_eq!(
            m.maximum_primary_column_width(1.0, 0.0),
            m.candidate_font_size()
        );

        let stacked = metrics(S::Standard, Stacked);
        let wide_annotation = CandidateCellContent::new("tâi", Some("台語齒盤".into()));
        let stacked_width = stacked.measure_width(&wide_annotation, &measurer);
        assert_eq!(
            stacked_width,
            stacked.horizontal_padding()
                + stacked.index_column_width()
                + measurer.width("台語齒盤", stacked.annotation_font()).ceil()
                + stacked.horizontal_padding(),
            "stacked width is the wider line, no gap"
        );
        assert!(stacked_width < m.measure_width(&wide_annotation, &measurer));
        assert!(stacked.item_height() > m.item_height());
    }

    /// `cell_width` over pre-measured texts is `measure_width` — the window
    /// lays out from one measuring pass, so the two must not drift apart.
    #[test]
    fn cell_width_from_premeasured_texts_matches_measure_width() {
        let measurer = EmMeasurer;
        let cells = [
            CandidateCellContent::new("tâi", Some("台".into())),
            CandidateCellContent::new("tâi", Some("台語齒盤".into())),
            CandidateCellContent::new("永", None),
            CandidateCellContent::new("永", Some("".into())),
        ];
        for m in [metrics(S::Standard, Inline), metrics(S::Standard, Stacked)] {
            for cell in &cells {
                let primary = m.measure_primary_width(&cell.text, &measurer);
                let annotation = m.measure_annotation(cell.annotation.as_deref(), &measurer);
                assert_eq!(
                    m.cell_width(primary, annotation),
                    m.measure_width(cell, &measurer),
                    "{:?} {:?}",
                    m.cell_arrangement(),
                    cell
                );
            }
        }
    }

    /// An inline row is the point size plus padding for the bundled faces,
    /// whose line boxes fit it. A typeface the user brought has no such
    /// guarantee — tall ascenders, stacked diacritics, a fallback glyph — so
    /// its row is given the measured line box instead, and the bundled
    /// arithmetic is left exactly as it shipped
    /// (`CandidateMetrics.swift`'s inline arm).
    #[test]
    fn an_inline_row_measures_a_custom_face_and_not_a_bundled_one() {
        use CandidateCellArrangement::Inline;
        let measurer = EmMeasurer;
        let bundled = CandidateMetrics::resolve(
            CandidateSizeChoice::Standard,
            CandidateFontSelection::BuiltIn(CandidateFontChoice::Iansui),
            Inline,
            &measurer,
        );
        assert_eq!(
            bundled.item_height(),
            bundled.candidate_font_size() + bundled.vertical_padding(),
        );

        let custom = CandidateMetrics::resolve(
            CandidateSizeChoice::Standard,
            CandidateFontSelection::Custom(CustomFontId(1)),
            Inline,
            &measurer,
        );
        assert_eq!(
            custom.item_height(),
            (measurer.line_height(custom.candidate_font()) + custom.vertical_padding()).ceil(),
        );
        assert!(custom.item_height() > bundled.item_height());
    }

    /// Two typefaces the user added are two metrics values — a window built
    /// for one must not be reused for the other.
    #[test]
    fn two_custom_typefaces_are_two_font_specs() {
        let first = CandidateFontSelection::Custom(CustomFontId(1));
        let second = CandidateFontSelection::Custom(CustomFontId(2));

        assert_ne!(first, second);
        assert_ne!(first, CandidateFontSelection::default());
        assert!(first.requires_line_box_measurement());
        assert!(first.built_in().is_none());
    }

    /// An installed family measures its line box like a custom face: neither
    /// carries the bundled roster's fits-the-row guarantee — and two installed
    /// ids are two typefaces.
    #[test]
    fn an_installed_family_is_measured_and_distinct_per_id() {
        use crate::settings::InstalledFontId;
        let first = CandidateFontSelection::Installed(InstalledFontId(1));
        let second = CandidateFontSelection::Installed(InstalledFontId(2));

        assert!(first.requires_line_box_measurement());
        assert_ne!(first, second);
        assert_ne!(first, CandidateFontSelection::Custom(CustomFontId(1)));
        assert_eq!(first.built_in(), None);
    }
}
