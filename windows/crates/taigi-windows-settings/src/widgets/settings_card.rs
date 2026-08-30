//! The SettingsCard: the one shape every setting in the Windows 11
//! Settings app sits in (Windows Community Toolkit `SettingsCard` /
//! `SettingsExpander`) — a full-width card on the ground, 1px stroke,
//! 4px corners, the setting's name at the left and its control at the
//! right. `row` is one setting in its own card; `group` stacks several in
//! one card with dividers between them (the expander's items); `action` is
//! the clickable card (`IsClickEnabled`), the whole card being the button.
//! No header icons: the panes have none, and none would be invented.

// 中文: 設定卡片 — 單列卡片、多列同卡(分隔線)、整張可點的動作卡、區段標題。

use crate::theme::{self, Palette, CONTROL_CORNER_RADIUS};
use crate::widgets::toggle_switch;

/// `SettingsCardPadding`: 16 across, 12 down.
const CARD_PADDING: egui::Margin = egui::Margin::symmetric(16, 12);
/// `SettingsCardMinHeight`.
const CARD_MIN_HEIGHT: f32 = 68.0;
/// A single card's content line, the padding taken off.
const ROW_MIN_HEIGHT: f32 = CARD_MIN_HEIGHT - 2.0 * CARD_PADDING.top as f32;
/// The expander's items are shorter (`SettingsExpanderItemMinHeight`).
const GROUP_ROW_MIN_HEIGHT: f32 = 48.0 - 2.0 * CARD_PADDING.top as f32;
/// An expander item that belongs to the one above it sits this much
/// further in.
const SUB_ROW_INDENT: f32 = 16.0;
/// Between two cards (the toolkit sample's `StackPanel Spacing`).
const CARD_SPACING: f32 = 4.0;
/// Between the header and the control, when the line is tight.
const CONTROL_GAP: f32 = 16.0;
/// A section's title (`BodyStrongTextBlockStyle`): the air above and
/// below it.
const SECTION_TITLE_SPACING: [f32; 2] = [24.0, 8.0];

fn card_stroke(palette: &Palette) -> egui::Stroke {
    egui::Stroke::new(1.0_f32, palette.card_stroke)
}

/// The card's frame alone, `padding` inside it: what `row`, `group` and
/// the table wear.
pub fn frame(ui: &mut egui::Ui, padding: egui::Margin, add_contents: impl FnOnce(&mut egui::Ui)) {
    let palette = theme::palette(ui.visuals());
    egui::Frame::NONE
        .fill(palette.card_fill)
        .stroke(card_stroke(palette))
        .corner_radius(CONTROL_CORNER_RADIUS)
        .inner_margin(padding)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            add_contents(ui);
        });
    ui.add_space(CARD_SPACING);
}

/// One setting in its own card: `header` at the left, `control` at the
/// right, vertically centred on the line.
pub fn row(ui: &mut egui::Ui, header: &str, control: impl FnOnce(&mut egui::Ui)) {
    frame(ui, egui::Margin::ZERO, |ui| {
        row_line(ui, ROW_MIN_HEIGHT, 0.0, header, control);
    });
}

/// One on/off setting in its own card; answers whether it was flipped.
pub fn switch_row(ui: &mut egui::Ui, header: &str, value: &mut bool) -> bool {
    let mut changed = false;
    row(ui, header, |ui| {
        changed = toggle_switch::show(ui, value, header).changed();
    });
    changed
}

/// Several settings in one card, a divider between each pair.
pub fn group(ui: &mut egui::Ui, rows: impl FnOnce(&mut CardRows)) {
    frame(ui, egui::Margin::ZERO, |ui| {
        let mut card_rows = CardRows { ui, is_first: true };
        rows(&mut card_rows);
    });
}

/// The rows of one `group`, in order.
pub struct CardRows<'ui> {
    ui: &'ui mut egui::Ui,
    is_first: bool,
}

impl CardRows<'_> {
    pub fn row(&mut self, header: &str, control: impl FnOnce(&mut egui::Ui)) {
        self.line(0.0, true, header, control);
    }

    /// A row that belongs to the one above it: indented, and greyed while
    /// `enabled` is false (the parent is off).
    pub fn sub_row(&mut self, enabled: bool, header: &str, control: impl FnOnce(&mut egui::Ui)) {
        self.line(SUB_ROW_INDENT, enabled, header, control);
    }

    fn line(
        &mut self,
        indent: f32,
        enabled: bool,
        header: &str,
        control: impl FnOnce(&mut egui::Ui),
    ) {
        if !self.is_first {
            // A hairline edge to edge, in the divider token
            // (`widgets.noninteractive.bg_stroke`).
            self.ui.add(egui::Separator::default().spacing(1.0));
        }
        self.is_first = false;
        self.ui.add_enabled_ui(enabled, |ui| {
            row_line(ui, GROUP_ROW_MIN_HEIGHT, indent, header, control);
        });
    }
}

/// One padded line: the control laid out from the right, the header
/// wrapping in what is left — less `indent`, which stays empty at the
/// left edge — both centred on the line's height.
fn row_line(
    ui: &mut egui::Ui,
    min_height: f32,
    indent: f32,
    header: &str,
    control: impl FnOnce(&mut egui::Ui),
) {
    egui::Frame::NONE.inner_margin(CARD_PADDING).show(ui, |ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), min_height),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                control(ui);
                ui.add_space(CONTROL_GAP);
                let header_width = (ui.available_width() - indent).max(0.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(header_width, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_min_width(header_width);
                        ui.add(egui::Label::new(header).wrap());
                    },
                );
            },
        );
    });
}

/// A clickable card: the whole card is the button, `text` at its left in
/// the accent (`destructive`: the error colour). Answers whether it was
/// pressed.
pub fn action(ui: &mut egui::Ui, text: &str, destructive: bool) -> bool {
    let palette = theme::palette(ui.visuals());
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), CARD_MIN_HEIGHT),
        egui::Sense::click(),
    );
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), text));
    if ui.is_rect_visible(rect) {
        let fill = if response.is_pointer_button_down_on() {
            palette.control_fill_pressed
        } else if response.hovered() {
            palette.control_fill_hover
        } else {
            palette.card_fill
        };
        let painter = ui.painter();
        painter.rect(
            rect,
            CONTROL_CORNER_RADIUS,
            fill,
            card_stroke(palette),
            egui::StrokeKind::Inside,
        );
        let color = if destructive {
            ui.visuals().error_fg_color
        } else {
            ui.visuals().hyperlink_color
        };
        painter.text(
            egui::pos2(rect.left() + f32::from(CARD_PADDING.left), rect.center().y),
            egui::Align2::LEFT_CENTER,
            text,
            egui::TextStyle::Body.resolve(ui.style()),
            color,
        );
        theme::paint_focus_ring(ui, &response, CONTROL_CORNER_RADIUS);
    }
    ui.add_space(CARD_SPACING);
    response.clicked()
}

/// A section's title above its cards, with an optional note at the
/// line's right (a count).
pub fn section_title(ui: &mut egui::Ui, text: &str, trailing: Option<&str>) {
    ui.add_space(SECTION_TITLE_SPACING[0]);
    ui.horizontal(|ui| {
        ui.strong(text);
        if let Some(trailing) = trailing {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.weak(trailing);
            });
        }
    });
    ui.add_space(SECTION_TITLE_SPACING[1]);
}
