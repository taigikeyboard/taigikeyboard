//! `LexiconError` — typed errors for lexicon ops.
//!
//! Maps to `engine/protos/proto/envelope.proto::ErrorCode` at the dispatch
//! boundary. Engine NEVER panics on user-supplied paths or queries; every
//! failure mode appears here.

// 中文: lexicon 操作的型別化錯誤;在 dispatch 層映射到 envelope.proto::ErrorCode。引擎絕不對使用者輸入 panic。

// 中文: lexicon 各類錯誤 enum;新增變體時記得同步更新 as_proto_error_code 對應。
#[derive(thiserror::Error, Debug)]
pub enum LexiconError {
    // 中文: 路徑含 NUL byte 不可作為檔案路徑使用。
    #[error("path `{0}` contains a NUL byte")]
    InvalidPath(String),

    // 中文: 路徑非絕對路徑;引擎不做 canonicalize,要求平台傳入完整路徑。
    #[error("path `{0}` is not absolute")]
    PathNotAbsolute(String),

    // 中文: mmap 開檔失敗 (檔案不存在 / 權限 / I/O 錯誤等)。
    #[error("mmap `{path}`: {source}")]
    Mmap {
        path: String,
        #[source]
        source: mmap_host::MmapError,
    },

    // 中文: 尚未呼叫 install,EngineHandle 全域狀態為空。
    #[error("not initialized — call install before search")]
    NotInitialized,

    // 中文: 二進位格式錯誤 (magic / 版本 / offset 表 / record 長度)。
    #[error("invalid binary format: {0}")]
    InvalidBinary(String),

    // 中文: 內部不應發生的錯誤 (mutex poisoned、狀態欄位缺失等)。
    #[error("internal lexicon error: {0}")]
    Internal(String),
}

/// Map `LexiconError` to `engine/protos::ErrorCode`. Caller wraps the
/// proto envelope around this.
// 中文: 將 LexiconError 映射到 envelope.proto 的 ErrorCode 整數值。
impl LexiconError {
    // 中文: 回傳對應的 proto ErrorCode (FAIL_PARSE=1 / FAIL_INTERNAL=2 / FAIL_IO=3 / FAIL_INVARIANT=4)。
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
