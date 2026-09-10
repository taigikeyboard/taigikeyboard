//! The settings window in WinUI 3, through `windows-reactor` (roadmap W17):
//! the same panes over the same `settings.json`, in native Windows 11
//! chrome — Mica, the system light/dark, Segoe UI Variable, UIA, and a
//! `TextBox` that is a real TSF host.
//!
//! Windows-only: `windows-reactor` has no host build, so the macOS gate
//! type-checks this for the gnu target and `make check-box` compiles and
//! tests it on the Windows box.

mod cards;
mod file_dialog;
mod list_selection;
mod pages;
#[cfg(test)]
mod pane_planning;
mod window;

pub use window::run;
