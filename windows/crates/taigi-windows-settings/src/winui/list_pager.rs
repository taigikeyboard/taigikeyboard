//! The bar under a paged list: its verbs (`+` / `−` …) leading, the `n / N`
//! readout and the two page arrows trailing (Custom Dictionary, Manage Typefaces). Port of
//! `UserDataListPager` beside `UserDataListControls`.
//!
//! A list is paged rather than scrolled because a fixed-height list inside the
//! pane's own scroll view is one scroll view inside another, and the inner one
//! does not scroll (USER, real device, 2026-08-26). A page that FITS the list
//! needs no scroller of its own, and every row is reachable by paging.
//!
//! Also what every paged user-data page (Custom Dictionary, Learning
//! Records) shares around the bar: the table metrics, the busy overlay and
//! empty state, the dialog a command that empties a store asks first, and
//! how a load or a job runs off the UI thread.

use super::cards;
use super::window::{Message as WindowMessage, SettingsWindow};
use taigi_desktop_core::settings::listing::{
    JobOutcome, JobState, ListedRow, LOAD_DID_NOT_FINISH, OVERLAY_DELAY,
};
use taigi_desktop_core::settings::presentation::Confirmation;
use taigi_desktop_core::strings::{StringKey, StringResolver};
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
                        strings.resolve(StringKey::CommonPagePrevious),
                        is_enabled && page > 0,
                        context.callback(move |()| backward(page.saturating_sub(1))),
                    ),
                    icon_button(
                        NEXT_GLYPH,
                        strings.resolve(StringKey::CommonPageNext),
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

/// Starts a load off the UI thread; `loaded` wraps what comes back in the
/// page's own message. A load has no overlay: the rows already on screen
/// stay put while it runs.
pub fn spawn_load<T: Send + 'static>(
    context: &ComponentContext<SettingsWindow>,
    generation: u64,
    fetch: impl FnOnce() -> Result<T, String> + Send + 'static,
    loaded: fn(u64, Box<Result<T, String>>) -> WindowMessage,
) {
    _ = context.spawn_background_with_rejection(
        move |_| {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(fetch))
                .unwrap_or_else(|_| Err(LOAD_DID_NOT_FINISH.to_owned()));
            loaded(generation, Box::new(outcome))
        },
        // A thread the runtime would not start is a failure the user sees,
        // never a load that quietly never lands.
        loaded(
            generation,
            Box::new(Err("the load could not be started".to_owned())),
        ),
    );
}

/// Takes the page's one work slot for `job` — `false`, and nothing runs,
/// while a job already holds it (the macOS model: refused, not queued).
/// `finished` and `busy` wrap the page's own messages; a list of `Row`
/// names the failure title.
pub fn spawn_job<Row: ListedRow>(
    slot: &mut JobState,
    context: &ComponentContext<SettingsWindow>,
    job: impl FnOnce() -> JobOutcome + Send + 'static,
    finished: fn(u64, Box<JobOutcome>) -> WindowMessage,
    busy: fn(u64) -> WindowMessage,
) -> bool {
    if slot.is_running() {
        return false;
    }
    let generation = slot.start_next();
    _ = context.spawn_background_with_rejection(
        move |_| {
            // A panicking store call must not leave the slot held for the
            // life of the window.
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job))
                .unwrap_or_else(|_| JobOutcome::did_not_finish::<Row>());
            finished(generation, Box::new(outcome))
        },
        finished(generation, Box::new(JobOutcome::could_not_start::<Row>())),
    );
    // The overlay waits, so a millisecond-long write does not flash it.
    _ = context.spawn_background(move |_| {
        std::thread::sleep(OVERLAY_DELAY);
        busy(generation)
    });
    true
}

/// The running job's name over a ring; `None` until it has run long
/// enough to say so.
pub fn busy_overlay(label: Option<StringKey>, strings: &StringResolver) -> View {
    let Some(label) = label else {
        return View::empty();
    };
    cards::frame(
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(CONTROL_GAP)
            .horizontal_alignment(HorizontalAlignment::Center)
            .children((
                ProgressRing::new()
                    .is_active(true)
                    .width(OVERLAY_RING_SIZE)
                    .height(OVERLAY_RING_SIZE),
                TextBlock::new()
                    .text(strings.resolve(label))
                    .vertical_alignment(VerticalAlignment::Center),
            )),
    )
}

/// The question before a command that empties a store (`Confirmation`):
/// Delete is the primary button, so only `ContentDialogResult::Primary`
/// runs the command.
pub fn confirm_dialog(
    confirmation: Confirmation,
    strings: &StringResolver,
    on_closed: Callback<ContentDialogResult>,
) -> View {
    ContentDialog::new()
        .title(strings.resolve(confirmation.title))
        .primary_button_text(strings.resolve(StringKey::CommonDelete))
        .close_button_text(strings.resolve(StringKey::CommonCancel))
        .is_open(true)
        .on_closed(on_closed)
        .content(
            TextBlock::new()
                .text(strings.resolve(confirmation.message))
                .text_wrapping(TextWrapping::Wrap),
        )
}

/// What a list with no rows says (`Listing::empty_state_key`), laid OVER
/// the list; `None` while there are rows.
///
/// Words rather than the Mac's `tray` symbol: Segoe Fluent Icons carries no
/// empty-container glyph, and a Windows 11 empty state is a line of text —
/// which also keeps the sentence a screen reader is told from being an
/// accessibility label bolted onto a picture.
pub fn empty_state(key: Option<StringKey>, strings: &StringResolver) -> View {
    let Some(key) = key else {
        return View::empty();
    };
    TextBlock::new()
        .text(strings.resolve(key))
        .text_wrapping(TextWrapping::Wrap)
        .opacity(SECONDARY_OPACITY)
        .horizontal_alignment(HorizontalAlignment::Center)
        .vertical_alignment(VerticalAlignment::Center)
        .into()
}

/// Between the small controls under a list, and in its dialogs.
pub const CONTROL_GAP: f64 = 8.0;
/// WinUI's secondary text, as opacity, so it follows the theme.
pub const SECONDARY_OPACITY: f64 = 0.65;
/// A definite height, not a floor (`Metrics.tableHeight`): the page is
/// sized from the page size, so a short page keeps the controls under the
/// table where they were.
pub const TABLE_HEIGHT: f64 = 300.0;
pub const TABLE_COLUMN_GAP: f64 = 12.0;
/// The column header sits over the list's own item inset.
pub const TABLE_HEADER_INSET: f64 = 12.0;
pub const TABLE_HEADER_GAP: f64 = 8.0;
pub const OVERLAY_RING_SIZE: f64 = 20.0;
/// Segoe Fluent Icons: Add, Edit, Remove.
pub const ADD_GLYPH: &str = "\u{E710}";
pub const EDIT_GLYPH: &str = "\u{E70F}";
pub const REMOVE_GLYPH: &str = "\u{E738}";
const PREVIOUS_GLYPH: &str = "\u{E76B}";
const NEXT_GLYPH: &str = "\u{E76C}";
