//! The one display attribute the composition is drawn with: a dotted
//! underline on the preedit (`TF_ATTR_INPUT`, rakukan `globals.rs`
//! `DISPLAY_ATTRIBUTE_INPUT`; khiin `display_attribute_info.rs`). The
//! atom the property carries is registered once per activation through
//! `ITfCategoryMgr::RegisterGUID` and applied by the composition editor
//! (`GUID_PROP_ATTRIBUTE` `Clear` then `SetValue`, rakukan
//! `on_compose.rs:379-396`).

use crate::guids::GUID_DISPLAY_ATTRIBUTE_INPUT;
use std::cell::Cell;
use windows::core::{Error, Result, BOOL, BSTR, GUID};
use windows::Win32::Foundation::{E_INVALIDARG, S_FALSE};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::UI::TextServices::{
    CLSID_TF_CategoryMgr, IEnumTfDisplayAttributeInfo, IEnumTfDisplayAttributeInfo_Impl,
    ITfCategoryMgr, ITfDisplayAttributeInfo, ITfDisplayAttributeInfo_Impl, TF_ATTR_INPUT,
    TF_CT_NONE, TF_DA_COLOR, TF_DA_COLOR_0, TF_DISPLAYATTRIBUTE, TF_LS_DOT,
};
use windows_core::implement;

/// Atom value TSF never hands out — "no attribute registered".
pub const INVALID_ATOM: u32 = 0;

const NO_COLOR: TF_DA_COLOR = TF_DA_COLOR {
    r#type: TF_CT_NONE,
    Anonymous: TF_DA_COLOR_0 { nIndex: 0 },
};

/// The preedit: a dotted underline, the host's own text colours.
pub const INPUT_ATTRIBUTE: TF_DISPLAYATTRIBUTE = TF_DISPLAYATTRIBUTE {
    crText: NO_COLOR,
    crBk: NO_COLOR,
    lsStyle: TF_LS_DOT,
    fBoldLine: BOOL(0),
    crLine: NO_COLOR,
    bAttr: TF_ATTR_INPUT,
};

/// Registers the attribute's GUID with TSF and answers its atom, or
/// `INVALID_ATOM` when the category manager is unavailable (the
/// composition then draws with the host's default styling).
pub fn register_input_atom() -> u32 {
    // SAFETY: TSF's category manager, created on the activating thread.
    let atom = unsafe {
        CoCreateInstance::<_, ITfCategoryMgr>(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)
            .and_then(|manager| manager.RegisterGUID(&GUID_DISPLAY_ATTRIBUTE_INPUT))
    };
    match atom {
        Ok(atom) => atom,
        Err(error) => {
            log::warn!("display_attribute.register_failed error={error}");
            INVALID_ATOM
        }
    }
}

#[implement(ITfDisplayAttributeInfo)]
pub struct DisplayAttributeInfo {
    attribute: Cell<TF_DISPLAYATTRIBUTE>,
}

impl DisplayAttributeInfo {
    pub fn input() -> Self {
        Self {
            attribute: Cell::new(INPUT_ATTRIBUTE),
        }
    }
}

impl ITfDisplayAttributeInfo_Impl for DisplayAttributeInfo_Impl {
    fn GetGUID(&self) -> Result<GUID> {
        Ok(GUID_DISPLAY_ATTRIBUTE_INPUT)
    }

    fn GetDescription(&self) -> Result<BSTR> {
        Ok(BSTR::from("TaigiKeyboard composition"))
    }

    fn GetAttributeInfo(&self, pda: *mut TF_DISPLAYATTRIBUTE) -> Result<()> {
        if pda.is_null() {
            return Err(Error::from_hresult(E_INVALIDARG));
        }
        // SAFETY: TSF passes a valid out-pointer.
        unsafe { *pda = self.attribute.get() };
        Ok(())
    }

    /// The user (Control Panel) may restyle the attribute; honoured until
    /// `Reset`.
    fn SetAttributeInfo(&self, pda: *const TF_DISPLAYATTRIBUTE) -> Result<()> {
        if pda.is_null() {
            return Err(Error::from_hresult(E_INVALIDARG));
        }
        // SAFETY: TSF passes a valid in-pointer.
        self.attribute.set(unsafe { *pda });
        Ok(())
    }

    fn Reset(&self) -> Result<()> {
        self.attribute.set(INPUT_ATTRIBUTE);
        Ok(())
    }
}

/// The enumerator over the one attribute.
#[implement(IEnumTfDisplayAttributeInfo)]
pub struct DisplayAttributeEnumerator {
    position: Cell<u32>,
}

impl DisplayAttributeEnumerator {
    pub fn new() -> Self {
        Self {
            position: Cell::new(0),
        }
    }

    const COUNT: u32 = 1;
}

impl Default for DisplayAttributeEnumerator {
    fn default() -> Self {
        Self::new()
    }
}

impl IEnumTfDisplayAttributeInfo_Impl for DisplayAttributeEnumerator_Impl {
    fn Clone(&self) -> Result<IEnumTfDisplayAttributeInfo> {
        let clone = DisplayAttributeEnumerator {
            position: Cell::new(self.position.get()),
        };
        Ok(clone.into())
    }

    fn Next(
        &self,
        ulcount: u32,
        rginfo: *mut Option<ITfDisplayAttributeInfo>,
        pcfetched: *mut u32,
    ) -> Result<()> {
        // COM's enumerator contract: `pcfetched` may be null only for a
        // single-element request.
        if rginfo.is_null() || (ulcount != 1 && pcfetched.is_null()) {
            return Err(Error::from_hresult(E_INVALIDARG));
        }
        let mut fetched = 0u32;
        while fetched < ulcount && self.position.get() < DisplayAttributeEnumerator::COUNT {
            // SAFETY: TSF provides an array of at least `ulcount` slots.
            unsafe { *rginfo.add(fetched as usize) = Some(DisplayAttributeInfo::input().into()) };
            self.position.set(self.position.get() + 1);
            fetched += 1;
        }
        if !pcfetched.is_null() {
            // SAFETY: null-checked out-pointer.
            unsafe { *pcfetched = fetched };
        }
        if fetched == ulcount {
            Ok(())
        } else {
            Err(Error::from_hresult(S_FALSE))
        }
    }

    fn Reset(&self) -> Result<()> {
        self.position.set(0);
        Ok(())
    }

    fn Skip(&self, ulcount: u32) -> Result<()> {
        let target = self.position.get().saturating_add(ulcount);
        if target > DisplayAttributeEnumerator::COUNT {
            self.position.set(DisplayAttributeEnumerator::COUNT);
            return Err(Error::from_hresult(S_FALSE));
        }
        self.position.set(target);
        Ok(())
    }
}

/// `ITfDisplayAttributeProvider::GetDisplayAttributeInfo` — only our GUID.
pub fn info_for(guid: &GUID) -> Result<ITfDisplayAttributeInfo> {
    if *guid == GUID_DISPLAY_ATTRIBUTE_INPUT {
        Ok(DisplayAttributeInfo::input().into())
    } else {
        Err(Error::from_hresult(E_INVALIDARG))
    }
}
