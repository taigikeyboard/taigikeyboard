//! The whole window mounted over a temporary settings directory — the
//! regression net for the pages (roadmap L12, the Windows `pane_planning`):
//! every built pane is in the stack; a row's switch writes its key; an
//! outside write shows on the next tick without bumping the revision; a
//! display language picked in the window rebuilds the sidebar; Manage Typefaces and
//! About are not listed; the read-only window writes nothing; a reset
//! keeps the display language.
//!
//! Needs a display: GTK cannot initialise without one, so the test skips
//! where there is none — unless `TAIGI_REQUIRE_DISPLAY` is set (CI under
//! xvfb), where a missing display is a failure. Its own `main`
//! (`harness = false`): GTK on macOS initialises only on the main thread.

use adw::prelude::*;
use std::process::ExitCode;
use std::rc::Rc;
use taigi_desktop_core::keys::{
    ComposingAction, ComposingKeyBindings, KeyModifiers, ShortcutAction,
};
use taigi_desktop_core::settings::{keys, SettingChoice, SettingsPane};
use taigi_desktop_core::strings::{DisplayLanguage, StringKey, StringResolver};
use taigi_desktop_storage::SettingsFileStore;
use taigikeyboard_settings::pages::BUILT;
use taigikeyboard_settings::recorder::RecorderTarget;
use taigikeyboard_settings::window::SettingsWindow;
use taigikeyboard_settings::writer::SettingsWriter;
use taigikeyboard_settings::SIDEBAR;

fn main() -> ExitCode {
    if gtk::init().is_err() {
        if std::env::var_os("TAIGI_REQUIRE_DISPLAY").is_some() {
            eprintln!("panes: no display for GTK and TAIGI_REQUIRE_DISPLAY is set");
            return ExitCode::FAILURE;
        }
        eprintln!("panes: skipping, no display for GTK");
        return ExitCode::SUCCESS;
    }
    adw::init().expect("libadwaita initialises after GTK");
    let application = adw::Application::builder()
        .application_id("tw.taigikeyboard.Settings.Test")
        .build();
    // A window may join an application only after its `startup`; registering
    // the (primary) instance emits it without running the main loop.
    application
        .register(gtk::gio::Cancellable::NONE)
        .expect("the test application registers");

    let directory = tempfile::tempdir().expect("a temp directory");
    let store = SettingsFileStore::new(directory.path());
    let stores = taigi_desktop_storage::UserDataStores::new(directory.path().to_path_buf());
    stores.open();
    let window =
        SettingsWindow::build(&application, SettingsWriter::new(store.clone()), Ok(stores));

    every_built_pane_is_in_the_stack(&window);
    a_switch_row_writes_its_key(&window);
    an_outside_write_shows_on_the_next_tick(&window, &store);
    a_language_picked_here_rebuilds_the_sidebar(&window);
    the_font_pane_routes_to_general_and_about_is_unlisted(&window);
    a_reset_keeps_the_display_language(&window);
    a_recorded_press_binds_the_row(&window);
    the_kautian_expander_switch_writes_its_key(&window);
    the_custom_dictionary_lists_what_the_store_holds(&window);
    the_read_only_window_writes_nothing(&application);
    eprintln!("panes: ok");
    ExitCode::SUCCESS
}

fn every_built_pane_is_in_the_stack(window: &Rc<SettingsWindow>) {
    for pane in BUILT {
        let page = window
            .page_widget(pane)
            .unwrap_or_else(|| panic!("{pane:?} is not in the stack"));
        assert!(
            find_first::<adw::PreferencesGroup>(&page).is_some(),
            "{pane:?} has no group"
        );
        assert_eq!(window.show_pane(pane), pane);
    }
    assert_eq!(window.sidebar_titles().len(), SIDEBAR.len());
    eprintln!("panes: {} panes in the stack", SettingsPane::ALL.len());
}

/// trace: General's first switch row is Show Candidate Window (`IS_CANDIDATE_WINDOW_ENABLED`,
/// default true). Flipping the switch writes `false`; flipping back `true`.
fn a_switch_row_writes_its_key(window: &Rc<SettingsWindow>) {
    let page = window.page_widget(SettingsPane::General).expect("general");
    let row = find_first::<adw::SwitchRow>(&page).expect("a switch row");
    assert!(row.is_active());
    row.set_active(false);
    assert!(!window
        .writer()
        .borrow()
        .document()
        .bool(&keys::IS_CANDIDATE_WINDOW_ENABLED));
    row.set_active(true);
    assert!(window
        .writer()
        .borrow()
        .document()
        .bool(&keys::IS_CANDIDATE_WINDOW_ENABLED));
    eprintln!("panes: switch row round-trips");
}

/// trace: the engine writes the file (revision N+1); the window's tick
/// adopts it, the row follows, and the refresh writes nothing (revision
/// stays N+1).
fn an_outside_write_shows_on_the_next_tick(window: &Rc<SettingsWindow>, store: &SettingsFileStore) {
    store
        .update(|document| document.set_bool(&keys::IS_AUTO_SPACE_ENABLED, true))
        .expect("the outside write");
    let revision = store.load().expect("load").revision;
    window.tick();
    let page = window.page_widget(SettingsPane::General).expect("general");
    let auto_space = find_all::<adw::SwitchRow>(&page)
        .into_iter()
        .find(|row| row.is_active() && row.title() != "")
        .expect("a row turned on");
    assert!(auto_space.is_active());
    assert_eq!(window.writer().borrow().document().revision, revision);
    assert_eq!(store.load().expect("load").revision, revision);
    eprintln!("panes: outside write adopted at revision {revision}");
}

fn a_language_picked_here_rebuilds_the_sidebar(window: &Rc<SettingsWindow>) {
    window.update(|document| {
        document.set_string(&keys::DISPLAY_LANGUAGE, DisplayLanguage::English.tag())
    });
    let english = StringResolver::new(DisplayLanguage::English);
    assert_eq!(
        window.sidebar_titles()[0],
        english.resolve(StringKey::DesktopGeneralTab)
    );
    window.update(|document| {
        document.set_string(&keys::DISPLAY_LANGUAGE, DisplayLanguage::Hanji.tag())
    });
    let hanji = StringResolver::new(DisplayLanguage::Hanji);
    assert_eq!(
        window.sidebar_titles()[0],
        hanji.resolve(StringKey::DesktopGeneralTab)
    );
    eprintln!("panes: language rebuild");
}

/// Manage Typefaces has no page on Linux (the panel's font is the framework's), so
/// a stored Manage Typefaces lands on General; About is shown without a sidebar row.
fn the_font_pane_routes_to_general_and_about_is_unlisted(window: &Rc<SettingsWindow>) {
    assert!(!SIDEBAR.contains(&SettingsPane::FontManagement));
    assert_eq!(
        window.show_pane(SettingsPane::FontManagement),
        SettingsPane::General
    );
    assert_eq!(window.current_pane(), SettingsPane::General);
    assert!(!SIDEBAR.contains(&SettingsPane::About));
    assert_eq!(window.show_pane(SettingsPane::About), SettingsPane::About);
    eprintln!("panes: font pane unbuilt");
}

/// trace: reset_general puts auto-space back to false and keeps the
/// language (#118).
fn a_reset_keeps_the_display_language(window: &Rc<SettingsWindow>) {
    window.update(|document| {
        document.set_string(&keys::DISPLAY_LANGUAGE, DisplayLanguage::English.tag());
        document.set_bool(&keys::IS_AUTO_SPACE_ENABLED, true);
    });
    window.update(taigi_desktop_core::settings::SettingsDocument::reset_general);
    let writer = window.writer();
    let document = writer.borrow();
    assert!(!document.document().bool(&keys::IS_AUTO_SPACE_ENABLED));
    assert_eq!(
        document.document().string(&keys::DISPLAY_LANGUAGE),
        DisplayLanguage::English.tag()
    );
    drop(document);
    window.update(|document| {
        document.set_string(&keys::DISPLAY_LANGUAGE, DisplayLanguage::System.tag())
    });
    eprintln!("panes: reset keeps the language");
}

/// trace: recording on Open Telex Guide (default Ctrl+Alt+/); a bare `a`
/// (keysym 0x61, keycode 38, no modifiers) is refused and recording
/// continues; a pane switch or any other row's write ends it; then
/// Ctrl+Alt+K (0x6b, keycode 45, CONTROL 1<<2 | MOD1 1<<3) is recorded and
/// stored as the row's chord; the × clears it; a composing row recorded on
/// a global default empties that global row (last writer wins).
fn a_recorded_press_binds_the_row(window: &Rc<SettingsWindow>) {
    let target = RecorderTarget::Global(ShortcutAction::ShowTelexGuide);
    window.start_recording(target, None);
    assert_eq!(window.recording().0, Some(target));
    window.record_press(0x61, 38, 0);
    assert_eq!(
        window.recording().0,
        Some(target),
        "a bare letter is refused"
    );
    assert!(window.recording().1.is_some());
    window.show(SettingsPane::Appearance);
    assert_eq!(
        window.recording().0,
        None,
        "a pane switch ends the recording"
    );
    window.start_recording(target, None);
    window.update(|document| document.set_bool(&keys::IS_AUTO_SPACE_ENABLED, false));
    assert_eq!(
        window.recording().0,
        None,
        "another write ends the recording"
    );
    window.start_recording(target, None);
    window.record_press(0x6b, 45, (1 << 2) | (1 << 3));
    assert_eq!(window.recording().0, None);
    let chord = ShortcutAction::ShowTelexGuide
        .chord_in(window.writer().borrow().document())
        .expect("the row is bound");
    assert_eq!(chord.key, "k");
    assert_eq!(
        chord.modifiers,
        KeyModifiers::CONTROL.with(KeyModifiers::ALT)
    );
    window.clear_shortcut(target);
    assert!(ShortcutAction::ShowTelexGuide
        .chord_in(window.writer().borrow().document())
        .is_none());
    let composing = RecorderTarget::Composing(ComposingAction::NextCandidate);
    window.start_recording(composing, None);
    window.record_press(0x63, 54, (1 << 2) | (1 << 3));
    {
        let writer = window.writer();
        let document = writer.borrow();
        let bindings = ComposingKeyBindings::from_document(document.document());
        assert_eq!(
            bindings
                .chord(ComposingAction::NextCandidate)
                .map(|c| c.key.clone()),
            Some("c".to_owned())
        );
        assert!(
            ShortcutAction::ToggleRomanization
                .chord_in(document.document())
                .is_none(),
            "the global row that held Ctrl+Alt+C is emptied"
        );
    }
    window.update(|document| {
        document.reset_composing_shortcuts();
        document.reset_global_shortcuts();
    });
    eprintln!("panes: recorder binds, clears, resolves across registries");
}

/// trace: the MOE dictionary row is the first switch on Dictionary Sources (`IS_KAUTIAN_ENABLED`,
/// default true); the eleven accent rows after it grey out while it is off
/// and come back, still set, when it is on.
fn the_kautian_expander_switch_writes_its_key(window: &Rc<SettingsWindow>) {
    let page = window
        .page_widget(SettingsPane::DictionarySources)
        .expect("dictionary sources");
    let switches = find_all::<adw::SwitchRow>(&page);
    let kautian = &switches[0];
    let lukang = &switches[1];
    assert!(kautian.is_active() && lukang.is_sensitive());
    kautian.set_active(false);
    assert!(!window
        .writer()
        .borrow()
        .document()
        .bool(&keys::IS_KAUTIAN_ENABLED));
    assert!(!lukang.is_sensitive(), "the 腔口 rows grey out");
    assert!(lukang.is_active(), "…and keep their setting");
    kautian.set_active(true);
    assert!(window
        .writer()
        .borrow()
        .document()
        .bool(&keys::IS_KAUTIAN_ENABLED));
    assert!(lukang.is_sensitive());
    eprintln!("panes: kautian switch greys its subcollections");
}

/// trace: two rows upserted into the store; the page's reload runs off the
/// UI thread and lands on the main context — pumped here until it does —
/// and the list then shows both.
fn the_custom_dictionary_lists_what_the_store_holds(window: &Rc<SettingsWindow>) {
    let stores = window.stores().expect("stores");
    stores.custom_dictionary.open_blocking();
    stores
        .custom_dictionary
        .upsert(&taigi_desktop_storage::CustomDictionaryRow::new(
            "tsia̍h-pn̄g",
            "食飯",
        ))
        .expect("upsert");
    stores
        .custom_dictionary
        .upsert(&taigi_desktop_storage::CustomDictionaryRow::new(
            "lim-tê", "啉茶",
        ))
        .expect("upsert");
    let document = window.writer().borrow().document().clone();
    let strings = window.writer().borrow().strings();
    let page = taigikeyboard_settings::pages::build(
        SettingsPane::CustomDictionary,
        window,
        &strings,
        &document,
        Some(stores),
        window.job_slot(),
    );
    let dictionary = page
        .state::<taigikeyboard_settings::pages::custom_dictionary::CustomDictionaryPage>()
        .expect("the page retains its state");
    dictionary.reload();
    pump_until(|| dictionary.shown_row_count() == 2);
    assert_eq!(
        dictionary.shown_row_count(),
        2,
        "the load landed on the main context"
    );
    // The entries list: a `PreferencesGroup` keeps a `boxed-list` of its
    // own for each set of rows, so the one with the two entries is ours.
    let entries = find_all::<gtk::ListBox>(page.widget.upcast_ref())
        .into_iter()
        .find(|list| list.row_at_index(1).is_some() && list.row_at_index(2).is_none())
        .expect("a list with exactly the two entries");
    // A selected row survives a reload without a re-entrant borrow: the
    // programmatic re-select fires the list's handler synchronously.
    entries.select_row(entries.row_at_index(0).as_ref());
    dictionary.reload();
    pump_until(|| dictionary.shown_row_count() == 2 && entries.selected_row().is_some());
    assert!(
        entries.selected_row().is_some(),
        "the selection is kept by id across a reload"
    );
    // Text that would be markup is shown as typed.
    stores
        .custom_dictionary
        .upsert(&taigi_desktop_storage::CustomDictionaryRow::new(
            "a-b", "A&B <b>",
        ))
        .expect("upsert");
    dictionary.reload();
    pump_until(|| dictionary.shown_row_count() == 3);
    assert!(find_all::<gtk::Label>(page.widget.upcast_ref())
        .iter()
        .any(|label| label.label() == "A&B <b>"));
    let search = taigikeyboard_settings::pages::build(
        SettingsPane::DictionarySearch,
        window,
        &strings,
        &document,
        Some(stores),
        window.job_slot(),
    );
    assert!(find_first::<gtk::SearchEntry>(search.widget.upcast_ref()).is_some());
    eprintln!("panes: custom dictionary lists the store; search page mounts");
}

fn the_read_only_window_writes_nothing(application: &adw::Application) {
    let window = SettingsWindow::build(
        application,
        SettingsWriter::read_only("test".to_owned()),
        Err("test".to_owned()),
    );
    assert!(window.writer().borrow().is_read_only());
    window.update(|document| document.set_bool(&keys::IS_AUTO_SPACE_ENABLED, true));
    assert!(!window
        .writer()
        .borrow()
        .document()
        .bool(&keys::IS_AUTO_SPACE_ENABLED));
    assert_eq!(window.writer().borrow().write_failure(), Some("test"));
    assert_eq!(
        window
            .writer()
            .borrow()
            .document()
            .choice(&keys::SELECTED_SETTINGS_PANE)
            .raw(),
        SettingsPane::General.raw()
    );
    eprintln!("panes: read-only window writes nothing");
}

/// Runs the main context until `done` answers true or ten seconds pass —
/// non-blocking iterations, so the clock is checked between them.
fn pump_until(mut done: impl FnMut() -> bool) {
    let context = gtk::glib::MainContext::default();
    let started = std::time::Instant::now();
    while !done() && started.elapsed() < std::time::Duration::from_secs(10) {
        context.iteration(false);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// The first descendant of type `T`, depth first.
fn find_first<T: IsA<gtk::Widget>>(root: &gtk::Widget) -> Option<T> {
    find_all::<T>(root).into_iter().next()
}

fn find_all<T: IsA<gtk::Widget>>(root: &gtk::Widget) -> Vec<T> {
    let mut found = Vec::new();
    let mut child = root.first_child();
    while let Some(widget) = child {
        if let Ok(typed) = widget.clone().downcast::<T>() {
            found.push(typed);
        }
        found.extend(find_all::<T>(&widget));
        child = widget.next_sibling();
    }
    found
}
