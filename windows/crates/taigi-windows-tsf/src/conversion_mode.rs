//! Publishing the 中/英 mode where the rest of Windows can read it.
//!
//! `GUID_COMPARTMENT_KEYBOARD_INPUTMODE_CONVERSION` is the thread-manager
//! compartment TSF defines for an input method's conversion flags:
//! `TF_CONVERSIONMODE_NATIVE` set means the service is composing in its own
//! script, cleared means it is passing alphanumeric text through
//! (<https://learn.microsoft.com/en-us/windows/win32/tsf/flags-for-conversion-mode>).
//! Our own tray letter and mode flash tell the USER which mode is on; this is
//! what tells an application, a framework or an accessibility client, which
//! ask TSF rather than look at our window.
//!
//! Only the `NATIVE` bit is ours: the value is read back and rewritten so any
//! other flag someone else set (full-width, symbol) survives the switch.
//!
//! NAMED SIMPLIFICATION: write-only. A compartment-change sink would let a
//! host drive the mode from outside; nothing we ship does that, and the sink
//! is a second source of truth to keep in step (Codex F7, 2026-09-04).

use taigi_windows_core::keys::LanguageMode;
use windows::core::Interface;
use windows::Win32::System::Variant::{VARIANT, VT_I4};
use windows::Win32::UI::TextServices::{
    ITfCompartment, ITfCompartmentMgr, ITfThreadMgr,
    GUID_COMPARTMENT_KEYBOARD_INPUTMODE_CONVERSION, TF_CONVERSIONMODE_NATIVE,
};

/// Writes `mode` into the conversion-mode compartment. Cosmetic to typing:
/// every failure is logged and swallowed, because a host that refuses the
/// compartment must still be able to compose.
pub fn publish(thread_mgr: &ITfThreadMgr, client_id: u32, mode: LanguageMode) {
    let compartment = match compartment(thread_mgr) {
        Ok(compartment) => compartment,
        Err(error) => {
            log::warn!("conversion_mode.compartment_unavailable error={error}");
            return;
        }
    };
    let flags = next_flags(read_flags(&compartment), mode);
    // SAFETY: the compartment is the thread manager's own; the value is the
    // crate's own `VT_I4` VARIANT, which owns nothing the callee must release.
    if let Err(error) = unsafe { compartment.SetValue(client_id, &VARIANT::from(flags as i32)) } {
        log::warn!("conversion_mode.set_failed error={error}");
        return;
    }
    log::debug!("conversion_mode.published mode={mode:?} flags={flags:#x}");
}

fn compartment(thread_mgr: &ITfThreadMgr) -> windows::core::Result<ITfCompartment> {
    let manager: ITfCompartmentMgr = thread_mgr.cast()?;
    // SAFETY: the GUID is a static constant; the manager is the live thread
    // manager's interface.
    unsafe { manager.GetCompartment(&GUID_COMPARTMENT_KEYBOARD_INPUTMODE_CONVERSION) }
}

/// The flags currently in the compartment, or none at all — an empty
/// compartment reads as `VT_EMPTY`, which is zero conversion flags.
fn read_flags(compartment: &ITfCompartment) -> u32 {
    // SAFETY: reading the compartment's own value.
    let Ok(value) = (unsafe { compartment.GetValue() }) else {
        return 0;
    };
    if value.vt() != VT_I4 {
        return 0;
    }
    // SAFETY: the tag was just read, so the `VT_I4` arm — a plain `i32` — is
    // the live one. An arm that owned a resource would still be released: the
    // `windows` crate implements `Drop` for `VARIANT` over `VariantClear`
    // (`extensions/Win32/System/Variant.rs:33`).
    unsafe { value.Anonymous.Anonymous.Anonymous.lVal as u32 }
}

/// `current` with only the native-mode bit moved to what `mode` says.
fn next_flags(current: u32, mode: LanguageMode) -> u32 {
    match mode {
        LanguageMode::Taigi => current | TF_CONVERSIONMODE_NATIVE,
        LanguageMode::English => current & !TF_CONVERSIONMODE_NATIVE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flag this input method does not own — full-width mode is another
    /// service's business, and a 中/英 switch may not drop it.
    const OTHER_FLAG: u32 = 0x0008;

    #[test]
    fn taigi_sets_the_native_bit_and_english_clears_it() {
        assert_eq!(next_flags(0, LanguageMode::Taigi), TF_CONVERSIONMODE_NATIVE);
        assert_eq!(
            next_flags(TF_CONVERSIONMODE_NATIVE, LanguageMode::English),
            0
        );
    }

    #[test]
    fn flags_that_are_not_ours_survive_both_directions() {
        let english = next_flags(TF_CONVERSIONMODE_NATIVE | OTHER_FLAG, LanguageMode::English);
        assert_eq!(english, OTHER_FLAG);
        assert_eq!(
            next_flags(english, LanguageMode::Taigi),
            TF_CONVERSIONMODE_NATIVE | OTHER_FLAG
        );
    }
}
