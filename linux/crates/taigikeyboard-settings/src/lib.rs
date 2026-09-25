//! The settings window as a library, so the pane-mount test can build the
//! pages without a running application (roadmap L12).

pub mod cli;
pub mod jobs;
pub mod pages;
pub mod presentation;
pub mod recorder;
pub mod search;
pub mod user_data;
pub mod window;
pub mod writer;

use adw::prelude::*;
use cli::LaunchOptions;
use std::cell::RefCell;
use std::rc::Rc;
use taigi_desktop_core::settings::{keys, SettingsPane};
use window::SettingsWindow;

/// The application id: one instance per session (`gio::Application`
/// forwards a second launch's command line to the first, roadmap L8).
pub const APPLICATION_ID: &str = "tw.taigikeyboard.Settings";

/// Runs the application to its exit code.
pub fn run() -> gtk::glib::ExitCode {
    let application = adw::Application::builder()
        .application_id(APPLICATION_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
    let window: Rc<RefCell<Option<Rc<SettingsWindow>>>> = Rc::new(RefCell::new(None));
    application.connect_command_line(move |application, command_line| {
        let arguments: Vec<String> = command_line
            .arguments()
            .into_iter()
            .skip(1)
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect();
        let launch = match LaunchOptions::parse(arguments) {
            Ok(launch) => launch,
            Err(error) => {
                // To the CALLER's stderr: a second launch's command line is
                // handled in the first process.
                log::error!("cli.refused error={error}");
                command_line.printerr_literal(&format!("taigikeyboard-settings: {error}\n"));
                return gtk::glib::ExitCode::from(2);
            }
        };
        // The first launch builds the window; a later one (the menu row
        // pressed again, another `--pane`) re-activates it on that pane —
        // the Windows single-instance mutex's contract, native here.
        let existing = window.borrow().clone();
        let shell = match existing {
            Some(shell) => shell,
            None => {
                // The window's own icon: the hicolor `taigikeyboard`, not one
                // named after the application id (which is not installed).
                gtk::Window::set_default_icon_name("taigikeyboard");
                let writer = writer::SettingsWriter::at_launch();
                // The stores follow the settings: no user directory, no
                // learning data either (the banner says so).
                let stores = if writer.is_read_only() {
                    Err("HOME / XDG_DATA_HOME".to_owned())
                } else {
                    user_data::open_at_launch()
                };
                let shell = SettingsWindow::build(application, writer, stores);
                *window.borrow_mut() = Some(Rc::clone(&shell));
                shell
            }
        };
        let pane = launch.pane.unwrap_or_else(|| {
            shell
                .writer()
                .borrow()
                .document()
                .choice(&keys::SELECTED_SETTINGS_PANE)
        });
        shell.show(pane);
        gtk::glib::ExitCode::SUCCESS
    });
    application.run()
}

/// The panes the sidebar lists on Linux, in order: the Mac's roster minus
/// 字型管理 — the framework's panel draws the candidates in its own font,
/// set in Fcitx5 / IBus, and the bundled typefaces install as system fonts
/// (roadmap L4, L7).
pub const SIDEBAR: [SettingsPane; 5] = [
    SettingsPane::General,
    SettingsPane::Appearance,
    SettingsPane::Shortcuts,
    SettingsPane::DictionarySources,
    SettingsPane::CustomDictionary,
];
