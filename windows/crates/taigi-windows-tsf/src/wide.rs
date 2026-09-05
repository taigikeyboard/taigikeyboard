//! UTF-16 conversions for the Win32 calls that want them.

// UTF-16 轉換小工具。

/// `text` as NUL-terminated UTF-16 — what `RegisterProfile`, `PreserveKey`,
/// `AppendMenuW` and the registry want.
pub fn to_wide_nul(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// `text` as UTF-16 WITHOUT the terminator — what `ITfRange::SetText` and
/// DirectWrite want (they take a length).
pub fn to_wide(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

/// Copies `text` into a fixed UTF-16 field, truncating to leave the NUL.
pub fn fill_fixed(field: &mut [u16], text: &str) {
    let limit = field.len().saturating_sub(1);
    field.fill(0);
    for (slot, unit) in field.iter_mut().zip(text.encode_utf16().take(limit)) {
        *slot = unit;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions() {
        assert_eq!(to_wide_nul("台"), [0x53F0, 0]);
        assert_eq!(to_wide("ab"), [0x61, 0x62]);
        let mut field = [0xFFFFu16; 4];
        fill_fixed(&mut field, "abcdef");
        assert_eq!(field, [0x61, 0x62, 0x63, 0], "truncated, NUL kept");
    }
}
