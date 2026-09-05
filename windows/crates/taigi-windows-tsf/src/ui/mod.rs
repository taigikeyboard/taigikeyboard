//! The candidate window and the mode flash: Win32 popups drawn with
//! Direct2D + DirectWrite over the pure geometry models in
//! `taigi_windows_core::candidates` (roadmap W4). The renderer owns NO
//! composition state: it draws what the models say and reports clicks and
//! scrolls back to them.

// 候選窗與模式提示 — Win32 popup + Direct2D/DirectWrite,幾何全由 core 的模型決定。

pub mod candidate_list_element;
pub mod candidate_window;
pub mod caret;
pub mod mode_flash;
pub mod presenter;
pub mod render;
pub mod theme;
pub mod window;
