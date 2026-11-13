//! The Appearance pane, the Linux row set (roadmap L4): layout, the two
//! scripts' sizes and typefaces, the window's padding and corners, and its
//! color theme. A text field at the top is for trying the running input method;
//! the candidate window that opens there is the real one. Default follows the
//! desktop's color scheme and accent inside that window. Light / dark mode
//! and the single Candidate Text Size stay hidden. The layout picker offers
//! Horizontal, Vertical, Horizontal Expandable and Vertical Expandable. The
//! four color rows show when the theme is Custom.

use super::PageContext;
use adw::prelude::*;
use taigi_desktop_core::settings::{
    keys, CandidateColorTheme, CandidateDisplayMode, CandidateLayout, SettingChoice,
    SettingsDocument,
};
use taigi_desktop_core::strings::StringKey;

const PANEL_LAYOUTS: &[CandidateLayout] = CandidateLayout::ALL;

fn panel_layout(document: &SettingsDocument) -> CandidateLayout {
    document.choice(&keys::CANDIDATE_LAYOUT)
}

pub fn build<'a>(
    mut context: PageContext<'a>,
    page: &adw::PreferencesPage,
) -> (PageContext<'a>, gtk::Box) {
    // The preferences page clamps and insets its rows. The field uses the
    // same clamp and the same 12px side margin, so its left edge lines up.
    let field = gtk::Entry::builder()
        .placeholder_text(
            context
                .strings
                .resolve(StringKey::DesktopAppearanceTryPlaceholder),
        )
        .hexpand(true)
        .build();
    let pinned = gtk::Box::builder()
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    pinned.append(&field);
    let clamp = adw::Clamp::new();
    clamp.set_child(Some(&pinned));
    let bar = gtk::Box::new(gtk::Orientation::Vertical, 0);
    bar.append(&clamp);

    let group = adw::PreferencesGroup::new();
    let strings = context.strings;
    let labels = PANEL_LAYOUTS
        .iter()
        .map(|layout| strings.resolve(layout.label_key()).to_owned())
        .collect();
    let current = panel_layout(context.document);
    context.picker_row(
        &group,
        strings.resolve(StringKey::DesktopCandidateWindowLayout),
        labels,
        PANEL_LAYOUTS,
        current,
        |layout, document| document.set_choice(&keys::CANDIDATE_LAYOUT, layout),
        panel_layout,
    );
    context.choice_row(
        &group,
        StringKey::SettingsCandidateDisplayMode,
        CandidateDisplayMode::ALL,
        keys::CANDIDATE_DISPLAY_MODE,
        CandidateDisplayMode::label_key,
    );
    context.spin_row(
        &group,
        StringKey::DesktopHanjiFontSize,
        keys::HANJI_FONT_SIZE,
        keys::CANDIDATE_FONT_PX_RANGE,
    );
    context.spin_row(
        &group,
        StringKey::DesktopRomanFontSize,
        keys::ROMAN_FONT_SIZE,
        keys::CANDIDATE_FONT_PX_RANGE,
    );
    context.spin_row(
        &group,
        StringKey::DesktopCandidateWindowPadding,
        keys::CANDIDATE_WINDOW_PADDING,
        keys::CANDIDATE_CHROME_PX_RANGE,
    );
    context.spin_row(
        &group,
        StringKey::DesktopCandidateCornerRadius,
        keys::CANDIDATE_CORNER_RADIUS,
        keys::CANDIDATE_CHROME_PX_RANGE,
    );
    context.highlight_fill_row(&group);
    page.add(&group);

    let fonts = adw::PreferencesGroup::new();
    context.font_row(&fonts, StringKey::DesktopHanjiFont, keys::HANJI_FONT_FAMILY);
    context.font_row(&fonts, StringKey::DesktopRomanFont, keys::ROMAN_FONT_FAMILY);
    page.add(&fonts);

    let colors = adw::PreferencesGroup::new();
    let theme_labels: Vec<String> = CandidateColorTheme::ALL
        .iter()
        .map(|theme| strings.resolve(theme.label_key()).to_owned())
        .collect();
    context.picker_row(
        &colors,
        strings.resolve(StringKey::DesktopCandidateColorTheme),
        theme_labels,
        CandidateColorTheme::ALL,
        keys::shown_candidate_color_theme(context.document),
        |theme, document| keys::apply_candidate_color_theme(document, theme),
        keys::shown_candidate_color_theme,
    );
    let text = context.color_row(
        &colors,
        StringKey::DesktopCandidateTextColor,
        keys::CANDIDATE_TEXT_COLOR,
    );
    let selected_text = context.color_row(
        &colors,
        StringKey::DesktopCandidateSelectedTextColor,
        keys::CANDIDATE_SELECTED_TEXT_COLOR,
    );
    let background = context.color_row(
        &colors,
        StringKey::DesktopCandidateBackgroundColor,
        keys::CANDIDATE_BACKGROUND_COLOR,
    );
    let selected_background = context.color_row(
        &colors,
        StringKey::DesktopCandidateSelectedBackgroundColor,
        keys::CANDIDATE_SELECTED_BACKGROUND_COLOR,
    );
    for row in [text, selected_text, background, selected_background] {
        context.bind_visible(&row, |document| {
            keys::shown_candidate_color_theme(document) == CandidateColorTheme::Custom
        });
    }
    page.add(&colors);

    context.reset_row(page, SettingsDocument::reset_appearance);
    (context, bar)
}
