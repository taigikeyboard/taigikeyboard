//! `LexiconError` — typed errors for lexicon ops.
//!
//! `engine/dispatch` (`lexicon_error_code`) maps each variant to
//! `envelope.proto::ErrorCode`. Engine NEVER panics on user-supplied paths or
//! queries; every failure mode appears here.

#[derive(thiserror::Error, Debug)]
pub enum LexiconError {
    #[error("path `{0}` contains a NUL byte")]
    InvalidPath(String),

    // Engine does not canonicalize; the platform must pass a full path.
    #[error("path `{0}` is not absolute")]
    PathNotAbsolute(String),

    #[error("mmap `{path}`: {source}")]
    Mmap {
        path: String,
        #[source]
        source: mmap_host::MmapError,
    },

    #[error("not initialized — call install before search")]
    NotInitialized,

    #[error("invalid binary format: {0}")]
    InvalidBinary(String),

    #[error("internal lexicon error: {0}")]
    Internal(String),
}
