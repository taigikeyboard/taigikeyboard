//! The published manifest and the version it names. Port of
//! `UpdateManifest` / `DottedVersion` (`UpdateChecker.swift:1-120`); the
//! wire format is `macos/updates/README.md` § Wire format, unchanged —
//! only the file name differs (`windows.json`).

// 中文: 更新 manifest 的解碼與版本比較 — 與 macOS 同一份線上格式。

use serde::{Deserialize, Serialize};

/// Compiled into every shipped build; old installs request it forever, so
/// it stays on a domain the project controls (`macos/updates/README.md`).
pub const PUBLISHED_URL: &str = "https://taigikeyboard.tw/appcast/windows.json";

/// A manifest is a few hundred bytes; anything past this is not one.
pub const MAXIMUM_MANIFEST_BYTES: u64 = 64 * 1024;

#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum ManifestError {
    #[error("the manifest is not the documented shape")]
    Malformed,
}

/// What the website says the newest version is, and where it lives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateManifest {
    /// Dotted integers only (`3.6.5`); a suffix is refused.
    pub version: String,
    /// The page the user lands on, `https` only. Required: the only route
    /// a notification, a development build or an old install has.
    pub download_page_url: String,
    /// The installer itself, `https` only. Optional; an invalid one is
    /// dropped on its own rather than failing the manifest.
    pub package_url: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Wire {
    version: String,
    #[serde(rename = "downloadPageURL")]
    download_page_url: String,
    #[serde(
        rename = "packageURL",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    package_url: Option<String>,
}

/// A parseable absolute URL whose scheme is `https` and which names a
/// host (`URL(string:)` + the scheme check on the Mac).
fn is_https(url: &str) -> bool {
    url.parse::<ureq::http::Uri>().is_ok_and(|uri| {
        uri.scheme_str()
            .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https"))
            && uri.host().is_some_and(|host| !host.is_empty())
    })
}

impl UpdateManifest {
    /// Unknown extra fields are ignored, so the format can grow.
    pub fn decode(bytes: &[u8]) -> Result<Self, ManifestError> {
        let wire: Wire = serde_json::from_slice(bytes).map_err(|_| ManifestError::Malformed)?;
        if DottedVersion::parse(&wire.version).is_none() || !is_https(&wire.download_page_url) {
            return Err(ManifestError::Malformed);
        }
        Ok(Self {
            version: wire.version,
            download_page_url: wire.download_page_url,
            package_url: wire.package_url.filter(|url| is_https(url)),
        })
    }

    /// The same shape back, for `updatePendingManifest`.
    pub fn encode(&self) -> String {
        serde_json::to_string(&Wire {
            version: self.version.clone(),
            download_page_url: self.download_page_url.clone(),
            package_url: self.package_url.clone(),
        })
        .expect("three plain strings always serialize")
    }
}

/// A version as the manifest and the build spell it: up to eight dotted
/// integers, compared numerically and zero-padded (`3.6` == `3.6.0`).
#[derive(Clone, Debug)]
pub struct DottedVersion {
    components: Vec<u64>,
}

/// Equality is the zero-padded comparison, not the component list
/// (`DottedVersion.==` on the Mac): `3.6` IS `3.6.0`.
impl PartialEq for DottedVersion {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl Eq for DottedVersion {}

impl DottedVersion {
    const MAXIMUM_COMPONENT_COUNT: usize = 8;
    const MAXIMUM_COMPONENT_VALUE: u64 = 1_000_000_000;

    pub fn parse(text: &str) -> Option<Self> {
        let segments: Vec<&str> = text.split('.').collect();
        if segments.len() > Self::MAXIMUM_COMPONENT_COUNT {
            return None;
        }
        let mut components = Vec::with_capacity(segments.len());
        for segment in segments {
            if segment.is_empty() || !segment.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            let value: u64 = segment.parse().ok()?;
            if value > Self::MAXIMUM_COMPONENT_VALUE {
                return None;
            }
            components.push(value);
        }
        Some(Self { components })
    }
}

impl PartialOrd for DottedVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DottedVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let length = self.components.len().max(other.components.len());
        for index in 0..length {
            let left = self.components.get(index).copied().unwrap_or(0);
            let right = other.components.get(index).copied().unwrap_or(0);
            if left != right {
                return left.cmp(&right);
            }
        }
        std::cmp::Ordering::Equal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_documented_manifest_decodes_and_an_invalid_package_url_is_dropped_alone() {
        // trace: macos/updates/README.md § Wire format.
        let manifest = UpdateManifest::decode(
            br#"{"version":"3.7.0","downloadPageURL":"https://taigikeyboard.tw/","packageURL":"http://x/y.exe","extra":1}"#,
        )
        .unwrap();
        assert_eq!(manifest.version, "3.7.0");
        assert_eq!(manifest.package_url, None);
        let with_package = UpdateManifest::decode(
            br#"{"version":"3.7.0","downloadPageURL":"https://taigikeyboard.tw/","packageURL":"https://x/y.exe"}"#,
        )
        .unwrap();
        assert_eq!(with_package.package_url.as_deref(), Some("https://x/y.exe"));
        let again = UpdateManifest::decode(with_package.encode().as_bytes()).unwrap();
        assert_eq!(again, with_package);
    }

    #[test]
    fn a_suffixed_version_or_a_non_https_page_is_malformed() {
        for wire in [
            r#"{"version":"3.7.0-beta","downloadPageURL":"https://taigikeyboard.tw/"}"#,
            r#"{"version":"3.7.0","downloadPageURL":"http://taigikeyboard.tw/"}"#,
            r#"{"version":"3.7.0"}"#,
            "not json",
        ] {
            assert_eq!(
                UpdateManifest::decode(wire.as_bytes()),
                Err(ManifestError::Malformed),
                "{wire}"
            );
        }
    }

    #[test]
    fn dotted_versions_compare_numerically_and_zero_padded() {
        let parse = |text: &str| DottedVersion::parse(text).unwrap();
        assert!(parse("3.10.0") > parse("3.9.9"));
        assert_eq!(parse("3.6"), parse("3.6.0"));
        assert!(parse("0.0.0") < parse("3.6.6"));
        assert!(DottedVersion::parse("3..6").is_none());
        assert!(DottedVersion::parse("1.2.3.4.5.6.7.8.9").is_none());
        assert!(DottedVersion::parse("3.6.6-rc1").is_none());
        assert!(DottedVersion::parse("").is_none());
    }
}
