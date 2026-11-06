//! The framework-independent input session: the [`Emit`] list a shell
//! replays, what one context remembers between keys, the candidate page
//! model. Nothing here knows a bus, a panel or a browser.
//!
//! Design record: `docs/architecture/web-roadmap.md` § Phase 2.

mod executor;
mod selection;
mod state;
mod table;

pub use executor::{Emit, KeySurface, LookupTableContent, Recorder};
pub use selection::LookupSelection;
pub use state::{EngineState, KeyReply, SymbolPicker};
pub use table::{is_vertical_layout, present_table};

use crate::keys::CandidateSlotKeySet;

/// The slot keys a page holds — nine on both sets (`BARE_KEY_ROW`, `1`–`9`).
pub const PAGE_SIZE: usize = CandidateSlotKeySet::BARE_KEY_ROW.len();
