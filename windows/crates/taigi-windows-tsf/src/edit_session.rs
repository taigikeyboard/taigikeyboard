//! A synchronous read-write edit session from the key sink, the only place
//! the document is touched (roadmap W3; khiin `tip/edit_session.rs:11-34`).
//! The closure runs INSIDE `DoEditSession`, so a session the host refuses
//! (`TF_E_SYNCHRONOUS`, read-only, teardown) never ran it: the engine and
//! the document stay untouched and the key goes back to the host.

// 中文: 從 key sink 開的同步讀寫 edit session;閉包在 DoEditSession 內執行,失敗即引擎與文件皆未動。

use crate::com_guard::guarded;
use std::cell::RefCell;
use windows::core::{Error, Result};
use windows::Win32::Foundation::S_OK;
use windows::Win32::UI::TextServices::{
    ITfContext, ITfEditSession, ITfEditSession_Impl, TF_CONTEXT_EDIT_CONTEXT_FLAGS,
    TF_ES_READWRITE, TF_ES_SYNC, TF_E_SYNCHRONOUS,
};
use windows_core::{implement, AsImpl};

/// The edit cookie one session runs under.
pub type EditCookie = u32;

type Body<'a> = Box<dyn FnOnce(EditCookie) -> Result<()> + 'a>;

#[implement(ITfEditSession)]
struct EditSession {
    body: RefCell<Option<Body<'static>>>,
}

impl ITfEditSession_Impl for EditSession_Impl {
    fn DoEditSession(&self, ec: EditCookie) -> Result<()> {
        guarded("ITfEditSession::DoEditSession", || {
            match self.body.borrow_mut().take() {
                Some(body) => body(ec),
                None => Ok(()),
            }
        })
    }
}

/// Runs `body` in a synchronous session on `context` and answers what it
/// answered. `Err` when the host would not grant the session (both the
/// call's `HRESULT` and the session's own are checked, roadmap W3) — the
/// caller must then treat the key as not handled.
pub fn run_sync<T>(
    context: &ITfContext,
    client_id: u32,
    flags: TF_CONTEXT_EDIT_CONTEXT_FLAGS,
    body: impl FnOnce(EditCookie) -> Result<T>,
) -> Result<T> {
    let mut answer: Option<T> = None;
    let capture: Body<'_> = Box::new(|ec| {
        answer = Some(body(ec)?);
        Ok(())
    });
    // SAFETY: the lifetime is erased so the COM object can own the closure.
    // The session is requested with TF_ES_SYNC: TSF either runs
    // `DoEditSession` before `RequestEditSession` returns or refuses with
    // TF_E_SYNCHRONOUS — it is never queued. After the call the closure is
    // taken out of the object below, so a host that kept the session alive
    // can never run a body whose borrows have ended.
    let body_static: Body<'static> =
        unsafe { std::mem::transmute::<Body<'_>, Body<'static>>(capture) };
    let session_object = EditSession {
        body: RefCell::new(Some(body_static)),
    };
    let session: ITfEditSession = session_object.into();
    // SAFETY: `context` is the live ITfContext the key sink was handed; the
    // session interface outlives the call.
    let outcome = unsafe { context.RequestEditSession(client_id, &session, flags | TF_ES_SYNC) };
    // Whatever happened, the closure must not survive this frame.
    // SAFETY: `session` was created from `EditSession` just above.
    let implementation: &EditSession = unsafe { session.as_impl() };
    if let Some(body) = implementation.body.borrow_mut().take() {
        drop(body);
    }
    // Only `S_OK` with an answer counts: `TF_S_ASYNC` is a SUCCESS code that
    // means "queued", and the closure has just been withdrawn — under this
    // contract it is a refusal, reported as the synchronous-refusal error.
    match (outcome, answer) {
        (Ok(session_result), Some(answer)) if session_result == S_OK => Ok(answer),
        (Ok(session_result), _) if session_result.is_err() => {
            Err(Error::from_hresult(session_result))
        }
        (Ok(_), _) => Err(Error::from_hresult(TF_E_SYNCHRONOUS)),
        (Err(error), _) => Err(error),
    }
}

/// A read-write session — every key that mutates the composition.
pub fn read_write<T>(
    context: &ITfContext,
    client_id: u32,
    body: impl FnOnce(EditCookie) -> Result<T>,
) -> Result<T> {
    run_sync(context, client_id, TF_ES_READWRITE, body)
}
