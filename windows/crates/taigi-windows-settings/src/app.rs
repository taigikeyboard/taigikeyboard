//! The window: a sidebar of panes and the selected pane's form, over one
//! live `settings.json`. Port of `SettingsSplitView.swift` +
//! `SettingsWindowController.swift`: the selection persists
//! (`selectedSettingsPane`), the title names the pane, the window follows
//! the 外觀 setting, and every value is re-read each frame — with a frame
//! requested every second while idle, since eframe repaints only on events
//! and the DLL's own writes (a TL/POJ chord) are not events — so a change
//! made outside shows without a restart (W10; `@AppStorage`'s job on the
//! Mac).

// 中文: 設定視窗本體 — 側欄 + 目前 pane;每一幀重讀 settings.json(閒置時每秒要一幀),寫入走原子更新;寫失敗顯示橫幅。

use crate::panes;
use crate::widgets::recorder::RecorderState;
use std::sync::Arc;
use std::time::Duration;
use taigi_windows_core::settings::{keys, AppearanceMode, SettingsDocument, SettingsPane};
use taigi_windows_core::strings::{DisplayLanguage, StringKey, StringResolver};
use taigi_windows_storage::LiveSettings;

/// How long an idle window waits before checking the file again: one
/// `stat` a second is nothing, and a chord's effect showing within a second
/// reads as live.
const IDLE_REFRESH_INTERVAL: Duration = Duration::from_secs(1);

/// The resolver for the document's display language, `system` resolved
/// against the machine (`DisplayLanguageStore.syncFromSettings`).
pub fn strings_for(document: &SettingsDocument) -> StringResolver {
    let tag = document.string(&keys::DISPLAY_LANGUAGE);
    let language =
        DisplayLanguage::from_tag(&tag).effective(&taigi_windows_platform::system_locale());
    StringResolver::new(language)
}

/// The window title = the selected pane's label (`SettingsSplitViewController`).
pub fn pane_title(strings: &StringResolver, pane: SettingsPane) -> String {
    strings
        .resolve(pane.title_key().unwrap_or(StringKey::DesktopGeneralTab))
        .to_owned()
}

pub struct SettingsApp {
    live: LiveSettings,
    document: Arc<SettingsDocument>,
    pane: SettingsPane,
    /// No per-user directory (`%APPDATA%` unset): the window shows the
    /// defaults and refuses every write, saying so — never a file the DLL
    /// would not read (roadmap W2's unsupported-capability rule).
    is_read_only: bool,
    /// The last write that failed, shown as a banner until a write
    /// succeeds: a control that snaps back with no word is a control that
    /// looks broken.
    write_failure: Option<String>,
    pub recorder: RecorderState,
    /// A URL the browser refused to open, shown until dismissed
    /// (`ExternalLinkButton.swift:740-744`).
    pub failed_url: Option<String>,
    title: String,
}

impl SettingsApp {
    pub fn new(
        creation: &eframe::CreationContext<'_>,
        live: LiveSettings,
        pane: SettingsPane,
        is_read_only: bool,
    ) -> Self {
        crate::fonts::install(&creation.egui_ctx);
        let document = live.refresh_if_changed();
        let title = pane_title(&strings_for(&document), pane);
        let mut app = Self {
            live,
            document,
            pane,
            is_read_only,
            write_failure: None,
            recorder: RecorderState::default(),
            failed_url: None,
            title,
        };
        // `--pane` is a selection like a click: persisted, so the next
        // plain launch reopens there too.
        if app.document.choice(&keys::SELECTED_SETTINGS_PANE) != pane {
            app.select_pane(pane);
        }
        app
    }

    pub fn document(&self) -> &SettingsDocument {
        &self.document
    }

    pub fn strings(&self) -> StringResolver {
        strings_for(&self.document)
    }

    pub fn pane(&self) -> SettingsPane {
        self.pane
    }

    /// The selection lives in `settings.json` like every other setting, as
    /// it does in `UserDefaults` on the Mac. The DLL reloads once for a
    /// pane click it does not care about — accepted: one `stat` and a parse
    /// of a small file, and one place for the window's state instead of two.
    pub fn select_pane(&mut self, pane: SettingsPane) {
        self.pane = pane;
        self.recorder = RecorderState::default();
        self.update_document(|document| document.set_choice(&keys::SELECTED_SETTINGS_PANE, pane));
    }

    /// One atomic edit of `settings.json` (lock, load, mutate, save), then
    /// the window's copy follows the file — the same path the DLL's own
    /// writes take, so two writers cannot lose each other's change. A
    /// failed write is reported, not swallowed.
    pub fn update_document(&mut self, mutate: impl FnOnce(&mut SettingsDocument)) {
        if self.is_read_only {
            self.write_failure = Some("APPDATA".to_owned());
            return;
        }
        match self.live.store().update(mutate) {
            Ok(_) => self.write_failure = None,
            Err(error) => {
                log::error!("settings.update_failed error={error}");
                self.write_failure = Some(error.to_string());
            }
        }
        self.document = self.live.refresh_if_changed();
    }

    fn sync_theme(&self, ctx: &egui::Context) {
        let mode: AppearanceMode = self.document.choice(&keys::APPEARANCE_MODE);
        let preference = match mode {
            AppearanceMode::Light => egui::ThemePreference::Light,
            AppearanceMode::Dark => egui::ThemePreference::Dark,
            AppearanceMode::Auto => egui::ThemePreference::System,
        };
        if ctx.options(|options| options.theme_preference) != preference {
            ctx.set_theme(preference);
        }
    }

    fn sync_title(&mut self, ctx: &egui::Context) {
        let title = pane_title(&self.strings(), self.pane);
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    fn show_write_failure(&self, ui: &mut egui::Ui) {
        let Some(detail) = &self.write_failure else {
            return;
        };
        let strings = self.strings();
        egui::Frame::NONE
            .fill(ui.visuals().error_fg_color.gamma_multiply(0.15))
            .inner_margin(10.0)
            .corner_radius(6.0)
            .show(ui, |ui| {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    strings.resolve(StringKey::DesktopSettingsWriteFailed),
                );
                ui.weak(detail);
            });
        ui.add_space(8.0);
    }
}

impl eframe::App for SettingsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Before any widget sees the frame's input: a recording row takes
        // every key of this frame for itself.
        self.recorder.intercept(ctx);
        // One `stat` per frame; the DLL's writes (a chord, a menu row) land
        // here without being told.
        self.document = self.live.refresh_if_changed();
        ctx.request_repaint_after(IDLE_REFRESH_INTERVAL);
        self.sync_theme(ctx);
        self.sync_title(ctx);
        panes::sidebar::show(ctx, self);
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                self.show_write_failure(ui);
                panes::show(ui, self);
            });
        });
        crate::widgets::external_link::show_failure(ctx, self);
    }
}
