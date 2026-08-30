//! The settings window as one Reactor component: a `NavigationView` of the
//! panes, the selected pane's page in its content, and the window's own
//! chrome — Mica, the 外觀 theme, the title, the write-failure banner and
//! the one alert at a time. Port of `app.rs`'s job, message-driven:
//! `view` is immutable, so every control sends a message and the state
//! moves in `update`.
//!
//! Named divergences from the Mac (and from the egui window), all
//! deliberate: the window frame is not persisted; the 外觀 mode is a native
//! pop-up rather than three drawn thumbnails; the window may be narrowed
//! until `NavigationView` compacts its pane.

// 中文: WinUI 設定視窗本體 — NavigationView + 目前 pane 的頁面;每秒一次 tick 重讀 settings.json,寫入走原子更新。

use crate::presentation::{self, pane_title, PageMessage};
use crate::settings_writer::{SettingsWriter, BUSY_REFRESH_INTERVAL, IDLE_REFRESH_INTERVAL};
use crate::updates::{UpdateState, INSTALLED_VERSION};
use crate::winui::cards;
use crate::winui::pages;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;
use taigi_windows_core::settings::{
    keys, AppearanceMode, SettingChoice, SettingsDocument, SettingsKey, SettingsPane,
};
use taigi_windows_core::strings::{DisplayLanguage, StringKey, StringResolver};
use taigi_windows_storage::{LiveSettings, UserDataStores};
use taigi_windows_update::checker;
use windows_reactor::*;

/// A pane's form, over the window's state. Stateless presentation — the
/// guide's rule for a subtree that owns nothing.
type PageView = fn(&SettingsWindow, &StringResolver, &mut ViewContext<SettingsWindow>) -> View;

/// The panes this window has pages for, in sidebar order, each with the
/// page that draws it — one table, so a pane cannot be listed without a
/// page or reachable without a row. 快捷鍵 arrives with W17-B and the
/// dictionary pages with W17-C; the egui window lists all five until the
/// W17-C cutover, so a pane missing here is still reachable there.
const PANES: [(SettingsPane, PageView); 2] = [
    (SettingsPane::General, pages::general::view),
    (SettingsPane::Appearance, pages::appearance::view),
];

fn page_view(pane: SettingsPane) -> Option<PageView> {
    PANES
        .iter()
        .find(|(listed, _)| *listed == pane)
        .map(|(_, view)| *view)
}

/// `SettingsPaneLayout.swift:668-690`: sidebar 215 + detail 545.
const SIDEBAR_WIDTH: f64 = 215.0;
const INITIAL_WIDTH: f64 = SIDEBAR_WIDTH + 545.0;
const INITIAL_HEIGHT: f64 = 560.0;
/// The Mac's floor. The width floor is NOT the Mac's fixed 760: WinUI's
/// `NavigationView` compacts its pane below ~641px, and a window that can
/// never reach that threshold cannot use the adaptive behaviour the
/// control exists for (named divergence, roadmap W17).
const MINIMUM_WIDTH: f64 = 600.0;
const MINIMUM_HEIGHT: f64 = 470.0;
/// The page title over the form (`theme::title_text_style`).
const TITLE_FONT_SIZE: f64 = 28.0;
/// Space between a pane's content and the window's edge.
const FORM_INSET: f64 = 24.0;

/// What the window is launched with. `Rc` because a component's input must
/// be `Clone + PartialEq` and none of this is either; the root input never
/// changes, so identity is the right comparison.
pub struct Launch {
    live: Rc<LiveSettings>,
    /// Held open for the window's life, exactly as the egui window holds
    /// it: the launch migrations run off it, and the W17-C pages read it.
    _stores: UserDataStores,
    is_read_only: bool,
    pane: SettingsPane,
    is_check_now: bool,
}

#[derive(Clone)]
pub struct SettingsWindowInput(Rc<Launch>);

impl PartialEq for SettingsWindowInput {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// Answers whether the window ran to a normal close; a failed launch is a
/// non-zero exit so a gate can tell it from success.
pub fn run(
    live: LiveSettings,
    directory: PathBuf,
    pane: SettingsPane,
    is_read_only: bool,
    is_check_now: bool,
) -> bool {
    let input = SettingsWindowInput(Rc::new(Launch {
        live: Rc::new(live),
        _stores: crate::user_data::open_at_launch(directory, is_read_only),
        is_read_only,
        pane,
        is_check_now,
    }));
    match App::run_component::<SettingsWindow>(input) {
        Ok(()) => true,
        Err(error) => {
            log::error!("winui.window_failed error={error}");
            false
        }
    }
}

/// A settings write a picker already resolved: the key's name and the
/// value's stored spelling, both `'static`. Only the typed constructors
/// build one, so a row cannot pair a key with another key's value — and
/// the window needs no per-picker arm to apply it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsWrite {
    name: &'static str,
    raw: &'static str,
}

impl SettingsWrite {
    pub fn choice<T: SettingChoice>(key: &SettingsKey<T>, value: T) -> Self {
        Self {
            name: key.name,
            raw: value.raw(),
        }
    }

    /// The one picker whose roster is not a `SettingChoice`: the display
    /// language is stored as its own tag.
    pub fn display_language(language: DisplayLanguage) -> Self {
        Self {
            name: keys::DISPLAY_LANGUAGE.name,
            raw: language.tag(),
        }
    }

    fn apply(self, document: &mut SettingsDocument) {
        document.set_raw_string(self.name, self.raw);
    }
}

#[derive(Clone)]
pub enum Message {
    /// The live-reload beat; a generation that is not the current one is a
    /// tick whose task was re-armed or cancelled.
    Tick(u64),
    SelectPane(Option<String>),
    /// `None` when a pop-up cleared its selection: nothing to write.
    SetChoice(Option<SettingsWrite>),
    SetAutoSpace(bool),
    ResetAppearance,
    CheckForUpdates,
    ActOnOffer,
    UpdateAlertClosed(ContentDialogResult),
    DismissAlert,
    OpenUrl(&'static str),
}

pub struct SettingsWindow {
    launch: Rc<Launch>,
    settings: SettingsWriter,
    pane: SettingsPane,
    /// A URL the browser refused to open, or another page's report.
    message: Option<PageMessage>,
    updates: UpdateState,
    tick_generation: u64,
    /// The beat in flight and the interval it was armed at, so a message
    /// that does not change the interval leaves it alone.
    tick: Option<(ComponentTask, Duration)>,
}

impl SettingsWindow {
    pub fn document(&self) -> &SettingsDocument {
        self.settings.document()
    }

    pub fn strings(&self) -> StringResolver {
        self.settings.strings()
    }

    pub fn updates(&self) -> &UpdateState {
        &self.updates
    }

    fn select_pane(&mut self, pane: SettingsPane) {
        self.pane = pane;
        self.settings
            .update(|document| document.set_choice(&keys::SELECTED_SETTINGS_PANE, pane));
    }

    /// The interval the beat should be running at: a check or a download
    /// in flight is worth looking for more often than once a second.
    fn tick_interval(&self) -> Duration {
        if self.updates.is_busy() {
            BUSY_REFRESH_INTERVAL
        } else {
            IDLE_REFRESH_INTERVAL
        }
    }

    /// Starts the next beat, retiring the one in flight. The sleep cannot
    /// be interrupted, so a retired task is discarded by its generation
    /// rather than by waking it.
    fn arm_tick(&mut self, context: &ComponentContext<Self>) {
        if let Some((task, _)) = self.tick.take() {
            task.cancel();
        }
        self.tick_generation += 1;
        let generation = self.tick_generation;
        let interval = self.tick_interval();
        let task = context.spawn_background(move |cancellation| {
            std::thread::sleep(interval);
            Message::Tick(if cancellation.is_cancelled() {
                CANCELLED_TICK
            } else {
                generation
            })
        });
        self.tick = Some((task, interval));
    }

    /// Re-arms only when the wanted interval changed. Every message would
    /// otherwise retire a sleeping task it has no quarrel with — and since
    /// the sleep runs out anyway, a click would leave a dead thread behind.
    fn retune_tick(&mut self, context: &ComponentContext<Self>) {
        if self.tick.as_ref().map(|(_, armed)| *armed) != Some(self.tick_interval()) {
            self.arm_tick(context);
        }
    }

    /// The banners over the form: the write failure stays until a write
    /// succeeds, so it is not closable.
    fn banners(&self, strings: &StringResolver) -> View {
        match self.settings.write_failure() {
            Some(detail) => InfoBar::new()
                .severity(InfoBarSeverity::Error)
                .is_open(true)
                .is_closable(false)
                .title(strings.resolve(StringKey::DesktopSettingsWriteFailed))
                .message(detail)
                .into(),
            None => View::empty(),
        }
    }

    /// ONE alert at a time, whichever was raised first in this order; the
    /// next shows once it is dismissed (two `.alert`s on one chain do not
    /// stack on the Mac either).
    fn dialogs(&self, strings: &StringResolver, context: &mut ViewContext<Self>) -> View {
        if let Some(message) = &self.message {
            return ContentDialog::new()
                .title(message.title(strings))
                .close_button_text(strings.resolve(StringKey::CommonOk))
                .is_open(true)
                .on_closed(context.callback(|_| Message::DismissAlert))
                .content(message.detail(strings).unwrap_or_default());
        }
        let Some(outcome) = self.updates.manual_outcome() else {
            return View::empty();
        };
        let (title, detail) = outcome.alert_text(strings);
        let dialog = ContentDialog::new()
            .title(title)
            .is_open(true)
            .on_closed(context.callback(Message::UpdateAlertClosed));
        let dialog = match outcome.proceed_key() {
            Some(key) => dialog
                .primary_button_text(strings.resolve(key))
                .close_button_text(strings.resolve(StringKey::DesktopUpdateLaterAction)),
            None => dialog.close_button_text(strings.resolve(StringKey::CommonOk)),
        };
        dialog.content(detail.unwrap_or_default())
    }
}

/// Never a live generation, so a tick whose task was retired is a no-op
/// even if it is still delivered. Live generations start at 1.
const CANCELLED_TICK: u64 = 0;

/// The 外觀 setting drives the whole window, not only the candidate window
/// (`AppearanceSettingsView`): WinUI resolves `System` against the machine.
fn window_theme(mode: AppearanceMode) -> WindowTheme {
    match mode {
        AppearanceMode::Light => WindowTheme::Light,
        AppearanceMode::Dark => WindowTheme::Dark,
        AppearanceMode::Auto => WindowTheme::System,
    }
}

impl Component for SettingsWindow {
    type Input = SettingsWindowInput;
    type Message = Message;

    fn create(input: &Self::Input, context: &ComponentContext<Self>) -> Self {
        let launch = Rc::clone(&input.0);
        // A pane this build has no page for opens on 一般 IN MEMORY and is
        // NOT written back: the egui window still has that pane, and a
        // preview launch must not move its stored selection.
        let listed = page_view(launch.pane).is_some();
        if !listed {
            log::info!("winui.pane_unsupported pane={}", launch.pane.raw());
        }
        let mut window = Self {
            settings: SettingsWriter::new(Rc::clone(&launch.live), launch.is_read_only),
            pane: if listed {
                launch.pane
            } else {
                SettingsPane::General
            },
            launch,
            message: None,
            updates: UpdateState::new(),
            tick_generation: 0,
            tick: None,
        };
        // The overdue daily check, or the menu's 檢查更新 (`--check-now`)
        // — the manual one always answers.
        if window.launch.is_check_now {
            window.updates.check_manually(&mut window.settings);
        } else if !window.launch.is_read_only {
            window.updates.check_if_due(&mut window.settings);
        }
        // `--pane` is a selection like a click: persisted, so the next
        // plain launch reopens there too.
        if listed && window.document().choice(&keys::SELECTED_SETTINGS_PANE) != window.pane {
            window.select_pane(window.pane);
        }
        window.arm_tick(context);
        window
    }

    fn update(&mut self, message: Self::Message, context: &ComponentContext<Self>) {
        match message {
            Message::Tick(generation) => {
                if generation != self.tick_generation {
                    // A retired beat: a live one is already sleeping, so
                    // this one neither reads nor re-arms.
                    return;
                }
                self.settings.refresh();
                self.updates.poll(&mut self.settings);
                self.arm_tick(context);
                return;
            }
            Message::SelectPane(tag) => {
                let Some(pane) = tag.as_deref().and_then(SettingsPane::from_raw) else {
                    return;
                };
                if pane != self.pane && page_view(pane).is_some() {
                    self.select_pane(pane);
                }
            }
            Message::SetChoice(Some(write)) => {
                self.settings.update(|document| write.apply(document))
            }
            Message::SetChoice(None) => {}
            Message::SetAutoSpace(is_on) => self
                .settings
                .update(move |document| document.set_bool(&keys::IS_AUTO_SPACE_ENABLED, is_on)),
            Message::ResetAppearance => self.settings.update(SettingsDocument::reset_appearance),
            Message::CheckForUpdates => self.updates.check_manually(&mut self.settings),
            Message::ActOnOffer => {
                let Some(manifest) = checker::pending_update(self.document(), INSTALLED_VERSION)
                else {
                    return;
                };
                self.message = self.updates.act_on_offer(&manifest);
            }
            Message::UpdateAlertClosed(result) => {
                self.message = if result == ContentDialogResult::Primary {
                    self.updates.proceed_with_manual_outcome()
                } else {
                    self.updates.dismiss_manual_outcome();
                    None
                };
            }
            Message::DismissAlert => self.message = None,
            Message::OpenUrl(url) => self.message = presentation::open_url(url),
        }
        self.retune_tick(context);
    }

    fn view(&self, _input: &Self::Input, context: &mut ViewContext<Self>) -> View {
        let strings = self.strings();
        let title = pane_title(&strings, self.pane);
        context.window_title(title.clone());
        context.window_visuals(
            WindowVisuals::new()
                .backdrop(WindowBackdrop::Mica)
                .theme(window_theme(self.document().choice(&keys::APPEARANCE_MODE)))
                .client_size(INITIAL_WIDTH, INITIAL_HEIGHT)
                .constraints(WindowConstraints {
                    min_width: Some(MINIMUM_WIDTH),
                    min_height: Some(MINIMUM_HEIGHT),
                    max_width: None,
                    max_height: None,
                }),
        );

        let items = PANES.map(|(pane, _)| {
            let label = pane
                .title_key()
                .map_or("", |key| strings.resolve(key))
                .to_owned();
            KeyedView::new(
                pane.raw(),
                NavigationViewItem::new()
                    .tag(pane.raw())
                    .is_selected(pane == self.pane)
                    .slots([
                        SlotView::new(
                            NavigationViewItemSlot::Icon,
                            FontIcon::new().glyph(pane.icon_glyph()),
                        ),
                        SlotView::new(NavigationViewItemSlot::Content, label),
                    ]),
            )
        });

        let page = page_view(self.pane).unwrap_or(pages::general::view);
        let content = ScrollViewer::new()
            .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
            .content(
                StackPanel::new()
                    .spacing(cards::CARD_SPACING)
                    .margin(Thickness::uniform(FORM_INSET))
                    .children((
                        self.banners(&strings),
                        page(self, &strings, context),
                        self.dialogs(&strings, context),
                    )),
            );

        NavigationView::new()
            .pane_display_mode(NavigationViewPaneDisplayMode::Left)
            .open_pane_length(SIDEBAR_WIDTH)
            .is_settings_visible(false)
            .is_pane_toggle_button_visible(false)
            .is_back_button_visible(NavigationViewBackButtonVisible::Collapsed)
            .always_show_header(true)
            .on_selected_tag_changed(context.callback(Message::SelectPane))
            .slots([
                SlotView::collection(NavigationViewSlot::MenuItems, items),
                SlotView::new(
                    NavigationViewSlot::Header,
                    TextBlock::new()
                        .text(title)
                        .font_size(TITLE_FONT_SIZE)
                        .font_weight(FontWeight::SEMI_BOLD),
                ),
                SlotView::new(NavigationViewSlot::Content, content),
            ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taigi_windows_core::settings::CandidateLayout;

    #[test]
    fn a_resolved_write_stores_exactly_what_the_typed_setter_would() {
        // trace: `SettingsWrite` erases the typed key and value to the
        // pair `set_raw_string` takes, which is what `set_choice` and
        // `set_string` do inside `SettingsDocument`.
        let mut typed = SettingsDocument::default();
        typed.set_choice(&keys::CANDIDATE_LAYOUT, CandidateLayout::Vertical);
        typed.set_string(&keys::DISPLAY_LANGUAGE, DisplayLanguage::English.tag());

        let mut resolved = SettingsDocument::default();
        SettingsWrite::choice(&keys::CANDIDATE_LAYOUT, CandidateLayout::Vertical)
            .apply(&mut resolved);
        SettingsWrite::display_language(DisplayLanguage::English).apply(&mut resolved);

        assert_eq!(resolved, typed);
    }

    #[test]
    fn only_the_panes_with_pages_are_listed_and_the_rest_have_no_view() {
        // trace: the roster and the dispatch are one table, so a pane
        // cannot be listed without a page or drawn without a row.
        assert!(page_view(SettingsPane::General).is_some());
        assert!(page_view(SettingsPane::Appearance).is_some());
        for pane in [
            SettingsPane::Shortcuts,
            SettingsPane::CustomDictionary,
            SettingsPane::DictionarySources,
            SettingsPane::DictionarySearch,
        ] {
            assert!(
                page_view(pane).is_none(),
                "{pane:?} has no page until W17-B/C"
            );
        }
    }
}
