//! The SettingsCard: the one shape every setting in the Windows 11
//! Settings app sits in (Windows Community Toolkit `SettingsCard`) — a
//! full-width card on the ground, 1px stroke, 4px corners, the setting's
//! name at the left and its control at the right. `action_row` puts a
//! command in that same shape: what it does at the left, the button that
//! runs it at the right.
//!
//! Every colour is a `ThemeBrush`, never a literal: light, dark and high
//! contrast are WinUI's to resolve.

use windows_reactor::*;

/// `SettingsCardPadding`: 16 across, 12 down.
const CARD_PADDING: (f64, f64) = (16.0, 12.0);
/// `SettingsCardMinHeight`.
const CARD_MIN_HEIGHT: f64 = 68.0;
/// `SettingsExpanderItemMinHeight`: a row inside a group is shorter than a
/// card of its own.
const SUB_ROW_MIN_HEIGHT: f64 = 48.0;
/// The left step that says a row belongs to the row above it, the way
/// `SettingsExpanderItem` steps its content in.
const GROUP_INDENT: f64 = 16.0;
/// `TextFillColorDisabled` is 36% of the theme's text colour, in both the
/// light and the dark resource dictionary.
const DISABLED_LABEL_OPACITY: f64 = 0.36;
/// `CaptionTextBlockStyle`'s size: a card's description.
const CAPTION_FONT_SIZE: f64 = 12.0;
/// `PrimaryText` at less than full is WinUI's secondary text, and it
/// follows the theme.
const SECONDARY_OPACITY: f64 = 0.65;
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
/// inside a card and inside a `group` alike.
pub fn line(header: &str, control: impl Into<View>) -> View {
    dimmable_line(header, true, control)
}

/// `line`, with the header greyed when the setting cannot be changed.
///
/// A control greys itself; its name does not, and a row whose switch is
/// disabled under a full-strength label reads as a switch that is merely
/// off. `TextFillColorDisabledBrush` is what WinUI would put there, and the
/// reactor exposes no such brush — the same 36% the light and dark
/// resources both resolve to, applied to the theme's own text colour, is
/// the nearest thing that stays theme- and high-contrast-correct.
fn dimmable_line(header: &str, is_enabled: bool, control: impl Into<View>) -> View {
    header_and_control(
        TextBlock::new()
            .text(header)
            .text_wrapping(TextWrapping::Wrap)
            .opacity(if is_enabled {
                1.0
            } else {
                DISABLED_LABEL_OPACITY
            }),
        control,
    )
}

/// `header` at the left, `control` at the right, both centred.
fn header_and_control(header: impl Into<View>, control: impl Into<View>) -> View {
    Grid::new()
        .columns([GridLength::STAR, GridLength::Auto])
        .column_spacing(CONTROL_GAP)
        .children((
            Border::new()
                .grid_column(0)
                .vertical_alignment(VerticalAlignment::Center)
                .content(header),
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
    card(line(header, control))
}

/// `row`, with `description` under the header in the secondary caption
/// (`SettingsCard.Description`).
pub fn described_row(header: &str, description: &str, control: impl Into<View>) -> View {
    card(header_and_control(
        StackPanel::new().children((
            TextBlock::new()
                .text(header)
                .text_wrapping(TextWrapping::Wrap),
            TextBlock::new()
                .text(description)
                .text_wrapping(TextWrapping::Wrap)
                .font_size(CAPTION_FONT_SIZE)
                .opacity(SECONDARY_OPACITY),
        )),
        control,
    ))
}

fn card(content: View) -> View {
    Border::new()
        .background(ThemeBrush::CardBackground)
        .border_brush(ThemeBrush::CardStroke)
        .border_thickness(Thickness::uniform(1.0))
        .corner_radius(CornerRadius::uniform(CARD_CORNER_RADIUS))
        .padding(padding())
        .min_height(CARD_MIN_HEIGHT)
        .content(content)
}

/// A setting that belongs to the row above it, inside a `group`: the same
/// line, without a card of its own — the group already draws one
/// (`SettingsExpanderItem`, which is shorter than a card).
///
/// The step in is at the LEFT only. The right padding stays the card's, so
/// every control in the group — the parent's and the children's — sits on
/// one vertical line while only the labels say which is which.
///
/// `is_enabled` greys the label as well as the control, the way the Mac's
/// `.disabled(!isKautianEnabled)` greys the whole `Group`. The row stays in
/// the tree and stays readable either way — greyed, never removed, never
/// cleared.
pub fn sub_row(header: &str, is_enabled: bool, control: impl Into<View>) -> View {
    Border::new()
        .padding(Thickness::new(
            CARD_PADDING.0 + GROUP_INDENT,
            CARD_PADDING.1,
            CARD_PADDING.0,
            CARD_PADDING.1,
        ))
        .min_height(SUB_ROW_MIN_HEIGHT)
        .content(dimmable_line(header, is_enabled, control))
}

/// A parent setting and the settings that belong to it, in ONE card that is
/// always open: what a `SettingsExpander` looks like expanded, without the
/// chevron.
///
/// Not an `Expander` with `is_expanded(true)`: its header is a
/// `ToggleButton`, so a click, Space or Enter still collapses it whatever
/// the property says, and taking that away needs a `ControlTemplate` the
/// reactor does not expose. These subcollections are browsed, never
/// collapsed (USER 2026-08-31), so the card that cannot collapse is the
/// honest shape — and it drops the two overlapping interactions the old
/// header carried (expand the group / flip the switch inside it).
///
/// The card carries no padding of its own; each row carries its own, which
/// is what lets `sub_row` step its label in without moving its control.
/// `children` are keyed so a row keeps its identity on its settings key.
pub fn group(
    parent: impl Into<View>,
    children: impl IntoIterator<Item = (&'static str, View)>,
) -> View {
    Border::new()
        .background(ThemeBrush::CardBackground)
        .border_brush(ThemeBrush::CardStroke)
        .border_thickness(Thickness::uniform(1.0))
        .corner_radius(CornerRadius::uniform(CARD_CORNER_RADIUS))
        // `Border`'s content is single-child, for the reason written out on
        // `frame`: the panel is what makes a parent row plus its children
        // one native root.
        .content(
            StackPanel::new().children([
                Border::new()
                    .padding(padding())
                    .min_height(CARD_MIN_HEIGHT)
                    .content(parent),
                StackPanel::new().keyed_children(children),
            ]),
        )
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
/// into `labels`. `is_enabled` is false for a row another setting has made
/// moot — greyed, never cleared, like `switch_row`, so the stored choice
/// shows and comes back with the setting that masks it.
pub fn choice_row(
    header: &str,
    labels: Vec<String>,
    selected: Option<usize>,
    is_enabled: bool,
    on_change: Callback<Option<usize>>,
) -> View {
    row(
        header,
        ComboBox::new()
            .items_source(labels)
            .is_enabled(is_enabled)
            // Safe where a `ListView`'s is not (`list_selection`): a pop-up's items are a
            // PROPERTY, and reactor's visitor emits `ComboBoxItemsSource` before
            // `ComboBoxSelectedIndex` in the same batch.
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

/// `OpenInNewWindow`, the glyph Windows 11 Settings puts at the right of a
/// card that opens somewhere outside the app.
const OPEN_IN_NEW_WINDOW_GLYPH: &str = "\u{E8A7}";
/// Between a link row's leading mark and its title: the gap a `Label`
/// leaves on the Mac (`ExternalLinkButton.Metrics.rowSpacing`).
const LINK_MARK_GAP: f64 = 8.0;
/// `ButtonPadding`'s horizontal 11, which this pinned `windows-reactor`
/// cannot change; the row's content adds the rest, so its text starts on
/// the same line as the card padding above it.
const BUTTON_PADDING_X: f64 = 11.0;
const LINK_ROW_INSET: f64 = CARD_PADDING.0 - BUTTON_PADDING_X;

/// Rows that are links, in ONE card — the Mac's grouped section of
/// `ExternalLinkButton.Style.row`s: a mark at the left, the title, and the
/// leave-the-app glyph at the right, each row the button. Windows 11
/// Settings groups its outbound links the same way (the "Related links"
/// card), so the shape is native on both sides.
///
/// Each row is a subtle `Button` rather than a `Border`: the row must take
/// the click, and subtle is the style that draws nothing until the pointer
/// lifts a fill under it — the card, not the button, is what the eye sees.
pub fn link_group(rows: impl IntoViews) -> View {
    Border::new()
        .background(ThemeBrush::CardBackground)
        .border_brush(ThemeBrush::CardStroke)
        .border_thickness(Thickness::uniform(1.0))
        .corner_radius(CornerRadius::uniform(CARD_CORNER_RADIUS))
        .content(StackPanel::new().children(rows))
}

/// One row of a `link_group`.
pub fn link_row(mark: impl Into<View>, title: &str, on_click: Callback<()>) -> View {
    Button::new()
        .style(ButtonStyle::Subtle)
        .on_click(on_click)
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .horizontal_content_alignment(HorizontalAlignment::Stretch)
        .min_height(SUB_ROW_MIN_HEIGHT)
        .automation_name(title)
        .content(
            Grid::new()
                .columns([GridLength::Auto, GridLength::STAR, GridLength::Auto])
                .column_spacing(LINK_MARK_GAP)
                .margin(Thickness::xy(LINK_ROW_INSET, 0.0))
                .children((
                    Border::new()
                        .grid_column(0)
                        .vertical_alignment(VerticalAlignment::Center)
                        .content(mark),
                    TextBlock::new()
                        .text(title)
                        .vertical_alignment(VerticalAlignment::Center)
                        .grid_column(1),
                    FontIcon::new()
                        .glyph(OPEN_IN_NEW_WINDOW_GLYPH)
                        .vertical_alignment(VerticalAlignment::Center)
                        .opacity(0.65)
                        .grid_column(2),
                )),
        )
}

/// A section's title with a count at the line's right (`{matched} / {total}`).
const BADGE_RADIUS: f64 = 4.0;
const BADGE_PADDING_X: f64 = 6.0;
const BADGE_PADDING_Y: f64 = 2.0;
const BADGE_FONT_SIZE: f64 = 12.0;
const BADGE_TEXT_OPACITY: f64 = 0.65;

/// A small bordered tag after a row's text — a dictionary source, a learned
/// row (§50). Mirrors iOS `TagBadge`.
pub fn badge(text: &str) -> View {
    Border::new()
        .background(ThemeBrush::CardBackground)
        .border_brush(ThemeBrush::CardStroke)
        .border_thickness(Thickness::uniform(1.0))
        .corner_radius(CornerRadius::uniform(BADGE_RADIUS))
        .padding(Thickness::xy(BADGE_PADDING_X, BADGE_PADDING_Y))
        .content(
            TextBlock::new()
                .text(text)
                .font_size(BADGE_FONT_SIZE)
                .opacity(BADGE_TEXT_OPACITY),
        )
}

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
