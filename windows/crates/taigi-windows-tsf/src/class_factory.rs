//! The one `IClassFactory` this DLL exports: creates the text service.

// 類別工廠 — 建立文字服務物件。

use crate::com_guard::guarded;
use crate::text_service::TextService;
use std::ffi::c_void;
use windows::core::{Error, IUnknown, Interface, Ref, Result, BOOL, GUID};
use windows::Win32::Foundation::{CLASS_E_NOAGGREGATION, E_POINTER};
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl};
use windows::Win32::UI::TextServices::ITfTextInputProcessorEx;
use windows_core::implement;

#[implement(IClassFactory)]
pub struct ClassFactory;

impl IClassFactory_Impl for ClassFactory_Impl {
    fn CreateInstance(
        &self,
        outer: Ref<IUnknown>,
        riid: *const GUID,
        object: *mut *mut c_void,
    ) -> Result<()> {
        guarded("IClassFactory::CreateInstance", || {
            if riid.is_null() || object.is_null() {
                return Err(Error::from_hresult(E_POINTER));
            }
            if outer.is_some() {
                return Err(Error::from_hresult(CLASS_E_NOAGGREGATION));
            }
            let service: ITfTextInputProcessorEx = TextService::new().into();
            // SAFETY: `riid` / `object` were null-checked; `query` writes the
            // requested interface (or nothing, with an error) into `object`.
            unsafe {
                *object = std::ptr::null_mut();
                service.query(riid, object).ok()
            }
        })
    }

    /// The DLL never unloads (`DllCanUnloadNow` = `S_FALSE`), so there is
    /// no server lock to count.
    fn LockServer(&self, _lock: BOOL) -> Result<()> {
        guarded("IClassFactory::LockServer", || Ok(()))
    }
}
