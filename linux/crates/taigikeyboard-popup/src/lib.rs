//! What the candidate window is told, and how a page is laid out.
//! The process that draws it is this crate's binary; the shells only
//! write these commands. No windowing crate lives here, so the engine
//! can link it from a cross build.

#[cfg(feature = "text")]
mod face;
mod layout;
pub mod paint;
#[cfg(feature = "window")]
mod portal;
mod protocol;
mod system_color;
#[cfg(feature = "text")]
pub mod text;

pub use layout::{
    arrange, horizontal_cell_extent, key_label_width, pads, px, Arrangement, CellStyle, Glyph, Hit,
    Measure, Pads, Rect, Role, Target,
};
pub use paint::{
    capped_radius, fill_clipped, fill_round, highlight_radii, mix, pack, stroke_round,
};
#[cfg(feature = "window")]
pub use portal::watch_appearance;
pub use protocol::{Caret, Cell, Chrome, Command, Event, ExpandableView, Show};
pub use system_color::{paint_chrome, SystemAppearance};
