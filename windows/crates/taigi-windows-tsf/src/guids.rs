//! Every GUID this text service owns. Allocated 2026-08-29 (roadmap Phase
//! 0); recorded in `memory/project_windows_ime.md`. Never reuse another
//! IME's.

use windows::core::GUID;

/// The COM class the host creates (`CLSID\{…}\InProcServer32`).
pub const CLSID_TEXT_SERVICE: GUID = GUID::from_u128(0x32C28A51_8939_4C8F_8F29_037F9FD3CF0A);
/// The one language profile, under LANGID 0x0404.
pub const GUID_PROFILE: GUID = GUID::from_u128(0xDCC522F5_B3FC_4D81_98DD_E4E0FD965DC8);
/// The display attribute the composition's preedit is drawn with (PR5b).
pub const GUID_DISPLAY_ATTRIBUTE_INPUT: GUID =
    GUID::from_u128(0xEE6EAB46_FF70_47D6_AEEA_F1E1C3C40234);
/// The UI-less candidate list element (`ITfUIElement::GetGUID`).
pub const GUID_CANDIDATE_UI_ELEMENT: GUID = GUID::from_u128(0xA4C87E11_DF98_4062_BC51_327C4C2043E9);
/// Preserved key: open the settings window (PR5b registers it).
pub const GUID_PRESERVED_KEY_SETTINGS: GUID =
    GUID::from_u128(0x1CED3F86_EFD5_4332_B166_68A1763E98CA);
/// Preserved key: TL ↔ POJ (PR5b registers it).
pub const GUID_PRESERVED_KEY_ROMANIZATION: GUID =
    GUID::from_u128(0x566EAAED_32AC_422E_9ACE_2A2A30EBC413);
/// Preserved key: cycle 候選詞顯示 (allocated 2026-09-02 for the shortcut).
pub const GUID_PRESERVED_KEY_CYCLE_CANDIDATE_DISPLAY_MODE: GUID =
    GUID::from_u128(0xD9F5E060_8D5F_4A05_86AF_CF6B2F5F9EB6);

/// Traditional Chinese (Taiwan). The ONLY profile language (roadmap W7,
/// Codex F5): no duplicate under en-US.
pub const LANGID_ZH_TW: u16 = 0x0404;
