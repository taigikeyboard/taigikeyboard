//! The candidate window's geometry and navigation, with no window in it.
//!
//! Metrics, the horizontal page packer, the expanded grid, the four
//! layouts' navigation state machines, index labels and caret positioning —
//! the reference for every desktop, pinned by this module's tests. macOS
//! keeps a Swift twin in `macos/Sources/TaigiInputMethodCore/Candidates/`
//! (candidate-window geometry stays Swift there; the known differences,
//! W1–W3, are listed in the roadmap rather than fixed on either side). The renderer
//! (PR6, Direct2D) draws what these models say and feeds back clicks. Roadmap D4/W4:
//! the headless model is unit-tested, the window is a renderer only. Each
//! model owns its scroll offset: the renderer reports wheel / scrollbar
//! scrolls back through `on_viewport_scrolled` so slot numbering, paging and
//! hit-testing read the same viewport the user sees. NAMED NUMERIC
//! DIVERGENCE: `f32` throughout (Direct2D's unit) where the Mac uses 64-bit
//! `CGFloat`; see `HorizontalPageLayout::pack`.

mod expandable;
mod grid;
mod horizontal;
mod index_label;
mod metrics;
mod positioning;
mod vertical;

pub use expandable::{
    CellMove, CollapsedCell, CollapsedLayout, ExpandableAxis, ExpandableDisplayMode,
    ExpandableGeometryInput, ExpandableListModel, ExpandableModeChange, ExpandedGeometry,
    UnfoldPlan,
};
pub use grid::{ExpandedGridLayout, GridCell};
pub use horizontal::{HorizontalListModel, HorizontalPageLayout, PageSlot};
pub use index_label::CandidateIndexLabel;
pub use metrics::{CandidateCellArrangement, CandidateMetrics, FontSpec, TextMeasurer};
pub use positioning::{panel_frame, Point, Rect, Size};
pub use vertical::{ScrollerStyle, VerticalGeometry, VerticalLayoutInput, VerticalListModel};

use crate::settings::CandidateLayout;

impl CandidateLayout {
    /// How this layout's cells hold their two scripts: the row-shaped
    /// layouts stack (a cell as wide as both scripts side by side fits far
    /// fewer of them), the column-shaped ones keep them inline
    /// (as macOS's `CandidateLayout.cellArrangement`).
    pub fn cell_arrangement(self) -> CandidateCellArrangement {
        match self {
            Self::Horizontal | Self::Expandable => CandidateCellArrangement::Stacked,
            Self::Vertical | Self::VerticalExpandable => CandidateCellArrangement::Inline,
        }
    }
}

/// Every layout shows at most this many candidates
/// (macOS: `CandidateBasePanel.maxDisplayCandidates`). Enforced by each model's constructor
/// (`HorizontalPageLayout::pack`, `VerticalListModel::new`,
/// `ExpandableListModel::new`), not by the caller.
pub const MAX_DISPLAY_CANDIDATES: usize = 200;
