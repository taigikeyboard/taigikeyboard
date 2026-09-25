//! The settings window: an `adw::NavigationSplitView` with the pane list
//! in the sidebar and the selected pane's page in the content, the
//! write-failure banner over it, a 1 s live-reload tick (roadmap L3 / L9).
//! Port of the Windows `winui/window.rs`'s job with GTK's own state model:
//! widgets hold their state, the writer holds the document, and a page is
//! rebuilt only when the display language changes.
//!
//! Named divergences from the Mac, all deliberate: the window frame is
//! not persisted; the Appearance mode is a combo row.

use crate::pages::{self, Page};
use crate::presentation::{pane_title, PageMessage};
use crate::recorder::{Recorded, Recorder, RecorderTarget};
use crate::writer::{SettingsWriter, REFRESH_INTERVAL};
use crate::SIDEBAR;
use adw::prelude::*;
use gtk::glib::translate::IntoGlib;
use gtk::{gio, glib};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use taigi_desktop_core::keys::{ChordRejection, RecordedPress};
use taigi_desktop_core::settings::{keys, SettingChoice, SettingsDocument, SettingsPane};
use taigi_desktop_core::strings::{DisplayLanguage, StringKey, StringResolver};
use taigi_desktop_storage::UserDataStores;
use taigi_linux_platform::{snapshot, RawKeyEvent};

/// `SettingsPaneLayout` in `SettingsSplitView.swift`: sidebar 215 + detail 545.
const SIDEBAR_WIDTH: f64 = 215.0;
const INITIAL_WIDTH: i32 = 760;
const INITIAL_HEIGHT: i32 = 560;

pub struct SettingsWindow {
    window: adw::ApplicationWindow,
    writer: RefCell<SettingsWriter>,
    /// Held open for the window's life: the Custom Dictionary and Dictionary Search pages
    /// read it. `None` when the data directory could not be had — the
    /// banner says so (`data_failure`).
    stores: Option<UserDataStores>,
    data_failure: Option<String>,
    /// The user-data pages' one work slot, owned HERE so a page rebuild (a
    /// display language change) cannot free a slot a job still holds.
    job_slot: JobSlot,
    toasts: adw::ToastOverlay,
    sidebar: gtk::ListBox,
    /// The content page: its title is what the header bar draws
    /// (libadwaita 1.5 `adw-header-bar.c` reads the enclosing
    /// `NavigationPage`, never the window title).
    content_page: adw::NavigationPage,
    stack: gtk::Stack,
    banner: adw::Banner,
    pages: RefCell<Vec<Page>>,
    /// The language the pages and the sidebar were built for; a document
    /// that names another rebuilds them.
    built_language: Cell<DisplayLanguage>,
    /// `true` while the code selects a sidebar row, so the handler does not
    /// write the selection back.
    is_selecting: Cell<bool>,
    current: Cell<SettingsPane>,
    recorder: RefCell<Recorder>,
    /// The field button whose row records: focus moving anywhere else ends
    /// the recording (the Mac's field losing first responder).
    recording_field: RefCell<Option<gtk::Widget>>,
}

impl SettingsWindow {
    pub fn build(
        application: &adw::Application,
        writer: SettingsWriter,
        stores: Result<UserDataStores, String>,
    ) -> Rc<Self> {
        let (stores, data_failure) = match stores {
            Ok(stores) => (Some(stores), None),
            Err(detail) => (None, Some(detail)),
        };
        let sidebar = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .css_classes(["navigation-sidebar"])
            .build();
        let stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .hexpand(true)
            .vexpand(true)
            .build();
        let banner = adw::Banner::builder().revealed(false).build();
        let content_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content_box.append(&banner);
        content_box.append(&stack);
        // Transient notices (a job's outcome) as toasts over the content.
        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&content_box));

        let sidebar_view = adw::ToolbarView::new();
        sidebar_view.add_top_bar(&adw::HeaderBar::new());
        sidebar_view.set_content(Some(
            &gtk::ScrolledWindow::builder()
                .child(&sidebar)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .build(),
        ));
        let content_view = adw::ToolbarView::new();
        content_view.add_top_bar(&adw::HeaderBar::new());
        content_view.set_content(Some(&toasts));

        let sidebar_title = crate::presentation::strings_for(writer.document())
            .resolve(StringKey::HomeAppHeaderTitle)
            .to_owned();
        let content_page = adw::NavigationPage::new(&content_view, "");
        let split = adw::NavigationSplitView::builder()
            .sidebar(&adw::NavigationPage::new(&sidebar_view, &sidebar_title))
            .content(&content_page)
            .min_sidebar_width(SIDEBAR_WIDTH)
            .max_sidebar_width(SIDEBAR_WIDTH)
            .build();
        let window = adw::ApplicationWindow::builder()
            .application(application)
            .default_width(INITIAL_WIDTH)
            .default_height(INITIAL_HEIGHT)
            .content(&split)
            .build();

        let language = crate::presentation::display_language_of(writer.document());
        let shell = Rc::new(Self {
            window,
            writer: RefCell::new(writer),
            stores,
            data_failure,
            job_slot: JobSlot::default(),
            toasts,
            sidebar,
            content_page,
            stack,
            banner,
            pages: RefCell::new(Vec::new()),
            built_language: Cell::new(language),
            is_selecting: Cell::new(false),
            current: Cell::new(SettingsPane::General),
            recorder: RefCell::new(Recorder::default()),
            recording_field: RefCell::new(None),
        });
        shell.build_pages();
        shell.apply_chrome();
        shell.install_recorder_controller();

        let weak = Rc::downgrade(&shell);
        shell.sidebar.connect_row_selected(move |_, row| {
            let Some(shell) = weak.upgrade() else { return };
            if shell.is_selecting.get() {
                return;
            }
            let Some(row) = row else { return };
            let Some(pane) = SIDEBAR.get(row.index() as usize).copied() else {
                return;
            };
            shell.show_pane(pane);
            shell.update(|document| document.set_choice(&keys::SELECTED_SETTINGS_PANE, pane));
        });
        // The live-reload beat, for as long as the window lives.
        let weak = Rc::downgrade(&shell);
        glib::timeout_add_local(REFRESH_INTERVAL, move || match weak.upgrade() {
            Some(shell) => {
                shell.tick();
                glib::ControlFlow::Continue
            }
            None => glib::ControlFlow::Break,
        });
        shell
    }

    pub fn writer(&self) -> &RefCell<SettingsWriter> {
        &self.writer
    }

    pub fn stores(&self) -> Option<&UserDataStores> {
        self.stores.as_ref()
    }

    pub fn job_slot(&self) -> &JobSlot {
        &self.job_slot
    }

    /// A transient notice: what a page has to tell the user after a job
    /// (`UserDataPageChrome.swift:19-52`), as a toast.
    pub fn toast(&self, title: &str, detail: Option<&str>) {
        let text = match detail {
            Some(detail) => format!("{title} — {detail}"),
            None => title.to_owned(),
        };
        self.toasts.add_toast(adw::Toast::new(&text));
    }

    /// The row recording right now and why its last press was refused.
    pub fn recording(&self) -> (Option<RecorderTarget>, Option<ChordRejection>) {
        let recorder = self.recorder.borrow();
        (recorder.target, recorder.rejection)
    }

    /// A field was clicked: it records (a row already recording is replaced;
    /// the same row clicked again stops). `field` is the button, so focus
    /// leaving it ends the recording.
    pub fn start_recording(self: &Rc<Self>, target: RecorderTarget, field: Option<&gtk::Widget>) {
        {
            let mut recorder = self.recorder.borrow_mut();
            if recorder.is_recording(target) {
                recorder.stop();
                *self.recording_field.borrow_mut() = None;
            } else {
                recorder.start(target);
                *self.recording_field.borrow_mut() = field.cloned();
            }
        }
        self.refresh_pages();
    }

    /// Ends a recording, whatever ended it (a pane switch, another row's
    /// write, focus leaving the field). Answers whether one was running.
    fn stop_recording(&self) -> bool {
        *self.recording_field.borrow_mut() = None;
        self.recorder.borrow_mut().stop()
    }

    /// A row's ×: the chord goes, the row shows None.
    pub fn clear_shortcut(self: &Rc<Self>, target: RecorderTarget) {
        self.update(|document| target.store(document, None));
    }

    /// Every key reaches the recorder first, in the capture phase, while a
    /// row is recording; it is swallowed there. Losing the window's focus
    /// ends a recording the way a click outside the field does on the Mac.
    fn install_recorder_controller(self: &Rc<Self>) {
        let controller = gtk::EventControllerKey::new();
        controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(self);
        controller.connect_key_pressed(move |_, key, keycode, state| {
            let Some(shell) = weak.upgrade() else {
                return glib::Propagation::Proceed;
            };
            if !shell.recorder.borrow().swallows(keycode) {
                return glib::Propagation::Proceed;
            }
            shell.record_press(key.into_glib(), keycode, state.bits());
            glib::Propagation::Stop
        });
        let weak = Rc::downgrade(self);
        controller.connect_key_released(move |_, _, keycode, _| {
            if let Some(shell) = weak.upgrade() {
                shell.recorder.borrow_mut().release(keycode);
            }
        });
        self.window.add_controller(controller);
        // Focus leaving the field — a click on any other control, the
        // window losing focus — ends the recording.
        let weak = Rc::downgrade(self);
        self.window.connect_focus_widget_notify(move |window| {
            let Some(shell) = weak.upgrade() else { return };
            let field = shell.recording_field.borrow().clone();
            let Some(field) = field else { return };
            if GtkWindowExt::focus(window).as_ref() != Some(&field) {
                shell.stop_recording();
                shell.refresh_pages();
            }
        });
        let weak = Rc::downgrade(self);
        self.window.connect_is_active_notify(move |window| {
            let Some(shell) = weak.upgrade() else { return };
            if !window.is_active() && shell.stop_recording() {
                shell.refresh_pages();
            }
        });
    }

    /// One press as the recorder sees it: the same translation the engine
    /// applies to an X key event (GDK keyvals are X keysyms, its modifier
    /// bits the X ones), so a chord recorded here is the chord the key path
    /// matches. Public for the pane test, which cannot synthesise a GDK key
    /// event.
    pub fn record_press(self: &Rc<Self>, keyval: u32, keycode: u32, state: u32) {
        let Some(snapshot) = snapshot(RawKeyEvent {
            keyval,
            keycode,
            state,
        }) else {
            // A bare modifier: nothing to judge, the field keeps waiting.
            return;
        };
        let press = RecordedPress {
            key: snapshot.characters_ignoring_modifiers,
            modifiers: snapshot.modifiers,
            key_code: snapshot.key_code,
            is_repeat: false,
        };
        let recorded = self.recorder.borrow_mut().press(keycode, press);
        match recorded {
            Recorded::Store(target, chord) => {
                *self.recording_field.borrow_mut() = None;
                self.update(|document| target.store(document, Some(&chord)));
            }
            Recorded::Nothing => self.refresh_pages(),
        }
    }

    pub fn window(&self) -> &adw::ApplicationWindow {
        &self.window
    }

    /// Opens (or re-activates) the window on `pane`. A `--pane` is a
    /// selection like a sidebar click and is remembered the same way
    /// (Windows `select_pane`); an unlisted page (About) is shown, not stored.
    pub fn show(self: &Rc<Self>, pane: SettingsPane) {
        if self.stop_recording() {
            self.refresh_pages();
        }
        let pane = self.show_pane(pane);
        if SIDEBAR.contains(&pane)
            && self
                .writer
                .borrow()
                .document()
                .choice(&keys::SELECTED_SETTINGS_PANE)
                != pane
        {
            self.update(|document| document.set_choice(&keys::SELECTED_SETTINGS_PANE, pane));
        }
        self.window.present();
    }

    /// The pane on screen.
    pub fn current_pane(&self) -> SettingsPane {
        self.current.get()
    }

    /// One settings write from a row, then every row follows the file —
    /// the same path an outside change takes (`tick`), so a display
    /// language picked here rebuilds the pages too.
    pub fn update(self: &Rc<Self>, mutate: impl FnOnce(&mut SettingsDocument)) {
        // Any write ends a recording: the row it would have written is not
        // the one the user just touched (the recorder's own store has
        // already stopped it).
        self.stop_recording();
        self.writer.borrow_mut().update(mutate);
        self.follow_document();
    }

    /// Opens `url` in the browser; a launcher that refused is logged and
    /// said in the banner (`ExternalLinkButton.swift:740-744`).
    pub fn open_url(self: &Rc<Self>, url: &str) {
        let weak = Rc::downgrade(self);
        let url = url.to_owned();
        gtk::UriLauncher::new(&url).launch(
            Some(&self.window),
            gio::Cancellable::NONE,
            move |result| {
                if let Err(error) = result {
                    log::error!("url.open_failed url={url} error={error}");
                    if let Some(shell) = weak.upgrade() {
                        let strings = shell.writer.borrow().strings();
                        shell.banner.set_title(&format!(
                            "{} — {url}",
                            strings.resolve(StringKey::DesktopOpenURLFailed)
                        ));
                        shell.banner.set_revealed(true);
                    }
                }
            },
        );
    }

    /// Puts `pane` on screen; a pane this crate has no page for (Manage Typefaces,
    /// a stored value from another desktop) lands on General, as on Windows.
    /// Answers the pane shown.
    pub fn show_pane(&self, pane: SettingsPane) -> SettingsPane {
        let pane = if pages::BUILT.contains(&pane) {
            pane
        } else {
            log::warn!("pane.not_built pane={} — showing general", pane.raw());
            SettingsPane::General
        };
        self.current.set(pane);
        self.stack.set_visible_child_name(pane.raw());
        self.is_selecting.set(true);
        match SIDEBAR.iter().position(|listed| *listed == pane) {
            Some(index) => self
                .sidebar
                .select_row(self.sidebar.row_at_index(index as i32).as_ref()),
            None => self.sidebar.select_row(None::<&gtk::ListBoxRow>),
        }
        self.is_selecting.set(false);
        let strings = self.writer.borrow().strings();
        let title = pane_title(&strings, pane);
        self.content_page.set_title(&title);
        self.window.set_title(Some(&title));
        pane
    }

    /// The pages and the sidebar rows, in the built language.
    fn build_pages(self: &Rc<Self>) {
        self.stop_recording();
        while let Some(child) = self.stack.first_child() {
            self.stack.remove(&child);
        }
        while let Some(row) = self.sidebar.row_at_index(0) {
            self.sidebar.remove(&row);
        }
        let (strings, document) = {
            let writer = self.writer.borrow();
            (writer.strings(), writer.document().clone())
        };
        for pane in SIDEBAR {
            let row = adw::ActionRow::builder()
                .title(pane_title(&strings, pane))
                .build();
            // `icon-name` on a row is deprecated since libadwaita 1.3; the
            // prefix image is the current shape.
            row.add_prefix(&gtk::Image::from_icon_name(sidebar_icon(pane)));
            self.sidebar.append(&row);
        }
        let mut pages = Vec::new();
        for pane in pages::BUILT {
            let page = pages::build(
                pane,
                self,
                &strings,
                &document,
                self.stores.as_ref(),
                &self.job_slot,
            );
            self.stack.add_named(&page.widget, Some(pane.raw()));
            pages.push(page);
        }
        *self.pages.borrow_mut() = pages;
        self.show_pane(self.current.get());
    }

    /// Every row follows the document (and the recorder) as it is now.
    fn refresh_pages(self: &Rc<Self>) {
        let document = self.writer.borrow().document().clone();
        for page in self.pages.borrow().iter() {
            page.refresh(&document);
        }
        self.apply_chrome();
    }

    /// The banner, from the document.
    fn apply_chrome(&self) {
        let writer = self.writer.borrow();
        let strings = writer.strings();
        // One banner: the settings file first (nothing writes), else the
        // user data (the pages over it show only their switch).
        let notice = match (writer.write_failure(), &self.data_failure) {
            (Some(detail), _) => Some(format!(
                "{} — {detail}",
                strings.resolve(StringKey::DesktopSettingsWriteFailed)
            )),
            (None, Some(detail)) => Some(format!(
                "{} — {detail}",
                strings.resolve(StringKey::DesktopCustomDictReadFailed)
            )),
            (None, None) => None,
        };
        match notice {
            Some(text) => {
                self.banner.set_title(&text);
                self.banner.set_revealed(true);
            }
            None => self.banner.set_revealed(false),
        }
    }

    /// The 1 s beat: the file re-read, and the window follows it.
    pub fn tick(self: &Rc<Self>) {
        if self.writer.borrow_mut().refresh() {
            self.follow_document();
        }
    }

    /// The window follows the document as it now stands: a display
    /// language that changed rebuilds every page, anything else refreshes
    /// the rows.
    fn follow_document(self: &Rc<Self>) {
        let language = crate::presentation::display_language_of(self.writer.borrow().document());
        if language != self.built_language.get() {
            self.built_language.set(language);
            self.build_pages();
        }
        self.refresh_pages();
    }

    /// The sidebar's row titles, top to bottom (for the tests).
    pub fn sidebar_titles(&self) -> Vec<String> {
        let mut titles = Vec::new();
        let mut index = 0;
        while let Some(row) = self.sidebar.row_at_index(index) {
            if let Ok(row) = row.downcast::<adw::ActionRow>() {
                titles.push(row.title().to_string());
            }
            index += 1;
        }
        titles
    }

    /// The page widget shown for `pane`, if the stack holds one.
    pub fn page_widget(&self, pane: SettingsPane) -> Option<gtk::Widget> {
        self.stack.child_by_name(pane.raw())
    }
}

/// The sidebar row's icon (`SettingsSplitView.swift` `symbolName`, in
/// Adwaita 46's symbolic set — every name checked against the theme
/// Ubuntu 24.04 ships): gearshape → preferences-system, paintpalette →
/// applications-graphics, keyboard → input-keyboard, books.vertical →
/// emblem-documents, character.book.closed → x-office-address-book.
fn sidebar_icon(pane: SettingsPane) -> &'static str {
    match pane {
        SettingsPane::General => "preferences-system-symbolic",
        SettingsPane::Appearance => "applications-graphics-symbolic",
        SettingsPane::Shortcuts => "input-keyboard-symbolic",
        SettingsPane::DictionarySources => "emblem-documents-symbolic",
        SettingsPane::CustomDictionary => "x-office-address-book-symbolic",
        SettingsPane::FontManagement => "preferences-desktop-font-symbolic",
        SettingsPane::DictionarySearch => "edit-find-symbolic",
        SettingsPane::About => "help-about-symbolic",
    }
}

/// The one work slot the user-data pages share (`Some` = the generation
/// of the job holding it). Refused, not queued: a queued delete would name
/// a row the list may no longer show. Lives in the window, outside any
/// page, so a rebuilt page finds the slot still taken by the job the old
/// page started.
#[derive(Clone, Default)]
pub struct JobSlot {
    holder: Rc<Cell<Option<u64>>>,
    next: Rc<Cell<u64>>,
}

impl JobSlot {
    /// Takes the slot; `None` when something holds it.
    pub fn take(&self) -> Option<u64> {
        if self.holder.get().is_some() {
            return None;
        }
        let generation = self.next.get().wrapping_add(1);
        self.next.set(generation);
        self.holder.set(Some(generation));
        Some(generation)
    }

    pub fn is_held_by(&self, generation: u64) -> bool {
        self.holder.get() == Some(generation)
    }

    pub fn is_taken(&self) -> bool {
        self.holder.get().is_some()
    }

    /// Frees the slot if `generation` holds it.
    pub fn release(&self, generation: u64) {
        if self.holder.get() == Some(generation) {
            self.holder.set(None);
        }
    }
}

/// A row's link back to the window: weak, so a page never keeps the window
/// alive, and silent once the window is gone.
#[derive(Clone)]
pub struct Shell(pub Weak<SettingsWindow>);

impl Shell {
    pub fn update(&self, mutate: impl FnOnce(&mut SettingsDocument)) {
        if let Some(shell) = self.0.upgrade() {
            shell.update(mutate);
        }
    }

    pub fn open_url(&self, url: &str) {
        if let Some(shell) = self.0.upgrade() {
            shell.open_url(url);
        }
    }

    pub fn recording(&self) -> (Option<RecorderTarget>, Option<ChordRejection>) {
        self.0
            .upgrade()
            .map_or((None, None), |shell| shell.recording())
    }

    pub fn start_recording(&self, target: RecorderTarget, field: Option<&gtk::Widget>) {
        if let Some(shell) = self.0.upgrade() {
            shell.start_recording(target, field);
        }
    }

    pub fn clear_shortcut(&self, target: RecorderTarget) {
        if let Some(shell) = self.0.upgrade() {
            shell.clear_shortcut(target);
        }
    }

    pub fn toast(&self, title: &str, detail: Option<&str>) {
        if let Some(shell) = self.0.upgrade() {
            shell.toast(title, detail);
        }
    }

    /// What a page has to say after a job, as a toast.
    pub fn report(&self, message: &PageMessage, strings: &StringResolver) {
        self.toast(&message.title(strings), message.detail(strings).as_deref());
    }

    pub fn window(&self) -> Option<adw::ApplicationWindow> {
        self.0.upgrade().map(|shell| shell.window().clone())
    }
}
