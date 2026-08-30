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
/// `SettingsExpanderItemMinHeight`: an expanded item is shorter than a card.
const SUB_ROW_MIN_HEIGHT: f64 = 48.0;
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
/// The air around a section's title, the stack's own spacing taken off.
const SECTION_TITLE_TOP: f64 = 24.0 - CARD_SPACING;
const SECTION_TITLE_BOTTOM: f64 = 8.0 - CARD_SPACING;

fn padding() -> Thickness {
    Thickness::xy(CARD_PADDING.0, CARD_PADDING.1)
}

/// One setting's line: `header` at the left, wrapping into what the
/// control leaves, and `control` at the right, both centred. The shape
/// inside a card, an `Expander`'s header, and an expanded item alike.
pub fn line(header: &str, control: impl Into<View>) -> View {
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
        ))
}

/// One setting in its own card.
pub fn row(header: &str, control: impl Into<View>) -> View {
    Border::new()
        .background(ThemeBrush::CardBackground)
        .border_brush(ThemeBrush::CardStroke)
        .border_thickness(Thickness::uniform(1.0))
        .corner_radius(CornerRadius::uniform(CARD_CORNER_RADIUS))
        .padding(padding())
        .min_height(CARD_MIN_HEIGHT)
        .content(line(header, control))
}

/// A setting inside an `Expander`'s content: the same line, without a card
/// of its own — the expander already draws the group
/// (`SettingsExpanderItem`, which is shorter than a card).
pub fn sub_row(header: &str, control: impl Into<View>) -> View {
    Border::new()
        .padding(padding())
        .min_height(SUB_ROW_MIN_HEIGHT)
        .content(line(header, control))
}

/// One on/off setting in its own card. The switch shows no On / Off word:
/// WinUI's default pair is in the SYSTEM's language, which is not
/// necessarily the display language this window was told to speak.
/// `is_enabled` is false for a row that follows a parent switch — greyed,
/// never cleared, so the choice comes back with its parent.
pub fn switch_row(header: &str, is_on: bool, is_enabled: bool, on_toggled: Callback<bool>) -> View {
    row(header, switch(is_on, is_enabled, on_toggled))
}

/// The switch itself, for a caller that places its own line.
pub fn switch(is_on: bool, is_enabled: bool, on_toggled: Callback<bool>) -> View {
    ToggleSwitch::new()
        .is_on(is_on)
        .is_enabled(is_enabled)
        .on_toggled(on_toggled)
        .slots([
            SlotView::new(ToggleSwitchSlot::OnContent, View::empty()),
            SlotView::new(ToggleSwitchSlot::OffContent, View::empty()),
        ])
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
///
/// Not a `Border` with a pointer handler and not a card-coloured override:
/// `ResourceValue` carries no theme brush, so a background override would
/// freeze a literal colour against light / dark / high contrast. A real
/// `Button` in native actionable-card chrome keeps the keyboard
/// activation, the UIA role and the hover / pressed / focus states.
pub fn action(text: &str, is_destructive: bool, on_click: Callback<()>) -> View {
    action_enabled(text, is_destructive, true, on_click)
}

/// The air between two groups of cards.
pub fn section_gap() -> View {
    Border::new().height(SECTION_GAP).into()
}

/// The card's frame around content that is not one setting's line — a
/// list and its controls, or a busy overlay.
pub fn frame(content: impl Into<View>) -> View {
    Border::new()
        .background(ThemeBrush::CardBackground)
        .border_brush(ThemeBrush::CardStroke)
        .border_thickness(Thickness::uniform(1.0))
        .corner_radius(CornerRadius::uniform(CARD_CORNER_RADIUS))
        .padding(padding())
        .content(content)
}

/// A clickable card that can be turned off while a job holds the page.
pub fn action_enabled(
    text: &str,
    is_destructive: bool,
    is_enabled: bool,
    on_click: Callback<()>,
) -> View {
    let foreground = if is_destructive {
        ThemeBrush::SystemCritical
    } else {
        ThemeBrush::Accent
    };
    Button::new()
        .is_enabled(is_enabled)
        .on_click(on_click)
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .horizontal_content_alignment(HorizontalAlignment::Left)
        .min_height(CARD_MIN_HEIGHT)
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

/// A section's title with a count at the line's right (`{matched} / {total}`).
pub fn section_title_with_count(text: &str, count: &str) -> View {
    Grid::new()
        .columns([GridLength::STAR, GridLength::Auto])
        .margin(Thickness::new(
            0.0,
            SECTION_TITLE_TOP,
            0.0,
            SECTION_TITLE_BOTTOM,
        ))
        .children((
            TextBlock::new()
                .text(text)
                .font_weight(FontWeight::SEMI_BOLD)
                .grid_column(0),
            TextBlock::new().text(count).opacity(0.65).grid_column(1),
        ))
}

/// A section's title above its cards (`BodyStrongTextBlockStyle`).
pub fn section_title(text: &str) -> View {
    TextBlock::new()
        .text(text)
        .font_weight(FontWeight::SEMI_BOLD)
        .margin(Thickness::new(
            0.0,
            SECTION_TITLE_TOP,
            0.0,
            SECTION_TITLE_BOTTOM,
        ))
        .into()
}
