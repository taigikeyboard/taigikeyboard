//! The text service: the COM object TSF activates once per thread manager.
//! PR5a scope — lifecycle, sinks, tray button + menu, settings reload,
//! context identity. Every key goes back to the host (`FALSE`); composing
//! arrives with PR5b.
//!
//! Threading (roadmap W3): TSF is STA — every method here runs on the
//! host's UI thread, so the state is a `RefCell`. RULE: no COM call of any
//! kind — not even an `AddRef` (`clone`) or a `Release` (`drop`) — while a
//! borrow is held. Values are MOVED in and out of the state under the
//! borrow; the reference counting happens outside it. Callbacks raised
//! from inside `msctf!_NotifyCallbacks` (`OnSetFocus`) only record and
//! return (rakukan `factory.rs:1304-1330`); the settings re-read they
//! request happens on the next key-sink call.

// 中文: 文字服務 COM 物件 — 生命週期、sink、系統匣按鈕與選單、設定重讀、context 身分;借用期間絕不做 COM 呼叫。

use crate::com_guard::guarded;
use crate::contexts::ContextRegistry;
use crate::lang_bar::{self, LANG_BAR_SINK_COOKIE, MENU_CHECK_FOR_UPDATES, MENU_OPEN_SETTINGS};
use crate::registration::SERVICE_DESCRIPTION;
use crate::runtime::Runtime;
use crate::settings_launcher;
use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use taigi_windows_core::composing::ContextToken;
use windows::core::{Error, IUnknown, Interface, Ref, Result, BOOL, BSTR, GUID};
use windows::Win32::Foundation::{E_FAIL, E_INVALIDARG, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::System::Ole::{CONNECT_E_ADVISELIMIT, CONNECT_E_NOCONNECTION};
use windows::Win32::UI::TextServices::{
    ITfContext, ITfDocumentMgr, ITfKeyEventSink, ITfKeyEventSink_Impl, ITfKeystrokeMgr,
    ITfLangBarItem, ITfLangBarItemButton, ITfLangBarItemButton_Impl, ITfLangBarItemMgr,
    ITfLangBarItemSink, ITfLangBarItem_Impl, ITfMenu, ITfSource, ITfSource_Impl,
    ITfTextInputProcessorEx, ITfTextInputProcessorEx_Impl, ITfTextInputProcessor_Impl,
    ITfThreadFocusSink, ITfThreadFocusSink_Impl, ITfThreadMgr, ITfThreadMgrEventSink,
    ITfThreadMgrEventSink_Impl, TfLBIClick, TF_INVALID_COOKIE, TF_LANGBARITEMINFO, TF_LBI_ICON,
    TF_LBI_TEXT,
};
use windows::Win32::UI::WindowsAndMessaging::HICON;
use windows_core::{implement, IUnknownImpl};

/// Everything `Activate` set up and `Deactivate` takes down.
#[derive(Default)]
struct ServiceState {
    thread_mgr: Option<ITfThreadMgr>,
    client_id: u32,
    activate_flags: u32,
    thread_mgr_sink_cookie: u32,
    thread_focus_sink_cookie: u32,
    is_key_sink_advised: bool,
    is_lang_bar_added: bool,
    lang_bar_sink: Option<ITfLangBarItemSink>,
    contexts: ContextRegistry,
    /// Tokens are allocated here until PR5b hands allocation to the
    /// composing coordinator (one counter, never a COM address).
    next_context_token: u64,
    /// The `IUnknown` identity of the focused document manager — recorded,
    /// never dereferenced; a late focus notification is a hint (contract 7).
    focused_document: usize,
    /// Set by the focus callbacks, consumed by the next key-sink call: the
    /// settings file is re-read there, never inside `_NotifyCallbacks`.
    is_settings_refresh_pending: bool,
}

#[implement(
    ITfTextInputProcessorEx,
    ITfThreadMgrEventSink,
    ITfThreadFocusSink,
    ITfKeyEventSink,
    ITfLangBarItemButton,
    ITfSource
)]
pub struct TextService {
    state: RefCell<ServiceState>,
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
        }
        // Probes paths and reads the settings file once per process; the
        // engine and the stores stay untouched until a key is CONSUMED (PR5b).
        let runtime = Runtime::shared();
        log::info!(
            "tsf.activate client_id={client_id} flags={flags:#x} learning={}",
            runtime.capability.learning
        );

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
        self.state.borrow_mut().is_key_sink_advised = true;

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
        Ok(())
    }

    /// Reverse order of `activate`; every step attempted. Every COM
    /// reference the state held is moved out under the borrow and released
    /// after it.
    fn deactivate(&self) -> Result<()> {
        let (
            thread_mgr,
            client_id,
            thread_mgr_cookie,
            thread_focus_cookie,
            key_sink,
            lang_bar,
            contexts,
            sink,
        ) = {
            let mut state = self.state.borrow_mut();
            (
                state.thread_mgr.take(),
                state.client_id,
                std::mem::replace(&mut state.thread_mgr_sink_cookie, TF_INVALID_COOKIE),
                std::mem::replace(&mut state.thread_focus_sink_cookie, TF_INVALID_COOKIE),
                std::mem::take(&mut state.is_key_sink_advised),
                std::mem::take(&mut state.is_lang_bar_added),
                std::mem::take(&mut state.contexts),
                state.lang_bar_sink.take(),
            )
        };
        let released = contexts.into_tokens();
        log::info!(
            "tsf.deactivate client_id={client_id} contexts_released={}",
            released.len()
        );
        drop(sink);
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
    /// under it. PR5b threads the coordinator's allocator through here.
    fn token_for(&self, context: &ITfContext) -> Option<ContextToken> {
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
                let token = ContextToken(*next_context_token);
                *next_context_token += 1;
                token
            })
        };
        // A reference the registry did not keep is released outside the borrow.
        drop(duplicate);
        Some(token)
    }

    /// Settings may have changed while another window had focus: the focus
    /// callbacks only flag it, and the next key-sink call re-reads (one
    /// `stat`, W10). Never inside `_NotifyCallbacks`.
    fn refresh_settings_if_pending(&self) {
        let pending = std::mem::take(&mut self.state.borrow_mut().is_settings_refresh_pending);
        if pending {
            Runtime::shared().settings.current();
        }
    }

    fn request_settings_refresh(&self) {
        self.state.borrow_mut().is_settings_refresh_pending = true;
    }

    fn notify_lang_bar(&self) {
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
    /// identity, flags the settings re-read, returns. No COM call, no I/O
    /// (rakukan `factory.rs:1304-1330`). The queue is the next key-sink
    /// call until PR6 gives the DLL a window to `PostMessage` to.
    fn OnSetFocus(
        &self,
        pdimfocus: Ref<ITfDocumentMgr>,
        _pdimprevfocus: Ref<ITfDocumentMgr>,
    ) -> Result<()> {
        guarded("ITfThreadMgrEventSink::OnSetFocus", || {
            let focused = pdimfocus
                .as_ref()
                .map_or(0, |document| document.as_raw() as usize);
            let mut state = self.state.borrow_mut();
            if state.focused_document != focused {
                state.focused_document = focused;
                if focused != 0 {
                    state.is_settings_refresh_pending = true;
                }
            }
            Ok(())
        })
    }

    fn OnPushContext(&self, pic: Ref<ITfContext>) -> Result<()> {
        guarded("ITfThreadMgrEventSink::OnPushContext", || {
            if let Some(context) = pic.as_ref() {
                self.token_for(context);
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
                // The registry's reference to the context is released HERE,
                // outside the borrow.
                if let Some((held, token)) = forgotten {
                    drop(held);
                    log::debug!("tsf.context_popped token={token:?}");
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

    /// Alt+Tab to another process: the candidate window (PR6) hides here.
    fn OnKillThreadFocus(&self) -> Result<()> {
        guarded("ITfThreadFocusSink::OnKillThreadFocus", || Ok(()))
    }
}

/// PR5a: a smoke TIP that composes nothing — every key goes back to the
/// host. `OnTestKeyDown` and `OnKeyDown` must always agree (terminals skip
/// the former); both answer through one function so they cannot drift.
impl ITfKeyEventSink_Impl for TextService_Impl {
    fn OnSetFocus(&self, fforeground: BOOL) -> Result<()> {
        guarded("ITfKeyEventSink::OnSetFocus", || {
            if fforeground.as_bool() {
                self.request_settings_refresh();
            }
            Ok(())
        })
    }

    fn OnTestKeyDown(
        &self,
        pic: Ref<ITfContext>,
        _wparam: WPARAM,
        _lparam: LPARAM,
    ) -> Result<BOOL> {
        guarded("ITfKeyEventSink::OnTestKeyDown", || {
            Ok(self.wants_key(pic.as_ref()))
        })
    }

    fn OnTestKeyUp(&self, _pic: Ref<ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        guarded("ITfKeyEventSink::OnTestKeyUp", || Ok(BOOL::from(false)))
    }

    fn OnKeyDown(&self, pic: Ref<ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        guarded("ITfKeyEventSink::OnKeyDown", || {
            Ok(self.wants_key(pic.as_ref()))
        })
    }

    fn OnKeyUp(&self, _pic: Ref<ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        guarded("ITfKeyEventSink::OnKeyUp", || Ok(BOOL::from(false)))
    }

    fn OnPreservedKey(&self, _pic: Ref<ITfContext>, _rguid: *const GUID) -> Result<BOOL> {
        guarded("ITfKeyEventSink::OnPreservedKey", || Ok(BOOL::from(false)))
    }
}

impl TextService_Impl {
    /// The one answer both key-down entry points give. The context's
    /// identity is registered so PR5b's classifier finds a token waiting;
    /// nothing heavier happens here — the engine and the stores come up
    /// only for a key the classifier CONSUMES (roadmap W3), never for one
    /// merely observed.
    fn wants_key(&self, context: Option<&ITfContext>) -> BOOL {
        self.refresh_settings_if_pending();
        if let Some(context) = context {
            let token = self.token_for(context);
            log::trace!("tsf.key context_token={token:?}");
        }
        BOOL::from(false)
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
            Ok(BSTR::from(SERVICE_DESCRIPTION))
        })
    }
}

impl ITfLangBarItemButton_Impl for TextService_Impl {
    /// A menu-style button: TSF asks `InitMenu` for the rows on click.
    fn OnClick(&self, _click: TfLBIClick, _pt: &POINT, _prcarea: *const RECT) -> Result<()> {
        guarded("ITfLangBarItemButton::OnClick", || Ok(()))
    }

    fn InitMenu(&self, pmenu: Ref<ITfMenu>) -> Result<()> {
        guarded("ITfLangBarItemButton::InitMenu", || {
            let menu = pmenu.ok()?;
            let runtime = Runtime::shared();
            let rows = lang_bar::menu_rows(&runtime.strings(), &runtime.settings.current());
            lang_bar::populate(menu, &rows)
        })
    }

    fn OnMenuSelect(&self, wid: u32) -> Result<()> {
        guarded("ITfLangBarItemButton::OnMenuSelect", || {
            match wid {
                MENU_OPEN_SETTINGS => settings_launcher::open_settings(),
                MENU_CHECK_FOR_UPDATES => settings_launcher::check_for_updates(),
                other => log::warn!("tsf.menu_unknown_id id={other}"),
            }
            self.notify_lang_bar();
            Ok(())
        })
    }

    /// A caller-owned icon: TSF destroys what it is given.
    fn GetIcon(&self) -> Result<HICON> {
        guarded("ITfLangBarItemButton::GetIcon", lang_bar::owned_icon)
    }

    fn GetText(&self) -> Result<BSTR> {
        guarded("ITfLangBarItemButton::GetText", || {
            Ok(BSTR::from(lang_bar::TRAY_TEXT))
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
