//! Pure document-text policies the controller applies around a commit:
//! auto-space and full-width punctuation. Port of
//! `macos/Sources/TaigiInputMethodCore/Controller/{AutoSpacePolicy,
//! AutoSpacePunctuation,FullWidthPunctuation}.swift`.

// 送出前後的純文字政策 — 自動空白與全形標點。

mod auto_space;
mod full_width;

pub use auto_space::{
    augment_insert, is_attaching_punctuation, is_gate_active, raw_preedit_writes_romanization,
    should_append_space, AugmentedInsert,
};
pub use full_width::full_width_mapped;
