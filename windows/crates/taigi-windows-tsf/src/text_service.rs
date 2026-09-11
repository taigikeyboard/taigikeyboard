//! The text service: the COM object TSF activates once per thread manager.
//! Lifecycle, sinks, tray button + menu, settings reload, context identity
//! (PR5a); the composing path itself is `session.rs` (PR5b).
//!
//! Threading (roadmap W3): TSF is STA — every method here runs on the
//! host's UI thread, so the state is a `RefCell`. RULE: no COM call of any
//! kind — not even an `AddRef` (`clone`) or a `Release` (`drop`) — while a
//! borrow is held. Values are MOVED in and out of the state under the
//! borrow; the reference counting happens outside it. Callbacks raised
//! from inside `msctf!_NotifyCallbacks` (`OnSetFocus`) only record and
//! return (rakukan `factory.rs:1304-1330`); the settings re-read they
//! request happens on the next key-sink call.

use crate::com_guard::guarded;
use crate::contexts::ContextRegistry;
use crate::conversion_mode;
use crate::display_attribute::{self, DisplayAttributeEnumerator};
use crate::key_translation;
use crate::lang_bar::{self, LANG_BAR_SINK_COOKIE, MENU_CHECK_FOR_UPDATES, MENU_OPEN_SETTINGS};
use crate::preserved_keys::{self, PreservedKeys};
use crate::product_name;
use crate::runtime::Runtime;
use crate::session::KeyPhase;
use crate::settings_launcher;
use crate::ui::mode_flash::ModeFlash;
use crate::ui::presenter::CandidatePresenter;
use crate::ui::render::RenderFactory;
use crate::ui::telex_guide::TelexGuide;
use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use taigi_windows_core::composing::ContextToken;
use taigi_windows_core::keys::{LanguageMode, ShiftTapTracker, ShortcutAction, VK_SHIFT_CODE};
use windows::core::{Error, IUnknown, Interface, Ref, Result, BOOL, BSTR, GUID};
use windows::Win32::Foundation::{E_FAIL, E_INVALIDARG, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::System::Ole::{CONNECT_E_ADVISELIMIT, CONNECT_E_NOCONNECTION};
use windows::Win32::System::SystemInformation::GetTickCount64;
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_CONTROL, VK_MENU};
use windows::Win32::UI::TextServices::{
    IEnumTfDisplayAttributeInfo, ITfComposition, ITfCompositionSink, ITfCompositionSink_Impl,
    ITfContext, ITfDisplayAttributeInfo, ITfDisplayAttributeProvider,
    ITfDisplayAttributeProvider_Impl, ITfDocumentMgr, ITfKeyEventSink, ITfKeyEventSink_Impl,
    ITfKeystrokeMgr, ITfLangBarItem, ITfLangBarItemButton, ITfLangBarItemButton_Impl,
    ITfLangBarItemMgr, ITfLangBarItemSink, ITfLangBarItem_Impl, ITfMenu, ITfSource, ITfSource_Impl,
    ITfTextInputProcessorEx, ITfTextInputProcessorEx_Impl, ITfTextInputProcessor_Impl,
    ITfThreadFocusSink, ITfThreadFocusSink_Impl, ITfThreadMgr, ITfThreadMgrEventSink,
    ITfThreadMgrEventSink_Impl, TfLBIClick, TF_INVALID_COOKIE, TF_LANGBARITEMINFO, TF_LBI_ICON,
    TF_LBI_TEXT,
};
use windows::Win32::UI::WindowsAndMessaging::HICON;
use windows_core::{implement, IUnknownImpl};

/// Milliseconds on a clock that only moves forward, for measuring how long a
/// key was held. NOT `GetMessageTime`: that is the time of the last message
/// this thread pulled off its queue with `GetMessage`, and a TSF key sink is
/// a COM call, so the two need not be the same event (Codex F3, 2026-09-04).
/// 64-bit, so there is no 49.7-day wrap to reason about.
fn now_milliseconds() -> u64 {
    // SAFETY: no parameters, no out-pointer; reads the system tick count.
    unsafe { GetTickCount64() }
}

/// Everything `Activate` set up and `Deactivate` takes down.
#[derive(Default)]
pub(crate) struct ServiceState {
    thread_mgr: Option<ITfThreadMgr>,
    pub(crate) client_id: u32,
    activate_flags: u32,
    thread_mgr_sink_cookie: u32,
    thread_focus_sink_cookie: u32,
    is_key_sink_advised: bool,
    /// Kept for the preserved-key re-registration on a settings change.
    keystroke_mgr: Option<ITfKeystrokeMgr>,
    preserved_keys: PreservedKeys,
    /// The atom `RegisterGUID` gave the preedit's display attribute.
    pub(crate) display_attribute_atom: u32,
    is_lang_bar_added: bool,
    lang_bar_sink: Option<ITfLangBarItemSink>,
    pub(crate) contexts: ContextRegistry,
    /// The counter context tokens are handed out from — one per service,
    /// never a COM address: an address is reused once its context is gone,
    /// and the next context would inherit the dead one's ownership.
    next_context_token: usize,
    /// The `IUnknown` identity of the focused document manager — recorded,
    /// never dereferenced; a late focus notification is a hint (contract 7).
    focused_document: usize,
    /// Set by the focus callbacks, consumed by the next key-sink call: the
    /// settings file is re-read there, never inside `_NotifyCallbacks`.
    is_settings_refresh_pending: bool,
    /// Bumped by every focus / context event. A key compares it around the
    /// ownership handover: a change means COM re-entrancy moved the focus,
    /// and the key goes back to the host (coordinator contract points 3 / 4).
    pub(crate) focus_generation: u64,
    /// Tokens whose engine ownership could not be released in a callback
    /// (the engine was busy); released under the next key's lock.
    pub(crate) deferred_releases: Vec<ContextToken>,
    /// The candidate window (one per service — its thread pumps it) and the
    /// mode flash, behind `Rc` so the key path borrows them, not the state.
    /// Where the mode flash goes: the screen point of the last caret the
    /// window anchored to, or the primary monitor's origin before any.
    pub(crate) focused_caret: windows::Win32::Foundation::POINT,
    /// A focus / context callback wanted the windows down but the presenter
    /// or the guide was busy (a session in flight): the next key hides first.
    pub(crate) is_ui_hide_pending: bool,
    /// A once-per-press chord (`ShortcutAction::fires_once_per_press`: the
    /// guide's or the picker's) is down and has already toggled through
    /// `OnPreservedKey`, which TSF may re-fire while the chord is held and
    /// which carries no repeat flag. Cleared by the chord's key-up, by Ctrl's
    /// or Alt's, by any other fresh key, by focus loss and by deactivation,
    /// so a missed key-up cannot wedge the toggle.
    pub(crate) held_toggle_chord: Option<ShortcutAction>,
    pub(crate) presenter: Option<Rc<RefCell<CandidatePresenter>>>,
    /// The symbol picker (USER 2026-09-09): the same window class over its
    /// own owner slot, popup only (`CandidatePresenter::attach_popup_only`).
    /// A second instance rather than a second list in the first: the
    /// composing list's ownership and `is_showing` are read by the composing
    /// key contract, and a picker sharing them would hand the arrows and the
    /// slot keys to whichever list showed last (macOS `CandidatePanel.symbolPicker`).
    pub(crate) symbol_picker: Option<Rc<RefCell<CandidatePresenter>>>,
    /// Whether the picker was opened — the one piece of picker state
    /// outside the window. WHO it is up for is the window's own owner, and
    /// the selection is the window's too; the flag is only ever read while
    /// the window says it is up for the asking context (`live_symbol_picker`).
    pub(crate) is_symbol_picker_open: bool,
    pub(crate) mode_flash: Option<Rc<RefCell<ModeFlash>>>,
    /// The Telex key table the `showTelexGuide` chord toggles; owned by the
    /// context that raised it, like the candidate window.
    pub(crate) telex_guide: Option<Rc<RefCell<TelexGuide>>>,
    /// Whether keys compose Taigi or go to the document as English. Per
    /// activation — one text service instance is one thread manager, which is
    /// one application, so switching to English in a terminal leaves the
    /// browser next door composing. Never persisted: a transient mode, the
    /// way Windows CJK input methods treat theirs.
    pub(crate) language_mode: LanguageMode,
    /// The Shift-tap recogniser behind that mode. Fed by all four key
    /// callbacks; nothing else reads the keyboard.
    pub(crate) shift_tap: ShiftTapTracker,
}

#[implement(
    ITfTextInputProcessorEx,
    ITfThreadMgrEventSink,
    ITfThreadFocusSink,
    ITfKeyEventSink,
    ITfCompositionSink,
    ITfDisplayAttributeProvider,
    ITfLangBarItemButton,
    ITfSource
)]
pub struct TextService {
    pub(crate) state: RefCell<ServiceState>,
}

impl TextService {
    pub fn new() -> Self {
        Self {
            state: RefCell::new(ServiceState {
                thread_mgr_sink_cookie: TF_INVALID_COOKIE,
                thread_focus_sink_cookie: TF_INVALID_COOKIE,
                next_context_token: 1,
                ..ServiceState::default()
            }),
        }
    }
}

impl Default for TextService {
    fn default() -> Self {
        Self::new()
    }
}

impl TextService_Impl {
    fn activate(&self, thread_mgr: &ITfThreadMgr, client_id: u32, flags: u32) -> Result<()> {
        // The AddRef happens here, outside the borrow; only the move is inside.
        let owned_thread_mgr = thread_mgr.clone();
        {
            let mut state = self.state.borrow_mut();
            state.thread_mgr = Some(owned_thread_mgr);
            state.client_id = client_id;
            state.activate_flags = flags;
            // The 中/英 mode is per ACTIVATION: TSF may deactivate and
            // reactivate the same object, and a mode carried over would leave
            // the tray letter, the compartment and the classifier disagreeing.
            // The Shift press goes with it — the one that armed it belonged to
            // the previous activation. The compartment is published at the end
            // of this method, once the thread manager is wired.
            state.language_mode = LanguageMode::default();
            state.shift_tap.clear();
        }
        // Probes paths and reads the settings file once per process; the
        // engine and the stores stay untouched until a key is CONSUMED (PR5b).
        let runtime = Runtime::shared();
        log::info!(
            "tsf.activate client_id={client_id} flags={flags:#x} learning={}",
            runtime.capability.learning
        );
        // The user has this input method selected somewhere, so the
        // settings window is one shortcut away: map its WinUI runtime now,
        // in a process that exits as soon as it has. Claimed across hosts,
        // spawned off this thread, and every failure inside it is logged
        // rather than returned — activation must not fail over a prewarm,
        // and a failed activation rolls the whole service back.
        settings_launcher::prewarm_once();

        let source: ITfSource = thread_mgr.cast()?;
        let object = self.to_object();
        // SAFETY: `source` is the live thread manager's ITfSource; the sinks
        // are counted references to this object, which TSF holds until the
        // matching UnadviseSink in `deactivate`.
        let thread_mgr_cookie = unsafe {
            let sink: IUnknown = object.to_interface::<ITfThreadMgrEventSink>().cast()?;
            source.AdviseSink(&ITfThreadMgrEventSink::IID, &sink)?
        };
        self.state.borrow_mut().thread_mgr_sink_cookie = thread_mgr_cookie;
        // Hosts that never raise the thread-manager focus event (non-TSF-aware
        // apps) still raise this one (rakukan `factory.rs:505-512`).
        // SAFETY: as above.
        let thread_focus_cookie = unsafe {
            let sink: IUnknown = object.to_interface::<ITfThreadFocusSink>().cast()?;
            source.AdviseSink(&ITfThreadFocusSink::IID, &sink)?
        };
        self.state.borrow_mut().thread_focus_sink_cookie = thread_focus_cookie;

        let keystroke_mgr: ITfKeystrokeMgr = thread_mgr.cast()?;
        // SAFETY: the keystroke manager is the thread manager's; the sink is
        // this object; `true` = foreground sink (khiin `text_service.rs`).
        unsafe {
            keystroke_mgr.AdviseKeyEventSink(
                client_id,
                &object.to_interface::<ITfKeyEventSink>(),
                true,
            )?
        };
        let kept_keystroke_mgr = keystroke_mgr.clone();
        {
            let mut state = self.state.borrow_mut();
            state.is_key_sink_advised = true;
            state.keystroke_mgr = Some(kept_keystroke_mgr);
        }
        let atom = display_attribute::register_input_atom();
        self.state.borrow_mut().display_attribute_atom = atom;
        // The global shortcuts, from the stored chords (roadmap W5).
        let settings = runtime.settings.current();
        let mut preserved = std::mem::take(&mut self.state.borrow_mut().preserved_keys);
        preserved.sync(&keystroke_mgr, client_id, &settings);
        self.state.borrow_mut().preserved_keys = preserved;

        // The renderer: Direct2D/DirectWrite factories + the bundled fonts,
        // once per activation. A host without Direct2D (a remote session's
        // basic display) keeps typing with no window (logged).
        match RenderFactory::new() {
            Ok(factory) => {
                let factory = Rc::new(factory);
                let presenter = Rc::new(RefCell::new(CandidatePresenter::new(Rc::clone(&factory))));
                presenter.borrow_mut().attach(
                    thread_mgr.clone(),
                    self.to_object(),
                    Rc::downgrade(&presenter),
                );
                let symbol_picker =
                    Rc::new(RefCell::new(CandidatePresenter::new(Rc::clone(&factory))));
                symbol_picker
                    .borrow_mut()
                    .attach_popup_only(Rc::downgrade(&symbol_picker));
                let flash = ModeFlash::new(Rc::clone(&factory));
                let guide = TelexGuide::new(factory);
                let mut state = self.state.borrow_mut();
                state.presenter = Some(presenter);
                state.symbol_picker = Some(symbol_picker);
                state.mode_flash = Some(Rc::new(RefCell::new(flash)));
                state.telex_guide = Some(Rc::new(RefCell::new(guide)));
            }
            Err(error) => {
                log::error!("ui.render_factory_failed error={error} — no candidate window")
            }
        }

        // Cosmetic: a tray button that fails to add is logged, not fatal.
        match thread_mgr.cast::<ITfLangBarItemMgr>() {
            Ok(lang_bar_mgr) => {
                match object
                    .to_interface::<ITfLangBarItemButton>()
                    .cast::<ITfLangBarItem>()
                    // SAFETY: the item is this object, alive as long as TSF holds it.
                    .and_then(|item| unsafe { lang_bar_mgr.AddItem(&item) })
                {
                    Ok(()) => self.state.borrow_mut().is_lang_bar_added = true,
                    Err(error) => log::warn!("tsf.lang_bar_add_failed error={error}"),
                }
            }
            Err(error) => log::warn!("tsf.lang_bar_mgr_unavailable error={error}"),
        }

        // A fresh activation composes Taigi, and the compartment says so from
        // the start rather than from the first switch — an application that
        // reads it before any key would otherwise see this service as
        // alphanumeric.
        conversion_mode::publish(thread_mgr, client_id, LanguageMode::default());
        Ok(())
    }

    /// Reverse order of `activate`; every step attempted. Every COM
    /// reference the state held is moved out under the borrow and released
    /// after it.
    fn deactivate(&self) -> Result<()> {
        // The windows first (no candidate may outlive its service), then the
        // compositions still open are finished into their documents — they
        // need the contexts and the engine still wired.
        let (presenter, symbol_picker, flash, guide) = {
            let mut state = self.state.borrow_mut();
            // The mode does not outlive the activation that switched it.
            // `activate` sets it too; this end is what a teardown `GetText`
            // between here and `RemoveItem` reads, so the tray never draws 英
            // for a service that is going away.
            state.language_mode = LanguageMode::default();
            state.shift_tap.clear();
            state.held_toggle_chord = None;
            // The windows are destroyed below; a hide still owed is moot.
            state.is_ui_hide_pending = false;
            state.is_symbol_picker_open = false;
            (
                state.presenter.take(),
                state.symbol_picker.take(),
                state.mode_flash.take(),
                state.telex_guide.take(),
            )
        };
        if let Some(presenter) = presenter {
            presenter.borrow_mut().detach();
        }
        if let Some(symbol_picker) = symbol_picker {
            symbol_picker.borrow_mut().detach();
        }
        if let Some(flash) = flash {
            flash.borrow_mut().destroy();
        }
        if let Some(guide) = guide {
            guide.borrow_mut().destroy();
        }
        let mut entries = std::mem::take(&mut self.state.borrow_mut().contexts).into_entries();
        self.finish_all_compositions(&mut entries);
        let (
            thread_mgr,
            client_id,
            thread_mgr_cookie,
            thread_focus_cookie,
            key_sink,
            lang_bar,
            sink,
            keystroke_mgr,
            mut preserved,
        ) = {
            let mut state = self.state.borrow_mut();
            (
                state.thread_mgr.take(),
                state.client_id,
                std::mem::replace(&mut state.thread_mgr_sink_cookie, TF_INVALID_COOKIE),
                std::mem::replace(&mut state.thread_focus_sink_cookie, TF_INVALID_COOKIE),
                std::mem::take(&mut state.is_key_sink_advised),
                std::mem::take(&mut state.is_lang_bar_added),
                state.lang_bar_sink.take(),
                state.keystroke_mgr.take(),
                std::mem::take(&mut state.preserved_keys),
            )
        };
        log::info!(
            "tsf.deactivate client_id={client_id} contexts_released={}",
            entries.len()
        );
        drop(entries);
        drop(sink);
        if let Some(keystroke_mgr) = &keystroke_mgr {
            preserved.unregister(keystroke_mgr);
        }
        drop(keystroke_mgr);
        let Some(thread_mgr) = thread_mgr else {
            return Ok(());
        };
        let object = self.to_object();
        // SAFETY: every call undoes one `activate` step on the same thread
        // manager, with the cookies / interfaces that step recorded.
        unsafe {
            if lang_bar {
                if let Ok(lang_bar_mgr) = thread_mgr.cast::<ITfLangBarItemMgr>() {
                    if let Ok(item) = object
                        .to_interface::<ITfLangBarItemButton>()
                        .cast::<ITfLangBarItem>()
                    {
                        lang_bar_mgr.RemoveItem(&item).ok();
                    }
                }
            }
            if key_sink {
                if let Ok(keystroke_mgr) = thread_mgr.cast::<ITfKeystrokeMgr>() {
                    keystroke_mgr.UnadviseKeyEventSink(client_id).ok();
                }
            }
            if let Ok(source) = thread_mgr.cast::<ITfSource>() {
                if thread_focus_cookie != TF_INVALID_COOKIE {
                    source.UnadviseSink(thread_focus_cookie).ok();
                }
                if thread_mgr_cookie != TF_INVALID_COOKIE {
                    source.UnadviseSink(thread_mgr_cookie).ok();
                }
            }
        }
        Ok(())
    }

    /// The token for `context`, allocating on first sight. The identity
    /// query and the `AddRef` happen before the borrow; only the insert is
    /// under it.
    pub(crate) fn token_for(&self, context: &ITfContext) -> Option<(ContextToken, usize)> {
        let identity = ContextRegistry::identity(context)?;
        let owned = context.clone();
        let (token, duplicate) = {
            let mut state = self.state.borrow_mut();
            let ServiceState {
                contexts,
                next_context_token,
                ..
            } = &mut *state;
            contexts.token_for(identity, owned, || {
                // `0` reads as "no token" in the window's queued messages, so
                // the counter must stop rather than wrap onto it. No process
                // reaches this; the panic is caught at the COM boundary like
                // any other.
                assert!(
                    *next_context_token != 0 && *next_context_token != usize::MAX,
                    "context token space exhausted"
                );
                let token = ContextToken(*next_context_token);
                *next_context_token += 1;
                token
            })
        };
        // A reference the registry did not keep is released outside the borrow.
        drop(duplicate);
        Some((token, identity))
    }

    /// Settings may have changed while another window had focus: the focus
    /// callbacks only flag it, and the next key-sink call re-reads (one
    /// `stat`, W10). Never inside `_NotifyCallbacks`.
    pub(crate) fn refresh_settings_if_pending(&self) {
        let pending = std::mem::take(&mut self.state.borrow_mut().is_settings_refresh_pending);
        if !pending {
            return;
        }
        let settings = Runtime::shared().settings.current();
        self.sync_preserved_keys(&settings);
    }

    fn request_settings_refresh(&self) {
        let mut state = self.state.borrow_mut();
        state.is_settings_refresh_pending = true;
        state.focus_generation += 1;
    }

    /// The candidate window, if this host got one. ALWAYS cloned out of the
    /// state before it is used: `CandidatePresenter`'s methods make COM
    /// calls, and this module's rule (header) is that none may run while
    /// the state's `RefCell` borrow is held — the `Rc` clone here ends the
    /// borrow before the caller's `borrow_mut()` on the presenter itself.
    pub(crate) fn presenter(&self) -> Option<Rc<RefCell<CandidatePresenter>>> {
        self.state.borrow().presenter.clone()
    }

    /// The Telex guide, cloned out of the state for the same reason as
    /// [`TextService_Impl::presenter`].
    pub(crate) fn telex_guide(&self) -> Option<Rc<RefCell<TelexGuide>>> {
        self.state.borrow().telex_guide.clone()
    }

    /// The symbol picker's window, cloned out for the same reason.
    pub(crate) fn symbol_picker(&self) -> Option<Rc<RefCell<CandidatePresenter>>> {
        self.state.borrow().symbol_picker.clone()
    }

    /// Takes the symbol picker down whoever raised it — the settings
    /// doorways, the handover and the pending-hide drain.
    pub(crate) fn hide_symbol_picker_now(&self) {
        self.state.borrow_mut().is_symbol_picker_open = false;
        if let Some(picker) = self.symbol_picker() {
            picker.borrow_mut().hide_for_handover();
        }
    }

    /// Takes the symbol picker down only if `token`'s context raised it.
    pub(crate) fn hide_symbol_picker_of(&self, token: ContextToken) {
        let Some(picker) = self.symbol_picker() else {
            return;
        };
        if !picker.borrow().is_showing(token) {
            return;
        }
        self.state.borrow_mut().is_symbol_picker_open = false;
        picker.borrow_mut().hide(token);
    }

    /// Takes the Telex guide down whoever raised it, without touching the
    /// composition — the settings doorways and the key path.
    pub(crate) fn hide_telex_guide_now(&self) {
        if let Some(guide) = self.telex_guide() {
            guide.borrow_mut().hide_now();
        }
    }

    /// Takes the Telex guide down only if `token`'s context raised it.
    pub(crate) fn hide_telex_guide_of(&self, token: ContextToken) {
        if let Some(guide) = self.telex_guide() {
            guide.borrow_mut().hide(token);
        }
    }

    /// The hide a focus / context callback could not post (a window was
    /// busy inside a session), applied now, outside any session: every key
    /// entry and `run_key` call this before anything new is shown.
    pub(crate) fn drain_pending_ui_hide(&self) {
        let pending = std::mem::take(&mut self.state.borrow_mut().is_ui_hide_pending);
        if !pending {
            return;
        }
        if let Some(presenter) = self.presenter() {
            presenter.borrow_mut().hide_for_handover();
        }
        self.hide_symbol_picker_now();
        self.hide_telex_guide_now();
    }

    /// The virtual key the held toggle chord is registered on, when one is.
    fn held_toggle_virtual_key(&self) -> Option<u32> {
        let state = self.state.borrow();
        state
            .held_toggle_chord
            .and_then(|action| state.preserved_keys.virtual_key_of(action))
    }

    /// A key-down that is not the held toggle chord repeating ends the press
    /// the preserved-key guard is holding (`held_toggle_chord`).
    pub(crate) fn release_toggle_chord_on_other_key(&self, wparam: WPARAM, lparam: LPARAM) {
        if key_translation::is_repeat(lparam) {
            return;
        }
        let virtual_key = u32::from(key_translation::virtual_key(wparam));
        if self.held_toggle_virtual_key() != Some(virtual_key) {
            self.state.borrow_mut().held_toggle_chord = None;
        }
    }

    /// The release of the held toggle chord's own key, or of Ctrl or Alt,
    /// ends the press the preserved-key guard is holding. Ctrl's and Alt's
    /// releases reach `OnKeyUp` in every host measured; the chord key's own
    /// release under a held Alt may not (the down never did).
    fn release_toggle_chord_on_key_up(&self, wparam: WPARAM) {
        let virtual_key = u32::from(key_translation::virtual_key(wparam));
        let is_chord_key = self.held_toggle_virtual_key() == Some(virtual_key)
            || virtual_key == u32::from(VK_CONTROL.0)
            || virtual_key == u32::from(VK_MENU.0);
        if is_chord_key {
            self.state.borrow_mut().held_toggle_chord = None;
        }
    }

    /// A focus / context callback's one permitted move on the windows: a
    /// posted hide (W3). `owner` = only that context's list and guide;
    /// `None` = whichever is up. The Telex guide goes with the candidate
    /// window — the host is asking for every piece of input-method UI to
    /// go, and the guide is one (`TaigiInputController.hidePalettes`); the
    /// composition is not touched. When either is mid-session (the callback
    /// re-entered us), the hide is flagged for the next key instead.
    fn request_ui_hide(&self, owner: Option<ContextToken>) {
        // A hide still owed from an earlier callback is folded in: the
        // windows may be free now, and the next key may never come.
        let owed = std::mem::take(&mut self.state.borrow_mut().is_ui_hide_pending);
        let owner = if owed { None } else { owner };
        let mut posted = true;
        if let Some(presenter) = self.presenter() {
            match presenter.try_borrow() {
                Ok(presenter) => presenter.request_hide(owner),
                Err(_) => posted = false,
            }
        }
        // The picker goes with the list.
        if let Some(picker) = self.symbol_picker() {
            match picker.try_borrow() {
                Ok(picker) => picker.request_hide(owner),
                Err(_) => posted = false,
            }
        }
        if let Some(guide) = self.telex_guide() {
            match guide.try_borrow() {
                Ok(guide) => guide.request_hide(owner),
                Err(_) => posted = false,
            }
        }
        if !posted {
            self.state.borrow_mut().is_ui_hide_pending = true;
        }
    }

    /// Re-registers the preserved keys when the settings revision moved.
    /// Cheap when it did not (one comparison); COM only when it did.
    pub(crate) fn sync_preserved_keys(
        &self,
        settings: &taigi_windows_core::settings::SettingsDocument,
    ) {
        let already_current =
            self.state.borrow().preserved_keys.revision == Some(settings.revision);
        if already_current {
            return;
        }
        let (keystroke_mgr, mut preserved, client_id) = {
            let mut state = self.state.borrow_mut();
            (
                state.keystroke_mgr.take(),
                std::mem::take(&mut state.preserved_keys),
                state.client_id,
            )
        };
        if let Some(keystroke_mgr) = &keystroke_mgr {
            preserved.sync(keystroke_mgr, client_id, settings);
        }
        let mut state = self.state.borrow_mut();
        state.preserved_keys = preserved;
        state.keystroke_mgr = keystroke_mgr;
    }

    /// Every key-down, before any of the reasons `key_down` gives up (no
    /// context, read-only, a modifier that builds no snapshot): the Shift tap
    /// is a key the classifier never sees, and the press it is made of has to
    /// be recorded even in a document this input method will not compose in.
    fn observe_key_down(&self, wparam: WPARAM, lparam: LPARAM) {
        let virtual_key = key_translation::virtual_key(wparam);
        let is_repeat = key_translation::is_repeat(lparam);
        let scan_code = key_translation::scan_code(lparam);
        // Asked only where the answer can change anything: a fresh Shift
        // press. Every other key disarms regardless, and a repeat is not a
        // press at all — both skip the keyboard-state read.
        let is_other_modifier_held = virtual_key == VK_SHIFT_CODE
            && !is_repeat
            && key_translation::is_other_modifier_held_at_shift_press(scan_code);
        self.state.borrow_mut().shift_tap.observe_key_down(
            virtual_key,
            scan_code,
            is_repeat,
            is_other_modifier_held,
            now_milliseconds(),
        );
    }

    /// Whether this release would switch 中/英 — the test callback's answer,
    /// which leaves the press unspent. Answering TRUE is what asks TSF for
    /// the delivery the switch itself runs in.
    fn is_language_switch_release(&self, wparam: WPARAM, lparam: LPARAM) -> bool {
        self.state.borrow().shift_tap.is_tap_on_release(
            key_translation::virtual_key(wparam),
            key_translation::scan_code(lparam),
            now_milliseconds(),
        )
    }

    /// The delivered release. The press is spent whatever it was, so a
    /// release can only ever switch the mode once.
    fn take_language_switch_release(
        &self,
        context: Option<&ITfContext>,
        wparam: WPARAM,
        lparam: LPARAM,
    ) {
        let is_tap = self.state.borrow_mut().shift_tap.take_tap_on_release(
            key_translation::virtual_key(wparam),
            key_translation::scan_code(lparam),
            now_milliseconds(),
        );
        // The press is spent above whatever this release turns out to be — a
        // release always ends the press it belongs to.
        if !is_tap {
            return;
        }
        let Some(context) = context else {
            return;
        };
        let Some((token, identity)) = self.token_for(context) else {
            return;
        };
        self.toggle_language_mode(context, token, identity);
    }

    /// The ONE way the 中/英 mode changes. Three things say which mode is on —
    /// the classifier's gate, the TSF conversion-mode compartment and the tray
    /// letter — and a switch that moved only some of them is a mode the user
    /// and the system disagree about. They move here, together, or not at all.
    /// The mode flash is the caller's (a restore nobody asked for shows none).
    pub(crate) fn set_language_mode(&self, mode: LanguageMode) {
        self.state.borrow_mut().language_mode = mode;
        let (thread_mgr, client_id) = {
            let state = self.state.borrow();
            (state.thread_mgr.clone(), state.client_id)
        };
        if let Some(thread_mgr) = thread_mgr {
            conversion_mode::publish(&thread_mgr, client_id, mode);
        }
        self.notify_lang_bar();
    }

    pub(crate) fn notify_lang_bar(&self) {
        // Moved out, called, moved back: no AddRef under the borrow.
        let sink = self.state.borrow_mut().lang_bar_sink.take();
        if let Some(sink) = sink {
            // SAFETY: the sink TSF advised through `AdviseSink`.
            unsafe { sink.OnUpdate(TF_LBI_ICON | TF_LBI_TEXT).ok() };
            let mut state = self.state.borrow_mut();
            if state.lang_bar_sink.is_none() {
                state.lang_bar_sink = Some(sink);
            }
        }
    }
}

impl ITfTextInputProcessor_Impl for TextService_Impl {
    fn Activate(&self, ptim: Ref<ITfThreadMgr>, tid: u32) -> Result<()> {
        self.ActivateEx(ptim, tid, 0)
    }

    fn Deactivate(&self) -> Result<()> {
        guarded("ITfTextInputProcessor::Deactivate", || self.deactivate())
    }
}

impl ITfTextInputProcessorEx_Impl for TextService_Impl {
    /// A failed OR panicked activation rolls every advised sink back, so
    /// the host is never left with a half-registered service.
    fn ActivateEx(&self, ptim: Ref<ITfThreadMgr>, tid: u32, dwflags: u32) -> Result<()> {
        guarded("ITfTextInputProcessorEx::ActivateEx", || {
            let thread_mgr = ptim.ok()?;
            let outcome =
                catch_unwind(AssertUnwindSafe(|| self.activate(thread_mgr, tid, dwflags)));
            let error = match outcome {
                Ok(Ok(())) => return Ok(()),
                Ok(Err(error)) => error,
                Err(_) => {
                    log::error!("tsf.panic entry=activate");
                    Error::from_hresult(E_FAIL)
                }
            };
            log::error!("tsf.activate_failed error={error}");
            catch_unwind(AssertUnwindSafe(|| self.deactivate().ok())).ok();
            Err(error)
        })
    }
}

impl ITfThreadMgrEventSink_Impl for TextService_Impl {
    fn OnInitDocumentMgr(&self, _pdim: Ref<ITfDocumentMgr>) -> Result<()> {
        guarded("ITfThreadMgrEventSink::OnInitDocumentMgr", || Ok(()))
    }

    fn OnUninitDocumentMgr(&self, _pdim: Ref<ITfDocumentMgr>) -> Result<()> {
        guarded("ITfThreadMgrEventSink::OnUninitDocumentMgr", || Ok(()))
    }

    /// Called synchronously from `msctf!_NotifyCallbacks`: records the
    /// identity, flags the settings re-read, posts the window's hide,
    /// returns. No COM call, no I/O (rakukan `factory.rs:1304-1330`); the
    /// candidate list of the document that lost focus comes down on the
    /// message loop, its composition at the next key (handover).
    fn OnSetFocus(
        &self,
        pdimfocus: Ref<ITfDocumentMgr>,
        _pdimprevfocus: Ref<ITfDocumentMgr>,
    ) -> Result<()> {
        guarded("ITfThreadMgrEventSink::OnSetFocus", || {
            let focused = pdimfocus
                .as_ref()
                .map_or(0, |document| document.as_raw() as usize);
            let changed = {
                let mut state = self.state.borrow_mut();
                let changed = state.focused_document != focused;
                if changed {
                    state.focused_document = focused;
                    state.focus_generation += 1;
                    // A toggle chord pressed in the document that just lost
                    // focus: its key-up goes to whatever has it now.
                    state.held_toggle_chord = None;
                    if focused != 0 {
                        state.is_settings_refresh_pending = true;
                    }
                }
                changed
            };
            if changed {
                self.request_ui_hide(None);
            }
            Ok(())
        })
    }

    /// A context pushed over the focused one (a modal edit, a transitory
    /// context): whatever list was up belongs to the context underneath.
    fn OnPushContext(&self, pic: Ref<ITfContext>) -> Result<()> {
        guarded("ITfThreadMgrEventSink::OnPushContext", || {
            if let Some(context) = pic.as_ref() {
                self.token_for(context);
                let mut state = self.state.borrow_mut();
                state.focus_generation += 1;
                state.held_toggle_chord = None;
                drop(state);
                self.request_ui_hide(None);
            }
            Ok(())
        })
    }

    fn OnPopContext(&self, pic: Ref<ITfContext>) -> Result<()> {
        guarded("ITfThreadMgrEventSink::OnPopContext", || {
            if let Some(context) = pic.as_ref() {
                let identity = ContextRegistry::identity(context);
                let forgotten =
                    identity.and_then(|identity| self.state.borrow_mut().contexts.forget(identity));
                // The registry's references (context, composition) are
                // released HERE, outside the borrow; the engine ownership too.
                if let Some(entry) = forgotten {
                    log::debug!("tsf.context_popped token={:?}", entry.token);
                    self.request_ui_hide(Some(entry.token));
                    let released = match Runtime::shared().try_coordinator() {
                        Some(mut coordinator) => {
                            coordinator.release(entry.token);
                            true
                        }
                        None => false,
                    };
                    let mut state = self.state.borrow_mut();
                    state.focus_generation += 1;
                    state.held_toggle_chord = None;
                    if !released {
                        // The engine was busy: released under the next key.
                        state.deferred_releases.push(entry.token);
                    }
                    drop(state);
                    drop(entry);
                }
            }
            Ok(())
        })
    }
}

impl ITfThreadFocusSink_Impl for TextService_Impl {
    fn OnSetThreadFocus(&self) -> Result<()> {
        guarded("ITfThreadFocusSink::OnSetThreadFocus", || {
            self.request_settings_refresh();
            Ok(())
        })
    }

    /// Alt+Tab to another process: the candidate window comes down (the
    /// list with it — a hidden list must not keep answering slot keys),
    /// through the same posted hide the document-focus callback uses.
    fn OnKillThreadFocus(&self) -> Result<()> {
        guarded("ITfThreadFocusSink::OnKillThreadFocus", || {
            let mut state = self.state.borrow_mut();
            state.focus_generation += 1;
            // A Shift still held belongs to whatever has the keyboard now;
            // its release is not a tap of ours — nor is the guide chord's.
            state.shift_tap.clear();
            state.held_toggle_chord = None;
            drop(state);
            self.request_ui_hide(None);
            Ok(())
        })
    }
}

/// `OnTestKeyDown` and `OnKeyDown` answer through one classification
/// (terminals skip the former) — `session::key_down`.
impl ITfKeyEventSink_Impl for TextService_Impl {
    fn OnSetFocus(&self, fforeground: BOOL) -> Result<()> {
        guarded("ITfKeyEventSink::OnSetFocus", || {
            // Keyboard focus moved: a press recorded before the move was made
            // in another document, and its release must not tap here.
            let mut state = self.state.borrow_mut();
            state.shift_tap.clear();
            state.held_toggle_chord = None;
            drop(state);
            if fforeground.as_bool() {
                self.request_settings_refresh();
            }
            Ok(())
        })
    }

    fn OnTestKeyDown(&self, pic: Ref<ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        guarded("ITfKeyEventSink::OnTestKeyDown", || {
            self.observe_key_down(wparam, lparam);
            Ok(self.key_down(pic.as_ref(), wparam, lparam, KeyPhase::Test))
        })
    }

    /// TRUE only for the release that would switch 中/英, and it switches
    /// nothing here: a test callback answers whether the service WOULD handle
    /// the key, and the answer is what asks TSF for the delivery below.
    fn OnTestKeyUp(&self, _pic: Ref<ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        guarded("ITfKeyEventSink::OnTestKeyUp", || {
            Ok(BOOL::from(self.is_language_switch_release(wparam, lparam)))
        })
    }

    fn OnKeyDown(&self, pic: Ref<ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        guarded("ITfKeyEventSink::OnKeyDown", || {
            self.observe_key_down(wparam, lparam);
            Ok(self.key_down(pic.as_ref(), wparam, lparam, KeyPhase::Deliver))
        })
    }

    /// The Shift tap switches the mode here, and the release still goes to
    /// the host: an application tracks its own Shift state, and a release it
    /// never sees leaves that state stuck down (新酷音 answers the same
    /// FALSE, `chewing_ime.py:736`).
    fn OnKeyUp(&self, pic: Ref<ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        guarded("ITfKeyEventSink::OnKeyUp", || {
            self.release_toggle_chord_on_key_up(wparam);
            self.take_language_switch_release(pic.as_ref(), wparam, lparam);
            Ok(BOOL::from(false))
        })
    }

    fn OnPreservedKey(&self, pic: Ref<ITfContext>, rguid: *const GUID) -> Result<BOOL> {
        guarded("ITfKeyEventSink::OnPreservedKey", || {
            if rguid.is_null() {
                return Ok(BOOL::from(false));
            }
            // SAFETY: null-checked; TSF's own GUID pointer.
            let Some(action) = preserved_keys::action_for_guid(unsafe { &*rguid }) else {
                return Ok(BOOL::from(false));
            };
            // The picker runs against the context the chord was pressed in
            // (`needs_key_context`); a host that named none gets the key
            // back, and so does a read-only one — the key sink's own answer,
            // decided BEFORE the latch so a refused press leaves none behind.
            let picker_target = match (action.needs_key_context(), pic.as_ref()) {
                (true, None) => return Ok(BOOL::from(false)),
                (true, Some(context)) => match self.symbol_picker_target(context) {
                    Some((token, identity)) => Some((context, token, identity)),
                    None => return Ok(BOOL::from(false)),
                },
                (false, _) => None,
            };
            // Once per press: TSF may deliver a held chord again, and this
            // callback cannot tell a repeat from a fresh press (no lParam).
            if action.fires_once_per_press() {
                let held = self.state.borrow_mut().held_toggle_chord.replace(action);
                if held == Some(action) {
                    return Ok(BOOL::from(true));
                }
            }
            if let Some((context, token, identity)) = picker_target {
                let settings = Runtime::shared().settings.current();
                self.toggle_symbol_picker(context, token, identity, &settings);
                return Ok(BOOL::from(true));
            }
            let identity = pic
                .as_ref()
                .and_then(|context| self.token_for(context))
                .map_or(0, |(_, identity)| identity);
            self.perform_global(action, identity);
            Ok(BOOL::from(true))
        })
    }
}

impl ITfCompositionSink_Impl for TextService_Impl {
    /// The host ended the composition (a click elsewhere, focus loss).
    fn OnCompositionTerminated(
        &self,
        _ecwrite: u32,
        pcomposition: Ref<ITfComposition>,
    ) -> Result<()> {
        guarded("ITfCompositionSink::OnCompositionTerminated", || {
            if let Some(composition) = pcomposition.as_ref() {
                self.composition_terminated(composition);
            }
            Ok(())
        })
    }
}

impl ITfDisplayAttributeProvider_Impl for TextService_Impl {
    fn EnumDisplayAttributeInfo(&self) -> Result<IEnumTfDisplayAttributeInfo> {
        guarded(
            "ITfDisplayAttributeProvider::EnumDisplayAttributeInfo",
            || Ok(DisplayAttributeEnumerator::new().into()),
        )
    }

    fn GetDisplayAttributeInfo(&self, guid: *const GUID) -> Result<ITfDisplayAttributeInfo> {
        guarded(
            "ITfDisplayAttributeProvider::GetDisplayAttributeInfo",
            || {
                if guid.is_null() {
                    return Err(Error::from_hresult(E_INVALIDARG));
                }
                // SAFETY: null-checked; TSF's own GUID pointer.
                display_attribute::info_for(unsafe { &*guid })
            },
        )
    }
}

impl ITfLangBarItem_Impl for TextService_Impl {
    fn GetInfo(&self, pinfo: *mut TF_LANGBARITEMINFO) -> Result<()> {
        guarded("ITfLangBarItem::GetInfo", || {
            if pinfo.is_null() {
                return Err(Error::from_hresult(E_INVALIDARG));
            }
            // SAFETY: TSF passes a valid out-pointer to its own struct.
            unsafe { *pinfo = lang_bar::item_info() };
            Ok(())
        })
    }

    fn GetStatus(&self) -> Result<u32> {
        guarded("ITfLangBarItem::GetStatus", || Ok(0))
    }

    fn Show(&self, _fshow: BOOL) -> Result<()> {
        guarded("ITfLangBarItem::Show", || Ok(()))
    }

    fn GetTooltipString(&self) -> Result<BSTR> {
        guarded("ITfLangBarItem::GetTooltipString", || {
            Ok(BSTR::from(product_name::localized().as_str()))
        })
    }
}

impl ITfLangBarItemButton_Impl for TextService_Impl {
    /// Both mouse buttons raise the menu — this button has no mode to
    /// toggle, and the Mac opens its menu on a plain click
    /// (`TaigiInputController.swift:264-329`). The rows are drawn here
    /// rather than through `InitMenu`: the taskbar input indicator never
    /// drives the TSF menu (`lang_bar`'s module header).
    fn OnClick(&self, _click: TfLBIClick, pt: &POINT, _prcarea: *const RECT) -> Result<()> {
        guarded("ITfLangBarItemButton::OnClick", || {
            let runtime = Runtime::shared();
            let rows = lang_bar::menu_rows(&runtime.strings(), &runtime.settings.current());
            if let Some(id) = lang_bar::show_popup(&rows, *pt) {
                match id {
                    MENU_OPEN_SETTINGS => {
                        // The guide and the picker come down first, whoever
                        // raised them: this path never reaches the session,
                        // and the settings window taking focus is not
                        // guaranteed to end the context that owns the card
                        // (`ShortcutHotkeys.openSettings`).
                        self.hide_telex_guide_now();
                        self.hide_symbol_picker_now();
                        settings_launcher::open_settings();
                    }
                    MENU_CHECK_FOR_UPDATES => {
                        // Also the settings window (on 一般): same doorway.
                        self.hide_telex_guide_now();
                        self.hide_symbol_picker_now();
                        settings_launcher::check_for_updates();
                    }
                    other => log::warn!("tsf.menu_unknown_id id={other}"),
                }
                self.notify_lang_bar();
            }
            Ok(())
        })
    }

    /// The declarative menu of a `TF_LBI_STYLE_BTN_MENU` button, which this
    /// item is not — `OnClick` draws its own. Answered as an empty menu
    /// rather than `E_NOTIMPL`: a host that probes it (the legacy desktop
    /// language bar) reads a failure as a broken item, and mozc leaves the
    /// same no-op `S_OK` for a non-menu button
    /// (`tip_lang_bar_menu.cc:619-624`).
    fn InitMenu(&self, _pmenu: Ref<ITfMenu>) -> Result<()> {
        guarded("ITfLangBarItemButton::InitMenu", || Ok(()))
    }

    /// The other half of the declarative menu; nothing selects from an
    /// empty one, and an unexpected id is not this item's error to raise.
    fn OnMenuSelect(&self, _wid: u32) -> Result<()> {
        guarded("ITfLangBarItemButton::OnMenuSelect", || Ok(()))
    }

    /// A caller-owned icon: TSF destroys what it is given.
    fn GetIcon(&self) -> Result<HICON> {
        guarded("ITfLangBarItemButton::GetIcon", lang_bar::owned_icon)
    }

    /// Re-read after every switch (`notify_lang_bar` pushes the update), so
    /// the taskbar letter names the mode the next key will be typed in.
    fn GetText(&self) -> Result<BSTR> {
        guarded("ITfLangBarItemButton::GetText", || {
            let mode = self.state.borrow().language_mode;
            Ok(BSTR::from(lang_bar::tray_text(mode)))
        })
    }
}

impl ITfSource_Impl for TextService_Impl {
    /// TSF advises exactly one `ITfLangBarItemSink` on a lang-bar item; a
    /// second subscriber is refused rather than silently replacing the
    /// first, so every advise has a symmetric unadvise.
    fn AdviseSink(&self, riid: *const GUID, punk: Ref<IUnknown>) -> Result<u32> {
        guarded("ITfSource::AdviseSink", || {
            if riid.is_null() {
                return Err(Error::from_hresult(E_INVALIDARG));
            }
            // SAFETY: null-checked; TSF's own IID pointer.
            if unsafe { *riid } != ITfLangBarItemSink::IID {
                return Err(Error::from_hresult(CONNECT_E_NOCONNECTION));
            }
            // The QueryInterface happens before the borrow.
            let sink: ITfLangBarItemSink = punk.ok()?.cast()?;
            let mut state = self.state.borrow_mut();
            if state.lang_bar_sink.is_some() {
                drop(state);
                drop(sink);
                return Err(Error::from_hresult(CONNECT_E_ADVISELIMIT));
            }
            state.lang_bar_sink = Some(sink);
            Ok(LANG_BAR_SINK_COOKIE)
        })
    }

    fn UnadviseSink(&self, dwcookie: u32) -> Result<()> {
        guarded("ITfSource::UnadviseSink", || {
            if dwcookie != LANG_BAR_SINK_COOKIE {
                return Err(Error::from_hresult(CONNECT_E_NOCONNECTION));
            }
            // Taken under the borrow, released after it.
            let sink = self.state.borrow_mut().lang_bar_sink.take();
            drop(sink);
            Ok(())
        })
    }
}

impl Drop for TextService {
    /// Reached through COM's `Release`: still a boundary.
    fn drop(&mut self) {
        let is_active = catch_unwind(AssertUnwindSafe(|| {
            self.state.get_mut().thread_mgr.is_some()
        }))
        .unwrap_or(false);
        if is_active {
            log::warn!("tsf.dropped_while_active");
        }
    }
}
