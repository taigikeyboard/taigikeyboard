//! The Manage Dictionaries search's presentation policy, shared by both
//! settings windows.
//! The search itself is `engine::dictionary_search`.

use std::time::Duration;

/// How many results the pane shows (`visibleResultLimit`).
pub const VISIBLE_RESULT_LIMIT: usize = 5;

/// How long the box waits after the last keystroke before it searches
/// (`debounce`).
pub const DEBOUNCE: Duration = Duration::from_millis(300);

/// The tallest the hit list grows, in device-independent pixels, before it
/// scrolls — so the toggles above keep most of the pane (mobile's
/// `.frame(maxHeight: 200)`).
pub const RESULTS_MAX_HEIGHT: u16 = 200;
