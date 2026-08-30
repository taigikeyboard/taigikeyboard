//! The window: a sidebar of panes and the selected pane's form, over one
//! live `settings.json` and the three user-data stores. Port of
//! `SettingsSplitView.swift` + `SettingsWindowController.swift`: the
//! selection persists (`selectedSettingsPane`), the title names the pane,
//! the window follows the 外觀 setting, and every value is re-read each
//! frame — with a frame requested every second while idle, since eframe
//! repaints only on events and the DLL's own writes (a TL/POJ chord) are
//! not events — so a change made outside shows without a restart (W10;
//! `@AppStorage`'s job on the Mac).

// 中文: 設定視窗本體 — 側欄 + 目前 pane;每一幀重讀 settings.json(閒置時每秒要一幀),寫入走原子更新;寫失敗顯示橫幅。

use crate::fonts::InstalledFonts;
use crate::panes;
use crate::panes::custom_dictionary::CustomDictionaryPageModel;
use crate::panes::dictionary_search::DictionarySearchModel;
use crate::presentation::{pane_title, PageMessage};
use crate::settings_writer::{SettingsWriter, BUSY_REFRESH_INTERVAL, IDLE_REFRESH_INTERVAL};
use crate::theme;
use crate::updates::UpdateState;
use crate::widgets::recorder::RecorderState;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::path::PathBuf;
use taigi_windows_core::settings::{keys, AppearanceMode, SettingsDocument, SettingsPane};
use taigi_windows_core::strings::{StringKey, StringResolver};
use taigi_windows_storage::{LiveSettings, UserDataStores};

pub struct SettingsApp {
    settings: SettingsWriter,
    stores: UserDataStores,
    /// Whether this process has loaded the dictionaries into the engine —
    /// only the search page needs them, so its first query loads them.
    is_lexicon_loaded: bool,
    pane: SettingsPane,
    pub recorder: RecorderState,
    pub custom_dictionary: CustomDictionaryPageModel,
    pub search: DictionarySearchModel,
    pub updates: UpdateState,
    /// A URL the browser refused to open, shown until dismissed
    /// (`ExternalLinkButton.swift:740-744`).
    pub message: Option<PageMessage>,
    title: String,
    fonts: InstalledFonts,
    /// The window's own HWND, for the DWM title-bar colour; `None` off
    /// Windows or when winit gives no handle.
    hwnd: Option<isize>,
    /// What the title bar was last painted as, so DWM is told only on a
    /// change (a frame every second while idle).
    is_title_bar_dark: Option<bool>,
    /// The accent the visuals were last built with; rebuilt on a change,
    /// not per frame.
    accent: Option<egui::Color32>,
}

/// The Windows default blue, when DWM reports no accent (a remote session).
const FALLBACK_ACCENT: egui::Color32 = egui::Color32::from_rgb(0x00, 0x78, 0xD4);

impl SettingsApp {
    pub fn new(
        creation: &eframe::CreationContext<'_>,
        live: LiveSettings,
        data_directory: PathBuf,
        pane: SettingsPane,
        is_read_only: bool,
        is_check_now: bool,
    ) -> Self {
        let fonts = crate::fonts::install(&creation.egui_ctx);
        // Typography and spacing once, for both themes; the colours follow
        // the accent in `sync_accent`.
        creation.egui_ctx.all_styles_mut(theme::apply_style);
        let hwnd = match creation.window_handle().map(|handle| handle.as_raw()) {
            Ok(RawWindowHandle::Win32(handle)) => Some(handle.hwnd.get()),
            _ => None,
        };
        let settings = SettingsWriter::new(std::rc::Rc::new(live), is_read_only);
        let title = pane_title(&settings.strings(), pane);
        let stores = crate::user_data::open_at_launch(data_directory, is_read_only);
        let mut app = Self {
            settings,
            stores,
            is_lexicon_loaded: false,
            pane,
            recorder: RecorderState::default(),
            custom_dictionary: CustomDictionaryPageModel::default(),
            search: DictionarySearchModel::default(),
            updates: UpdateState::new(),
            message: None,
            title,
            fonts,
            hwnd,
            is_title_bar_dark: None,
            accent: None,
        };
        // The overdue daily check, or the menu's 檢查更新 (`--check-now`)
        // — the manual one always answers.
        if is_check_now {
            app.updates.check_manually(&mut app.settings);
        } else if !is_read_only {
            app.updates.check_if_due(&mut app.settings);
        }
        // `--pane` is a selection like a click: persisted, so the next
        // plain launch reopens there too.
        if app.document().choice(&keys::SELECTED_SETTINGS_PANE) != pane {
            app.select_pane(pane);
        }
        app
    }

    pub fn document(&self) -> &SettingsDocument {
        self.settings.document()
    }

    pub fn stores(&self) -> &UserDataStores {
        &self.stores
    }

    pub fn is_read_only(&self) -> bool {
        self.settings.is_read_only()
    }

    pub fn is_lexicon_loaded(&self) -> bool {
        self.is_lexicon_loaded
    }

    /// The search job loads the dictionaries when it has to; what it found
    /// is remembered so the next query does not load them again.
    pub fn note_lexicon_loaded(&mut self, is_loaded: bool) {
        self.is_lexicon_loaded = is_loaded;
    }

    pub fn strings(&self) -> StringResolver {
        self.settings.strings()
    }

    pub fn pane(&self) -> SettingsPane {
        self.pane
    }

    /// The selected pane's name — the window's caption and the page's
    /// heading, kept current by `sync_title` before the panes draw.
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn fonts(&self) -> InstalledFonts {
        self.fonts
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

    pub fn update_document(&mut self, mutate: impl FnOnce(&mut SettingsDocument)) {
        self.settings.update(mutate);
    }

    /// The 一般 pane's 檢查更新 press: the update state and the settings
    /// are both the window's, and they are disjoint fields.
    pub fn check_for_updates(&mut self) {
        self.updates.check_manually(&mut self.settings);
    }

    fn sync_theme(&mut self, ctx: &egui::Context) {
        let mode: AppearanceMode = self.document().choice(&keys::APPEARANCE_MODE);
        let preference = match mode {
            AppearanceMode::Light => egui::ThemePreference::Light,
            AppearanceMode::Dark => egui::ThemePreference::Dark,
            AppearanceMode::Auto => egui::ThemePreference::System,
        };
        if ctx.options(|options| options.theme_preference) != preference {
            ctx.set_theme(preference);
        }
        self.sync_accent(ctx);
        self.sync_title_bar(ctx);
    }

    /// The selection colour is the user's accent, as it is in every native
    /// Windows window (`Color.accentColor` on the Mac) — not egui's blue.
    /// Both themes' visuals are rebuilt on a change so a theme flip keeps it.
    fn sync_accent(&mut self, ctx: &egui::Context) {
        let accent = taigi_windows_platform::system_accent()
            .map_or(FALLBACK_ACCENT, |(r, g, b)| {
                egui::Color32::from_rgb(r, g, b)
            });
        if self.accent == Some(accent) {
            return;
        }
        self.accent = Some(accent);
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            ctx.set_visuals_of(theme, theme::visuals(theme, accent));
        }
    }

    /// The caption follows the client area: winit paints it light whatever
    /// the form draws, and a light caption over a dark form is what makes a
    /// window look foreign on Windows 11.
    fn sync_title_bar(&mut self, ctx: &egui::Context) {
        let Some(hwnd) = self.hwnd else {
            return;
        };
        let is_dark = ctx.theme() == egui::Theme::Dark;
        if self.is_title_bar_dark == Some(is_dark) {
            return;
        }
        self.is_title_bar_dark = Some(is_dark);
        taigi_windows_platform::set_dark_title_bar(hwnd, is_dark);
    }

    fn sync_title(&mut self, ctx: &egui::Context) {
        let title = pane_title(&self.strings(), self.pane);
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    fn show_write_failure(&self, ui: &mut egui::Ui) {
        let Some(detail) = self.settings.write_failure() else {
            return;
        };
        let strings = self.strings();
        egui::Frame::NONE
            .fill(ui.visuals().error_fg_color.gamma_multiply(0.15))
            .inner_margin(10.0)
            .corner_radius(theme::CONTROL_CORNER_RADIUS)
            .show(ui, |ui| {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    strings.resolve(StringKey::DesktopSettingsWriteFailed),
                );
                ui.weak(detail);
            });
        ui.add_space(8.0);
    }

    /// Collects the check and the download in flight; the manual outcome
    /// is shown as its own alert with its own buttons.
    fn drive_updates(&mut self, ctx: &egui::Context) {
        self.updates.poll(&mut self.settings);
        if self.updates.is_busy() {
            ctx.request_repaint_after(BUSY_REFRESH_INTERVAL);
        }
        let Some(outcome) = self.updates.manual_outcome().cloned() else {
            return;
        };
        let strings = self.strings();
        match crate::widgets::update_alert::show(ctx, &strings, &outcome) {
            Some(crate::widgets::update_alert::UpdateAlertAction::Proceed) => {
                self.message = self.updates.proceed_with_manual_outcome();
            }
            Some(crate::widgets::update_alert::UpdateAlertAction::Dismiss) => {
                self.updates.dismiss_manual_outcome();
            }
            None => {}
        }
    }

    /// ONE alert at a time, whichever page raised it first in this order;
    /// the next shows once it is dismissed (two `.alert`s on one chain do
    /// not stack on the Mac either).
    fn show_alert(&mut self, ctx: &egui::Context) {
        let strings = self.strings();
        let slot = if self.message.is_some() {
            &mut self.message
        } else if self.custom_dictionary.message.is_some() {
            &mut self.custom_dictionary.message
        } else {
            &mut self.search.message
        };
        crate::widgets::alert::show(ctx, &strings, slot);
    }
}

impl eframe::App for SettingsApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Before any widget sees the frame's input: a recording row takes
        // every key of this frame for itself.
        self.recorder.intercept(ctx);
        // One `stat` per frame; the DLL's writes (a chord, a menu row) land
        // here without being told.
        self.settings.refresh();
        ctx.request_repaint_after(IDLE_REFRESH_INTERVAL);
        self.sync_theme(ctx);
        self.sync_title(ctx);
        self.drive_updates(ctx);
        panes::sidebar::show(ctx, self);
        // No margin of its own: the pane's inset (`panes::FORM_INSET`) is
        // the only one, not egui's 8px plus it.
        let ground = egui::Frame::central_panel(&ctx.style()).inner_margin(0);
        egui::CentralPanel::default().frame(ground).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                self.show_write_failure(ui);
                panes::show(ui, self, frame);
            });
        });
        self.show_alert(ctx);
    }
}
