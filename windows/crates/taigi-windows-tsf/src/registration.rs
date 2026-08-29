//! `DllRegisterServer` / `DllUnregisterServer`: the CLSID key, the language
//! profile and the categories, registered in that order and unregistered
//! item by item (roadmap W7; rakukan `registration.rs:80-143`, khiin
//! `reg/registrar.rs`). Only the categories that are TRUE are declared
//! (Codex W6): keyboard, display-attribute provider, tray support, UI-less
//! candidate list. NOT COMLESS / IMMERSIVESUPPORT / INPUTMODECOMPARTMENT.

// 中文: 註冊/解除註冊 — CLSID、語言 profile、四個真實類別;解除逐項對稱。

use crate::guids::{CLSID_TEXT_SERVICE, GUID_PROFILE, LANGID_ZH_TW};
use crate::module::module_path;
use crate::registry::{delete_tree, Key};
use crate::wide::to_wide_nul;
use windows::core::{Error, Interface, Result, GUID};
use windows::Win32::Foundation::E_UNEXPECTED;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::System::Registry::HKEY_CLASSES_ROOT;
use windows::Win32::UI::Input::KeyboardAndMouse::HKL;
use windows::Win32::UI::TextServices::{
    CLSID_TF_CategoryMgr, CLSID_TF_InputProcessorProfiles, ITfCategoryMgr,
    ITfInputProcessorProfileMgr, ITfInputProcessorProfiles, GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
    GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT, GUID_TFCAT_TIPCAP_UIELEMENTENABLED, GUID_TFCAT_TIP_KEYBOARD,
};

/// What the CLSID key and the profile are named. The profile description is
/// what Windows Settings lists under 中文(台灣) — the product name, hanji
/// first like the macOS `CFBundleName`.
pub const SERVICE_DESCRIPTION: &str = "台語齒盤 Taigi Keyboard";

/// STACKED-PR NOTE: the display-attribute provider lands with PR5b and the
/// UI-less candidate list with PR6; this DLL is only installed as the
/// complete train (PR10), so the two categories are declared here once
/// rather than staged.
const CATEGORIES: [GUID; 4] = [
    GUID_TFCAT_TIP_KEYBOARD,
    GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
    GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT,
    GUID_TFCAT_TIPCAP_UIELEMENTENABLED,
];

/// `{XXXXXXXX-XXXX-…}` as the registry spells a CLSID.
pub fn guid_key(guid: &GUID) -> String {
    format!("{{{guid:?}}}")
}

pub fn register_server() -> Result<()> {
    let dll_path = module_path().ok_or_else(|| Error::from_hresult(E_UNEXPECTED))?;
    let dll_path = dll_path.to_string_lossy().into_owned();
    register_clsid(&dll_path)?;
    log::info!("tsf.register step=clsid");
    register_profile(&dll_path)?;
    log::info!("tsf.register step=profile");
    register_categories()?;
    log::info!("tsf.register step=categories");
    Ok(())
}

/// Every step is attempted; the first failure is reported after the rest
/// ran, so a half-registered install is not left behind by an early return.
pub fn unregister_server() -> Result<()> {
    let categories = unregister_categories();
    let profile = unregister_profile();
    let clsid = delete_tree(
        HKEY_CLASSES_ROOT,
        &format!("CLSID\\{}", guid_key(&CLSID_TEXT_SERVICE)),
    );
    categories.and(profile).and(clsid)
}

/// `HKCR\CLSID\{clsid}` = description; `\InProcServer32` = DLL path,
/// `ThreadingModel` = `Apartment` (TSF is STA).
fn register_clsid(dll_path: &str) -> Result<()> {
    let clsid_path = format!("CLSID\\{}", guid_key(&CLSID_TEXT_SERVICE));
    Key::create(HKEY_CLASSES_ROOT, &clsid_path)?.set_string("", SERVICE_DESCRIPTION)?;
    let inproc = Key::create(HKEY_CLASSES_ROOT, &format!("{clsid_path}\\InProcServer32"))?;
    inproc.set_string("", dll_path)?;
    inproc.set_string("ThreadingModel", "Apartment")
}

/// `Register` first (the CLSID must be known to TSF), then `RegisterProfile`
/// on the manager interface (rakukan `registration.rs:80-115`). The icon is
/// the DLL's own resource, index 0 — added with the installer (PR10); until
/// then Windows shows its generic keyboard glyph.
fn register_profile(dll_path: &str) -> Result<()> {
    let description = to_wide_nul(SERVICE_DESCRIPTION);
    let icon_file = to_wide_nul(dll_path);
    // SAFETY: TSF's own registration objects, created and used on the
    // regsvr32 thread inside the apartment `with_apartment` opened; every
    // pointer argument is a live local.
    unsafe {
        let profiles: ITfInputProcessorProfiles =
            CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?;
        profiles.Register(&CLSID_TEXT_SERVICE)?;
        let manager: ITfInputProcessorProfileMgr = profiles.cast()?;
        manager.RegisterProfile(
            &CLSID_TEXT_SERVICE,
            LANGID_ZH_TW,
            &GUID_PROFILE,
            &description,
            &icon_file,
            0,
            HKL::default(),
            0,
            true,
            0,
        )
    }
}

fn unregister_profile() -> Result<()> {
    // SAFETY: as `register_profile`; failures are collected, not fatal.
    unsafe {
        let profiles: ITfInputProcessorProfiles =
            CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?;
        let by_manager = profiles
            .cast::<ITfInputProcessorProfileMgr>()
            .and_then(|manager| {
                manager.UnregisterProfile(&CLSID_TEXT_SERVICE, LANGID_ZH_TW, &GUID_PROFILE, 0)
            });
        let by_clsid = profiles.Unregister(&CLSID_TEXT_SERVICE);
        by_manager.and(by_clsid)
    }
}

fn register_categories() -> Result<()> {
    // SAFETY: as `register_profile`.
    unsafe {
        let manager: ITfCategoryMgr =
            CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)?;
        for category in &CATEGORIES {
            manager.RegisterCategory(&CLSID_TEXT_SERVICE, category, &CLSID_TEXT_SERVICE)?;
        }
    }
    Ok(())
}

fn unregister_categories() -> Result<()> {
    // SAFETY: as `register_profile`; every category is attempted.
    unsafe {
        let manager: ITfCategoryMgr =
            CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)?;
        let mut first_error = None;
        for category in &CATEGORIES {
            if let Err(error) =
                manager.UnregisterCategory(&CLSID_TEXT_SERVICE, category, &CLSID_TEXT_SERVICE)
            {
                first_error.get_or_insert(error);
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clsid_spells_the_way_the_registry_reads_it() {
        assert_eq!(
            guid_key(&CLSID_TEXT_SERVICE),
            "{32C28A51-8939-4C8F-8F29-037F9FD3CF0A}"
        );
        assert_eq!(CATEGORIES.len(), 4, "only the true categories (Codex W6)");
    }
}
