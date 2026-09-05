//! The registry operations `DllRegisterServer` and `DllUnregisterServer` need,
//! over the raw advapi32 API (khiin `reg/hkey.rs`). Identifiers are literals
//! owned by `registration.rs`; no user value reaches a key path.

use crate::wide::to_wide_nul;
use windows::core::{Error, Result, PCWSTR};
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegOpenKeyExW, RegSetValueExW, HKEY, KEY_READ,
    KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
};

fn check(status: WIN32_ERROR) -> Result<()> {
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(Error::from_hresult(status.to_hresult()))
    }
}

/// An open key, closed on drop.
pub struct Key(HKEY);

impl Key {
    /// Creates (or opens) `subkey` under `parent`.
    pub fn create(parent: HKEY, subkey: &str) -> Result<Self> {
        let path = to_wide_nul(subkey);
        let mut handle = HKEY::default();
        // SAFETY: `path` is NUL-terminated and outlives the call; `handle`
        // is a valid out-pointer; no security attributes / disposition asked.
        let status = unsafe {
            RegCreateKeyExW(
                parent,
                PCWSTR(path.as_ptr()),
                None,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_READ | KEY_WRITE,
                None,
                &mut handle,
                None,
            )
        };
        check(status)?;
        Ok(Self(handle))
    }

    /// Whether `subkey` exists under `parent`. Opening rather than creating, so
    /// asking cannot answer its own question — and three-valued on purpose:
    /// only `ERROR_FILE_NOT_FOUND` means absent. Access denied or an I/O
    /// failure means the question could not be answered, which a caller
    /// checking that something is GONE must not read as gone.
    pub fn exists(parent: HKEY, subkey: &str) -> Result<bool> {
        let path = to_wide_nul(subkey);
        let mut handle = HKEY::default();
        // SAFETY: `path` is NUL-terminated and outlives the call; `handle` is a
        // valid out-pointer, closed below when the open succeeded.
        let status =
            unsafe { RegOpenKeyExW(parent, PCWSTR(path.as_ptr()), None, KEY_READ, &mut handle) };
        if status == ERROR_FILE_NOT_FOUND {
            return Ok(false);
        }
        check(status)?;
        // SAFETY: `handle` was just opened by this function and is not used again.
        // Closing cannot change the answer, so its status is not consulted.
        let _ = unsafe { RegCloseKey(handle) };
        Ok(true)
    }

    /// Writes a `REG_SZ` value; `""` names the key's default value.
    pub fn set_string(&self, name: &str, value: &str) -> Result<()> {
        let name = to_wide_nul(name);
        let bytes: Vec<u8> = to_wide_nul(value)
            .iter()
            .flat_map(|unit| unit.to_le_bytes())
            .collect();
        // SAFETY: both buffers are valid for the call; `bytes` is the
        // NUL-terminated UTF-16 the REG_SZ type requires.
        let status =
            unsafe { RegSetValueExW(self.0, PCWSTR(name.as_ptr()), None, REG_SZ, Some(&bytes)) };
        check(status)
    }
}

impl Drop for Key {
    fn drop(&mut self) {
        // SAFETY: the handle was opened by `create` and is closed once.
        let _ = unsafe { RegCloseKey(self.0) };
    }
}

/// Deletes `subkey` and everything under it. A missing key is not an error
/// — unregistration is idempotent.
pub fn delete_tree(parent: HKEY, subkey: &str) -> Result<()> {
    let path = to_wide_nul(subkey);
    // SAFETY: `path` is NUL-terminated and outlives the call.
    let status = unsafe { RegDeleteTreeW(parent, PCWSTR(path.as_ptr())) };
    if status == windows::Win32::Foundation::ERROR_FILE_NOT_FOUND {
        return Ok(());
    }
    check(status)
}
