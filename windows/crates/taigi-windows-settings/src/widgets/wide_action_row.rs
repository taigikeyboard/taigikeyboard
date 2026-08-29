//! A form row that is one action across the whole width of the pane,
//! drawn as tinted text rather than a push button (`WideActionRow.swift`,
//! USER 2026-08-24): the tint says the row acts, the width says it acts on
//! all of it. `destructive` colours it red.

// 中文: 整列寬的動作列 — 著色文字,不畫按鈕邊框;破壞性用紅色。

pub fn show(ui: &mut egui::Ui, text: &str, destructive: bool) -> bool {
    let color = if destructive {
        ui.visuals().error_fg_color
    } else {
        ui.visuals().hyperlink_color
    };
    let button = egui::Button::new(egui::RichText::new(text).color(color))
        .frame(false)
        .min_size(egui::vec2(
            ui.available_width(),
            ui.spacing().interact_size.y,
        ));
    ui.add(button).clicked()
}
