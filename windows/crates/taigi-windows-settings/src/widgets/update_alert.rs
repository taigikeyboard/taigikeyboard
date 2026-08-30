//! What a manual update check answers with (`UpdateAlertPresenter`): an
//! update — with its download (or download-and-install) button and 稍後;
//! up to date; or the check failed. The wording is `ManualOutcome`'s, so
//! this window and the WinUI one say the same thing.

// 中文: 手動檢查更新的結果視窗 — 有更新(下載/下載並安裝 + 稍後)、已是最新、檢查失敗。

use crate::updates::ManualOutcome;
use taigi_windows_core::strings::{StringKey, StringResolver};

/// What the user pressed.
pub enum UpdateAlertAction {
    /// The first button on an available update.
    Proceed,
    Dismiss,
}

pub fn show(
    ctx: &egui::Context,
    strings: &StringResolver,
    outcome: &ManualOutcome,
) -> Option<UpdateAlertAction> {
    let (title, detail) = outcome.alert_text(strings);
    let proceed = outcome.proceed_key();
    let mut action = None;
    egui::Window::new(title)
        .id(egui::Id::new("update_alert"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_max_width(360.0);
            if let Some(detail) = &detail {
                ui.label(detail);
                ui.add_space(8.0);
            }
            ui.with_layout(
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| match proceed {
                    Some(key) => {
                        if ui.button(strings.resolve(key)).clicked() {
                            action = Some(UpdateAlertAction::Proceed);
                        }
                        if ui
                            .button(strings.resolve(StringKey::DesktopUpdateLaterAction))
                            .clicked()
                        {
                            action = Some(UpdateAlertAction::Dismiss);
                        }
                    }
                    None => {
                        if ui.button(strings.resolve(StringKey::CommonOk)).clicked() {
                            action = Some(UpdateAlertAction::Dismiss);
                        }
                    }
                },
            );
        });
    action
}
