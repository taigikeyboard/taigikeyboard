//! The framework-independent half of the Linux input method (roadmap L1,
//! revised 2026-09-23): what both shells — the Fcitx5 addon through the
//! `taigi-linux-ffi` C ABI, the IBus engine through zbus — drive and
//! replay. Nothing here knows a bus or a panel: a key goes in as a
//! [`taigi_desktop_core::keys::KeyEventSnapshot`], and what comes back is a
//! list of [`Emit`]s the shell renders with whatever its framework offers.
//! A behaviour that differs between the shells is therefore a shell bug.
//!
//! Counterpart of `taigi-windows-tsf`'s `runtime` + `session`, minus the
//! COM.

pub mod chrome;
pub mod executor;
pub mod runtime;
pub mod selection;
pub mod session;
#[cfg(feature = "e2e-trace")]
mod trace;

pub use chrome::{
    activate_menu, menu_items, mode_label, mode_symbol, MenuItem, MENU_ABOUT, MENU_SETTINGS,
};
pub use executor::{Emit, LookupTableContent};
pub use runtime::{dictionary_version, FirstKeySetup, Runtime};
pub use selection::LookupSelection;
pub use session::{
    click_from_panel, end_session, navigate_from_panel, process_key, process_raw_key, EngineState,
    KeyReply, CAP_SURROUNDING_TEXT,
};
