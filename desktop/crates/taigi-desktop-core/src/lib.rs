//! Platform-independent core shared by the desktop input methods (Windows
//! TSF, Linux IBus / Fcitx5, macOS IMKit through `taigi-macos-ffi`).
//!
//! Everything an OS handle is not needed for lives here so it can be unit
//! tested on any host: the settings model the input method and the settings
//! window share, the protobuf envelope to the shared engine, the composing
//! orchestration, the key classifier, the candidate window geometry and the
//! generated UI strings. The shells (`taigi-windows-tsf`,
//! `taigi-windows-settings`, `taigikeyboard-ibus`, `taigikeyboard-settings`,
//! `taigi-macos-ffi`) are thin adapters over it.
//!
//! This crate and its tests are the behaviour oracle for the three desktops
//! — macOS, Windows, Linux (`docs/contributing/windows-guidelines.md`
//! § desktop-core is the behaviour oracle). Design records:
//! `docs/architecture/windows-roadmap.md` (W1–W17, where this crate was
//! born) and `linux-roadmap.md` (L2, the move here).

pub mod candidates;
pub mod composing;
pub mod dictionary_artifacts;
pub mod engine;
pub mod keys;
pub mod mode_indicator;
pub mod platform;
pub mod policies;
pub mod runtime;
pub mod session;
pub mod settings;
pub mod strings;
pub mod symbols;
