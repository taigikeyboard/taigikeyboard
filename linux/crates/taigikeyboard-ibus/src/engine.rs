//! One `org.freedesktop.IBus.Engine` object per client context the daemon
//! asks the factory for (roadmap L1 / L13). The method roster is the
//! introspection XML in ibus `src/ibusengine.c`; the base
//! `org.freedesktop.IBus.Service` (`Destroy`) is served beside it at the
//! same path (`src/ibusservice.c`).
//!
//! Every method body runs under `catch_unwind` and answers `false` / `()`
//! on a panic: the daemon respawns a dead engine and the user loses the
//! composition, which is worse than one key dropped. Signals are emitted
//! AFTER the key's work, never under the engine lock (`session`).

use crate::wire::{self, LookupTable, Orientation, PropType, Property};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Mutex};
use taigi_desktop_core::composing::ContextToken;
use taigi_desktop_core::keys::CandidateNavigation;
use taigi_linux_core::Emit;
use taigi_linux_core::{chrome, MenuItem, Runtime};
use taigi_linux_core::{
    new_engine_state,
    popup::{self, Caret},
};
use taigi_linux_core::{session, EngineState};
use taigi_linux_platform::RawKeyEvent;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{OwnedObjectPath, Value};
use zbus::{interface, Connection};

/// `IBUS_ENGINE_PREEDIT_COMMIT` (ibus `src/ibustypes.h:139`): a focus loss
/// commits what is typed, as the Mac's `commitComposition` does.
const PREEDIT_MODE_COMMIT: u32 = 1;

/// `IBUS_INPUT_PURPOSE_PASSWORD` / `_PIN` (ibus `src/ibustypes.h`,
/// `IBusInputPurpose` = 8 / 9): both hide what is typed. PIN counts because
/// Fcitx5 maps it to `Password | Digit` (`ibusfrontend.cpp`), and the two
/// shells must agree.
const HIDDEN_INPUT_PURPOSES: [u32; 2] = [8, 9];

/// The panel menu's root property (roadmap L6): its `symbol` and `icon` are
/// what the panel indicator shows for this engine, its sub-properties the
/// rows. The key is the one GNOME Shell reads the indicator text from
/// (`js/ui/status/keyboard.js`, GNOME 46: only `InputMode`, and only a
/// symbol of one or two characters) and the one the component file names
/// as `<icon_prop_key>` for the other panels (ibus `ui/gtk3/panel.vala`).
const MENU_ROOT_KEY: &str = "InputMode";

pub struct Engine {
    runtime: Arc<Runtime>,
    token: ContextToken,
    path: OwnedObjectPath,
    state: EngineState,
    /// The caret from the last `SetCursorLocation`, in screen pixels.
    caret: Mutex<Caret>,
}

impl Engine {
    pub fn new(runtime: Arc<Runtime>, token: ContextToken, path: OwnedObjectPath) -> Self {
        Self {
            runtime,
            token,
            path,
            state: new_engine_state(),
            caret: Mutex::new(Caret {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            }),
        }
    }

    /// A click or a page turn from the candidate window. Selecting never
    /// commits; a page turn is the panel's page up / page down.
    async fn on_popup(&mut self, emitter: &SignalEmitter<'_>, event: popup::Event) {
        match event {
            popup::Event::Click { position } => {
                let emits = self.guarded("popup.click", Vec::new(), |engine| {
                    session::click_from_panel(&engine.runtime, &mut engine.state, position as usize)
                });
                self.replay(emitter, emits).await;
            }
            popup::Event::Page { forward } => {
                self.navigate(
                    "popup.page",
                    emitter,
                    if forward {
                        CandidateNavigation::PageDown
                    } else {
                        CandidateNavigation::PageUp
                    },
                )
                .await;
            }
        }
    }

    /// Runs `work` against this engine's state under a panic boundary;
    /// answers `fallback` on a panic.
    fn guarded<T>(&mut self, what: &str, fallback: T, work: impl FnOnce(&mut Self) -> T) -> T {
        match catch_unwind(AssertUnwindSafe(|| work(self))) {
            Ok(answer) => answer,
            Err(_) => {
                log::error!(
                    "engine.panic method={what} token={:?} path={}",
                    self.token,
                    self.path
                );
                fallback
            }
        }
    }

    async fn replay(&self, emitter: &SignalEmitter<'_>, emits: Vec<Emit>) {
        for emit in emits {
            let sent = match emit {
                Emit::Preedit { text, caret } => {
                    Self::update_preedit_text(emitter, wire::preedit_text(&text), caret, true).await
                }
                Emit::ClearPreedit => {
                    Self::update_preedit_text(emitter, wire::plain_text(""), 0, false).await
                }
                Emit::Commit(text) => Self::commit_text(emitter, wire::plain_text(&text)).await,
                Emit::DeleteSurrounding { offset, count } => {
                    Self::delete_surrounding_text(emitter, offset, count).await
                }
                Emit::LookupTable(content) => {
                    // The window draws the page. An empty table goes to the
                    // daemon as well, so GNOME Shell does not draw a second
                    // list from the same reply.
                    let style = popup::PopupStyle::from_document(&self.runtime.settings.current());
                    let caret = *self
                        .caret
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    set_popup_target(self.path.clone());
                    popup::present(&content, &self.state, style, caret);
                    Self::update_lookup_table(emitter, empty_table_value(), false).await
                }
                Emit::HideLookupTable => {
                    popup::hide();
                    clear_popup_target(&self.path);
                    Self::update_lookup_table(emitter, empty_table_value(), false).await
                }
                Emit::ModeChanged => {
                    Self::update_property(emitter, menu_values(&self.runtime).root).await
                }
                // NAMED DIVERGENCE (roadmap L4): the daemon has no HUD; the
                // indicator's symbol is the only notice.
                Emit::AnnounceMode => Ok(()),
                // `taigi_linux_core` starts these before any reply is handed
                // back; one arriving here is a bug there.
                Emit::OpenSettings | Emit::ToggleTpsKeyboard | Emit::TpsKeyboardCapTyped(_) => {
                    log::error!("engine.launch_reached_shell token={:?}", self.token);
                    Ok(())
                }
            };
            if let Err(error) = sent {
                log::error!("engine.signal_failed token={:?} error={error}", self.token);
            }
        }
    }

    async fn end_session(&mut self, what: &str, emitter: &SignalEmitter<'_>) {
        let emits = self.guarded(what, Vec::new(), |engine| {
            session::end_session(&engine.runtime, engine.token, &mut engine.state)
        });
        self.replay(emitter, emits).await;
    }

    async fn begin_session(&mut self, what: &str, emitter: &SignalEmitter<'_>) {
        let emits = self.guarded(what, Vec::new(), |engine| {
            session::begin_session(&engine.runtime, &mut engine.state)
        });
        self.replay(emitter, emits).await;
    }

    /// `RegisterProperties` with the menu as it stands — on `Enable`, and
    /// again on every `FocusIn` so an Input Script, a display language or a
    /// chord changed in the settings window shows on the next focus.
    ///
    /// GNOME Shell honours only the FIRST registration after an engine
    /// change (GNOME 46 `js/misc/ibusManager.js` `_engineChanged` disconnects
    /// its `register-properties` handler once it fires), so a re-registration
    /// alone never reaches its indicator. `UpdateProperty` always does, and
    /// it matches by key and copies no sub-properties (ibus
    /// `ibus_property_update`): every row, then the root, goes out as its own
    /// update. Panels that do take a re-registration (ibus-ui-gtk3) receive
    /// the same values twice.
    async fn register_menu(&self, emitter: &SignalEmitter<'_>) {
        let menu = menu_values(&self.runtime);
        let props = wire::prop_list(vec![menu.root.clone()]);
        if let Err(error) = Self::register_properties(emitter, props).await {
            log::error!("engine.register_properties_failed error={error}");
        }
        for prop in menu.rows.into_iter().chain([menu.root]) {
            if let Err(error) = Self::update_property(emitter, prop).await {
                log::error!("engine.update_property_failed error={error}");
            }
        }
    }

    async fn navigate(
        &mut self,
        what: &str,
        emitter: &SignalEmitter<'_>,
        direction: CandidateNavigation,
    ) {
        let emits = self.guarded(what, Vec::new(), |engine| {
            session::navigate_from_panel(
                &engine.runtime,
                engine.token,
                &mut engine.state,
                direction,
            )
        });
        self.replay(emitter, emits).await;
    }
}

/// The engine the window's clicks belong to: the one that last showed a page.
fn set_popup_target(path: OwnedObjectPath) {
    *POPUP_TARGET
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(path);
}

fn clear_popup_target(path: &OwnedObjectPath) {
    let mut target = POPUP_TARGET
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if target.as_ref() == Some(path) {
        *target = None;
    }
}

fn popup_target() -> Option<OwnedObjectPath> {
    POPUP_TARGET
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

static POPUP_TARGET: Mutex<Option<OwnedObjectPath>> = Mutex::new(None);

/// Blocks on the window and dispatches clicks onto the engine that showed
/// it. A second `block_on` from this thread is the same shape as `Destroy`:
/// the daemon's executor is already running on the main thread.
pub fn watch_popup(connection: Connection) {
    let _ = std::thread::Builder::new()
        .name("taigi-popup".into())
        .spawn(move || loop {
            let events = popup::wait();
            if events.is_empty() {
                continue;
            }
            let connection = connection.clone();
            zbus::block_on(apply_popup(connection, events));
        });
}

async fn apply_popup(connection: Connection, events: Vec<popup::Event>) {
    let Some(path) = popup_target() else {
        return;
    };
    let Ok(iface) = connection
        .object_server()
        .interface::<_, Engine>(&path)
        .await
    else {
        return;
    };
    let Ok(emitter) = SignalEmitter::new(&connection, &path) else {
        return;
    };
    let mut engine = iface.get_mut().await;
    for event in events {
        engine.on_popup(&emitter, event).await;
    }
}

/// The menu root: a `PROP_TYPE_MENU` whose symbol and icon are the mode
/// indicator and whose sub-properties mirror `chrome::menu_items` row for
/// row — the recorded chord rides in the tooltip, the one text column the
/// panel draws beside a row. Label, symbol and icon come from one settings
/// snapshot, so a change made in the settings window between two reads
/// cannot split them. The rows ride beside the root as well: a registration
/// sends the one tree, an in-place refresh sends each property on its own.
struct MenuValues {
    root: Value<'static>,
    rows: Vec<Value<'static>>,
}

fn menu_values(runtime: &Runtime) -> MenuValues {
    let settings = runtime.settings.current();
    let label = chrome::mode_label(runtime, &settings);
    let indicator = chrome::mode_indicator(&settings);
    let items = chrome::menu_items(runtime);
    let rows: Vec<Value<'static>> = items
        .iter()
        .enumerate()
        .map(|(index, item)| match item {
            MenuItem::Separator => Property {
                key: &format!("separator-{index}"),
                kind: PropType::Separator,
                label: "",
                tooltip: "",
                icon: "",
                symbol: "",
                sub_props: Vec::new(),
            }
            .to_value(),
            MenuItem::Action { id, title, detail } => Property {
                key: id,
                kind: PropType::Normal,
                label: title,
                tooltip: detail.as_deref().unwrap_or(""),
                icon: "",
                symbol: "",
                sub_props: Vec::new(),
            }
            .to_value(),
        })
        .collect();
    let root = Property {
        key: MENU_ROOT_KEY,
        kind: PropType::Menu,
        label: &label,
        tooltip: "",
        icon: indicator.icon_name(),
        symbol: indicator.symbol(),
        sub_props: rows.clone(),
    }
    .to_value();
    MenuValues { root, rows }
}

fn empty_table_value() -> Value<'static> {
    LookupTable {
        page_size: 9,
        cursor_pos: 0,
        cursor_visible: false,
        round: false,
        orientation: Orientation::Horizontal,
        candidates: &[],
        labels: &[],
    }
    .to_value()
}

/// `spawn = false`: keys are handled in the order they arrive — a
/// composition is a sequence.
#[interface(name = "org.freedesktop.IBus.Engine", spawn = false)]
impl Engine {
    async fn process_key_event(
        &mut self,
        keyval: u32,
        keycode: u32,
        state: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> bool {
        let reply = self.guarded("ProcessKeyEvent", None, |engine| {
            Some(session::process_raw_key(
                &engine.runtime,
                engine.token,
                &mut engine.state,
                RawKeyEvent::from_ibus(keyval, keycode, state),
            ))
        });
        let Some(reply) = reply else {
            return false;
        };
        self.replay(&emitter, reply.emits).await;
        reply.handled
    }

    async fn set_cursor_location(&self, x: i32, y: i32, w: i32, h: i32) {
        let caret = Caret { x, y, w, h };
        *self
            .caret
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = caret;
        popup::move_to(caret);
    }

    async fn process_hand_writing_event(&self, _coordinates: Vec<f64>) {}

    async fn cancel_hand_writing(&self, _n_strokes: u32) {}

    async fn set_capabilities(&mut self, caps: u32) {
        log::debug!("engine.capabilities token={:?} caps={caps:#x}", self.token);
        session::set_capabilities(&mut self.state, caps);
    }

    async fn property_activate(
        &mut self,
        name: String,
        state: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        log::debug!("engine.property_activate name={name} state={state}");
        if name == MENU_ROOT_KEY || name.starts_with("separator-") {
            return;
        }
        let emits = self.guarded("PropertyActivate", Vec::new(), |engine| {
            chrome::activate_menu(&engine.runtime, engine.token, &mut engine.state, &name)
        });
        self.replay(&emitter, emits).await;
    }

    async fn property_show(&self, _name: String) {}

    async fn property_hide(&self, _name: String) {}

    async fn candidate_clicked(
        &mut self,
        index: u32,
        _button: u32,
        _state: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        let emits = self.guarded("CandidateClicked", Vec::new(), |engine| {
            session::click_from_panel(&engine.runtime, &mut engine.state, index as usize)
        });
        self.replay(&emitter, emits).await;
    }

    async fn focus_in(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        log::debug!("engine.focus_in token={:?}", self.token);
        // The client sends its surrounding text only once an engine asks
        // (ibus `ibus_input_context_needs_surrounding_text`). Asked here
        // rather than through `ActiveSurroundingText`, which daemons before
        // 1.5.28 never read.
        if let Err(error) = Self::require_surrounding_text(&emitter).await {
            log::error!("engine.require_surrounding_text_failed error={error}");
        }
        self.register_menu(&emitter).await;
        self.begin_session("FocusIn", &emitter).await;
    }

    async fn focus_in_id(&self, _object_path: String, _client: String) {}

    async fn focus_out(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        log::debug!("engine.focus_out token={:?}", self.token);
        self.end_session("FocusOut", &emitter).await;
    }

    async fn focus_out_id(
        &mut self,
        _object_path: String,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        self.end_session("FocusOutId", &emitter).await;
    }

    async fn reset(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.end_session("Reset", &emitter).await;
        self.begin_session("Reset", &emitter).await;
    }

    async fn enable(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        log::debug!("engine.enable token={:?}", self.token);
        self.register_menu(&emitter).await;
    }

    async fn disable(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.end_session("Disable", &emitter).await;
    }

    async fn page_up(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.navigate("PageUp", &emitter, CandidateNavigation::PageUp)
            .await;
    }

    async fn page_down(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.navigate("PageDown", &emitter, CandidateNavigation::PageDown)
            .await;
    }

    async fn cursor_up(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.navigate("CursorUp", &emitter, CandidateNavigation::PreviousCandidate)
            .await;
    }

    async fn cursor_down(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.navigate("CursorDown", &emitter, CandidateNavigation::NextCandidate)
            .await;
    }

    /// What precedes the caret, for the auto-space swap to check before it
    /// deletes (#866). Positions count characters.
    async fn set_surrounding_text(&mut self, text: Value<'_>, cursor_pos: u32, anchor_pos: u32) {
        let text = wire::text_of(&text);
        if text.is_none() {
            log::warn!("engine.surrounding_text_unreadable token={:?}", self.token);
        }
        session::set_surrounding_text(
            &mut self.state,
            text.map(|text| (text, cursor_pos as usize, anchor_pos as usize)),
        );
    }

    async fn panel_extension_received(&self, _event: Value<'_>) {}

    async fn panel_extension_register_keys(&self, _data: Value<'_>) {}

    /// `(uu)` — the client's input purpose and hints. Only the purpose is
    /// read: `IBUS_INPUT_PURPOSE_PASSWORD` / `_PIN` turn composing off in the field.
    #[zbus(property)]
    async fn set_content_type(&mut self, content_type: (u32, u32)) {
        let (purpose, _hints) = content_type;
        self.state.is_password_field = HIDDEN_INPUT_PURPOSES.contains(&purpose);
    }

    /// `false`: the daemon uses the plain `FocusIn` / `FocusOut` pair.
    #[zbus(property)]
    async fn focus_id(&self) -> (bool,) {
        (false,)
    }

    /// `false`: `FocusIn` asks for the surrounding text itself
    /// (`RequireSurroundingText`), the way every daemon version honours.
    #[zbus(property)]
    async fn active_surrounding_text(&self) -> (bool,) {
        (false,)
    }

    #[zbus(signal)]
    async fn commit_text(emitter: &SignalEmitter<'_>, text: Value<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_auxiliary_text(
        emitter: &SignalEmitter<'_>,
        text: Value<'_>,
        visible: bool,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_lookup_table(
        emitter: &SignalEmitter<'_>,
        table: Value<'_>,
        visible: bool,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn register_properties(emitter: &SignalEmitter<'_>, props: Value<'_>)
        -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_property(emitter: &SignalEmitter<'_>, prop: Value<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn forward_key_event(
        emitter: &SignalEmitter<'_>,
        keyval: u32,
        keycode: u32,
        state: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn require_surrounding_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn delete_surrounding_text(
        emitter: &SignalEmitter<'_>,
        offset: i32,
        n_chars: u32,
    ) -> zbus::Result<()>;
}

/// `UpdatePreeditText(v text, u cursor_pos, b visible, u mode)`
/// (`src/ibusengine.c`): emitted by hand so the mode is always
/// `PREEDIT_MODE_COMMIT` — the one place the preedit's focus-loss
/// behaviour is decided (roadmap L4).
impl Engine {
    async fn update_preedit_text(
        emitter: &SignalEmitter<'_>,
        text: Value<'_>,
        cursor_pos: u32,
        visible: bool,
    ) -> zbus::Result<()> {
        emitter
            .emit(
                "org.freedesktop.IBus.Engine",
                "UpdatePreeditText",
                &(text, cursor_pos, visible, PREEDIT_MODE_COMMIT),
            )
            .await
    }
}

/// `org.freedesktop.IBus.Service`, served at the same path: `Destroy`
/// takes the engine object down.
pub struct Service {
    runtime: Arc<Runtime>,
    token: ContextToken,
    path: OwnedObjectPath,
}

impl Service {
    pub fn new(runtime: Arc<Runtime>, token: ContextToken, path: OwnedObjectPath) -> Self {
        Self {
            runtime,
            token,
            path,
        }
    }
}

#[interface(name = "org.freedesktop.IBus.Service", spawn = false)]
impl Service {
    /// Releases the engine's ownership and unregisters both interfaces.
    /// The unregistration runs on its own thread: an object cannot remove
    /// itself from inside one of its own method calls (the object server
    /// holds it for the call), and `Destroy` wants no reply body anyway.
    async fn destroy(&self, #[zbus(connection)] connection: &Connection) {
        log::info!("engine.destroy token={:?} path={}", self.token, self.path);
        if let Some(coordinator) = self.runtime.coordinator_if_built() {
            let mut coordinator = coordinator
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            coordinator.release(self.token);
        }
        let connection = connection.clone();
        let path = self.path.clone();
        std::thread::Builder::new()
            .name("taigi-engine-destroy".into())
            .spawn(move || {
                zbus::block_on(async move {
                    let server = connection.object_server();
                    if let Err(error) = server.remove::<Engine, _>(&path).await {
                        log::warn!("engine.remove_failed path={path} error={error}");
                    }
                    if let Err(error) = server.remove::<Service, _>(&path).await {
                        log::warn!("service.remove_failed path={path} error={error}");
                    }
                })
            })
            .ok();
    }
}
