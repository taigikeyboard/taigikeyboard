//! The framework-independent input session the Linux shells and the web
//! version share: a key goes in as a [`crate::keys::KeyEventSnapshot`], and
//! what comes back is a list of [`Emit`]s the shell renders with whatever
//! its framework offers. Nothing here depends on a bus, a panel or a
//! browser; the contract is written in IBus's terms because Linux came first
//! ([`Emit`] names the signal each one maps to).
//!
//! Design record: `docs/architecture/web-roadmap.md` § Phase 2.

mod executor;
mod global;
mod key;
mod selection;
mod state;
mod table;

pub use executor::{Emit, LookupTableContent};
pub(crate) use executor::{KeySurface, Recorder};
pub use global::perform_global;
pub use key::{click_from_panel, end_session, is_composing, navigate_from_panel, process_key};
pub use selection::LookupSelection;
pub use state::{EngineState, KeyReply, SymbolPicker};
pub(crate) use table::{is_vertical_layout, present_table};

use crate::candidates::HorizontalPageLayout;

/// The slot keys a page holds, the same on every desktop.
pub const PAGE_SIZE: usize = HorizontalPageLayout::PAGE_SIZE;
