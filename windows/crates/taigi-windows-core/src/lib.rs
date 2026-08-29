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

// 中文: Windows 輸入法的平台無關核心 — 設定模型、引擎橋接、組字流程、鍵盤分類、候選幾何、字串。
// 中文: 行為以 macOS 版為準,每個移植項目都註明對應的 Swift 檔。

pub mod dictionary_artifacts;
pub mod engine;
pub mod settings;
pub mod strings;
