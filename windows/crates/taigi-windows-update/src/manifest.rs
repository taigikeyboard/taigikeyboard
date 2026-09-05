//! The published manifest and the version it names. Port of
//! `UpdateManifest` / `DottedVersion` (`UpdateChecker.swift:1-120`); the
//! wire format is `windows/updates/README.md` § Wire format — the macOS
//! manifest's twin but for the file name and ONE added field,
//! `packageSHA256`, which the Mac has no use for: it pins a downloaded
//! package by its Developer ID signature.

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
    /// The installer to fetch and the digest it must hash to. One fact, so
    /// one field: neither half is usable alone, and a manifest carrying only
    /// one of them (or an invalid one) reads as no package at all — the
    /// update is still announced, with the download page as its action.
    pub package: Option<PublishedPackage>,
}

/// What an in-app install downloads, and what admits it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublishedPackage {
    /// `https` only.
    pub url: String,
    /// 64 lowercase hex.
    pub sha256: String,
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
    #[serde(
        rename = "packageSHA256",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    package_sha256: Option<String>,
}

/// A SHA-256 as it is published: 64 hex digits, taken case-insensitively and
/// kept lowercase so a comparison is a string comparison.
pub(crate) fn normalized_sha256(digest: &str) -> Option<String> {
    let digest = digest.trim();
    (digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| digest.to_ascii_lowercase())
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

/// Both halves, both valid, or nothing.
fn published_package(url: Option<String>, digest: Option<&str>) -> Option<PublishedPackage> {
    let url = url.filter(|url| is_https(url))?;
    Some(PublishedPackage {
        url,
        sha256: normalized_sha256(digest?)?,
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
            package: published_package(wire.package_url, wire.package_sha256.as_deref()),
        })
    }

    /// The same shape back, for `updatePendingManifest`.
    pub fn encode(&self) -> String {
        serde_json::to_string(&Wire {
            version: self.version.clone(),
            download_page_url: self.download_page_url.clone(),
            package_url: self.package.as_ref().map(|package| package.url.clone()),
            package_sha256: self.package.as_ref().map(|package| package.sha256.clone()),
        })
        .expect("plain strings always serialize")
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

    fn wire(package: &str) -> String {
        format!(
            r#"{{"version":"3.7.0","downloadPageURL":"https://taigikeyboard.tw/",{package}"extra":1}}"#
        )
    }

    const DIGEST: &str = "b2b3a11c02f14e36f2a5c148db1b5924fa96141da4b2fbfb262a469df53f6750";

    #[test]
    fn a_package_is_both_halves_or_neither_and_the_digest_is_case_insensitive() {
        // trace: published_package — a package is a URL AND its digest, so
        // half of one is none of one. Either half being absent or invalid
        // leaves the manifest itself valid (the update is still announced,
        // with the download page as its action), the way an invalid
        // packageURL alone already did.
        let both = UpdateManifest::decode(
            wire(&format!(
                r#""packageURL":"https://x/y.exe","packageSHA256":"{}","#,
                DIGEST.to_ascii_uppercase()
            ))
            .as_bytes(),
        )
        .unwrap();
        assert_eq!(both.version, "3.7.0");
        assert_eq!(
            both.package,
            Some(PublishedPackage {
                url: "https://x/y.exe".to_owned(),
                sha256: DIGEST.to_owned(),
            }),
            "a digest published in upper case is kept lowercase"
        );
        assert_eq!(
            UpdateManifest::decode(both.encode().as_bytes()).unwrap(),
            both,
            "the pending manifest in settings.json round trips both halves"
        );

        for half in [
            String::new(),
            r#""packageURL":"https://x/y.exe","#.to_owned(),
            format!(r#""packageSHA256":"{DIGEST}","#),
            format!(r#""packageURL":"http://x/y.exe","packageSHA256":"{DIGEST}","#),
            r#""packageURL":"https://x/y.exe","packageSHA256":"abc","#.to_owned(),
            format!(
                r#""packageURL":"https://x/y.exe","packageSHA256":"{}","#,
                "z".repeat(64)
            ),
            format!(
                r#""packageURL":"https://x/y.exe","packageSHA256":"{}","#,
                "a".repeat(63)
            ),
        ] {
            let manifest = UpdateManifest::decode(wire(&half).as_bytes())
                .unwrap_or_else(|_| panic!("half a package must not fail the manifest: {half}"));
            assert_eq!(manifest.package, None, "{half}");
        }
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
