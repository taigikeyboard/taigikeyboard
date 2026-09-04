//! `DllRegisterServer` / `DllUnregisterServer`: the CLSID key, the language
//! profile and the categories, registered in that order and unregistered
//! item by item (roadmap W7; rakukan `registration.rs:80-143`, khiin
//! `reg/registrar.rs`). Only the categories that are TRUE are declared
//! (Codex W6): keyboard, display-attribute provider, tray support, UI-less
//! candidate list. NOT COMLESS / IMMERSIVESUPPORT / INPUTMODECOMPARTMENT.

// 中文: 註冊/解除註冊 — CLSID、語言 profile、四個真實類別;解除逐項對稱。

use crate::com_out_buffer;
use crate::guids::{CLSID_TEXT_SERVICE, GUID_PROFILE, LANGID_ZH_TW};
use crate::module::module_path;
use crate::registry::{delete_tree, Key};
use crate::wide::to_wide_nul;
use windows::core::{Error, Interface, Result, GUID, HRESULT};
use windows::Win32::Foundation::{E_FAIL, E_UNEXPECTED, S_FALSE, S_OK};
use windows::Win32::System::Com::{CoCreateInstance, IEnumGUID, CLSCTX_INPROC_SERVER};
use windows::Win32::System::Registry::HKEY_CLASSES_ROOT;
use windows::Win32::UI::Input::KeyboardAndMouse::HKL;
use windows::Win32::UI::TextServices::{
    CLSID_TF_CategoryMgr, CLSID_TF_InputProcessorProfiles, ITfCategoryMgr,
    ITfInputProcessorProfileMgr, ITfInputProcessorProfiles, GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
    GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT, GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT,
    GUID_TFCAT_TIPCAP_UIELEMENTENABLED, GUID_TFCAT_TIP_KEYBOARD, TF_INPUTPROCESSORPROFILE,
};

/// What the CLSID key is named, and what the language bar falls back to when
/// the DLL carries no string resources (a build without a resource
/// compiler). Untranslated on purpose — the Mac's `CFBundleName` — because the
/// OS-visible name is localized: see [`profile_description`].
pub const SERVICE_DESCRIPTION: &str = "TaigiKeyboard";

/// The STRINGTABLE id the build script stores the localized product name
/// under, in every language `product_name_strings.rs` carries
/// (`build-support/resource.rs`, exported as this env var so the two cannot
/// drift).
pub const PRODUCT_NAME_STRING_ID: &str = env!("TAIGI_PRODUCT_NAME_STRING_ID");

/// The profile description Windows Settings and the language bar show —
/// an indirect string, `@<dll>,-<id>`, which the shell resolves against the
/// user's UI language every time it is displayed (Microsoft's own IMEs
/// register theirs this way). The twin of the Mac's `.lproj/InfoPlist.strings`
/// (#613): the name follows the SYSTEM language, not the app's display-language
/// picker.
pub fn profile_description(dll_path: &str) -> String {
    format!("@{dll_path},-{PRODUCT_NAME_STRING_ID}")
}

/// STACKED-PR NOTE: the display-attribute provider lands with PR5b and the
/// UI-less candidate list with PR6; this DLL is only installed as the
/// complete train (PR10), so the two categories are declared here once
/// rather than staged.
const CATEGORIES: [GUID; 5] = [
    GUID_TFCAT_TIP_KEYBOARD,
    GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
    GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT,
    GUID_TFCAT_TIPCAP_UIELEMENTENABLED,
    // True since the Shift tap gave this service a 中/英 mode: it keeps
    // `GUID_COMPARTMENT_KEYBOARD_INPUTMODE_CONVERSION` current
    // (`conversion_mode.rs`). Declared only because it IS true — the roadmap
    // refused it while there was no mode to publish.
    GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT,
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
    // Remove everything, then judge by what is left rather than by what the
    // removals returned. TSF answers a bare `E_FAIL` (0x80004005) both for a
    // real failure and for a thing that was never there — measured on Windows
    // 11: a second `regsvr32 /u` has `UnregisterCategory` and
    // `UnregisterProfile` fail that way while the state is correctly empty —
    // so an HRESULT here cannot tell "nothing to do" from "could not do it",
    // and no allowlist would be honest. Only the postcondition can.
    verify_unregistered([
        unregister_categories(),
        unregister_profile(),
        delete_tree(
            HKEY_CLASSES_ROOT,
            &format!("CLSID\\{}", guid_key(&CLSID_TEXT_SERVICE)),
        ),
    ])
}

/// What "unregistered" means, asked rather than assumed: TSF no longer knows
/// this text service, no longer knows its language profile, it belongs to no
/// category, and the CLSID tree is gone. This is what makes `regsvr32 /u`
/// idempotent — a second run finds nothing to remove, is told so in the only
/// vocabulary TSF has, and still reports success because the end state is the
/// one it promised.
///
/// Every probe is three-valued. "I could not find out" is never "it is gone":
/// an unreachable category manager or a registry read that fails on anything
/// but "not found" leaves the service possibly registered, and saying
/// otherwise would be the same lie the HRESULTs tell.
fn verify_unregistered(removals: [Result<()>; 3]) -> Result<()> {
    let mut remaining: Vec<String> = Vec::new();
    let mut record = |what: &str, probe: Result<bool>| match probe {
        Ok(true) => remaining.push(what.to_owned()),
        Ok(false) => {}
        Err(error) => remaining.push(format!("{what} (could not be checked: {error})")),
    };
    record("text service", service_is_registered());
    record("language profile", profile_is_registered());
    record(
        "category membership",
        registered_category_count().map(|count| count > 0),
    );
    record(
        "CLSID registration",
        Key::exists(
            HKEY_CLASSES_ROOT,
            &format!("CLSID\\{}", guid_key(&CLSID_TEXT_SERVICE)),
        ),
    );
    if remaining.is_empty() {
        return Ok(());
    }
    // Only now are the removal HRESULTs worth anything: with state left over,
    // why a removal complained is the first thing a maintainer wants.
    for (step, result) in ["categories", "profile", "clsid"].iter().zip(&removals) {
        if let Err(error) = result {
            log::error!("tsf.unregister.step_failed step={step} error={error}");
        }
    }
    let remaining = remaining.join(", ");
    log::error!("tsf.unregister.incomplete remaining={remaining}");
    Err(Error::new(
        E_FAIL,
        format!("still registered after unregistering: {remaining}"),
    ))
}

/// Whether TSF still lists this CLSID as a text service. `RegisterProfile` is
/// not the only registration `register_server` makes — `Register` puts the
/// service itself in TSF's list — so an absent profile alone does not prove
/// the service is gone.
fn service_is_registered() -> Result<bool> {
    // SAFETY: as `register_profile`; the enumerator is TSF's own and is only
    // read through the checked helper below.
    unsafe {
        let profiles: ITfInputProcessorProfiles =
            CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?;
        let services = profiles.EnumInputProcessorInfo()?;
        Ok(enumerated_guids(&services)?.contains(&CLSID_TEXT_SERVICE))
    }
}

/// Whether TSF still knows our language profile, found by enumerating rather
/// than by asking for it: `GetProfile` answers a bare `E_FAIL` both for a
/// profile that is absent and for one it could not look up, so its failure
/// proves nothing either way.
fn profile_is_registered() -> Result<bool> {
    // SAFETY: as `register_profile`. `batch` is written for as many entries as
    // `fetched` reports and only that many are read.
    unsafe {
        let profiles: ITfInputProcessorProfiles =
            CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?;
        let manager: ITfInputProcessorProfileMgr = profiles.cast()?;
        let enumerator = manager.EnumProfiles(LANGID_ZH_TW)?;
        loop {
            let mut batch = [TF_INPUTPROCESSORPROFILE::default(); 8];
            let mut fetched = 0u32;
            // This call answers `Result<()>`, so `S_FALSE` and `S_OK` are
            // both `Ok` and only the count says whether the enumeration is
            // done. A failure still propagates — an enumeration that broke is
            // not an enumeration that ended. Through `com_out_buffer`, since
            // `batch` is read back.
            com_out_buffer::next_input_processor_profiles(&enumerator, &mut batch, &mut fetched)?;
            let fetched = usize::try_from(fetched).unwrap_or(0);
            if batch[..fetched].iter().any(|profile| {
                profile.clsid == CLSID_TEXT_SERVICE && profile.guidProfile == GUID_PROFILE
            }) {
                return Ok(true);
            }
            if fetched == 0 {
                return Ok(false);
            }
        }
    }
}

/// How many categories this text service still belongs to — asked of the
/// category manager rather than of the registry, because where TSF keeps
/// category membership is its own business.
fn registered_category_count() -> Result<usize> {
    // SAFETY: as `register_categories`.
    unsafe {
        let manager: ITfCategoryMgr =
            CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)?;
        Ok(enumerated_guids(&manager.EnumCategoriesInItem(&CLSID_TEXT_SERVICE)?)?.len())
    }
}

/// Drains an `IEnumGUID`, refusing to call a failed enumeration an empty one.
fn enumerated_guids(enumerator: &IEnumGUID) -> Result<Vec<GUID>> {
    let mut all = Vec::new();
    loop {
        let mut batch = [GUID::zeroed(); 8];
        let mut fetched = 0u32;
        // SAFETY: `batch` is written for as many entries as `fetched` reports.
        // Through `com_out_buffer` — the generated wrapper hands the
        // enumerator a read-only pointer for a batch we read back.
        let status = unsafe { com_out_buffer::next_guids(enumerator, &mut batch, &mut fetched) };
        let fetched = usize::try_from(fetched).unwrap_or(0);
        all.extend_from_slice(&batch[..fetched]);
        if !enumeration_continues(status, fetched)? {
            return Ok(all);
        }
    }
}

/// The COM enumerator contract: `S_OK` filled the batch and there may be more,
/// `S_FALSE` is the last (possibly partial) one, and any failure means the
/// enumeration did not finish — which must not be mistaken for reaching the
/// end, since that is exactly how a broken probe reports "nothing there".
fn enumeration_continues(status: HRESULT, fetched: usize) -> Result<bool> {
    if status == S_OK {
        // A conforming enumerator fills the batch on S_OK; a zero fetch would
        // otherwise spin forever.
        return Ok(fetched > 0);
    }
    if status == S_FALSE {
        return Ok(false);
    }
    Err(Error::from_hresult(status))
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
    let description = to_wide_nul(&profile_description(dll_path));
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
        assert_eq!(CATEGORIES.len(), 5, "only the true categories (Codex W6)");
    }
}
