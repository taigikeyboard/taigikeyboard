//! The WinUI 3 window (roadmap W17), grown pane by pane from this smoke.
//! Windows-only: `windows-reactor` has no host build.

// 中文: WinUI 3 視窗(W17)— 先只有一個煙霧測試視窗,之後逐 pane 長出來。

use windows_reactor::*;

/// `TaigiKeyboardSettings.exe --winui-smoke`: prove the runtime beside the
/// exe starts a WinUI window on this machine. Not part of the launcher
/// contract; removed at the W17-C cutover.
pub const SMOKE_FLAG: &str = "--winui-smoke";

/// Answers whether the window ran to a normal close; a failed launch is a
/// non-zero exit so a gate can tell it from success.
pub fn run_smoke() -> bool {
    let view = StackPanel::new().spacing(8.0).margin(24.0).children((
        TextBlock::new()
            .text("Taigi Keyboard")
            .font_size(28.0)
            .font_weight(FontWeight::SEMI_BOLD),
        TextBlock::new().text("WinUI 3 foundation smoke (W17-A0)"),
    ));
    match App::run(view) {
        Ok(()) => true,
        Err(error) => {
            log::error!("winui.smoke_failed error={error}");
            false
        }
    }
}
