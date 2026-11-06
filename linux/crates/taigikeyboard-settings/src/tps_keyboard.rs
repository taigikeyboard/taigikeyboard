//! The on-screen TPS key panel on Linux (desktop TPS roadmap D6, U6): an
//! ordinary window of this app, show-only. The input method owns no window
//! (`taigi-linux-core/src/session.rs` `run_launches`), so Show TPS Keyboard launches this app
//! with `--tps-keyboard`, and the one instance opens the panel or closes the
//! one that is up. Taking focus ends the IME session, so it opens only on that
//! explicit request: it closes when the input mode leaves TPS and stays closed
//! when TPS comes back — the Linux exception to "persists until the shortcut
//! hides it". Where it appears is the compositor's call.

use adw::prelude::*;
use gtk::glib;
use std::cell::RefCell;
use std::rc::Rc;
use taigi_desktop_core::keys::{tps_keyboard_rows, TpsKeyCap, TpsKeyboardRow};
use taigi_desktop_core::strings::StringKey;
use taigi_desktop_storage::{SettingsWriter, IDLE_REFRESH_INTERVAL};

use crate::presentation::strings_for;

/// A key cap's side and the gap between caps, in pixels — the Windows and
/// macOS panels' geometry (`ui/tps_keyboard.rs` `CAP_SIZE` / `CAP_GAP`).
const CAP_SIZE: i32 = 48;
const CAP_GAP: i32 = 6;
const PADDING: i32 = 12;

/// Where the open panel lives, if one is: what a second `--tps-keyboard`
/// launch reads to close it, cleared when the window goes.
pub type PanelSlot = Rc<RefCell<Option<Rc<TpsKeyboardWindow>>>>;

pub struct TpsKeyboardWindow {
    window: adw::ApplicationWindow,
    /// Read, never written: the input mode the panel follows.
    writer: RefCell<SettingsWriter>,
}

/// The `--tps-keyboard` launch: closes the panel that is up, or opens one
/// over `writer` when TPS is being typed.
pub fn toggle(application: &adw::Application, slot: &PanelSlot, writer: SettingsWriter) {
    let open = slot.borrow().clone();
    match open {
        Some(panel) => panel.close(),
        None => TpsKeyboardWindow::open(application, slot, writer),
    }
}

impl TpsKeyboardWindow {
    /// Builds and presents the panel and puts it in `slot` — nothing, when
    /// the input mode is not TPS (the chord is inert there, but the mode may
    /// have changed since it was pressed).
    pub fn open(application: &adw::Application, slot: &PanelSlot, writer: SettingsWriter) {
        if !writer.document().is_typing_tps() {
            log::info!("tps_keyboard.not_opened reason=not_tps");
            return;
        }
        let rows = tps_keyboard_rows();
        let title = strings_for(writer.document())
            .resolve(StringKey::SettingsTpsMode)
            .to_owned();
        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(CAP_GAP)
            .margin_top(PADDING)
            .margin_bottom(PADDING)
            .margin_start(PADDING)
            .margin_end(PADDING)
            .build();
        for row in &rows {
            content.append(&row_widget(row));
        }
        let view = adw::ToolbarView::new();
        view.add_top_bar(&adw::HeaderBar::new());
        view.set_content(Some(&content));
        let window = adw::ApplicationWindow::builder()
            .application(application)
            .title(title)
            .resizable(false)
            .content(&view)
            .build();
        let panel = Rc::new(Self {
            window,
            writer: RefCell::new(writer),
        });
        // Every way the window goes — its close button, a second launch,
        // the mode leaving TPS — empties the slot, so the next launch opens
        // a new one; the tick below then finds no panel and stops.
        let closing_slot = Rc::clone(slot);
        panel.window.connect_close_request(move |_| {
            closing_slot.borrow_mut().take();
            glib::Propagation::Proceed
        });
        let weak = Rc::downgrade(&panel);
        glib::timeout_add_local(IDLE_REFRESH_INTERVAL, move || match weak.upgrade() {
            Some(panel) if panel.tick() => glib::ControlFlow::Continue,
            _ => glib::ControlFlow::Break,
        });
        panel.window.present();
        *slot.borrow_mut() = Some(panel);
    }

    pub fn close(&self) {
        self.window.close();
    }

    /// The live-reload beat: closes the panel once the input mode has left
    /// TPS. Answers whether it is still open.
    pub fn tick(&self) -> bool {
        let mut writer = self.writer.borrow_mut();
        writer.refresh();
        if writer.document().is_typing_tps() {
            return true;
        }
        drop(writer);
        log::info!("tps_keyboard.closed reason=left_tps");
        self.close();
        false
    }
}

/// One row: indented by the row's stagger, its caps side by side.
fn row_widget(row: &TpsKeyboardRow) -> gtk::Box {
    let widget = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(CAP_GAP)
        .margin_start((row.indent * (CAP_SIZE + CAP_GAP) as f32).round() as i32)
        .build();
    for cap in &row.caps {
        widget.append(&cap_widget(cap));
    }
    widget
}

/// One cap as the other desktops draw it: the key's label top left, the
/// Shift glyph top right, the glyph large in the middle.
fn cap_widget(cap: &TpsKeyCap) -> gtk::Box {
    let label = gtk::Label::builder()
        .label(cap.label.to_string())
        .css_classes(["caption", "dim-label"])
        .halign(gtk::Align::Start)
        .build();
    let shift_glyph = gtk::Label::builder()
        .label(cap.shift_glyph.unwrap_or(""))
        .css_classes(["caption"])
        .halign(gtk::Align::End)
        .build();
    let top = gtk::CenterBox::new();
    top.set_start_widget(Some(&label));
    top.set_end_widget(Some(&shift_glyph));
    let glyph = gtk::Label::builder()
        .label(cap.glyph)
        .css_classes(["title-3"])
        .vexpand(true)
        .build();
    let widget = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .css_classes(["card"])
        .width_request(CAP_SIZE)
        .height_request(CAP_SIZE)
        .build();
    top.set_margin_start(4);
    top.set_margin_end(4);
    widget.append(&top);
    widget.append(&glyph);
    widget
}
