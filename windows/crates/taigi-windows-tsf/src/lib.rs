//! `TaigiKeyboard.dll`: the Text Services Framework text service. The five
//! DLL exports live here; everything else is a module. Every export and
//! every COM method body is a `catch_unwind` boundary — a panic must never
//! cross into the host process (`.claude/rules/windows-guidelines.md` § TSF).
//!
//! PR5a gave it the lifecycle (activation, sinks, tray button + menu,
//! settings reload); PR5b the composing: key sink → classifier → engine +
//! document inside synchronous edit sessions, display attribute, preserved
//! keys, password / read-only gating, context handover; PR6 the candidate
//! window (Direct2D over the core models), the UI-less list and the mode flash.

// 中文: TSF 文字服務 DLL 的五個匯出點;每個 COM 進入點都包 catch_unwind,PR5a 只做生命週期不組字。

#![allow(non_snake_case)]

mod class_factory;
mod com_guard;
mod com_out_buffer;
mod composition;
mod contexts;
mod display_attribute;
mod edit_session;
mod guids;
mod key_translation;
mod lang_bar;
mod module;
mod preserved_keys;
mod registration;
mod registry;
mod runtime;
mod session;
mod settings_launcher;
mod text_service;
mod ui;
mod wide;

use std::ffi::c_void;
use windows::core::BOOL;
use windows::core::{Interface, GUID, HRESULT};
use windows::Win32::Foundation::{
    CLASS_E_CLASSNOTAVAILABLE, E_FAIL, E_POINTER, HINSTANCE, S_FALSE, S_OK,
};
use windows::Win32::System::Com::{
    CoInitializeEx, CoUninitialize, IClassFactory, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;

/// Stores the module handle and nothing else: no threads, no COM, no file
/// I/O from `DllMain` (loader lock). The install directory, the logger and
/// the runtime all resolve lazily from the handle saved here.
#[no_mangle]
pub extern "system" fn DllMain(instance: HINSTANCE, reason: u32, _reserved: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        module::remember_instance(instance);
    }
    BOOL::from(true)
}

/// Hands out the class factory for this DLL's one CLSID.
#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    com_guard::guarded_hresult("DllGetClassObject", || {
        if rclsid.is_null() || riid.is_null() || ppv.is_null() {
            return E_POINTER;
        }
        // SAFETY: the three pointers were null-checked above and are the
        // host's, valid for this call by the COM contract.
        unsafe {
            *ppv = std::ptr::null_mut();
            if *rclsid != guids::CLSID_TEXT_SERVICE {
                return CLASS_E_CLASSNOTAVAILABLE;
            }
            let factory: IClassFactory = class_factory::ClassFactory.into();
            factory.query(riid, ppv)
        }
    })
}

/// `S_FALSE` forever: the DLL stays resident. A `RegisterClassW`'d window
/// procedure (the candidate window, PR6) outlives `FreeLibrary`, and an
/// in-flight message dispatched to an unloaded address is a host crash
/// (rakukan `lib.rs:157-169`). Microsoft's own IMEs stay resident too.
#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    S_FALSE
}

/// `regsvr32`: CLSID + profile + categories, in that order.
#[no_mangle]
pub extern "system" fn DllRegisterServer() -> HRESULT {
    com_guard::guarded_hresult("DllRegisterServer", || {
        with_apartment(registration::register_server)
    })
}

/// `regsvr32 /u`: symmetric, item by item, never stopping at the first
/// failure — Microsoft requires each category removed individually.
#[no_mangle]
pub extern "system" fn DllUnregisterServer() -> HRESULT {
    com_guard::guarded_hresult("DllUnregisterServer", || {
        with_apartment(registration::unregister_server)
    })
}

fn with_apartment(body: fn() -> windows::core::Result<()>) -> HRESULT {
    // SAFETY: plain COM initialisation on the calling (regsvr32) thread,
    // balanced by the CoUninitialize below whatever `body` did.
    let initialised = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
    let result = match body() {
        Ok(()) => S_OK,
        Err(error) => {
            log::error!("tsf.register_failed error={error}");
            E_FAIL
        }
    };
    if initialised {
        // SAFETY: balances the successful CoInitializeEx above.
        unsafe { CoUninitialize() };
    }
    result
}
