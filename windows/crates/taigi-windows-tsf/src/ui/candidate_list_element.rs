//! The UI-less candidate list (`ITfUIElement`, `ITfCandidateListUIElement`
//! and its `Behavior` extension) over the same content the popup draws, so
//! a host that renders candidates itself (games, some Store apps) gets the
//! list (roadmap W4, Codex: v1 architecture item; khiin `candidate_list_ui.rs`).
//!
//! One "page": the strings are the cells' leading script. `SetSelection`
//! selects; `Finalize` commits and `Abort` cancels by running the key path
//! with the matching intent — a host-initiated synchronous call on the TIP
//! thread, the one edit-session entry besides the key sink (W3).

// 無視窗候選列表 — 讓自畫候選的宿主拿到同一份清單;Finalize/Abort 走與按鍵相同的路徑。

use super::candidate_window::CandidateWindow;
use super::presenter::CandidatePresenter;
use crate::com_guard::guarded;
use crate::guids::GUID_CANDIDATE_UI_ELEMENT;
use crate::text_service::TextService;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use windows::core::{Error, Result, BOOL, BSTR, GUID};
use windows::Win32::Foundation::{E_INVALIDARG, E_NOTIMPL, E_UNEXPECTED};
use windows::Win32::UI::TextServices::{
    ITfCandidateListUIElement, ITfCandidateListUIElementBehavior,
    ITfCandidateListUIElementBehavior_Impl, ITfCandidateListUIElement_Impl, ITfDocumentMgr,
    ITfUIElement, ITfUIElement_Impl, TF_CLUIE_COUNT, TF_CLUIE_CURRENTPAGE, TF_CLUIE_PAGEINDEX,
    TF_CLUIE_SELECTION, TF_CLUIE_STRING,
};
use windows_core::{implement, AsImpl, ComObject};

#[implement(ITfUIElement, ITfCandidateListUIElementBehavior)]
pub struct CandidateListElement {
    content: Rc<RefCell<CandidateWindow>>,
    presenter: Weak<RefCell<CandidatePresenter>>,
    service: ComObject<TextService>,
    document: RefCell<Option<ITfDocumentMgr>>,
}

impl CandidateListElement {
    pub fn new(
        content: Rc<RefCell<CandidateWindow>>,
        presenter: Weak<RefCell<CandidatePresenter>>,
        service: ComObject<TextService>,
        document: Option<ITfDocumentMgr>,
    ) -> Self {
        Self {
            content,
            presenter,
            service,
            document: RefCell::new(document),
        }
    }

    /// The document the list belongs to now (a new context's list).
    pub fn set_document(element: &ITfUIElement, document: Option<ITfDocumentMgr>) {
        // SAFETY: the element was created from `CandidateListElement`.
        let this: &CandidateListElement = unsafe { element.as_impl() };
        let previous = this.document.replace(document);
        drop(previous);
    }
}

impl ITfUIElement_Impl for CandidateListElement_Impl {
    fn GetDescription(&self) -> Result<BSTR> {
        guarded("ITfUIElement::GetDescription", || {
            Ok(BSTR::from("TaigiKeyboard candidates"))
        })
    }

    fn GetGUID(&self) -> Result<GUID> {
        guarded("ITfUIElement::GetGUID", || Ok(GUID_CANDIDATE_UI_ELEMENT))
    }

    /// The host hides or re-shows the popup; the list stays.
    fn Show(&self, bshow: BOOL) -> Result<()> {
        guarded("ITfUIElement::Show", || {
            let Some(presenter) = self.presenter.upgrade() else {
                return Ok(());
            };
            match presenter.try_borrow_mut() {
                Ok(mut presenter) => presenter.set_visible_by_host(bshow.as_bool()),
                // Re-entered from inside our own presentation: the answer
                // the host just gave to BeginUIElement already applies.
                Err(_) => log::warn!("ui.show_reentered shown={}", bshow.as_bool()),
            }
            Ok(())
        })
    }

    fn IsShown(&self) -> Result<BOOL> {
        guarded("ITfUIElement::IsShown", || {
            let visible = match self.presenter.upgrade() {
                Some(presenter) => match presenter.try_borrow() {
                    Ok(presenter) => presenter.is_popup_visible(),
                    Err(_) => !self.content.borrow().is_empty(),
                },
                None => false,
            };
            Ok(BOOL::from(visible))
        })
    }
}

impl ITfCandidateListUIElement_Impl for CandidateListElement_Impl {
    fn GetUpdatedFlags(&self) -> Result<u32> {
        guarded("ITfCandidateListUIElement::GetUpdatedFlags", || {
            Ok(TF_CLUIE_STRING
                | TF_CLUIE_COUNT
                | TF_CLUIE_SELECTION
                | TF_CLUIE_CURRENTPAGE
                | TF_CLUIE_PAGEINDEX)
        })
    }

    fn GetDocumentMgr(&self) -> Result<ITfDocumentMgr> {
        guarded("ITfCandidateListUIElement::GetDocumentMgr", || {
            self.document
                .borrow()
                .clone()
                .ok_or_else(|| Error::from_hresult(E_UNEXPECTED))
        })
    }

    fn GetCount(&self) -> Result<u32> {
        guarded("ITfCandidateListUIElement::GetCount", || {
            Ok(self.content.borrow().cell_count() as u32)
        })
    }

    fn GetSelection(&self) -> Result<u32> {
        guarded("ITfCandidateListUIElement::GetSelection", || {
            Ok(self.content.borrow().selected_index().unwrap_or(0) as u32)
        })
    }

    fn GetString(&self, uindex: u32) -> Result<BSTR> {
        guarded("ITfCandidateListUIElement::GetString", || {
            let content = self.content.borrow();
            let text = content
                .cell_text(uindex as usize)
                .ok_or_else(|| Error::from_hresult(E_INVALIDARG))?;
            Ok(BSTR::from(text))
        })
    }

    /// One page holding every candidate: a host drawing the list pages it
    /// as it likes. The two-call shape: a null index buffer asks for the
    /// page count; a buffer too small for it is an error (khiin
    /// `candidate_list_ui.rs:290-317`).
    fn GetPageIndex(&self, pindex: *mut u32, usize_: u32, pupagecnt: *mut u32) -> Result<()> {
        guarded("ITfCandidateListUIElement::GetPageIndex", || {
            if pupagecnt.is_null() {
                return Err(Error::from_hresult(E_INVALIDARG));
            }
            const PAGE_COUNT: u32 = 1;
            // SAFETY: TSF's out-pointers; the index buffer is written only
            // when the host provided room for the whole page list.
            unsafe {
                *pupagecnt = PAGE_COUNT;
                if pindex.is_null() {
                    return Ok(());
                }
                if usize_ < PAGE_COUNT {
                    return Err(Error::from_hresult(E_INVALIDARG));
                }
                *pindex = 0;
            }
            Ok(())
        })
    }

    fn SetPageIndex(&self, _pindex: *const u32, _upagecnt: u32) -> Result<()> {
        Err(Error::from_hresult(E_NOTIMPL))
    }

    fn GetCurrentPage(&self) -> Result<u32> {
        guarded("ITfCandidateListUIElement::GetCurrentPage", || Ok(0))
    }
}

impl ITfCandidateListUIElementBehavior_Impl for CandidateListElement_Impl {
    /// The host moved the selection: mirrored into the model (a click
    /// selects, never commits).
    fn SetSelection(&self, nindex: u32) -> Result<()> {
        guarded("ITfCandidateListUIElementBehavior::SetSelection", || {
            let mut content = self.content.borrow_mut();
            if nindex as usize >= content.cell_count() {
                return Err(Error::from_hresult(E_INVALIDARG));
            }
            content.select(nindex as usize);
            Ok(())
        })
    }

    /// The host commits the highlighted candidate: the key path, with the
    /// commit intent, under the owning context's own session.
    fn Finalize(&self) -> Result<()> {
        guarded("ITfCandidateListUIElementBehavior::Finalize", || {
            self.service.ui_element_finalize()
        })
    }

    /// The host cancels the composition: the key path with Escape's intent.
    fn Abort(&self) -> Result<()> {
        guarded("ITfCandidateListUIElementBehavior::Abort", || {
            self.service.ui_element_abort()
        })
    }
}

#[allow(dead_code)]
fn _behavior_chain(element: &ITfCandidateListUIElementBehavior) -> &ITfCandidateListUIElement {
    element
}
