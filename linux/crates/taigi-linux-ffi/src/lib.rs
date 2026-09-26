//! The C ABI the Fcitx5 addon calls (roadmap L1, revised 2026-09-23).
//! `include/taigikeyboard.h` is the contract; this file is its
//! implementation, one exported function per declaration, in the same
//! order. The shape follows `references/ChiaKey` `ChiaKeyCore`: opaque
//! handles, a key in, an opaque reply out that the caller reads through
//! accessors and frees — no struct crosses the boundary, so the header
//! never has to agree with Rust on layout.
//!
//! Every entry point runs under `catch_unwind` (`docs/engine/ffi-safety.md`
//! §2): a panic answers the null / false / zero the header documents and is
//! logged; it never unwinds into C++.

use std::ffi::{c_char, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::Arc;
use taigi_desktop_core::composing::ContextToken;
use taigi_desktop_core::keys::CandidateNavigation;
use taigi_linux_core::{chrome, session, Emit, EngineState, LookupTableContent, MenuItem, Runtime};
use taigi_linux_platform::{open_settings, RawKeyEvent};

/// One process-wide runtime (settings, the user-data directory, lexicon,
/// coordinator).
pub struct TaigiRuntime {
    inner: Arc<Runtime>,
}

/// One input context's engine: its token and its between-keys state.
pub struct TaigiEngine {
    runtime: Arc<Runtime>,
    token: ContextToken,
    state: EngineState,
}

/// What one call asked of the shell. Strings are owned here and handed out
/// as pointers valid until `taigi_reply_free`.
pub struct TaigiReply {
    handled: bool,
    emits: Vec<ReplyEmit>,
}

struct ReplyEmit {
    kind: u32,
    text: CString,
    caret: u32,
    delete_offset: i32,
    delete_count: u32,
    table: Option<ReplyTable>,
}

struct ReplyTable {
    candidates: Vec<CString>,
    labels: Vec<CString>,
    cursor: u32,
    cursor_visible: bool,
    page_size: u32,
    vertical: bool,
}

/// The panel menu rows, resolved in the display language of the moment.
pub struct TaigiMenu {
    items: Vec<MenuEntry>,
}

struct MenuEntry {
    separator: bool,
    id: CString,
    title: CString,
    detail: CString,
}

/// `TAIGI_EMIT_*` in the header — the same numbers.
pub const EMIT_PREEDIT: u32 = 1;
pub const EMIT_CLEAR_PREEDIT: u32 = 2;
pub const EMIT_COMMIT: u32 = 3;
pub const EMIT_DELETE_SURROUNDING: u32 = 4;
pub const EMIT_LOOKUP_TABLE: u32 = 5;
pub const EMIT_HIDE_LOOKUP_TABLE: u32 = 6;
pub const EMIT_MODE_CHANGED: u32 = 7;
pub const EMIT_ANNOUNCE_MODE: u32 = 8;

/// `TAIGI_NAVIGATE_*` in the header.
pub const NAVIGATE_PREVIOUS: u32 = 0;
pub const NAVIGATE_NEXT: u32 = 1;
pub const NAVIGATE_PAGE_UP: u32 = 2;
pub const NAVIGATE_PAGE_DOWN: u32 = 3;

fn c_string(text: &str) -> CString {
    // A NUL inside a string the engine produced cannot happen (it comes from
    // UTF-8 dictionary data and typed keys); if it ever did, the text is cut
    // there rather than the call failing.
    CString::new(text).unwrap_or_else(|error| {
        let position = error.nul_position();
        CString::new(&error.into_vec()[..position]).expect("prefix has no NUL")
    })
}

fn reply_emit(emit: Emit) -> ReplyEmit {
    let mut reply = ReplyEmit {
        kind: 0,
        text: CString::default(),
        caret: 0,
        delete_offset: 0,
        delete_count: 0,
        table: None,
    };
    match emit {
        Emit::Preedit { text, caret } => {
            reply.kind = EMIT_PREEDIT;
            reply.text = c_string(&text);
            reply.caret = caret;
        }
        Emit::ClearPreedit => reply.kind = EMIT_CLEAR_PREEDIT,
        Emit::Commit(text) => {
            reply.kind = EMIT_COMMIT;
            reply.text = c_string(&text);
        }
        Emit::DeleteSurrounding { offset, count } => {
            reply.kind = EMIT_DELETE_SURROUNDING;
            reply.delete_offset = offset;
            reply.delete_count = count;
        }
        Emit::LookupTable(content) => {
            reply.kind = EMIT_LOOKUP_TABLE;
            reply.table = Some(reply_table(content));
        }
        Emit::HideLookupTable => reply.kind = EMIT_HIDE_LOOKUP_TABLE,
        Emit::ModeChanged => reply.kind = EMIT_MODE_CHANGED,
        Emit::AnnounceMode => reply.kind = EMIT_ANNOUNCE_MODE,
    }
    reply
}

fn reply_table(content: LookupTableContent) -> ReplyTable {
    ReplyTable {
        candidates: content.candidates.iter().map(|c| c_string(c)).collect(),
        labels: content.labels.iter().map(|l| c_string(l)).collect(),
        cursor: content.cursor,
        cursor_visible: content.cursor_visible,
        page_size: content.page_size,
        vertical: content.vertical,
    }
}

fn reply_from(handled: bool, emits: Vec<Emit>) -> *mut TaigiReply {
    Box::into_raw(Box::new(TaigiReply {
        handled,
        emits: emits.into_iter().map(reply_emit).collect(),
    }))
}

/// Runs `work` under the panic boundary; `fallback` on a panic.
fn guarded<T>(what: &str, fallback: T, work: impl FnOnce() -> T) -> T {
    match catch_unwind(AssertUnwindSafe(work)) {
        Ok(answer) => answer,
        Err(_) => {
            log::error!("ffi.panic function={what}");
            fallback
        }
    }
}

// ---------------------------------------------------------------------------
// Runtime
// ---------------------------------------------------------------------------

/// Answers the crate version; the first call the addon makes.
#[no_mangle]
pub extern "C" fn taigi_version() -> *const c_char {
    const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");
    VERSION.as_ptr().cast()
}

#[no_mangle]
pub extern "C" fn taigi_runtime_new() -> *mut TaigiRuntime {
    guarded("taigi_runtime_new", ptr::null_mut(), || {
        taigi_linux_platform::install_debug_logger();
        log::info!("ffi.runtime_new version={}", env!("CARGO_PKG_VERSION"));
        Box::into_raw(Box::new(TaigiRuntime {
            inner: Arc::new(Runtime::probe()),
        }))
    })
}

/// # Safety
/// `runtime` came from `taigi_runtime_new` and is freed once; every engine
/// made from it must already be freed.
#[no_mangle]
pub unsafe extern "C" fn taigi_runtime_free(runtime: *mut TaigiRuntime) {
    if runtime.is_null() {
        return;
    }
    // SAFETY: the header's contract — a pointer this crate boxed, freed once.
    drop(unsafe { Box::from_raw(runtime) });
}

/// # Safety
/// `runtime` is a live runtime. The string is freed with `taigi_string_free`.
#[no_mangle]
pub unsafe extern "C" fn taigi_runtime_mode_label(runtime: *const TaigiRuntime) -> *mut c_char {
    if runtime.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: non-null and, by contract, a live runtime.
    let runtime = unsafe { &*runtime };
    guarded("taigi_runtime_mode_label", ptr::null_mut(), || {
        c_string(&chrome::mode_label(&runtime.inner)).into_raw()
    })
}

/// # Safety
/// `runtime` is a live runtime. The string is freed with `taigi_string_free`.
#[no_mangle]
pub unsafe extern "C" fn taigi_runtime_mode_symbol(runtime: *const TaigiRuntime) -> *mut c_char {
    if runtime.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: non-null and, by contract, a live runtime.
    let runtime = unsafe { &*runtime };
    guarded("taigi_runtime_mode_symbol", ptr::null_mut(), || {
        c_string(chrome::mode_symbol(&runtime.inner)).into_raw()
    })
}

/// Opens the settings window where the user left it — the framework's own
/// configure button (Fcitx5 `setSubConfig`), beside the Settings menu row.
#[no_mangle]
pub extern "C" fn taigi_open_settings() -> bool {
    guarded("taigi_open_settings", false, || open_settings(None))
}

/// # Safety
/// `text` came from this crate (`taigi_runtime_mode_label` / `_mode_symbol`) and is freed once.
#[no_mangle]
pub unsafe extern "C" fn taigi_string_free(text: *mut c_char) {
    if text.is_null() {
        return;
    }
    // SAFETY: the header's contract — a `CString` this crate handed out.
    drop(unsafe { CString::from_raw(text) });
}

/// # Safety
/// `runtime` is a live runtime. The menu is freed with `taigi_menu_free`.
#[no_mangle]
pub unsafe extern "C" fn taigi_runtime_menu(runtime: *const TaigiRuntime) -> *mut TaigiMenu {
    if runtime.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: non-null and, by contract, a live runtime.
    let runtime = unsafe { &*runtime };
    guarded("taigi_runtime_menu", ptr::null_mut(), || {
        let items = chrome::menu_items(&runtime.inner)
            .into_iter()
            .map(|item| match item {
                MenuItem::Separator => MenuEntry {
                    separator: true,
                    id: CString::default(),
                    title: CString::default(),
                    detail: CString::default(),
                },
                MenuItem::Action { id, title, detail } => MenuEntry {
                    separator: false,
                    id: c_string(id),
                    title: c_string(&title),
                    detail: c_string(detail.as_deref().unwrap_or("")),
                },
            })
            .collect();
        Box::into_raw(Box::new(TaigiMenu { items }))
    })
}

/// # Safety
/// `menu` is a live menu or null.
unsafe fn menu_entry<'a>(menu: *const TaigiMenu, index: usize) -> Option<&'a MenuEntry> {
    if menu.is_null() {
        return None;
    }
    // SAFETY: non-null and, by contract, a live menu the caller owns.
    unsafe { &*menu }.items.get(index)
}

/// # Safety
/// `menu` is a live menu or null.
#[no_mangle]
pub unsafe extern "C" fn taigi_menu_count(menu: *const TaigiMenu) -> usize {
    if menu.is_null() {
        return 0;
    }
    // SAFETY: non-null and, by contract, a live menu the caller owns.
    unsafe { &*menu }.items.len()
}

/// # Safety
/// `menu` is a live menu or null.
#[no_mangle]
pub unsafe extern "C" fn taigi_menu_is_separator(menu: *const TaigiMenu, index: usize) -> bool {
    // SAFETY: forwarded contract.
    unsafe { menu_entry(menu, index) }.is_some_and(|entry| entry.separator)
}

/// # Safety
/// `menu` is a live menu or null.
#[no_mangle]
pub unsafe extern "C" fn taigi_menu_id(menu: *const TaigiMenu, index: usize) -> *const c_char {
    static EMPTY: &CStr = c"";
    // SAFETY: forwarded contract.
    unsafe { menu_entry(menu, index) }.map_or(EMPTY.as_ptr(), |entry| entry.id.as_ptr())
}

/// # Safety
/// `menu` is a live menu or null.
#[no_mangle]
pub unsafe extern "C" fn taigi_menu_title(menu: *const TaigiMenu, index: usize) -> *const c_char {
    static EMPTY: &CStr = c"";
    // SAFETY: forwarded contract.
    unsafe { menu_entry(menu, index) }.map_or(EMPTY.as_ptr(), |entry| entry.title.as_ptr())
}

/// # Safety
/// `menu` is a live menu or null.
#[no_mangle]
pub unsafe extern "C" fn taigi_menu_detail(menu: *const TaigiMenu, index: usize) -> *const c_char {
    static EMPTY: &CStr = c"";
    // SAFETY: forwarded contract.
    unsafe { menu_entry(menu, index) }.map_or(EMPTY.as_ptr(), |entry| entry.detail.as_ptr())
}

/// # Safety
/// `menu` came from `taigi_runtime_menu` and is freed once (null is ignored).
#[no_mangle]
pub unsafe extern "C" fn taigi_menu_free(menu: *mut TaigiMenu) {
    if menu.is_null() {
        return;
    }
    // SAFETY: the header's contract — a pointer this crate boxed, freed once.
    drop(unsafe { Box::from_raw(menu) });
}

// ---------------------------------------------------------------------------
// Engine (one per input context)
// ---------------------------------------------------------------------------

/// # Safety
/// `runtime` came from `taigi_runtime_new` and outlives the engine.
#[no_mangle]
pub unsafe extern "C" fn taigi_engine_new(runtime: *const TaigiRuntime) -> *mut TaigiEngine {
    if runtime.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: non-null and, by contract, a live runtime.
    let runtime = unsafe { &*runtime };
    guarded("taigi_engine_new", ptr::null_mut(), || {
        let inner = Arc::clone(&runtime.inner);
        let token = inner.allocate_token();
        log::debug!("ffi.engine_new token={token:?}");
        Box::into_raw(Box::new(TaigiEngine {
            runtime: inner,
            token,
            state: EngineState::default(),
        }))
    })
}

/// # Safety
/// `engine` came from `taigi_engine_new` and is freed once. The session is
/// ended first (the framework has already dropped or committed the preedit).
#[no_mangle]
pub unsafe extern "C" fn taigi_engine_free(engine: *mut TaigiEngine) {
    if engine.is_null() {
        return;
    }
    // SAFETY: the header's contract — a pointer this crate boxed, freed once.
    let mut engine = unsafe { Box::from_raw(engine) };
    guarded("taigi_engine_free", (), || {
        let _ = session::end_session(&engine.runtime, engine.token, &mut engine.state);
    });
    drop(engine);
}

/// # Safety
/// `engine` is a live engine.
#[no_mangle]
pub unsafe extern "C" fn taigi_engine_set_capabilities(engine: *mut TaigiEngine, caps: u32) {
    if engine.is_null() {
        return;
    }
    // SAFETY: non-null and, by contract, a live engine the caller owns.
    let engine = unsafe { &mut *engine };
    engine.state.capabilities = caps;
}

/// # Safety
/// `engine` is a live engine.
#[no_mangle]
pub unsafe extern "C" fn taigi_engine_set_password_field(
    engine: *mut TaigiEngine,
    is_password: bool,
) {
    if engine.is_null() {
        return;
    }
    // SAFETY: non-null and, by contract, a live engine the caller owns.
    let engine = unsafe { &mut *engine };
    engine.state.is_password_field = is_password;
}

/// # Safety
/// `engine` is a live engine. The reply must be freed with `taigi_reply_free`.
#[no_mangle]
pub unsafe extern "C" fn taigi_engine_key(
    engine: *mut TaigiEngine,
    keysym: u32,
    keycode: u32,
    states: u32,
) -> *mut TaigiReply {
    if engine.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: non-null and, by contract, a live engine the caller owns.
    let engine = unsafe { &mut *engine };
    guarded("taigi_engine_key", ptr::null_mut(), || {
        let reply = session::process_raw_key(
            &engine.runtime,
            engine.token,
            &mut engine.state,
            RawKeyEvent {
                keyval: keysym,
                keycode,
                state: states,
            },
        );
        reply_from(reply.handled, reply.emits)
    })
}

/// # Safety
/// `engine` is a live engine; `id` is a NUL-terminated string from
/// `taigi_menu_id`.
#[no_mangle]
pub unsafe extern "C" fn taigi_engine_menu_activate(
    engine: *mut TaigiEngine,
    id: *const c_char,
) -> *mut TaigiReply {
    if engine.is_null() || id.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: non-null and, by contract, a live engine the caller owns.
    let engine = unsafe { &mut *engine };
    // SAFETY: non-null and, by contract, NUL-terminated.
    let id = unsafe { CStr::from_ptr(id) }.to_string_lossy().into_owned();
    guarded("taigi_engine_menu_activate", ptr::null_mut(), || {
        let emits = chrome::activate_menu(&engine.runtime, engine.token, &mut engine.state, &id);
        reply_from(true, emits)
    })
}

/// # Safety
/// `engine` is a live engine.
#[no_mangle]
pub unsafe extern "C" fn taigi_engine_end_session(engine: *mut TaigiEngine) -> *mut TaigiReply {
    if engine.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: non-null and, by contract, a live engine the caller owns.
    let engine = unsafe { &mut *engine };
    guarded("taigi_engine_end_session", ptr::null_mut(), || {
        let emits = session::end_session(&engine.runtime, engine.token, &mut engine.state);
        reply_from(true, emits)
    })
}

/// # Safety
/// `engine` is a live engine.
#[no_mangle]
pub unsafe extern "C" fn taigi_engine_navigate(
    engine: *mut TaigiEngine,
    direction: u32,
) -> *mut TaigiReply {
    if engine.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: non-null and, by contract, a live engine the caller owns.
    let engine = unsafe { &mut *engine };
    guarded("taigi_engine_navigate", ptr::null_mut(), || {
        let direction = match direction {
            NAVIGATE_PREVIOUS => CandidateNavigation::PreviousCandidate,
            NAVIGATE_NEXT => CandidateNavigation::NextCandidate,
            NAVIGATE_PAGE_UP => CandidateNavigation::PageUp,
            NAVIGATE_PAGE_DOWN => CandidateNavigation::PageDown,
            _ => return reply_from(false, Vec::new()),
        };
        let emits = session::navigate_from_panel(
            &engine.runtime,
            engine.token,
            &mut engine.state,
            direction,
        );
        reply_from(true, emits)
    })
}

/// # Safety
/// `engine` is a live engine.
#[no_mangle]
pub unsafe extern "C" fn taigi_engine_click(
    engine: *mut TaigiEngine,
    position: u32,
) -> *mut TaigiReply {
    if engine.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: non-null and, by contract, a live engine the caller owns.
    let engine = unsafe { &mut *engine };
    guarded("taigi_engine_click", ptr::null_mut(), || {
        let emits =
            session::click_from_panel(&engine.runtime, &mut engine.state, position as usize);
        reply_from(true, emits)
    })
}

// ---------------------------------------------------------------------------
// Reply accessors
// ---------------------------------------------------------------------------

/// # Safety
/// `reply` is a live reply or null.
unsafe fn emit_at<'a>(reply: *const TaigiReply, index: usize) -> Option<&'a ReplyEmit> {
    if reply.is_null() {
        return None;
    }
    // SAFETY: non-null and, by contract, a live reply the caller owns.
    unsafe { &*reply }.emits.get(index)
}

/// # Safety
/// `reply` is a live reply or null.
#[no_mangle]
pub unsafe extern "C" fn taigi_reply_handled(reply: *const TaigiReply) -> bool {
    if reply.is_null() {
        return false;
    }
    // SAFETY: non-null and, by contract, a live reply the caller owns.
    unsafe { &*reply }.handled
}

/// # Safety
/// `reply` is a live reply or null.
#[no_mangle]
pub unsafe extern "C" fn taigi_reply_count(reply: *const TaigiReply) -> usize {
    if reply.is_null() {
        return 0;
    }
    // SAFETY: non-null and, by contract, a live reply the caller owns.
    unsafe { &*reply }.emits.len()
}

/// # Safety
/// `reply` is a live reply or null.
#[no_mangle]
pub unsafe extern "C" fn taigi_reply_kind(reply: *const TaigiReply, index: usize) -> u32 {
    // SAFETY: forwarded contract.
    unsafe { emit_at(reply, index) }.map_or(0, |emit| emit.kind)
}

/// # Safety
/// `reply` is a live reply or null. The string lives until `taigi_reply_free`.
#[no_mangle]
pub unsafe extern "C" fn taigi_reply_text(reply: *const TaigiReply, index: usize) -> *const c_char {
    static EMPTY: &CStr = c"";
    // SAFETY: forwarded contract.
    unsafe { emit_at(reply, index) }.map_or(EMPTY.as_ptr(), |emit| emit.text.as_ptr())
}

/// # Safety
/// `reply` is a live reply or null.
#[no_mangle]
pub unsafe extern "C" fn taigi_reply_caret(reply: *const TaigiReply, index: usize) -> u32 {
    // SAFETY: forwarded contract.
    unsafe { emit_at(reply, index) }.map_or(0, |emit| emit.caret)
}

/// # Safety
/// `reply` is a live reply or null.
#[no_mangle]
pub unsafe extern "C" fn taigi_reply_delete_offset(reply: *const TaigiReply, index: usize) -> i32 {
    // SAFETY: forwarded contract.
    unsafe { emit_at(reply, index) }.map_or(0, |emit| emit.delete_offset)
}

/// # Safety
/// `reply` is a live reply or null.
#[no_mangle]
pub unsafe extern "C" fn taigi_reply_delete_count(reply: *const TaigiReply, index: usize) -> u32 {
    // SAFETY: forwarded contract.
    unsafe { emit_at(reply, index) }.map_or(0, |emit| emit.delete_count)
}

/// # Safety
/// `reply` is a live reply or null.
#[no_mangle]
pub unsafe extern "C" fn taigi_reply_table_count(reply: *const TaigiReply, index: usize) -> usize {
    // SAFETY: forwarded contract.
    unsafe { emit_at(reply, index) }
        .and_then(|emit| emit.table.as_ref())
        .map_or(0, |table| table.candidates.len())
}

/// # Safety
/// `reply` is a live reply or null.
#[no_mangle]
pub unsafe extern "C" fn taigi_reply_table_candidate(
    reply: *const TaigiReply,
    index: usize,
    row: usize,
) -> *const c_char {
    static EMPTY: &CStr = c"";
    // SAFETY: forwarded contract.
    unsafe { emit_at(reply, index) }
        .and_then(|emit| emit.table.as_ref())
        .and_then(|table| table.candidates.get(row))
        .map_or(EMPTY.as_ptr(), |text| text.as_ptr())
}

/// # Safety
/// `reply` is a live reply or null.
#[no_mangle]
pub unsafe extern "C" fn taigi_reply_table_label_count(
    reply: *const TaigiReply,
    index: usize,
) -> usize {
    // SAFETY: forwarded contract.
    unsafe { emit_at(reply, index) }
        .and_then(|emit| emit.table.as_ref())
        .map_or(0, |table| table.labels.len())
}

/// # Safety
/// `reply` is a live reply or null.
#[no_mangle]
pub unsafe extern "C" fn taigi_reply_table_label(
    reply: *const TaigiReply,
    index: usize,
    position: usize,
) -> *const c_char {
    static EMPTY: &CStr = c"";
    // SAFETY: forwarded contract.
    unsafe { emit_at(reply, index) }
        .and_then(|emit| emit.table.as_ref())
        .and_then(|table| table.labels.get(position))
        .map_or(EMPTY.as_ptr(), |text| text.as_ptr())
}

/// # Safety
/// `reply` is a live reply or null.
#[no_mangle]
pub unsafe extern "C" fn taigi_reply_table_cursor(reply: *const TaigiReply, index: usize) -> u32 {
    // SAFETY: forwarded contract.
    unsafe { emit_at(reply, index) }
        .and_then(|emit| emit.table.as_ref())
        .map_or(0, |table| table.cursor)
}

/// # Safety
/// `reply` is a live reply or null.
#[no_mangle]
pub unsafe extern "C" fn taigi_reply_table_cursor_visible(
    reply: *const TaigiReply,
    index: usize,
) -> bool {
    // SAFETY: forwarded contract.
    unsafe { emit_at(reply, index) }
        .and_then(|emit| emit.table.as_ref())
        .is_some_and(|table| table.cursor_visible)
}

/// # Safety
/// `reply` is a live reply or null.
#[no_mangle]
pub unsafe extern "C" fn taigi_reply_table_page_size(
    reply: *const TaigiReply,
    index: usize,
) -> u32 {
    // SAFETY: forwarded contract.
    unsafe { emit_at(reply, index) }
        .and_then(|emit| emit.table.as_ref())
        .map_or(0, |table| table.page_size)
}

/// # Safety
/// `reply` is a live reply or null.
#[no_mangle]
pub unsafe extern "C" fn taigi_reply_table_vertical(
    reply: *const TaigiReply,
    index: usize,
) -> bool {
    // SAFETY: forwarded contract.
    unsafe { emit_at(reply, index) }
        .and_then(|emit| emit.table.as_ref())
        .is_some_and(|table| table.vertical)
}

/// # Safety
/// `reply` came from this crate and is freed once (null is ignored).
#[no_mangle]
pub unsafe extern "C" fn taigi_reply_free(reply: *mut TaigiReply) {
    if reply.is_null() {
        return;
    }
    // SAFETY: the header's contract — a pointer this crate boxed, freed once.
    drop(unsafe { Box::from_raw(reply) });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reply_round_trips_through_the_accessors() {
        // trace: two emits — a preedit and a table with two rows.
        let reply = reply_from(
            true,
            vec![
                Emit::Preedit {
                    text: "tâi".into(),
                    caret: 3,
                },
                Emit::LookupTable(LookupTableContent {
                    candidates: vec!["台".into(), "tâi".into()],
                    labels: vec!["q".into(), "w".into()],
                    cursor: 1,
                    cursor_visible: true,
                    page_size: 9,
                    vertical: false,
                }),
            ],
        );
        // SAFETY: a reply this test just boxed.
        unsafe {
            assert!(taigi_reply_handled(reply));
            assert_eq!(taigi_reply_count(reply), 2);
            assert_eq!(taigi_reply_kind(reply, 0), EMIT_PREEDIT);
            assert_eq!(
                CStr::from_ptr(taigi_reply_text(reply, 0)).to_str(),
                Ok("tâi")
            );
            assert_eq!(taigi_reply_caret(reply, 0), 3);
            assert_eq!(taigi_reply_kind(reply, 1), EMIT_LOOKUP_TABLE);
            assert_eq!(taigi_reply_table_count(reply, 1), 2);
            assert_eq!(
                CStr::from_ptr(taigi_reply_table_candidate(reply, 1, 1)).to_str(),
                Ok("tâi")
            );
            assert_eq!(
                CStr::from_ptr(taigi_reply_table_label(reply, 1, 0)).to_str(),
                Ok("q")
            );
            assert_eq!(taigi_reply_table_cursor(reply, 1), 1);
            assert_eq!(taigi_reply_table_page_size(reply, 1), 9);
            assert!(!taigi_reply_table_vertical(reply, 1));
            // Out of range answers the documented zero / empty, never a fault.
            assert_eq!(taigi_reply_kind(reply, 9), 0);
            assert_eq!(CStr::from_ptr(taigi_reply_text(reply, 9)).to_str(), Ok(""));
            assert_eq!(taigi_reply_table_count(reply, 0), 0);
            taigi_reply_free(reply);
        }
    }

    #[test]
    fn null_handles_are_ignored_everywhere() {
        // SAFETY: nulls are the documented no-op input of every accessor.
        unsafe {
            assert!(taigi_engine_new(ptr::null()).is_null());
            assert!(taigi_engine_key(ptr::null_mut(), 0x61, 38, 0).is_null());
            assert!(!taigi_reply_handled(ptr::null()));
            assert_eq!(taigi_reply_count(ptr::null()), 0);
            taigi_reply_free(ptr::null_mut());
            taigi_engine_free(ptr::null_mut());
            taigi_runtime_free(ptr::null_mut());
        }
    }

    #[test]
    fn the_version_is_this_crate_version() {
        assert_eq!(
            // SAFETY: a static NUL-terminated literal.
            unsafe { CStr::from_ptr(taigi_version()) }.to_str(),
            Ok(env!("CARGO_PKG_VERSION"))
        );
    }
}
