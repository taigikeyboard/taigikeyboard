//! Compute the display form for a raw composition buffer.
//!
//! TPS inputs are already display-ready (return as-is). POJ/TL inputs go
//! through the full `phonetics::api::normalize_tone` chain (POJ doubletap
//! preprocessing → tone-mark application → nasal-marker case adjustment) per
//! plan §3.2a. The platform `RustEngineBridge.normalizeTone` call sites are
//! replaced by this in-process call.

use protos::engine::AppConfig;

pub(crate) fn derived_display(raw: &str, config: &AppConfig) -> String {
    if raw.is_empty() {
        return String::new();
    }
    if phonetics::api::contains_tps(raw) {
        return raw.to_string();
    }
    phonetics::api::normalize_tone(raw, config)
}
