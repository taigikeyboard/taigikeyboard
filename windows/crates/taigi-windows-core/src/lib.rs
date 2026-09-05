//! Platform-independent core of the Windows input method.
//!
//! Everything a Win32 handle is not needed for lives here so it can be unit
//! tested on any host: the settings model the TIP and the settings window
//! share, the protobuf envelope to the shared engine, the composing
//! orchestration ported from macOS, the key classifier, the candidate window
//! geometry and the generated UI strings. The two shells
//! (`taigi-windows-tsf`, `taigi-windows-settings`) are thin adapters over it.
//!
//! Behaviour oracle is the macOS input method
//! (`macos/Sources/TaigiInputMethodCore`); ported items cite the Swift they
//! mirror. Design record: `docs/architecture/windows-roadmap.md`.

pub mod candidates;
pub mod composing;
pub mod dictionary_artifacts;
pub mod engine;
pub mod keys;
pub mod policies;
pub mod settings;
pub mod strings;
