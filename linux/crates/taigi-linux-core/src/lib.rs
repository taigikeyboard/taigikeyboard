//! The framework-independent half of the Linux input method (roadmap L1,
//! revised 2026-09-23): what both shells — the Fcitx5 addon through the
//! `taigi-linux-ffi` C ABI, the IBus engine through zbus — drive and
//! replay. Nothing here knows a bus or a panel: a framework key goes in, and
//! what comes back from `taigi_desktop_core::session` is a list of [`Emit`]s
//! the shell renders with whatever its framework offers.
//! A behaviour that differs between the shells is therefore a shell bug.
//!
//! Counterpart of `taigi-windows-tsf`'s `runtime` + `session`, minus the
//! COM.

pub mod chrome;
pub mod runtime;
pub mod session;
#[cfg(feature = "e2e-trace")]
mod trace;

pub use chrome::{
    activate_menu, menu_items, mode_indicator, mode_label, MenuItem, MENU_ABOUT, MENU_SETTINGS,
};
pub use runtime::{dictionary_version, FirstKeySetup, Runtime};
pub use session::{
    begin_session, click_from_panel, end_session, navigate_from_panel, process_raw_key,
    EngineState, KeyReply,
};
pub use taigi_desktop_core::session::{Emit, LookupSelection, LookupTableContent};
