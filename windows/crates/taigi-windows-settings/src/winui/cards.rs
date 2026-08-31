//! The SettingsCard: the one shape every setting in the Windows 11
//! Settings app sits in (Windows Community Toolkit `SettingsCard`) — a
//! full-width card on the ground, 1px stroke, 4px corners, the setting's
//! name at the left and its control at the right. `action_row` puts a
//! command in that same shape: what it does at the left, the button that
//! runs it at the right.
//!
//! Every colour is a `ThemeBrush`, never a literal: light, dark and high
//! contrast are WinUI's to resolve.

// 中文: 設定卡片 — 單列卡片;命令也是同一個形狀(左標題、右按鈕);顏色一律用 ThemeBrush,不寫死色值。

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

/// The air between two groups of cards.
pub fn section_gap() -> View {
    Border::new().height(SECTION_GAP).into()
}

/// The card's frame around content that is not one setting's line — a
/// list and its controls, or a busy overlay.
///
/// The content is stacked before it is framed, and that is not cosmetic: a
/// `Border`'s content is SINGLE-CHILD, and a `View::fragment` of several
/// rows resolves to one native root per row, which the reactor refuses to
/// plan (`PumpError::StructureUnsupported`). That refusal reaches the user
/// as a process fail-fast with no message — it shipped twice in W17 — so
/// the one place that can make it impossible does. The panel adds no
/// spacing and no alignment of its own: a single child fills the frame the
/// way it did when `Border` held it directly.
pub fn frame(content: impl Into<View>) -> View {
    Border::new()
        .background(ThemeBrush::CardBackground)
        .border_brush(ThemeBrush::CardStroke)
        .border_thickness(Thickness::uniform(1.0))
        .corner_radius(CornerRadius::uniform(CARD_CORNER_RADIUS))
        .padding(padding())
        .content(StackPanel::new().children([content.into()]))
}

/// A command in a card: `header` says what it does, `verb` is the button
/// that runs it, at the card's right where Windows 11 Settings puts a
/// row's action button.
///
/// NOT a full-width clickable card, which this drew while it was a port of
/// the Mac's `WideActionRow`: in Windows 11 Settings a whole-card button
/// means NAVIGATION and carries a chevron, so an action wearing that shape
/// reads as a link to somewhere.
///
/// A destructive command keeps its critical colour — on the button's text,
/// the only channel left once the card stops being the button. It is the
/// standing cue for these two commands, and it is not paid for by the
/// confirmation the caller puts in front of them; both are wanted.
pub fn action_row(
    header: &str,
    verb: &str,
    is_destructive: bool,
    is_enabled: bool,
    on_click: Callback<()>,
) -> View {
    let label = TextBlock::new().text(verb);
    let label = if is_destructive {
        label.foreground(ThemeBrush::SystemCritical)
    } else {
        label
    };
    row(
        header,
        Button::new()
            .is_enabled(is_enabled)
            .on_click(on_click)
            .content(label),
    )
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
