//! The candidate window, the mode flash and the Telex guide: Win32 popups
//! drawn with Direct2D + DirectWrite over the pure geometry models in
//! `taigi_windows_core::candidates` (roadmap W4). The renderer owns NO
//! composition state: it draws what the models say and reports clicks and
//! scrolls back to them.

pub mod candidate_list_element;
pub mod candidate_window;
pub mod caret;
pub mod mode_flash;
pub mod presenter;
pub mod render;
pub mod telex_guide;
pub mod theme;
pub mod window;
