//! What a downloaded package has to prove before the 安裝 button appears:
//! an Authenticode signature Windows trusts (`WinVerifyTrust`), signed by
//! the SAME certificate as the running copy (the leaf's thumbprint, not a
//! subject string — Codex W9), and a VERSIONINFO that names this product at
//! the manifest's version. Port of `UpdatePackageVerifier` +
//! `UpdatePackageIdentity`: the Mac pins the Developer ID team and the
//! package's bundle id + version; the Windows analogue is the signer's
//! thumbprint and the executable's product name + version.
//!
//! Pinning the LEAF is the stronger, less continuous choice (PR9 Codex): a
//! renewed signing certificate is not accepted by the copies signed with
//! the old one, so the first release under a new certificate ships through
//! the download page and in-app updates resume from there. Windows has no
//! equivalent of Apple's team id to pin instead.
//!
//! An unsigned running copy (a development build) has no identity and
//! therefore no in-app install — the download page, as ad-hoc builds on
//! the Mac. On a non-Windows host every question answers "unavailable".

// 中文: 安裝檔驗證 — WinVerifyTrust + 簽章憑證指紋釘死為執行中程式的簽章 + VERSIONINFO 產品名/版本。

use std::path::Path;

/// What the running copy is signed as, and named: the bar a package must
/// clear.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageIdentity {
    /// SHA-1 thumbprint of the signer's leaf certificate.
    pub signer_thumbprint: Vec<u8>,
    /// VERSIONINFO `ProductName`.
    pub product_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rejection {
    /// No signature, or one Windows does not trust.
    Untrusted,
    /// Trusted, but not by the certificate the running copy was signed with.
    WrongSigner,
    /// Signed right, but not this product at the manifest's version.
    WrongPackage {
        product_name: String,
        version: String,
    },
    Unreadable,
    /// Not a Windows host: nothing can be verified here.
    Unavailable,
}

/// The `ProductName` a VERSIONINFO string block holds, as text.
///
/// The block is a fixed-size buffer, so what `VerQueryValueW` hands back is
/// the name plus whatever fills the rest. Two producers write the packages
/// this crate compares, and they fill it differently: rustc emits exactly
/// the name (NUL-terminated), while Inno Setup patches its stub's
/// placeholder in place and pads to the placeholder's width with SPACES. So
/// a downloaded installer reads `"TaigiKeyboard          "` where the
/// running settings exe reads `"TaigiKeyboard"`, and comparing the two raw
/// rejects every genuine package (observed 2026-09-01, the first time the
/// installer was ever compiled).
#[cfg(any(windows, test))]
fn product_name_from_versioninfo(units: &[u16]) -> String {
    String::from_utf16_lossy(units)
        .trim_end_matches('\0')
        .trim()
        .to_owned()
}

/// The running executable's identity, or `None` when it carries no
/// trusted signature — then nothing is ever installed in-app.
#[cfg(windows)]
pub fn running_identity() -> Option<PackageIdentity> {
    let exe = std::env::current_exe().ok()?;
    if win::verify_trust(&exe).is_err() {
        return None;
    }
    let signer_thumbprint = win::signer_thumbprint(&exe).ok()?;
    let (product_name, _) = win::version_info(&exe).ok()?;
    Some(PackageIdentity {
        signer_thumbprint,
        product_name,
    })
}

#[cfg(not(windows))]
pub fn running_identity() -> Option<PackageIdentity> {
    None
}

/// `package` is signed by `expected`'s certificate and declares
/// `expected`'s product at `version`.
#[cfg(windows)]
pub fn verify(package: &Path, expected: &PackageIdentity, version: &str) -> Result<(), Rejection> {
    win::verify_trust(package).map_err(|_| Rejection::Untrusted)?;
    let thumbprint = win::signer_thumbprint(package).map_err(|_| Rejection::Unreadable)?;
    if thumbprint != expected.signer_thumbprint {
        return Err(Rejection::WrongSigner);
    }
    let (product_name, declared_version) =
        win::version_info(package).map_err(|_| Rejection::Unreadable)?;
    let same_version = match (
        crate::manifest::DottedVersion::parse(&declared_version),
        crate::manifest::DottedVersion::parse(version),
    ) {
        (Some(declared), Some(wanted)) => declared == wanted,
        _ => false,
    };
    if product_name != expected.product_name || !same_version {
        return Err(Rejection::WrongPackage {
            product_name,
            version: declared_version,
        });
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn verify(
    _package: &Path,
    _expected: &PackageIdentity,
    _version: &str,
) -> Result<(), Rejection> {
    Err(Rejection::Unavailable)
}

#[cfg(windows)]
mod win {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use windows::core::{Error, Result, PCWSTR};
    use windows::Win32::Foundation::{E_UNEXPECTED, HANDLE, HWND};
    use windows::Win32::Security::Cryptography::{
        CertCloseStore, CertFindCertificateInStore, CertFreeCertificateContext,
        CertGetCertificateContextProperty, CryptMsgClose, CryptMsgGetParam, CryptQueryObject,
        CERT_CONTEXT, CERT_FIND_SUBJECT_CERT, CERT_HASH_PROP_ID, CERT_INFO,
        CERT_QUERY_CONTENT_FLAG_PKCS7_SIGNED_EMBED, CERT_QUERY_ENCODING_TYPE,
        CERT_QUERY_FORMAT_FLAG_BINARY, CERT_QUERY_OBJECT_FILE, CMSG_SIGNER_INFO,
        CMSG_SIGNER_INFO_PARAM, HCERTSTORE,
    };
    use windows::Win32::Security::WinTrust::{
        WinVerifyTrust, WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_DATA_0,
        WINTRUST_FILE_INFO, WTD_CACHE_ONLY_URL_RETRIEVAL, WTD_CHOICE_FILE, WTD_REVOKE_NONE,
        WTD_STATEACTION_CLOSE, WTD_STATEACTION_VERIFY, WTD_UI_NONE,
    };
    use windows::Win32::Storage::FileSystem::{
        GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW, VS_FIXEDFILEINFO,
    };

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    fn wide_str(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Every CryptoAPI handle is released by a guard, so a failure between
    /// acquiring and using one cannot leak it — whatever the API filled in
    /// before it failed included.
    struct StoreGuard(HCERTSTORE);

    impl Drop for StoreGuard {
        fn drop(&mut self) {
            if !self.0.is_invalid() {
                // SAFETY: a store this guard owns, closed once.
                unsafe { CertCloseStore(Some(self.0), 0).ok() };
            }
        }
    }

    struct MessageGuard(*mut c_void);

    impl Drop for MessageGuard {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: a message handle this guard owns, closed once.
                unsafe { CryptMsgClose(Some(self.0)).ok() };
            }
        }
    }

    struct CertificateGuard(*mut CERT_CONTEXT);

    impl Drop for CertificateGuard {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: a context this guard owns, freed once.
                let _ = unsafe { CertFreeCertificateContext(Some(self.0)) };
            }
        }
    }

    /// Authenticode: the file's embedded signature chains to a trusted
    /// root. No UI, no online revocation (a check must not hang on a
    /// network); the state handle is closed whatever the answer.
    pub fn verify_trust(path: &Path) -> Result<()> {
        let path = wide(path);
        let mut file_info = WINTRUST_FILE_INFO {
            cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
            pcwszFilePath: PCWSTR(path.as_ptr()),
            hFile: HANDLE::default(),
            pgKnownSubject: std::ptr::null_mut(),
        };
        let mut data = WINTRUST_DATA {
            cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
            dwUIChoice: WTD_UI_NONE,
            fdwRevocationChecks: WTD_REVOKE_NONE,
            dwUnionChoice: WTD_CHOICE_FILE,
            Anonymous: WINTRUST_DATA_0 {
                pFile: &mut file_info,
            },
            dwStateAction: WTD_STATEACTION_VERIFY,
            dwProvFlags: WTD_CACHE_ONLY_URL_RETRIEVAL,
            ..Default::default()
        };
        let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
        // SAFETY: both structs are valid locals for the two calls; the
        // path buffer outlives them.
        let status = unsafe {
            let status = WinVerifyTrust(
                HWND::default(),
                &mut action,
                &mut data as *mut WINTRUST_DATA as *mut c_void,
            );
            data.dwStateAction = WTD_STATEACTION_CLOSE;
            WinVerifyTrust(
                HWND::default(),
                &mut action,
                &mut data as *mut WINTRUST_DATA as *mut c_void,
            );
            status
        };
        if status == 0 {
            Ok(())
        } else {
            Err(Error::from_hresult(windows::core::HRESULT(status)))
        }
    }

    /// The SHA-1 thumbprint of the certificate that signed `path`: the
    /// signer info names issuer + serial, the store the signature carries
    /// holds the certificate, the hash property is the thumbprint.
    pub fn signer_thumbprint(path: &Path) -> Result<Vec<u8>> {
        let path = wide(path);
        let mut encoding = CERT_QUERY_ENCODING_TYPE::default();
        let mut store = HCERTSTORE::default();
        let mut message: *mut c_void = std::ptr::null_mut();
        // SAFETY: out-pointers to locals; whatever the call filled in is
        // owned by the guards from here on, on every path.
        unsafe {
            let queried = CryptQueryObject(
                CERT_QUERY_OBJECT_FILE,
                path.as_ptr() as *const c_void,
                CERT_QUERY_CONTENT_FLAG_PKCS7_SIGNED_EMBED,
                CERT_QUERY_FORMAT_FLAG_BINARY,
                0,
                Some(&mut encoding),
                None,
                None,
                Some(&mut store),
                Some(&mut message),
                None,
            );
            let store = StoreGuard(store);
            let message = MessageGuard(message);
            queried?;
            thumbprint_from(message.0, store.0, encoding)
        }
    }

    /// The signer's issuer + serial from the message, then the matching
    /// certificate's hash. The signer info is copied out of a byte buffer
    /// with `read_unaligned` (a `Vec<u8>` promises no alignment); its blob
    /// pointers point back into that buffer, which outlives the lookup.
    unsafe fn thumbprint_from(
        message: *mut c_void,
        store: HCERTSTORE,
        encoding: CERT_QUERY_ENCODING_TYPE,
    ) -> Result<Vec<u8>> {
        let mut size = 0u32;
        CryptMsgGetParam(message, CMSG_SIGNER_INFO_PARAM, 0, None, &mut size)?;
        let mut buffer = vec![0u8; size as usize];
        CryptMsgGetParam(
            message,
            CMSG_SIGNER_INFO_PARAM,
            0,
            Some(buffer.as_mut_ptr() as *mut c_void),
            &mut size,
        )?;
        if (size as usize) < std::mem::size_of::<CMSG_SIGNER_INFO>() {
            return Err(Error::from_hresult(E_UNEXPECTED));
        }
        let signer = std::ptr::read_unaligned(buffer.as_ptr() as *const CMSG_SIGNER_INFO);
        let wanted = CERT_INFO {
            Issuer: signer.Issuer,
            SerialNumber: signer.SerialNumber,
            ..Default::default()
        };
        let certificate = CertificateGuard(CertFindCertificateInStore(
            store,
            encoding,
            0,
            CERT_FIND_SUBJECT_CERT,
            Some(&wanted as *const CERT_INFO as *const c_void),
            None,
        ));
        if certificate.0.is_null() {
            return Err(Error::from_thread());
        }
        let mut hash_size = 0u32;
        CertGetCertificateContextProperty(certificate.0, CERT_HASH_PROP_ID, None, &mut hash_size)?;
        let mut hash = vec![0u8; hash_size as usize];
        CertGetCertificateContextProperty(
            certificate.0,
            CERT_HASH_PROP_ID,
            Some(hash.as_mut_ptr() as *mut c_void),
            &mut hash_size,
        )?;
        hash.truncate(hash_size as usize);
        // The buffer the signer's blobs point into stays alive to here.
        drop(buffer);
        Ok(hash)
    }

    /// VERSIONINFO: the product name (first translation) and the product
    /// version as dotted integers. Every value `VerQueryValueW` answers is a
    /// pointer into the block and is copied out unaligned.
    pub fn version_info(path: &Path) -> Result<(String, String)> {
        let path = wide(path);
        // SAFETY: the block is sized by the first call and filled by the
        // second; every answer points inside it and is read unaligned
        // while it lives.
        unsafe {
            let size = GetFileVersionInfoSizeW(PCWSTR(path.as_ptr()), None);
            if size == 0 {
                return Err(Error::from_thread());
            }
            let mut block = vec![0u8; size as usize];
            GetFileVersionInfoW(
                PCWSTR(path.as_ptr()),
                None,
                size,
                block.as_mut_ptr() as *mut c_void,
            )?;
            let query = |sub_block: &str| -> Result<(*const c_void, u32)> {
                let name = wide_str(sub_block);
                let mut value: *mut c_void = std::ptr::null_mut();
                let mut length = 0u32;
                let found = VerQueryValueW(
                    block.as_ptr() as *const c_void,
                    PCWSTR(name.as_ptr()),
                    &mut value,
                    &mut length,
                );
                if !found.as_bool() || value.is_null() {
                    return Err(Error::from_thread());
                }
                Ok((value as *const c_void, length))
            };
            let (fixed, fixed_length) = query("\\")?;
            if (fixed_length as usize) < std::mem::size_of::<VS_FIXEDFILEINFO>() {
                return Err(Error::from_hresult(E_UNEXPECTED));
            }
            let info = std::ptr::read_unaligned(fixed as *const VS_FIXEDFILEINFO);
            let version = format!(
                "{}.{}.{}",
                info.dwProductVersionMS >> 16,
                info.dwProductVersionMS & 0xFFFF,
                info.dwProductVersionLS >> 16
            );
            let (translation, translation_length) = query("\\VarFileInfo\\Translation")?;
            if translation_length < 4 {
                return Err(Error::from_hresult(E_UNEXPECTED));
            }
            let pair = std::ptr::read_unaligned(translation as *const [u16; 2]);
            let (name, name_length) = query(&format!(
                "\\StringFileInfo\\{:04x}{:04x}\\ProductName",
                pair[0], pair[1]
            ))?;
            let units: Vec<u16> = (0..name_length as usize)
                .map(|index| std::ptr::read_unaligned((name as *const u16).add(index)))
                .collect();
            let product_name = super::product_name_from_versioninfo(&units);
            Ok((product_name, version))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    #[test]
    fn the_two_producers_of_a_package_name_read_as_the_same_name() {
        // What rustc writes for the settings exe, and what Inno Setup writes
        // for the installer: the same product, one NUL-terminated, one padded
        // to its placeholder's width.
        let from_rustc = product_name_from_versioninfo(&utf16("TaigiKeyboard\0"));
        let from_inno = product_name_from_versioninfo(&utf16(
            "TaigiKeyboard                                              \0\0",
        ));
        assert_eq!(from_rustc, "TaigiKeyboard");
        assert_eq!(
            from_rustc, from_inno,
            "a padded name must not reject a genuine package"
        );
    }

    #[test]
    fn a_different_product_still_reads_as_a_different_name() {
        assert_ne!(
            product_name_from_versioninfo(&utf16("TaigiKeyboard\0")),
            product_name_from_versioninfo(&utf16("TaigiKeyboard Setup    \0")),
        );
    }
}
