//! The settings window as one Reactor component: a `NavigationView` of the
//! panes, the selected pane's page in its content, and the window's own
//! chrome — Mica, the Appearance theme, the title, the write-failure banner and
//! the one alert at a time. Port of `app.rs`'s job, message-driven:
//! `view` is immutable, so every control sends a message and the state
//! moves in `update`.
//!
//! Named divergences from the Mac (and from the egui window), all
//! deliberate: the window frame is not persisted; the Appearance mode is a native
//! pop-up rather than three drawn thumbnails; the window may be narrowed
//! until `NavigationView` compacts its pane.

use crate::presentation;
use crate::updates::{UpdateState, INSTALLED_VERSION};
use crate::winui::cards;
use crate::winui::pages;
use crate::winui::pages::custom_dictionary::CustomDictionaryModel;
use crate::winui::pages::dictionary_search::DictionarySearchModel;
use crate::winui::pages::font_management::FontManagementModel;
use crate::winui::pages::learning_records::LearningRecordsModel;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;
use taigi_desktop_core::keys::{
    evaluate_press, ChordRejection, RecordedPress, RecorderOutcome, RecorderTarget,
};
use taigi_desktop_core::settings::presentation::{pane_title, PageMessage};
use taigi_desktop_core::settings::{
    keys, AppearanceMode, InputMode, InputModeRequest, SettingChoice, SettingsDocument,
    SettingsKey, SettingsPane,
};
use taigi_desktop_core::strings::{DisplayLanguage, StringKey, StringResolver};
use taigi_desktop_storage::{LiveSettings, SettingsWriter, IDLE_REFRESH_INTERVAL};
use taigi_desktop_update::checker;
use taigi_windows_platform::keyboard_hook::{Delivery, KeyboardHook};
use taigi_windows_platform::DESKTOP_PLATFORM;
use windows_reactor::*;

/// A pane's form, over the window's state. Stateless presentation — the
/// guide's rule for a subtree that owns nothing.
type PageView = fn(&SettingsWindow, &StringResolver, &mut ViewContext<SettingsWindow>) -> View;

/// The panes the sidebar lists, in order, each with the page that draws
/// it — one table, so a pane cannot be listed without a page or reachable
/// without a row.
const PANES: [(SettingsPane, PageView); 7] = [
    (SettingsPane::General, pages::general::view),
    (SettingsPane::Appearance, pages::appearance::view),
    (SettingsPane::Shortcuts, pages::shortcuts::view),
    (
        SettingsPane::DictionarySources,
        pages::dictionary_sources::view,
    ),
    (
        SettingsPane::CustomDictionary,
        pages::custom_dictionary::view,
    ),
    (SettingsPane::LearningRecords, pages::learning_records::view),
    (SettingsPane::FontManagement, pages::font_management::view),
];

/// The page for a pane the sidebar does not list: built, reachable only by
/// `--pane dictionarySearch` (USER 2026-08-21) and the tray menu's `--pane
/// about` (USER 2026-09-20), exactly as on macOS.
fn page_view(pane: SettingsPane) -> Option<PageView> {
    match pane {
        SettingsPane::DictionarySearch => return Some(pages::dictionary_search::view),
        SettingsPane::About => return Some(pages::about::view),
        _ => {}
    }
    PANES
        .iter()
        .find(|(listed, _)| *listed == pane)
        .map(|(_, view)| *view)
}

/// `SettingsPaneLayout` in `SettingsSplitView.swift`: sidebar 215 + detail 545.
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
/// While a check or a download is in flight the answer is wanted sooner
/// than the idle beat.
const BUSY_REFRESH_INTERVAL: Duration = Duration::from_millis(100);
/// What the banner says when there is no `%APPDATA%`: the missing
/// variable's name is the whole diagnosis.
const READ_ONLY_DETAIL: &str = "APPDATA";

/// What the window is launched with. `Rc` because a component's input must
/// be `Clone + PartialEq` and none of this is either; the root input never
/// changes, so identity is the right comparison.
pub struct Launch {
    live: Rc<LiveSettings>,
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

impl SettingsWindowInput {
    /// The one place a launch is assembled, so `run` and the pane-planning
    /// tests reach the component through the same path. The user-data
    /// stores are not opened here: opening them is a LAUNCH-time side
    /// effect (`user_data::open_at_launch`, migrations on the engine's own
    /// thread), and a test that only plans a view tree must not start one.
    pub(crate) fn new(
        live: LiveSettings,
        is_read_only: bool,
        pane: SettingsPane,
        is_check_now: bool,
    ) -> Self {
        Self(Rc::new(Launch {
            live: Rc::new(live),
            is_read_only,
            pane,
            is_check_now,
        }))
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
    crate::user_data::open_at_launch(directory, is_read_only);
    let input = SettingsWindowInput::new(live, is_read_only, pane, is_check_now);
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

/// Which pane's Reset to Defaults card was pressed. Each puts back exactly the keys
/// that pane owns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResetScope {
    General,
    Appearance,
    /// Both shortcut registries at once, and no conflict pass afterwards:
    /// the shipped defaults hold no chord in common
    /// (`ShortcutSettingsView.restoreDefaults`).
    Shortcuts,
    DictionarySources,
}

impl Message {
    /// What a pop-up over a `SettingChoice` means; a pop-up that cleared
    /// its selection writes nothing.
    pub fn set_choice<T: SettingChoice>(choice: Option<T>, key: &SettingsKey<T>) -> Self {
        Self::SetChoice(choice.map(|value| SettingsWrite::choice(key, value)))
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
    /// The Input Script pop-up, through the one mode writer
    /// (`SettingsDocument::switch_input_mode`); `None` writes nothing.
    SetInputMode(Option<InputMode>),
    SetSwitch(SettingsKey<bool>, bool),
    Reset(ResetScope),
    CustomDictionary(pages::custom_dictionary::Message),
    LearningRecords(pages::learning_records::Message),
    FontManagement(pages::font_management::Message),
    DictionarySearch(pages::dictionary_search::Message),
    StartRecording(RecorderTarget),
    /// A key the hook took while a row was recording. The generation is
    /// the recording it was taken for: a press that arrives after that row
    /// stopped is not this row's.
    RecordedPress(u64, RecordedPress),
    ClearShortcut(RecorderTarget),
    CheckForUpdates,
    ActOnOffer,
    UpdateAlertClosed(ContentDialogResult),
    DismissAlert,
    OpenUrl(String),
}

pub struct SettingsWindow {
    launch: Rc<Launch>,
    settings: SettingsWriter,
    pane: SettingsPane,
    /// A URL the browser refused to open, or another page's report.
    message: Option<PageMessage>,
    updates: UpdateState,
    custom_dictionary: CustomDictionaryModel,
    learning_records: LearningRecordsModel,
    font_management: FontManagementModel,
    dictionary_search: DictionarySearchModel,
    recorder: Recorder,
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
        presentation::strings_for(self.settings.document())
    }

    pub fn updates(&self) -> &UpdateState {
        &self.updates
    }

    pub fn is_read_only(&self) -> bool {
        self.settings.is_read_only()
    }

    pub fn custom_dictionary(&self) -> &CustomDictionaryModel {
        &self.custom_dictionary
    }

    pub fn learning_records(&self) -> &LearningRecordsModel {
        &self.learning_records
    }

    pub fn font_management(&self) -> &FontManagementModel {
        &self.font_management
    }

    pub fn dictionary_search(&self) -> &DictionarySearchModel {
        &self.dictionary_search
    }

    pub fn is_recording(&self, target: RecorderTarget) -> bool {
        self.recorder.target == Some(target)
    }

    pub fn recorder_rejection(&self) -> Option<ChordRejection> {
        self.recorder.rejection
    }

    /// Starts recording on `target`, over a hook that reports every press
    /// to this component. A row already recording is replaced: its hook
    /// guard drops here, and the new one takes it over with the keys the
    /// old one had hidden still owed their key-ups.
    fn start_recording(&mut self, target: RecorderTarget, context: &ComponentContext<Self>) {
        self.recorder.generation = self.recorder.generation.wrapping_add(1);
        let generation = self.recorder.generation;
        let sender = context.sender();
        // Bounded: the send queues the press on this component and wakes
        // it. A refused send means the queue is full or the component is
        // going away — either way the key stays hidden from the form
        // rather than being typed into it.
        self.recorder.hook = KeyboardHook::install(move |press| {
            if sender.send(Message::RecordedPress(generation, press)) {
                Delivery::Accepted
            } else {
                Delivery::Full
            }
        });
        if self.recorder.hook.is_none() {
            log::error!("recorder.hook_unavailable");
            return;
        }
        self.recorder.target = Some(target);
        self.recorder.rejection = None;
    }

    /// Ends the recording, whatever ended it. Bumping the generation makes
    /// every press still in flight this row's no longer.
    fn stop_recording(&mut self) {
        self.recorder.generation = self.recorder.generation.wrapping_add(1);
        self.recorder.target = None;
        self.recorder.rejection = None;
        self.recorder.hook = None;
    }

    /// One press, judged by the shared decision (`keys::evaluate_press`) —
    /// the same one the Mac and the egui window ask.
    fn record_press(&mut self, press: &RecordedPress) {
        let Some(target) = self.recorder.target else {
            return;
        };
        match evaluate_press(target.tier(), press, DESKTOP_PLATFORM) {
            RecorderOutcome::Recorded(chord) => {
                self.stop_recording();
                self.settings
                    .update(|document| target.store(document, Some(&chord), DESKTOP_PLATFORM));
            }
            RecorderOutcome::Refused(reason) => {
                self.recorder.rejection = Some(reason);
                taigi_windows_platform::beep();
            }
            // Escape leaves the row as it was.
            RecorderOutcome::Blurred => self.stop_recording(),
            RecorderOutcome::Ignored => {}
        }
    }

    fn select_pane(&mut self, pane: SettingsPane, context: &ComponentContext<Self>) {
        self.pane = pane;
        self.settings
            .update(|document| document.set_choice(&keys::SELECTED_SETTINGS_PANE, pane));
        self.enter_pane(context);
    }

    /// What a pane needs the first time it is shown: Custom Dictionary and
    /// Learning Records fetch their first page, Manage Typefaces reads the
    /// user's font folder. Each once.
    ///
    /// Manage Typefaces reads even in a read-only launch: the folder is the user's
    /// own, and a window that cannot WRITE the selection can still say which
    /// typefaces are there.
    fn enter_pane(&mut self, context: &ComponentContext<Self>) {
        if self.pane == SettingsPane::CustomDictionary && !self.settings.is_read_only() {
            pages::custom_dictionary::ensure_loaded(&mut self.custom_dictionary, context);
        }
        if self.pane == SettingsPane::LearningRecords && !self.settings.is_read_only() {
            pages::learning_records::ensure_loaded(&mut self.learning_records, context);
        }
        if self.pane == SettingsPane::FontManagement {
            pages::font_management::on_enter(&mut self.font_management, self.settings.document());
        }
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

/// The row that is recording, if any, and the hook it listens through.
/// The hook is dropped the moment the row stops — and drains itself, so a
/// key still held when it stops does not reach the form as a lone key-up.
#[derive(Default)]
struct Recorder {
    target: Option<RecorderTarget>,
    /// Why the last press was turned down, shown in place of the prompt
    /// until the next press.
    rejection: Option<ChordRejection>,
    /// Which recording a press belongs to; a press from an earlier one is
    /// dropped rather than recorded against the row now showing.
    generation: u64,
    hook: Option<KeyboardHook>,
}

/// The Appearance setting drives the whole window, not only the candidate window
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
        // A pane this build has no page for opens on General IN MEMORY and is
        // NOT written back: the stored selection stays what the user (or
        // another build) chose.
        let listed = page_view(launch.pane).is_some();
        if !listed {
            log::info!("winui.pane_unsupported pane={}", launch.pane.raw());
        }
        // No per-user directory: the window still follows the file `live`
        // was opened over but refuses every write, saying so from the first
        // frame — never a write to a file the DLL would not read (roadmap
        // W2's unsupported-capability rule).
        let live = Rc::clone(&launch.live);
        let settings = if launch.is_read_only {
            SettingsWriter::read_only_over(live, READ_ONLY_DETAIL)
        } else {
            SettingsWriter::new(live)
        };
        let mut window = Self {
            settings,
            pane: if listed {
                launch.pane
            } else {
                SettingsPane::General
            },
            launch,
            message: None,
            updates: UpdateState::new(),
            custom_dictionary: CustomDictionaryModel::default(),
            learning_records: LearningRecordsModel::default(),
            font_management: FontManagementModel::default(),
            dictionary_search: DictionarySearchModel::default(),
            recorder: Recorder::default(),
            tick_generation: 0,
            tick: None,
        };
        // The overdue daily check, or the menu's Check for Updates (`--check-now`)
        // — the manual one always answers.
        if window.launch.is_check_now {
            window.updates.check_manually(&mut window.settings);
        } else if !window.launch.is_read_only {
            window.updates.check_if_due(&mut window.settings);
        }
        // `--pane` is a selection like a click: persisted, so the next
        // plain launch reopens there too.
        if listed && window.document().choice(&keys::SELECTED_SETTINGS_PANE) != window.pane {
            window.select_pane(window.pane, context);
        } else {
            window.enter_pane(context);
        }
        window.arm_tick(context);
        window
    }

    fn update(&mut self, message: Self::Message, context: &ComponentContext<Self>) {
        // A row records until something else happens. Reactor exposes no
        // focus event, so every message that is not the recording itself
        // IS the "clicked elsewhere" the egui field watched for.
        if !matches!(
            message,
            Message::Tick(_) | Message::RecordedPress(..) | Message::StartRecording(_)
        ) {
            self.stop_recording();
        }
        match message {
            Message::Tick(generation) => {
                if generation != self.tick_generation {
                    // A retired beat: a live one is already sleeping, so
                    // this one neither reads nor re-arms.
                    return;
                }
                self.settings.refresh();
                self.updates.poll(&mut self.settings);
                // Reactor exposes no activation event, so the beat is
                // where a row the user walked away from is released
                // (`WindowFocused(false)` on the egui side). Asked only
                // while a row records, and only once a second.
                if self.recorder.target.is_some() && !taigi_windows_platform::is_foreground_thread()
                {
                    self.stop_recording();
                }
                self.arm_tick(context);
                return;
            }
            Message::SelectPane(tag) => {
                let Some(pane) = tag.as_deref().and_then(SettingsPane::from_raw) else {
                    return;
                };
                if pane != self.pane && page_view(pane).is_some() {
                    self.select_pane(pane, context);
                }
            }
            Message::SetChoice(Some(write)) => {
                self.settings.update(|document| write.apply(document))
            }
            Message::SetChoice(None) => {}
            Message::SetInputMode(Some(mode)) => self.settings.update(|document| {
                document.switch_input_mode(InputModeRequest::Pick(mode));
            }),
            Message::SetInputMode(None) => {}
            Message::SetSwitch(key, is_on) => self
                .settings
                .update(move |document| document.set_bool(&key, is_on)),
            Message::Reset(scope) => self.settings.update(|document| match scope {
                ResetScope::General => document.reset_general(),
                ResetScope::Appearance => document.reset_appearance(),
                ResetScope::Shortcuts => {
                    document.reset_composing_shortcuts();
                    document.reset_global_shortcuts();
                }
                ResetScope::DictionarySources => document.reset_dictionary_sources(),
            }),
            Message::FontManagement(message) => {
                let Self {
                    settings,
                    font_management,
                    message: alert,
                    ..
                } = self;
                pages::font_management::update(
                    font_management,
                    message,
                    pages::font_management::PageEnvironment {
                        settings,
                        message: alert,
                    },
                );
                pages::font_management::sync_faces(font_management, settings.document());
            }
            Message::CustomDictionary(message) => {
                pages::custom_dictionary::update(
                    &mut self.custom_dictionary,
                    message,
                    &mut self.message,
                    context,
                );
            }
            Message::LearningRecords(message) => {
                pages::learning_records::update(
                    &mut self.learning_records,
                    message,
                    &mut self.message,
                    context,
                );
            }
            Message::DictionarySearch(message) => {
                pages::dictionary_search::update(
                    &mut self.dictionary_search,
                    message,
                    self.settings.document(),
                    &mut self.message,
                    context,
                );
            }
            Message::StartRecording(target) => self.start_recording(target, context),
            Message::RecordedPress(generation, press) => {
                if generation == self.recorder.generation {
                    self.record_press(&press);
                }
            }
            Message::ClearShortcut(target) => self
                .settings
                .update(|document| target.store(document, None, DESKTOP_PLATFORM)),
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
            Message::OpenUrl(url) => {
                if !url.is_empty() {
                    self.message = presentation::open_url(&url);
                }
            }
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
    use taigi_desktop_core::settings::CandidateLayout;

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
    fn every_pane_has_a_page_and_only_the_search_one_is_unlisted() {
        // trace: the roster and the dispatch are one table, so a pane
        // cannot be listed without a page or drawn without a row — and
        // Dictionary Search is built but unlisted, reachable only by `--pane`
        // (macOS does the same, USER 2026-08-21).
        for pane in SettingsPane::SIDEBAR {
            assert!(
                PANES.iter().any(|(listed, _)| *listed == pane),
                "{pane:?} is in the sidebar but has no row"
            );
            assert!(page_view(pane).is_some(), "{pane:?} has no page");
        }
        // The sidebar draws `PANES`, so its order has to be `SIDEBAR`'s —
        // a reorder in one place alone changes nothing on screen.
        assert_eq!(PANES.map(|(pane, _)| pane), SettingsPane::SIDEBAR);
        assert!(page_view(SettingsPane::DictionarySearch).is_some());
        assert!(
            !PANES
                .iter()
                .any(|(listed, _)| *listed == SettingsPane::DictionarySearch),
            "辭典搜尋 stays out of the sidebar"
        );
    }
}
