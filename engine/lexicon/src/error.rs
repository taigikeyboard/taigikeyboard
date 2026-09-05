//! `LexiconError` — typed errors for lexicon ops.
//!
//! Maps to `engine/protos/proto/envelope.proto::ErrorCode` at the dispatch
//! boundary. Engine NEVER panics on user-supplied paths or queries; every
//! failure mode appears here.

// Adding a variant? Update `as_proto_error_code` below to match.
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

/// Map `LexiconError` to `engine/protos::ErrorCode`. Caller wraps the
/// proto envelope around this.
impl LexiconError {
    pub fn as_proto_error_code(&self) -> i32 {
        // Manual integer mapping to avoid cross-crate enum dependency
        // (envelope.proto::ErrorCode lives in protos crate). Values
        // match envelope.proto literally:
        //   FAIL_PARSE = 1, FAIL_INTERNAL = 2, FAIL_IO = 3, FAIL_INVARIANT = 4
        match self {
            Self::InvalidPath(_) | Self::PathNotAbsolute(_) | Self::InvalidBinary(_) => 1,
            Self::Mmap { .. } => 3,
            Self::NotInitialized => 4,
            Self::Internal(_) => 2,
        }
    }
}
