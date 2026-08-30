//! The window's copy of `settings.json` and the one way it changes.
//! Both windows own one — the egui window today, the WinUI one from
//! roadmap W17 — so the atomic-write-and-report rule is written once.

// 中文: 視窗持有的 settings.json 副本與唯一寫入路徑 — 原子更新、失敗回報、跟隨檔案。

use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use taigi_windows_core::settings::SettingsDocument;
use taigi_windows_core::strings::StringResolver;
use taigi_windows_storage::LiveSettings;

/// How long an idle window waits before reading the file again: one `stat`
/// a second is nothing, and a chord's effect showing within a second reads
/// as live (roadmap W10 — `@AppStorage`'s job on the Mac).
pub const IDLE_REFRESH_INTERVAL: Duration = Duration::from_secs(1);
/// While a check or a download is in flight the answer is wanted sooner.
pub const BUSY_REFRESH_INTERVAL: Duration = Duration::from_millis(100);

/// What the banner says when there is no `%APPDATA%`: the missing
/// variable's name is the whole diagnosis.
const READ_ONLY_DETAIL: &str = "APPDATA";

pub struct SettingsWriter {
    /// Shared rather than owned: the WinUI window's launch record is
    /// `Rc`-held (a Reactor component's input must be `Clone`), and one
    /// `LiveSettings` per process is the point — it holds the fingerprint
    /// that decides whether the file moved.
    live: Rc<LiveSettings>,
    /// No per-user directory: the window shows the defaults and refuses
    /// every write, saying so from the first frame — never a file the DLL
    /// would not read (roadmap W2's unsupported-capability rule).
    is_read_only: bool,
    document: Arc<SettingsDocument>,
    write_failure: Option<String>,
}

impl SettingsWriter {
    pub fn new(live: Rc<LiveSettings>, is_read_only: bool) -> Self {
        let document = live.refresh_if_changed();
        Self {
            live,
            is_read_only,
            document,
            write_failure: is_read_only.then(|| READ_ONLY_DETAIL.to_owned()),
        }
    }

    pub fn document(&self) -> &SettingsDocument {
        &self.document
    }

    pub fn strings(&self) -> StringResolver {
        crate::presentation::strings_for(&self.document)
    }

    pub fn is_read_only(&self) -> bool {
        self.is_read_only
    }

    /// The last write that failed, shown as a banner until a write
    /// succeeds: a control that snaps back with no word looks broken.
    pub fn write_failure(&self) -> Option<&str> {
        self.write_failure.as_deref()
    }

    /// Re-reads the file if its fingerprint moved, so a change made
    /// outside — the DLL's own write for a chord or a menu row — shows
    /// without a restart.
    pub fn refresh(&mut self) {
        self.document = self.live.refresh_if_changed();
    }

    /// One atomic edit (lock, load, mutate, save), then this copy follows
    /// the file — the same path the DLL's own writes take, so two writers
    /// cannot lose each other's change. A failed write is reported, not
    /// swallowed; a read-only window refuses without touching its banner.
    pub fn update(&mut self, mutate: impl FnOnce(&mut SettingsDocument)) {
        if self.is_read_only {
            return;
        }
        match self.live.store().update(mutate) {
            Ok(_) => self.write_failure = None,
            Err(error) => {
                log::error!("settings.update_failed error={error}");
                self.write_failure = Some(error.to_string());
            }
        }
        self.refresh();
    }
}
