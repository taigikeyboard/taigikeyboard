//! The network, behind two traits so the checker and the installation can
//! be tested without it. The one implementation is `ureq` over the system
//! TLS stack. Port of `UpdateHTTP` + `UpdateManifest.fetchPublished` +
//! `UpdatePackageDownload.run`: HTTPS only, 200 only, a size ceiling on
//! both bodies, timeouts a stalled server cannot stretch.

// 中文: 網路傳輸 — manifest 抓取與安裝檔下載,走 trait,實作用 ureq。

use crate::manifest::{ManifestError, UpdateManifest, MAXIMUM_MANIFEST_BYTES, PUBLISHED_URL};
use std::io::Write;
use std::path::Path;
use std::time::Duration;
use ureq::ResponseExt;

/// `UpdatePackageDownload.maximumPackageBytes`.
pub const MAXIMUM_PACKAGE_BYTES: u64 = 200 * 1024 * 1024;

#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum FetchError {
    #[error("the server did not answer 200 over HTTPS")]
    Rejected,
    #[error("the answer is larger than allowed")]
    TooLarge,
    #[error("{0}")]
    Manifest(#[from] ManifestError),
    #[error("{0}")]
    Transport(String),
}

pub trait ManifestFetcher: Send + Sync {
    fn fetch_published(&self) -> Result<UpdateManifest, FetchError>;
}

pub trait PackageDownloader: Send + Sync {
    /// Streams `url` into `destination`, whole or not at all.
    fn download(&self, url: &str, destination: &Path) -> Result<(), FetchError>;
}

/// `ureq` with the macOS session's timeouts: 5 s to connect and 15 s in all
/// for the manifest; 30 s to connect and 20 minutes in all for a package.
#[derive(Clone, Copy, Debug, Default)]
pub struct HttpTransport;

impl HttpTransport {
    /// The system TLS stack (schannel on Windows) with the SYSTEM roots:
    /// ureq's default provider is rustls, and the `native-tls` feature
    /// alone does not switch it — both are chosen here, explicitly.
    fn agent(connect: Duration, total: Duration) -> ureq::Agent {
        let tls = ureq::tls::TlsConfig::builder()
            .provider(ureq::tls::TlsProvider::NativeTls)
            .root_certs(ureq::tls::RootCerts::PlatformVerifier)
            .build();
        ureq::Agent::config_builder()
            .tls_config(tls)
            .timeout_connect(Some(connect))
            .timeout_global(Some(total))
            .http_status_as_error(false)
            .build()
            .new_agent()
    }

    /// 200, and the final URL (after redirects) still `https`.
    fn is_acceptable(response: &ureq::http::Response<ureq::Body>) -> bool {
        response.status() == ureq::http::StatusCode::OK
            && response
                .get_uri()
                .scheme_str()
                .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https"))
    }
}

impl ManifestFetcher for HttpTransport {
    fn fetch_published(&self) -> Result<UpdateManifest, FetchError> {
        let agent = Self::agent(Duration::from_secs(5), Duration::from_secs(15));
        let mut response = agent
            .get(PUBLISHED_URL)
            .call()
            .map_err(|error| FetchError::Transport(error.to_string()))?;
        if !Self::is_acceptable(&response) {
            return Err(FetchError::Rejected);
        }
        // One byte over the ceiling is enough to know (`limit` errors at
        // the limit rather than truncating): read ceiling + 1, judge after.
        let bytes = response
            .body_mut()
            .with_config()
            .limit(MAXIMUM_MANIFEST_BYTES + 1)
            .read_to_vec()
            .map_err(|error| FetchError::Transport(error.to_string()))?;
        if bytes.len() as u64 > MAXIMUM_MANIFEST_BYTES {
            return Err(FetchError::TooLarge);
        }
        Ok(UpdateManifest::decode(&bytes)?)
    }
}

impl PackageDownloader for HttpTransport {
    fn download(&self, url: &str, destination: &Path) -> Result<(), FetchError> {
        let agent = Self::agent(Duration::from_secs(30), Duration::from_secs(20 * 60));
        let mut response = agent
            .get(url)
            .call()
            .map_err(|error| FetchError::Transport(error.to_string()))?;
        if !Self::is_acceptable(&response) {
            return Err(FetchError::Rejected);
        }
        // One byte over the ceiling is enough to know: the copy is capped
        // at ceiling + 1 and the count checked after.
        let mut reader = response
            .body_mut()
            .with_config()
            .limit(MAXIMUM_PACKAGE_BYTES + 1)
            .reader();
        let mut file = std::fs::File::create(destination)
            .map_err(|error| FetchError::Transport(error.to_string()))?;
        let copied = std::io::copy(&mut reader, &mut file)
            .and_then(|copied| file.flush().map(|()| copied))
            .map_err(|error| FetchError::Transport(error.to_string()))?;
        if copied > MAXIMUM_PACKAGE_BYTES {
            std::fs::remove_file(destination).ok();
            return Err(FetchError::TooLarge);
        }
        Ok(())
    }
}
