//! Every point value the candidate window's geometry is built from, resolved
//! once from the two size choices and the typeface. Port of
//! `CandidateMetrics.swift`; the AppKit measurements go through the
//! [`TextMeasurer`] the renderer supplies (DirectWrite in PR6, a stub in tests).

// 中文: 候選窗所有點數尺寸,從兩個大小選項 + 字型一次解析;文字量測交給 TextMeasurer。

use super::index_label::CandidateIndexLabel;
use crate::composing::CandidateCellContent;
use crate::settings::{CandidateFontChoice, CandidateTextSizeChoice, CandidateWindowSizeChoice};

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
    pub choice: CandidateFontChoice,
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

/// The metrics, resolved. Immutable: a size change rebuilds the window.
#[derive(Clone, Debug, PartialEq)]
pub struct CandidateMetrics {
    text_size: CandidateTextSizeChoice,
    window_size: CandidateWindowSizeChoice,
    font_choice: CandidateFontChoice,
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
    /// Resolves every value. Scaled values round to whole points; the text
    /// choice scales the fonts and the text-anchored gaps, the window choice
    /// scales the paddings.
    pub fn resolve(
        text_size: CandidateTextSizeChoice,
        window_size: CandidateWindowSizeChoice,
        font_choice: CandidateFontChoice,
        cell_arrangement: CandidateCellArrangement,
        measurer: &dyn TextMeasurer,
    ) -> Self {
        let candidate_font_size = text_size.font_size();
        let text_scale = candidate_font_size / BASE_CANDIDATE_FONT_SIZE;
        let chrome_scale = window_size.scale();
        let annotation_font_size = (BASE_ANNOTATION_FONT_SIZE * text_scale).round();
        let index_font_size = (BASE_INDEX_FONT_SIZE * text_scale).round();
        let stacked_line_gap = (BASE_STACKED_LINE_GAP * text_scale).round();
        let vertical_padding = (BASE_VERTICAL_PADDING * chrome_scale).round();
        let candidate_font = FontSpec {
            choice: font_choice,
            size: candidate_font_size,
        };
        let annotation_font = FontSpec {
            choice: font_choice,
            size: annotation_font_size,
        };
        let index_font = FontSpec {
            choice: CandidateFontChoice::System,
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
            text_size,
            window_size,
            font_choice,
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
    /// in the list has an annotation — 羅馬字, or 合用's one-script cells —
    /// and the two-line box otherwise, so 並排's mixed lists keep lining up.
    /// Inline is one line either way. Idempotent: resolved once per list,
    /// on `show` and on `update_cells`.
    // 中文: 依內容決定格高 — 整份清單無副標時只留一行。
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
    pub fn text_size(&self) -> CandidateTextSizeChoice {
        self.text_size
    }

    pub fn window_size(&self) -> CandidateWindowSizeChoice {
        self.window_size
    }

    pub fn font_choice(&self) -> CandidateFontChoice {
        self.font_choice
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
            choice: self.font_choice,
            size: self.candidate_font_size,
        }
    }

    pub fn annotation_font(&self) -> FontSpec {
        FontSpec {
            choice: self.font_choice,
            size: self.annotation_font_size,
        }
    }

    /// The system font, never the user's candidate typeface.
    pub fn index_font(&self) -> FontSpec {
        FontSpec {
            choice: CandidateFontChoice::System,
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
        let text = self
            .primary_column_floor
            .max(self.measure_primary_width(&cell.text, measurer));
        match self.cell_arrangement {
            CandidateCellArrangement::Inline => {
                self.horizontal_padding
                    + self.index_column_width()
                    + text
                    + self.annotation_width(cell.annotation.as_deref(), measurer)
                    + self.horizontal_padding
            }
            // The two scripts are on top of each other, so the cell is as wide
            // as the WIDER of them — not both plus a gap.
            CandidateCellArrangement::Stacked => {
                self.horizontal_padding
                    + self.index_column_width()
                    + text.max(self.annotation_text_width(cell.annotation.as_deref(), measurer))
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

    /// The gap plus the annotation, or nothing when absent or empty.
    pub fn annotation_width(&self, annotation: Option<&str>, measurer: &dyn TextMeasurer) -> f32 {
        match annotation.filter(|a| !a.is_empty()) {
            None => 0.0,
            Some(annotation) => {
                self.candidate_annotation_gap
                    + self.annotation_text_width(Some(annotation), measurer)
            }
        }
    }

    /// The annotation's own width, without the gap — what a stacked cell
    /// measures.
    pub fn annotation_text_width(
        &self,
        annotation: Option<&str>,
        measurer: &dyn TextMeasurer,
    ) -> f32 {
        match annotation.filter(|a| !a.is_empty()) {
            None => 0.0,
            Some(annotation) => measurer.width(annotation, self.annotation_font()).ceil(),
        }
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
        text: CandidateTextSizeChoice,
        window: CandidateWindowSizeChoice,
        arrangement: CandidateCellArrangement,
    ) -> CandidateMetrics {
        CandidateMetrics::resolve(
            text,
            window,
            CandidateFontChoice::System,
            arrangement,
            &EmMeasurer,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;
    use crate::settings::SettingChoice;
    use CandidateCellArrangement::{Inline, Stacked};
    use CandidateTextSizeChoice as T;
    use CandidateWindowSizeChoice as W;

    #[test]
    fn small_small_and_medium_medium_resolve_the_traced_values() {
        // Both ladders are the tighter Windows ones (`CandidateTextSizeChoice::font_size`,
        // `CandidateWindowSizeChoice::scale`), so the shapes follow the macOS trace
        // (CandidateMetricsTests.swift:23-50) but not its numbers.
        // trace: small — text 13/16=0.8125, so 14*0.8125=11.375→11, 7*0.8125=5.6875→6;
        // chrome 0.45, so 9*0.45=4.05→4, 12*0.45=5.4→5, 8*0.45=3.6→4; item 13+5=18.
        // medium — text 16/16=1, so 14, 7; chrome 0.55, so 9*0.55=4.95→5,
        // 12*0.55=6.6→7; item 16+7=23.
        let m = metrics(T::Small, W::Small, Inline);
        assert_eq!(
            (
                m.candidate_font_size(),
                m.annotation_font_size(),
                m.candidate_annotation_gap(),
                m.horizontal_padding(),
                m.vertical_padding(),
                m.tahoe_separator_inset(),
                m.item_height()
            ),
            (13.0, 11.0, 6.0, 4.0, 5.0, 4.0, 18.0)
        );
        // trace: 11*0.8125=8.9375→9, 8*0.8125=6.5→7 (Rust rounds half away from zero).
        assert_eq!(m.scaled_symbol_metric(11.0), 9.0);
        assert_eq!(m.scaled_symbol_metric(8.0), 7.0);
        let m = metrics(T::Medium, W::Medium, Inline);
        assert_eq!(
            (
                m.candidate_font_size(),
                m.annotation_font_size(),
                m.candidate_annotation_gap(),
                m.horizontal_padding(),
                m.vertical_padding(),
                m.item_height()
            ),
            (16.0, 14.0, 7.0, 5.0, 7.0, 23.0)
        );
    }

    #[test]
    fn knobs_are_independent_and_every_pair_is_distinct_and_whole() {
        let mut seen = Vec::new();
        for text in T::ALL {
            for window in W::ALL {
                let m = metrics(*text, *window, Inline);
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
                    assert_eq!(value, value.round(), "{text:?}/{window:?}: {value}");
                }
                assert!(
                    !seen.contains(&m),
                    "{text:?}/{window:?} duplicates an earlier pair"
                );
                seen.push(m);
            }
        }
        let text_moves = metrics(T::Large, W::Medium, Inline);
        let base = metrics(T::Medium, W::Medium, Inline);
        assert_ne!(text_moves.annotation_font_size, base.annotation_font_size);
        assert_eq!(
            text_moves.horizontal_padding, base.horizontal_padding,
            "text knob leaves chrome alone"
        );
        let window_moves = metrics(T::Medium, W::Large, Inline);
        assert_eq!(
            window_moves.annotation_font_size, base.annotation_font_size,
            "window knob leaves text alone"
        );
        assert_ne!(window_moves.horizontal_padding, base.horizontal_padding);
    }

    #[test]
    fn corner_geometry_matches_macos() {
        // trace: CandidateMetricsTests.swift:132-203.
        let inline = metrics(T::Medium, W::Medium, Inline);
        assert_eq!(
            inline.tahoe_container_corner_radius(),
            inline.item_height / 2.0
        );
        assert_eq!(inline.tahoe_highlight_inset(), 2.0);
        for text in T::ALL {
            for window in W::ALL {
                let stacked = metrics(*text, *window, Stacked);
                assert_eq!(stacked.tahoe_container_corner_radius(), 16.0);
                assert_eq!(stacked.tahoe_highlight_inset(), 4.0);
                assert_eq!(stacked.tahoe_highlight_corner_radius(), 12.0);
                assert!(
                    16.0 < stacked.item_height() / 2.0,
                    "{text:?}/{window:?} {}",
                    stacked.item_height()
                );
            }
        }
        assert_eq!(CandidateMetrics::corner_radius(16.0, 200.0, 57.0), 16.0);
        assert_eq!(CandidateMetrics::corner_radius(16.0, 10.0, 57.0), 5.0);
        assert_eq!(CandidateMetrics::corner_radius(16.0, 200.0, 24.0), 12.0);
    }

    #[test]
    fn stacked_content_without_annotations_is_one_line_tall() {
        // trace: Medium/Medium — inline item 16+7=23; stacked resolve =
        // ceil(20+17+2+7)=46 (candidate line ceil(16*1.2)=20, annotation line
        // ceil(14*1.2)=17, gap 2*1=2). No annotated cell → 23 (the inline
        // height); any annotated cell → 46; Inline is 23 either way and
        // measures no stacked lines; the variant round-trips back to the
        // resolved box.
        for text in T::ALL {
            for window in W::ALL {
                let stacked = metrics(*text, *window, Stacked);
                let inline = metrics(*text, *window, Inline);
                let single = stacked.for_content(false);
                assert_eq!(
                    single.item_height(),
                    inline.item_height(),
                    "{text:?}/{window:?}"
                );
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
        }
        let stacked = metrics(T::Medium, W::Medium, Stacked);
        assert_eq!(stacked.item_height(), 46.0);
        assert_eq!(stacked.for_content(false).item_height(), 23.0);
    }

    #[test]
    fn width_arithmetic_matches_macos() {
        // trace: CandidateMetricsTests.swift:220-353.
        let m = metrics(T::Medium, W::Medium, Inline);
        let measurer = EmMeasurer;
        assert_eq!(m.annotation_width(None, &measurer), 0.0);
        assert_eq!(m.annotation_width(Some(""), &measurer), 0.0);
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

        let stacked = metrics(T::Medium, W::Medium, Stacked);
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
}
