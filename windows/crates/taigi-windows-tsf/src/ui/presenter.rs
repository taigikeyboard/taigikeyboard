//! The candidate window as the composing path sees it: the owner-token
//! guard over one window, port of `CandidatePresenter` /
//! `CandidatePanel.swift`. One window per text service (its thread pumps the
//! messages); ownership decides which context may hide it, so a session
//! losing focus AFTER the next one gained it cannot take the incoming
//! window down. Hiding drops the list, not merely the pixels.
//!
//! The presenter is also the window's [`WindowHandler`]: every message the
//! popup receives lands here first, so a hide posted by a focus callback
//! and a host's `ITfUIElement::Show` end the UI-less element exactly as a
//! key-path hide does — one teardown, balanced `BeginUIElement` /
//! `EndUIElement` on every route.

use super::candidate_list_element::CandidateListElement;
use super::candidate_window::{CandidateWindow, CandidateWindowContent, UNFOLD_TIMER};
use super::render::RenderFactory;
use super::window::{PopupWindow, WindowHandler, WindowRef};
use crate::text_service::TextService;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use taigi_windows_core::composing::ContextToken;
use taigi_windows_core::keys::CandidateNavigation;
use taigi_windows_core::settings::SettingsDocument;
use windows::core::Interface;
use windows::Win32::Foundation::RECT;
use windows::Win32::UI::TextServices::{
    ITfDocumentMgr, ITfThreadMgr, ITfUIElement, ITfUIElementMgr,
};
use windows_core::ComObject;

const WINDOW_CLASS: &str = "TaigiKeyboardCandidates";

pub struct CandidatePresenter {
    content: Rc<RefCell<CandidateWindow>>,
    window: Option<PopupWindow>,
    owner: Option<ContextToken>,
    /// The UI-less host contract: registered while a list is up; when the
    /// host answers "do not show", the popup stays hidden and the host
    /// draws the list itself (khiin `candidate_list_ui.rs`).
    ui_element: Option<ITfUIElement>,
    ui_element_id: Option<u32>,
    is_host_drawing: bool,
    /// `ITfUIElement::Show(FALSE)` from the host after `BeginUIElement`:
    /// the popup is hidden while the list stays (the host took over the
    /// drawing mid-list). Reset when the element ends.
    is_hidden_by_host: bool,
    thread_mgr: Option<ITfThreadMgr>,
    /// The service, for the UI-less element's `Finalize` / `Abort`.
    service: Option<ComObject<TextService>>,
    /// Handed to the popup as its handler (the window borrows the
    /// presenter, never the reverse — no `Rc` cycle).
    weak_self: Weak<RefCell<CandidatePresenter>>,
}

impl CandidatePresenter {
    pub fn new(factory: Rc<RenderFactory>) -> Self {
        Self {
            content: Rc::new(RefCell::new(CandidateWindow::new(factory))),
            window: None,
            owner: None,
            ui_element: None,
            ui_element_id: None,
            is_host_drawing: false,
            is_hidden_by_host: false,
            thread_mgr: None,
            service: None,
            weak_self: Weak::new(),
        }
    }

    /// Wires the presenter to its service (activation). `weak_self` is the
    /// cell this presenter lives in — the popup's handler.
    pub fn attach(
        &mut self,
        thread_mgr: ITfThreadMgr,
        service: ComObject<TextService>,
        weak_self: Weak<RefCell<CandidatePresenter>>,
    ) {
        self.thread_mgr = Some(thread_mgr);
        self.service = Some(service);
        self.weak_self = weak_self;
    }

    /// Wires a presenter that only ever draws its own popup — the symbol
    /// picker (`session.rs`). No thread manager and no service, so
    /// `begin_or_update_ui_element` never registers a UI-less element: the
    /// host-drawn candidate contract is the composing list's, and a host
    /// that draws candidate lists itself would otherwise be handed a menu
    /// of punctuation as one. Documented divergence from the composing
    /// window, not an oversight.
    pub fn attach_popup_only(&mut self, weak_self: Weak<RefCell<CandidatePresenter>>) {
        self.weak_self = weak_self;
    }

    /// Tears everything down (deactivation).
    pub fn detach(&mut self) {
        self.hide_now();
        if let Some(window) = self.window.take() {
            window.destroy();
        }
        self.thread_mgr = None;
        self.ui_element = None;
        self.service = None;
    }

    fn window(&mut self) -> Option<&WindowRef> {
        if self.window.is_none() {
            let handler: Rc<RefCell<dyn WindowHandler>> = self.weak_self.upgrade()?;
            match PopupWindow::create(WINDOW_CLASS, handler) {
                Ok(window) => self.window = Some(window),
                Err(error) => {
                    log::error!("ui.window_create_failed error={error}");
                    return None;
                }
            }
        }
        self.window.as_deref()
    }

    /// Whether the popup itself is on screen (the UI-less element's
    /// `IsShown`): a list is up, and neither the host's answer to
    /// `BeginUIElement` nor a later `Show(FALSE)` keeps it hidden.
    pub fn is_popup_visible(&self) -> bool {
        self.owner.is_some()
            && !self.content.borrow().is_empty()
            && !self.is_host_drawing
            && !self.is_hidden_by_host
            && self.window.is_some()
    }

    pub fn owner(&self) -> Option<ContextToken> {
        self.owner
    }

    /// `ITfUIElement::Show` from the host: hides or re-shows the popup
    /// without touching the list.
    pub fn set_visible_by_host(&mut self, shown: bool) {
        self.is_hidden_by_host = !shown;
        if self.owner.is_none() || self.is_host_drawing {
            return;
        }
        if shown {
            let frame = self.content.borrow().current_frame();
            if let (Some(frame), Some(window)) = (frame, self.window()) {
                window.show_at(frame.frame);
            }
        } else if let Some(window) = self.window.as_deref() {
            window.hide();
        }
    }

    /// Queues a hide for the message loop — what a focus / context
    /// callback calls instead of `hide` (W3). `owner` = only if that
    /// context still owns the window; `None` = whichever is up. Nothing
    /// to do when no window exists yet: nothing is showing.
    pub fn request_hide(&self, owner: Option<ContextToken>) {
        if let Some(window) = self.window.as_deref() {
            window.post_hide_request(owner);
        }
    }

    /// Shows `cells` anchored to `caret` (physical screen pixels),
    /// selecting the first, owned by `owner`. Ownership is taken only once
    /// the list is really up.
    pub fn show(
        &mut self,
        content: CandidateWindowContent,
        caret: RECT,
        settings: &SettingsDocument,
        owner: ContextToken,
        document: Option<ITfDocumentMgr>,
    ) {
        let frame = self.content.borrow_mut().show(content, caret, settings);
        let Some(frame) = frame else {
            log::debug!("ui.no_monitor_for_caret");
            self.hide_now();
            return;
        };
        self.owner = Some(owner);
        self.begin_or_update_ui_element(document);
        if self.is_host_drawing || self.is_hidden_by_host {
            return;
        }
        if let Some(window) = self.window() {
            window.show_at(frame.frame);
        }
    }

    /// Same list, new rendering; the window stays where it is.
    pub fn update_cells(
        &mut self,
        content: CandidateWindowContent,
        settings: &SettingsDocument,
        owner: ContextToken,
    ) {
        if self.owner != Some(owner) {
            return;
        }
        let frame = self.content.borrow_mut().update_cells(content, settings);
        self.update_ui_element();
        if let (Some(frame), Some(window), false) = (
            frame,
            self.window.as_deref(),
            self.is_host_drawing || self.is_hidden_by_host,
        ) {
            window.show_at(frame.frame);
        }
    }

    pub fn navigate(&mut self, direction: CandidateNavigation, owner: ContextToken) {
        if self.owner != Some(owner) {
            return;
        }
        let frame = self.content.borrow_mut().navigate(direction);
        let transitioning = self.content.borrow().is_transitioning();
        self.update_ui_element();
        let popup_hidden = self.is_host_drawing || self.is_hidden_by_host;
        if let Some(window) = self.window.as_deref() {
            if let Some(frame) = frame {
                if !popup_hidden {
                    window.show_at(frame.frame);
                }
            }
            if transitioning {
                window.set_timer(UNFOLD_TIMER.0, UNFOLD_TIMER.1);
            }
            window.invalidate();
        }
    }

    pub fn is_showing(&self, owner: ContextToken) -> bool {
        self.owner == Some(owner) && !self.content.borrow().is_empty()
    }

    pub fn selected_index(&self, owner: ContextToken) -> Option<usize> {
        if self.owner != Some(owner) {
            return None;
        }
        self.content.borrow().selected_index()
    }

    /// The absolute index the `slot`-th KEY addresses — the seam the drawn
    /// labels also come from, so a key beside a cell is the key that commits
    /// it (`CandidateWindow::candidate_index_for_key_slot`).
    pub fn candidate_index_for_key_slot(&self, slot: usize, owner: ContextToken) -> Option<usize> {
        if self.owner != Some(owner) {
            return None;
        }
        self.content.borrow().candidate_index_for_key_slot(slot)
    }

    /// Hides if `owner` still owns the window; a no-op once another context
    /// took it over.
    pub fn hide(&mut self, owner: ContextToken) {
        if self.owner == Some(owner) {
            self.hide_now();
        }
    }

    /// Hides whoever owns it and leaves it unowned (the incoming context's
    /// step).
    pub fn hide_for_handover(&mut self) {
        self.hide_now();
    }

    fn hide_now(&mut self) {
        self.owner = None;
        self.content.borrow_mut().clear();
        if let Some(window) = self.window.as_deref() {
            window.kill_timer(UNFOLD_TIMER.0);
            window.hide();
        }
        self.end_ui_element();
    }

    // The UI-less contract

    fn ui_element_mgr(&self) -> Option<ITfUIElementMgr> {
        self.thread_mgr.as_ref()?.cast().ok()
    }

    fn begin_or_update_ui_element(&mut self, document: Option<ITfDocumentMgr>) {
        let Some(service) = self.service.clone() else {
            return;
        };
        let content = Rc::clone(&self.content);
        let weak = self.weak_self.clone();
        let element = self
            .ui_element
            .get_or_insert_with(|| {
                CandidateListElement::new(content, weak, service, document.clone()).into()
            })
            .clone();
        CandidateListElement::set_document(&element, document);
        if self.ui_element_id.is_some() {
            self.update_ui_element();
            return;
        }
        let Some(manager) = self.ui_element_mgr() else {
            return;
        };
        let mut show = windows::core::BOOL(1);
        let mut id = 0u32;
        // SAFETY: TSF's element manager; out-pointers to locals. The call
        // is a COM re-entry point: no `RefCell` borrow is held across it
        // (`self` is borrowed by the caller, which is the presenter's own
        // cell — the element manager does not call back into the presenter
        // synchronously; a host that did would see `try_borrow` fail and
        // get the last known answer).
        match unsafe { manager.BeginUIElement(&element, &mut show, &mut id) } {
            Ok(()) => {
                self.ui_element_id = Some(id);
                self.is_host_drawing = !show.as_bool();
                if self.is_host_drawing {
                    log::info!("ui.host_draws_candidates");
                }
            }
            Err(error) => log::warn!("ui.begin_element_failed error={error}"),
        }
    }

    fn update_ui_element(&self) {
        if let (Some(manager), Some(id)) = (self.ui_element_mgr(), self.ui_element_id) {
            // SAFETY: the id BeginUIElement handed back.
            unsafe { manager.UpdateUIElement(id).ok() };
        }
    }

    fn end_ui_element(&mut self) {
        if let (Some(manager), Some(id)) = (self.ui_element_mgr(), self.ui_element_id.take()) {
            // SAFETY: the id BeginUIElement handed back.
            unsafe { manager.EndUIElement(id).ok() };
        }
        self.is_host_drawing = false;
        self.is_hidden_by_host = false;
    }
}

impl WindowHandler for CandidatePresenter {
    fn paint(&mut self, window: &WindowRef) {
        self.content.borrow_mut().paint(window);
    }

    fn click(&mut self, window: &WindowRef, point: (f32, f32)) {
        self.content.borrow_mut().click(window, point);
        self.update_ui_element();
    }

    fn wheel(&mut self, window: &WindowRef, delta: f32) {
        self.content.borrow_mut().wheel(window, delta);
    }

    fn dpi_changed(&mut self, window: &WindowRef, dpi: f32, suggested: RECT) {
        self.content
            .borrow_mut()
            .dpi_changed(window, dpi, suggested);
    }

    fn timer(&mut self, window: &WindowRef, id: usize) {
        self.content.borrow_mut().timer(window, id);
    }

    fn hide_requested(&mut self, _window: &WindowRef, owner: Option<ContextToken>) {
        match owner {
            Some(owner) => self.hide(owner),
            None => self.hide_now(),
        }
    }

    fn system_theme_changed(&mut self) {
        self.content.borrow_mut().system_theme_changed();
    }
}
