//! One paged, filtered user-data list on a settings page, shared by both
//! settings windows and by the Custom Dictionary and Learning Records
//! panes: which page of which filter is on screen and which load put it
//! there, and the selection. The shells own the widgets, the timers, and
//! the work slot a job runs in.

use super::presentation::PageMessage;
use crate::engine::user_data::{UserDataError, UserDataPage};
use crate::strings::StringKey;
use std::path::Path;
use std::time::Duration;

/// Rows per page (`CustomDictionaryPageModel.pageSize`).
pub const PAGE_SIZE: usize = 10;
/// `reloadWhenFilterSettles`: a word typed letter by letter is one query,
/// not six.
pub const FILTER_SETTLE: Duration = Duration::from_millis(200);
/// How long a job may run before the page says so (`overlayDelay`): a
/// millisecond-long write must not flash a spinner.
pub const OVERLAY_DELAY: Duration = Duration::from_millis(400);
/// The detail when a load's worker died before it answered.
pub const LOAD_DID_NOT_FINISH: &str = "the load did not finish";

/// A row a [`Listing`] holds: how it is selected, and what its pane says
/// when there is none or a read or write failed.
pub trait ListedRow: Clone {
    /// What a selection holds — never an index, which moves under a reload.
    type Id: Clone + PartialEq + std::fmt::Debug;
    /// The empty list while no filter is typed: a STATE of the store.
    const EMPTY: StringKey;
    /// The notice a failed load shows.
    const READ_FAILED: StringKey;
    /// The notice a failed write shows.
    const WRITE_FAILED: StringKey;
    fn id(&self) -> &Self::Id;
}

/// What a job hands back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobOutcome {
    pub message: Option<PageMessage>,
    /// Whether the list changed and must be reloaded.
    pub is_reload_wanted: bool,
}

impl JobOutcome {
    /// A job on a list of `Row` whose worker died: said under the pane's
    /// write-failure title, and the list reloaded in case the store changed
    /// before it did.
    pub fn did_not_finish<Row: ListedRow>() -> Self {
        Self {
            message: Some(PageMessage::failure(
                Row::WRITE_FAILED,
                "the operation did not finish",
            )),
            is_reload_wanted: true,
        }
    }

    /// A job on a list of `Row` whose worker the runtime would not start:
    /// nothing ran, so nothing reloads.
    pub fn could_not_start<Row: ListedRow>() -> Self {
        Self {
            message: Some(PageMessage::failure(
                Row::WRITE_FAILED,
                "the operation could not be started",
            )),
            is_reload_wanted: false,
        }
    }
}

/// Writes an export's bytes, answering only its failure.
pub(super) fn write_export(
    bytes: Result<Vec<u8>, UserDataError>,
    path: &Path,
    write_file: impl FnOnce(&Path, &[u8]) -> std::io::Result<()>,
) -> JobOutcome {
    let outcome = bytes
        .map_err(|error| error.to_string())
        .and_then(|bytes| write_file(path, &bytes).map_err(|error| error.to_string()));
    JobOutcome {
        message: outcome
            .err()
            .map(|error| PageMessage::failure(StringKey::CommonExportFailed, error)),
        is_reload_wanted: false,
    }
}

/// The job a page started and still waits on, and whether its busy
/// overlay is showing. The work slot itself (one job at a time, refused
/// not queued) is the shell's; this is the page's side of it.
#[derive(Debug, Default)]
pub struct JobState {
    generation: Option<u64>,
    is_busy_shown: bool,
    /// The generation `start_next` handed out last.
    last_generation: u64,
}

impl JobState {
    /// The slot's job `generation` is now this page's; nothing shows busy
    /// until the overlay delay passes.
    pub fn start(&mut self, generation: u64) {
        self.generation = Some(generation);
        self.is_busy_shown = false;
    }

    /// Starts the page's next job under a generation of its own counting —
    /// for a shell whose page owns its work slot (Windows); the Linux
    /// window's shared slot hands its generation to `start`.
    pub fn start_next(&mut self) -> u64 {
        self.last_generation = self.last_generation.wrapping_add(1);
        self.start(self.last_generation);
        self.last_generation
    }

    /// The job at `generation` came back. `true` when it is still this
    /// page's — the page reloads or redraws; `false` for a job a rebuilt
    /// page inherited, whose outcome the shell has already reported.
    pub fn finish(&mut self, generation: u64) -> bool {
        if self.generation != Some(generation) {
            return false;
        }
        self.generation = None;
        self.is_busy_shown = false;
        true
    }

    /// The overlay delay for `generation` passed. `true` when that job
    /// still runs: the page now shows busy.
    pub fn show_busy(&mut self, generation: u64) -> bool {
        if self.generation != Some(generation) {
            return false;
        }
        self.is_busy_shown = true;
        true
    }

    pub fn is_busy_shown(&self) -> bool {
        self.is_busy_shown
    }

    /// Whether a job this page started has not come back yet — for a shell
    /// whose page owns its work slot (Windows), what refuses a second job.
    pub fn is_running(&self) -> bool {
        self.generation.is_some()
    }
}

/// The page of a list on screen and the filter that chose it.
#[derive(Debug, Default)]
pub struct Listing<Row: ListedRow> {
    pub rows: Vec<Row>,
    /// Every row, for the section header — what the store HOLDS, which is
    /// not what the current filter matches.
    pub total_count: usize,
    /// How many rows the current filter matches; what the pager divides.
    pub match_count: usize,
    /// Which page is on screen, zero-based.
    pub page: usize,
    /// The row the list has selected.
    pub selected_id: Option<Row::Id>,
    /// The box as typed; written only through `set_filter`, which also
    /// makes every load in flight stale.
    filter: String,
    /// Which load the rows on screen came from: a load started under an
    /// older filter can come back after a newer one has, and would put rows
    /// on screen that do not match the box. The newest wins by number.
    load_generation: u64,
}

/// A load to run off the UI thread.
#[derive(Debug)]
pub struct LoadRequest {
    pub generation: u64,
    pub filter: String,
    pub page: usize,
}

/// What a finished load did to the listing.
#[derive(Debug, PartialEq, Eq)]
pub enum LoadLanded {
    /// A newer load or filter has started since; nothing changed.
    Stale,
    Adopted,
    /// Not an empty list: "empty" and "could not be read" look the same on
    /// screen, and only one is worth doing something about. The rows
    /// already shown stay; this is the notice to show.
    Failed(PageMessage),
}

impl<Row: ListedRow> Listing<Row> {
    pub fn filter(&self) -> &str {
        &self.filter
    }

    /// The filter box changed. `None` when it did not really; otherwise the
    /// generation the settle timer carries — the reload waits for the box
    /// to settle.
    pub fn set_filter(&mut self, filter: String) -> Option<u64> {
        if filter == self.filter {
            return None;
        }
        self.filter = filter;
        Some(self.next_generation())
    }

    /// The settle timer for `generation` fired. `true` when the box has not
    /// moved since: the list reloads from the first page.
    pub fn settle(&mut self, generation: u64) -> bool {
        if generation != self.load_generation {
            return false;
        }
        self.page = 0;
        true
    }

    /// A load of the page on screen starts; every earlier one is now stale.
    /// The engine answers the page it really served — pulled back inside
    /// the list if the list shrank under it — and `land` adopts that one.
    /// The filter goes as typed: the engine trims it (`userdata`
    /// `custom_dictionary.rs` / `learning_records.rs`), as for macOS.
    pub fn begin_load(&mut self) -> LoadRequest {
        LoadRequest {
            generation: self.next_generation(),
            filter: self.filter.clone(),
            page: self.page,
        }
    }

    /// The load started at `generation` came back.
    pub fn land(
        &mut self,
        generation: u64,
        outcome: Result<UserDataPage<Row>, String>,
    ) -> LoadLanded {
        if generation != self.load_generation {
            return LoadLanded::Stale;
        }
        match outcome {
            Ok(loaded) => {
                self.page = loaded.page;
                self.rows = loaded.rows;
                self.match_count = loaded.match_count;
                self.total_count = loaded.total_count;
                // A selection the new page does not hold is no selection:
                // the ✎ and − buttons must not act on a row that is not on
                // screen.
                if self.selected_index().is_none() {
                    self.selected_id = None;
                }
                LoadLanded::Adopted
            }
            Err(detail) => LoadLanded::Failed(PageMessage::failure(Row::READ_FAILED, detail)),
        }
    }

    /// Back to the first page with nothing selected, the filter kept — for
    /// another kind or order of the same list.
    pub fn rewind(&mut self) {
        self.page = 0;
        self.selected_id = None;
    }

    /// Steps `delta` pages. `false` past either end: nothing to load.
    pub fn step_page(&mut self, delta: isize) -> bool {
        match self.page.checked_add_signed(delta) {
            Some(target) if target < self.page_count() => {
                self.page = target;
                true
            }
            _ => false,
        }
    }

    fn next_generation(&mut self) -> u64 {
        self.load_generation = self.load_generation.wrapping_add(1);
        self.load_generation
    }

    pub fn selected_row(&self) -> Option<&Row> {
        let id = self.selected_id.as_ref()?;
        self.rows.iter().find(|row| row.id() == id)
    }

    pub fn selected_index(&self) -> Option<usize> {
        let id = self.selected_id.as_ref()?;
        self.rows.iter().position(|row| row.id() == id)
    }

    /// Never fewer than one: an empty list is still page 1 of 1.
    pub fn page_count(&self) -> usize {
        self.match_count.div_ceil(PAGE_SIZE).max(1)
    }

    /// What the store holds — and, while a filter narrows it, how much of
    /// that the filter matches. Against the counts, not against the filter
    /// box: a filter that matches everything says nothing by saying
    /// "17000 / 17000".
    pub fn count_label(&self) -> String {
        if self.match_count < self.total_count {
            format!("{} / {}", self.match_count, self.total_count)
        } else {
            self.total_count.to_string()
        }
    }

    /// Which sentence the empty list shows, or `None` while there are rows:
    /// an empty store is a STATE, a filter matching nothing is a RESULT of
    /// what was typed (`CustomDictionaryPage.swift:378-396`).
    pub fn empty_state_key(&self) -> Option<StringKey> {
        if !self.rows.is_empty() {
            return None;
        }
        Some(if self.filter.is_empty() {
            Row::EMPTY
        } else {
            StringKey::DictionaryNoResults
        })
    }
}
