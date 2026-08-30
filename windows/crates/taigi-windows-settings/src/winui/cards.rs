//! The SettingsCard: the one shape every setting in the Windows 11
//! Settings app sits in (Windows Community Toolkit `SettingsCard`) — a
//! full-width card on the ground, 1px stroke, 4px corners, the setting's
//! name at the left and its control at the right. `action` is the
//! clickable card, the whole card being the button.
//!
//! Every colour is a `ThemeBrush`, never a literal: light, dark and high
//! contrast are WinUI's to resolve. That rules out the toolkit's
//! `IsClickEnabled` card built from a `Border` plus overridden button
//! brushes — `ResourceValue` carries no theme brush — so `action` is a
//! real `Button` wearing native actionable-card chrome, which keeps the
//! keyboard activation, the UIA role and the hover / pressed / focus
//! states that a pointer-only `Border` would lose.

// 中文: 設定卡片 — 單列卡片與整張可點的動作卡;顏色一律用 ThemeBrush,不寫死色值。

use windows_reactor::*;

/// `SettingsCardPadding`: 16 across, 12 down.
const CARD_PADDING: (f64, f64) = (16.0, 12.0);
/// `SettingsCardMinHeight`.
const CARD_MIN_HEIGHT: f64 = 68.0;
/// `ControlCornerRadius`.
const CARD_CORNER_RADIUS: f64 = 4.0;
/// Between the header and the control, when the line is tight.
const CONTROL_GAP: f64 = 16.0;
/// Between two cards (the toolkit sample's `StackPanel Spacing`).
pub const CARD_SPACING: f64 = 4.0;
/// A pop-up's width, so the pickers line up down the pane.
const PICKER_WIDTH: f64 = 220.0;
/// Between two groups of cards; Settings draws no rule between them. The
/// stack's own spacing is already there, so the spacer carries the rest.
const SECTION_GAP: f64 = 16.0 - CARD_SPACING;

fn padding() -> Thickness {
    Thickness::xy(CARD_PADDING.0, CARD_PADDING.1)
}

/// One setting in its own card: `header` at the left, wrapping into what
/// the control leaves, and `control` at the right, both centred on the line.
pub fn row(header: &str, control: impl Into<View>) -> View {
    Border::new()
        .background(ThemeBrush::CardBackground)
        .border_brush(ThemeBrush::CardStroke)
        .border_thickness(Thickness::uniform(1.0))
        .corner_radius(CornerRadius::uniform(CARD_CORNER_RADIUS))
        .padding(padding())
        .min_height(CARD_MIN_HEIGHT)
        .content(
            Grid::new()
                .columns([GridLength::STAR, GridLength::Auto])
                .column_spacing(CONTROL_GAP)
                .children((
                    TextBlock::new()
                        .text(header)
                        .text_wrapping(TextWrapping::Wrap)
                        .vertical_alignment(VerticalAlignment::Center)
                        .grid_column(0),
                    // The control is already a `View` (a builder that took its
                    // slots), which carries no attached grid property — a
                    // `Border` is the thinnest thing that can carry one.
                    Border::new()
                        .grid_column(1)
                        .vertical_alignment(VerticalAlignment::Center)
                        .content(control),
                )),
        )
}

/// One on/off setting in its own card. The switch shows no On / Off word:
/// WinUI's default pair is in the SYSTEM's language, which is not
/// necessarily the display language this window was told to speak.
pub fn switch_row(header: &str, is_on: bool, on_toggled: Callback<bool>) -> View {
    row(
        header,
        ToggleSwitch::new()
            .is_on(is_on)
            .on_toggled(on_toggled)
            .slots([
                SlotView::new(ToggleSwitchSlot::OnContent, View::empty()),
                SlotView::new(ToggleSwitchSlot::OffContent, View::empty()),
            ]),
    )
}

/// A pop-up of named choices in one card; the answer is the chosen index
/// into `labels`.
pub fn choice_row(
    header: &str,
    labels: Vec<String>,
    selected: Option<usize>,
    on_change: Callback<Option<usize>>,
) -> View {
    row(
        header,
        ComboBox::new()
            .items_source(labels)
            .selected_index(selected)
            .on_selection_changed(on_change)
            .width(PICKER_WIDTH),
    )
}

/// A clickable card: the whole card is the button, `text` at its left in
/// the accent (`is_destructive`: the critical colour).
pub fn action(text: &str, is_destructive: bool, on_click: Callback<()>) -> View {
    let foreground = if is_destructive {
        ThemeBrush::SystemCritical
    } else {
        ThemeBrush::Accent
    };
    Button::new()
        .on_click(on_click)
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .horizontal_content_alignment(HorizontalAlignment::Left)
        .min_height(CARD_MIN_HEIGHT)
        // Geometry only — a colour override here would be a literal, and a
        // literal does not follow light / dark / high contrast.
        .resource_overrides(
            ResourceOverrides::new()
                .set(
                    "ControlCornerRadius",
                    CornerRadius::uniform(CARD_CORNER_RADIUS),
                )
                .set("ButtonPadding", padding()),
        )
        .content(TextBlock::new().text(text).foreground(foreground))
}

/// The air between two groups of cards.
pub fn section_gap() -> View {
    Border::new().height(SECTION_GAP).into()
}
