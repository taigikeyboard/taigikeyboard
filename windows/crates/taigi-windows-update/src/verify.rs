//! What a downloaded package has to prove before the 安裝 button appears:
//! the SHA-256 the manifest published, always — and, when the running copy
//! carries a trusted Authenticode signature of its own, that the package is
//! signed by the SAME certificate (the leaf's thumbprint) with a VERSIONINFO
//! naming this product at the manifest's version. Both, never one instead of
//! the other, and only through `Admission`.
//!
//! Why a digest at all, what it is and is not worth, and when the signed
//! half starts applying: `docs/architecture/windows-release.md` § Signing
//! status — the single source for that policy.
//!
//! Port of `UpdatePackageVerifier` + `UpdatePackageIdentity`: the Mac pins
//! the Developer ID team and the package's bundle id + version; the Windows
//! analogue is the signer's thumbprint and the executable's product name +
//! version.
//!
//! Pinning the LEAF is the stronger, less continuous choice (PR9 Codex): a
//! renewed signing certificate is not accepted by the copies signed with the
//! old one, so the first release under a new certificate is admitted on its
//! digest alone. Windows has no equivalent of Apple's team id to pin instead.

// 中文: 安裝檔驗證 — SHA-256 必驗;執行中程式有簽章身分時,再加 WinVerifyTrust + 指紋 + VERSIONINFO。

use sha2::{Digest, Sha256};
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
    /// Not the file the manifest published.
    HashMismatch,
    Unreadable,
    /// Not a Windows host: nothing can be verified here.
    Unavailable,
}

/// What this copy admits a package on: the requirements it can apply, held
/// once. The running copy's signature is read at construction (once — a
/// signature check is not free) and lives here rather than in the download
/// state machine, which has no decision to make with it.
#[derive(Clone, Debug)]
pub struct Admission {
    identity: Option<PackageIdentity>,
}

impl Default for Admission {
    fn default() -> Self {
        Self::of_running_copy()
    }
}

impl Admission {
    /// Reads the running executable's own signature. `None` — no signature,
    /// or one that cannot be READ — leaves the digest as the whole bar.
    /// Deliberately not fail-closed: refusing every update because one
    /// CryptoAPI call failed strands the copy with no route but the browser,
    /// and the digest still has to match either way.
    pub fn of_running_copy() -> Self {
        Self {
            identity: running_identity(),
        }
    }

    /// A copy with no signature of its own — what every unsigned release
    /// runs as, and what the crate's tests admit packages through.
    #[cfg(test)]
    pub(crate) fn unsigned() -> Self {
        Self { identity: None }
    }

    /// The identity a package must ALSO be signed with, for the test that
    /// pins the "a signed copy cannot skip Authenticode" half.
    #[cfg(test)]
    fn pinned_to(identity: PackageIdentity) -> Self {
        Self {
            identity: Some(identity),
        }
    }

    /// Every requirement this copy has, applied in one place so a caller can
    /// neither choose between them nor let one stand in for the other. The
    /// digest goes first: it is the cheap answer to "is this even the
    /// published file".
    pub fn admit(
        &self,
        package: &Path,
        expected_sha256: &str,
        version: &str,
    ) -> Result<(), Rejection> {
        let digest = file_sha256(package).map_err(|error| {
            log::debug!("update.package_unreadable error={error}");
            Rejection::Unreadable
        })?;
        // `normalized_sha256` owns what a published digest looks like (64
        // hex, case-insensitive), so a malformed expectation cannot match.
        if crate::manifest::normalized_sha256(expected_sha256) != Some(digest) {
            return Err(Rejection::HashMismatch);
        }
        match &self.identity {
            Some(identity) => verify(package, identity, version),
            None => Ok(()),
        }
    }
}

/// The file's SHA-256 as lowercase hex. Streamed: a 48 MB installer is never
/// held in memory.
pub(crate) fn file_sha256(package: &Path) -> Result<String, std::io::Error> {
    let mut hasher = Sha256::new();
    std::io::copy(&mut std::fs::File::open(package)?, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
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

/// The running executable's identity: no trusted signature, or one that
/// cannot be read, answers `None` (`Admission::of_running_copy`).
#[cfg(windows)]
fn running_identity() -> Option<PackageIdentity> {
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
fn running_identity() -> Option<PackageIdentity> {
    None
}

/// `package` is signed by `expected`'s certificate and declares
/// `expected`'s product at `version`. Reached only through
/// `Admission::admit`, which has already matched the published digest.
#[cfg(windows)]
fn verify(package: &Path, expected: &PackageIdentity, version: &str) -> Result<(), Rejection> {
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
fn verify(_package: &Path, _expected: &PackageIdentity, _version: &str) -> Result<(), Rejection> {
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
    fn the_digest_admits_a_package_and_an_identity_is_an_addition_not_an_alternative() {
        // trace: Admission::admit hashes the staged file, compares it to the
        // manifest's digest, and only then asks the Authenticode half. On
        // this host that half answers `Unavailable` — which is the proof
        // wanted: a matching digest did NOT let a copy with an identity
        // return Ok on its own.
        let staged = tempfile::NamedTempFile::new().expect("a temporary file");
        std::fs::write(staged.path(), b"an installer").expect("write");
        let digest = file_sha256(staged.path()).expect("hash");
        let unsigned = Admission::unsigned();
        assert_eq!(unsigned.admit(staged.path(), &digest, "3.7.0"), Ok(()));
        assert_eq!(
            unsigned.admit(staged.path(), &digest.to_ascii_uppercase(), "3.7.0"),
            Ok(()),
            "a digest published in upper case is the same digest"
        );
        assert_eq!(
            unsigned.admit(staged.path(), &"a".repeat(64), "3.7.0"),
            Err(Rejection::HashMismatch)
        );
        assert_eq!(
            unsigned.admit(&staged.path().join("no-such-file"), &digest, "3.7.0"),
            Err(Rejection::Unreadable)
        );
        let signed = Admission::pinned_to(PackageIdentity {
            signer_thumbprint: vec![1, 2, 3],
            product_name: "TaigiKeyboard".to_owned(),
        });
        assert_eq!(
            signed.admit(staged.path(), &digest, "3.7.0"),
            Err(Rejection::Unavailable)
        );
    }

    #[test]
    fn an_empty_file_hashes_to_the_published_sha256_of_nothing() {
        // The one digest with an external oracle, so a wrong hex encoding
        // (byte order, padding) cannot pass unnoticed: NIST's SHA-256 of the
        // empty string.
        let empty = tempfile::NamedTempFile::new().expect("a temporary file");
        assert_eq!(
            file_sha256(empty.path()).expect("hash"),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
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
