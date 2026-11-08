//! The Manage Dictionaries search's presentation policy, shared by both
//! settings windows.
//! The search itself is `engine::dictionary_search`.

use std::time::Duration;

/// How many results the pane shows (`visibleResultLimit`) — all of them at
/// full height: mobile's 200-point cap on the list holds four and a half of
/// a phone's rows, but under five of a desktop's taller ones, so the
/// desktops bound the list by this count instead.
pub const VISIBLE_RESULT_LIMIT: usize = 5;

/// How long the box waits after the last keystroke before it searches
/// (`debounce`).
pub const DEBOUNCE: Duration = Duration::from_millis(300);
