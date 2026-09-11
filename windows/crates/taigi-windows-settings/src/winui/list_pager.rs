//! The bar under a paged list: its verbs (`+` / `−` …) leading, the `n / N`
//! readout and the two page arrows trailing (自訂詞庫, 字型管理). Port of
//! `UserDataListPager` beside `UserDataListControls`.
//!
//! A list is paged rather than scrolled because a fixed-height list inside the
//! pane's own scroll view is one scroll view inside another, and the inner one
//! does not scroll (USER, real device, 2026-08-26). A page that FITS the list
//! needs no scroller of its own, and every row is reachable by paging.

use super::window::{Message as WindowMessage, SettingsWindow};
use taigi_windows_core::strings::{StringKey, StringResolver};
use windows_reactor::*;

/// The whole bar. `page` is zero-based and shown one-based; `is_enabled` gates
/// both arrows on top of their own edge conditions; `on_page` is what turning
/// to a page sends.
pub fn bar(
    verbs: impl IntoViews,
    page: usize,
    page_count: usize,
    is_enabled: bool,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
    on_page: impl Fn(usize) -> WindowMessage + Clone + 'static,
) -> View {
    let backward = on_page.clone();
    Grid::new()
        .columns([GridLength::Auto, GridLength::STAR, GridLength::Auto])
        .margin(Thickness::new(0.0, CONTROL_GAP, 0.0, 0.0))
        .children((
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(CONTROL_GAP)
                .grid_column(0)
                .children(verbs),
            // Digits only: the pager needs no wording in five languages.
            TextBlock::new()
                .text(format!("{} / {}", page + 1, page_count))
                .opacity(SECONDARY_OPACITY)
                .horizontal_alignment(HorizontalAlignment::Right)
                .vertical_alignment(VerticalAlignment::Center)
                .margin(Thickness::xy(CONTROL_GAP, 0.0))
                .grid_column(1),
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(CONTROL_GAP)
                .grid_column(2)
                .children((
                    icon_button(
                        PREVIOUS_GLYPH,
                        strings.resolve(StringKey::DesktopActionPageBackward),
                        is_enabled && page > 0,
                        context.callback(move |()| backward(page.saturating_sub(1))),
                    ),
                    icon_button(
                        NEXT_GLYPH,
                        strings.resolve(StringKey::DesktopActionPageForward),
                        is_enabled && page + 1 < page_count,
                        context.callback(move |()| on_page(page + 1)),
                    ),
                )),
        ))
}

/// How many pages `match_count` rows fill — at least one, so an empty list
/// still reads as "1 / 1" rather than as a pager with nothing in it.
pub fn page_count(match_count: usize, page_size: usize) -> usize {
    match_count.div_ceil(page_size).max(1)
}

pub fn icon_button(glyph: &str, tooltip: &str, is_enabled: bool, on_click: Callback<()>) -> View {
    Button::new()
        .is_enabled(is_enabled)
        .on_click(on_click)
        .content(FontIcon::new().glyph(glyph))
        .tooltip(tooltip)
}

pub const CONTROL_GAP: f64 = 8.0;
pub const SECONDARY_OPACITY: f64 = 0.65;
const PREVIOUS_GLYPH: &str = "\u{E76B}";
const NEXT_GLYPH: &str = "\u{E76C}";
