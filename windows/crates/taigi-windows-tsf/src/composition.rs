//! The composition as TSF sees it: one `ITfComposition` per context, edited
//! only inside a synchronous edit session. [`CompositionEditor`] is the
//! `ComposingEffectExecutor` for one session — the engine says WHAT happens
//! to the document (`Effect`), this says HOW, in TSF terms (rakukan
//! `on_compose.rs`; khiin `composition_mgr.rs`). Every rule the Mac executor
//! states (`ClientEffectExecutor.swift`) holds here with TSF spellings:
//! the preedit lives in the composition until commit; a commit is ONE
//! `SetText` + selection + `EndComposition`; the selection is placed BEFORE
//! `EndComposition` (rakukan Fix3: after it the host resets the caret).

// 中文: TSF 組字區操作 — 每個 context 一個 ITfComposition,只在同步 edit session 內改;效果→TSF 呼叫的對照。

use crate::edit_session::EditCookie;
use crate::wide::to_wide;
use std::mem::ManuallyDrop;
use taigi_windows_core::composing::ComposingEffectExecutor;
use taigi_windows_core::engine::Effect;
use windows::core::{IUnknown, Interface, Result, BOOL};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::Variant::{VariantClear, VARIANT, VT_I4, VT_UNKNOWN};
use windows::Win32::UI::TextServices::{
    ITfComposition, ITfCompositionSink, ITfContext, ITfContextComposition, ITfInputScope,
    ITfInsertAtSelection, ITfRange, TfActiveSelEnd, GUID_PROP_ATTRIBUTE, GUID_PROP_INPUTSCOPE,
    INSERT_TEXT_AT_SELECTION_FLAGS, IS_PASSWORD, TF_AE_NONE, TF_ANCHOR_END, TF_DEFAULT_SELECTION,
    TF_IAS_QUERYONLY, TF_SELECTION, TF_SELECTIONSTYLE, TS_SD_READONLY,
};

/// A `TF_SELECTION` collapsed on `range` — how every edit here leaves the
/// caret: after what was just written.
fn collapsed_selection(range: ITfRange) -> TF_SELECTION {
    TF_SELECTION {
        range: ManuallyDrop::new(Some(range)),
        style: TF_SELECTIONSTYLE {
            ase: TfActiveSelEnd(TF_AE_NONE.0),
            fInterimChar: BOOL(0),
        },
    }
}

/// Places the caret at the END of `range`.
unsafe fn select_end_of(context: &ITfContext, ec: EditCookie, range: &ITfRange) -> Result<()> {
    let caret = range.Clone()?;
    caret.Collapse(ec, TF_ANCHOR_END)?;
    let selection = collapsed_selection(caret);
    let outcome = context.SetSelection(ec, std::slice::from_ref(&selection));
    // The struct's ManuallyDrop range is ours to release.
    drop(ManuallyDrop::into_inner(selection.range));
    outcome
}

/// The caret's own range (a clone of the selection, collapsed at its end),
/// or `None` when the host reports no selection.
unsafe fn caret_range(context: &ITfContext, ec: EditCookie) -> Option<ITfRange> {
    let mut selections = [TF_SELECTION {
        range: ManuallyDrop::new(None),
        style: TF_SELECTIONSTYLE::default(),
    }];
    let mut fetched = 0u32;
    context
        .GetSelection(ec, TF_DEFAULT_SELECTION, &mut selections, &mut fetched)
        .ok()?;
    if fetched == 0 {
        return None;
    }
    let owned = ManuallyDrop::take(&mut selections[0].range);
    let range = owned?;
    let caret = range.Clone().ok()?;
    caret.Collapse(ec, TF_ANCHOR_END).ok()?;
    drop(range);
    Some(caret)
}

/// Whether the context refuses writes right now (`TS_SD_READONLY`): no
/// session is even requested for one.
pub fn is_read_only(context: &ITfContext) -> bool {
    // SAFETY: a status query on the live context.
    unsafe { context.GetStatus() }.is_ok_and(|status| status.dwDynamicFlags & TS_SD_READONLY != 0)
}

/// Whether the caret sits in a password field (`GUID_PROP_INPUTSCOPE`
/// carrying `IS_PASSWORD`, roadmap W3 / Codex F14): the TIP composes
/// nothing there and learns nothing. A field with no input-scope object is
/// not a password; one whose scope object refuses to be read is (fail
/// closed). DOGFOOD: browser, Office and Win32 password controls.
pub fn is_password_field(context: &ITfContext, ec: EditCookie) -> bool {
    // SAFETY: property + range reads under the session's cookie; the
    // VARIANT is cleared and the scope array freed on every path.
    unsafe {
        let Ok(property) = context.GetProperty(&GUID_PROP_INPUTSCOPE) else {
            return false;
        };
        let Some(range) = caret_range(context, ec) else {
            return false;
        };
        let Ok(mut value) = property.GetValue(ec, &range) else {
            return false;
        };
        // No input-scope object on this field is the common case (not a
        // password). One that IS there but cannot be read is treated as a
        // password: the fail-CLOSED side, since composing into a secret
        // costs more than composing nothing.
        let has_scope_object = value.Anonymous.Anonymous.vt == VT_UNKNOWN;
        let scope: Option<ITfInputScope> = if has_scope_object {
            (*value.Anonymous.Anonymous.Anonymous.punkVal)
                .as_ref()
                .and_then(|unknown: &IUnknown| unknown.cast().ok())
        } else {
            None
        };
        VariantClear(&mut value).ok();
        let Some(scope) = scope else {
            return has_scope_object;
        };
        let mut scopes = std::ptr::null_mut();
        let mut count = 0u32;
        if scope.GetInputScopes(&mut scopes, &mut count).is_err() || scopes.is_null() {
            return true;
        }
        let is_password = std::slice::from_raw_parts(scopes, count as usize).contains(&IS_PASSWORD);
        CoTaskMemFree(Some(scopes as *const _));
        is_password
    }
}

/// One edit session's view of one context: the composition it holds (moved
/// in from the context's state before the session, moved back after — the
/// state is never borrowed across these calls), the display-attribute atom
/// and the sink a new composition is started with.
pub struct CompositionEditor<'a> {
    context: &'a ITfContext,
    ec: EditCookie,
    sink: &'a ITfCompositionSink,
    display_attribute_atom: u32,
    pub composition: Option<ITfComposition>,
    /// The first TSF error the session hit — the executor's trait has no
    /// error channel, so it is read back after the effects ran.
    pub failure: Option<windows::core::Error>,
    /// Set by `arm_swap`: the caret's range when this IME wrote an auto space.
    /// Its existence is the verdict — a display mode changed afterwards does
    /// not rewrite what is already in the document.
    pub armed: Option<ITfRange>,
}

impl<'a> CompositionEditor<'a> {
    pub fn new(
        context: &'a ITfContext,
        ec: EditCookie,
        sink: &'a ITfCompositionSink,
        display_attribute_atom: u32,
        composition: Option<ITfComposition>,
    ) -> Self {
        Self {
            context,
            ec,
            sink,
            display_attribute_atom,
            composition,
            failure: None,
            armed: None,
        }
    }

    /// Where the composition's caret is on screen (pixels), for the window.
    pub fn caret_rect(&self) -> Option<windows::Win32::Foundation::RECT> {
        crate::ui::caret::caret_rect(self.context, self.ec, self.composition.as_ref())
    }

    /// The document this session's context belongs to (the UI-less list
    /// reports it to the host).
    pub fn document(&self) -> Option<windows::Win32::UI::TextServices::ITfDocumentMgr> {
        // SAFETY: a query on the live context.
        unsafe { self.context.GetDocumentMgr() }.ok()
    }

    fn record(&mut self, outcome: Result<()>) {
        if let Err(error) = outcome {
            log::error!("composition.edit_failed error={error}");
            self.failure.get_or_insert(error);
        }
    }

    /// The composition, started at the caret if there is none yet
    /// (`InsertTextAtSelection(QUERYONLY)` → `StartComposition`, khiin
    /// `composition_mgr.rs:new_composition`).
    unsafe fn composition_or_start(&mut self) -> Result<ITfComposition> {
        if let Some(composition) = &self.composition {
            return Ok(composition.clone());
        }
        let insert_at: ITfInsertAtSelection = self.context.cast()?;
        let range = insert_at.InsertTextAtSelection(self.ec, TF_IAS_QUERYONLY, &[])?;
        let context_composition: ITfContextComposition = self.context.cast()?;
        let composition = context_composition.StartComposition(self.ec, &range, self.sink)?;
        self.composition = Some(composition.clone());
        Ok(composition)
    }

    unsafe fn set_preedit(&mut self, text: &str) -> Result<()> {
        let composition = self.composition_or_start()?;
        let range = composition.GetRange()?;
        range.SetText(self.ec, 0, &to_wide(text))?;
        if self.display_attribute_atom != 0 {
            if let Ok(property) = self.context.GetProperty(&GUID_PROP_ATTRIBUTE) {
                // Clear then set: the clear is what makes TSF notify the
                // host of a change (rakukan `on_compose.rs:379-396`). Both
                // are NON-FATAL styling: a failure loses the underline and
                // nothing else, so it is logged and the text still lands.
                if let Err(error) = property.Clear(self.ec, &range) {
                    log::debug!("composition.attribute_clear_failed error={error}");
                }
                let mut variant = VARIANT::default();
                {
                    // A VT_I4 payload: the discriminant and the matching
                    // union member, written through the ManuallyDrop explicitly.
                    let inner = &mut *variant.Anonymous.Anonymous;
                    inner.vt = VT_I4;
                    inner.Anonymous.lVal = self.display_attribute_atom as i32;
                }
                if let Err(error) = property.SetValue(self.ec, &range, &variant) {
                    log::debug!("composition.attribute_set_failed error={error}");
                }
            }
        }
        select_end_of(self.context, self.ec, &range)
    }

    /// Ends the composition with `text` in its place: one `SetText`, the
    /// caret after it, then `EndComposition` (rakukan `end_composition`).
    unsafe fn end_with(&mut self, text: &str) -> Result<()> {
        let Some(composition) = self.composition.clone() else {
            return self.insert_at_caret(text);
        };
        let range = composition.GetRange()?;
        range.SetText(self.ec, 0, &to_wide(text))?;
        select_end_of(self.context, self.ec, &range)?;
        composition.EndComposition(self.ec)?;
        // The handle is dropped only once the composition really ended: a
        // failure above leaves a live composition the next session must
        // still be able to reach.
        self.composition = None;
        Ok(())
    }

    /// After a failed document write: the composition is ended with
    /// whatever it holds, best effort, so the engine (reset by the caller)
    /// and the document agree that there is no composition. Errors are
    /// logged, not returned — there is nothing left to do about them.
    pub fn abandon(&mut self) {
        // SAFETY: the session's cookie is live for this editor's lifetime.
        unsafe {
            if let Some(composition) = self.composition.take() {
                if let Err(error) = composition.EndComposition(self.ec) {
                    log::error!("composition.abandon_failed error={error}");
                }
            }
        }
    }

    /// Remembers where the caret sits now that an auto space is in front
    /// of it — the position the swap re-checks. A host that cannot answer
    /// never arms (the swap degrades to pass-through, as on the Mac).
    pub fn arm_swap(&mut self) {
        // SAFETY: a selection read under the live cookie.
        if let Some(caret) = unsafe { caret_range(self.context, self.ec) } {
            self.armed = Some(caret);
        }
    }

    /// Text written with no composition open (the auto space, a full-width
    /// glyph outside a composition): at the caret, caret moved after it.
    unsafe fn insert_at_caret(&mut self, text: &str) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        let insert_at: ITfInsertAtSelection = self.context.cast()?;
        let range = insert_at.InsertTextAtSelection(
            self.ec,
            INSERT_TEXT_AT_SELECTION_FLAGS(0),
            &to_wide(text),
        )?;
        select_end_of(self.context, self.ec, &range)
    }

    /// Writes `text` outside the composition (public face of `insert_at_caret`).
    pub fn insert_external(&mut self, text: &str) {
        // SAFETY: the session's cookie is live for this editor's lifetime.
        let outcome = unsafe { self.insert_at_caret(text) };
        self.record(outcome);
    }

    /// The auto-space swap (`guá ` + `?` → `guá? `, §23): only when the
    /// caret is still exactly where the space left it (`armed_caret`, an
    /// `ITfRange` that tracks the document), is a collapsed selection, and
    /// the character before it is a space — the Mac's three verifications
    /// against the document, never assumed. Answers whether the rewrite
    /// happened.
    pub fn swap_preceding_space(&mut self, replacement: &str, armed_caret: &ITfRange) -> bool {
        // SAFETY: range arithmetic under the live cookie; every range is a
        // clone this function owns.
        unsafe {
            let Some(caret) = caret_range(self.context, self.ec) else {
                return false;
            };
            if armed_caret
                .IsEqualEnd(self.ec, &caret, TF_ANCHOR_END)
                .map(|equal| !equal.as_bool())
                .unwrap_or(true)
            {
                return false;
            }
            let probe = match caret.Clone() {
                Ok(probe) => probe,
                Err(_) => return false,
            };
            let mut shifted = 0i32;
            if probe
                .ShiftStart(self.ec, -1, &mut shifted, std::ptr::null())
                .is_err()
                || shifted != -1
            {
                return false;
            }
            let mut text = [0u16; 2];
            let mut length = 0u32;
            if probe.GetText(self.ec, 0, &mut text, &mut length).is_err()
                || length != 1
                || text[0] != u16::from(b' ')
            {
                return false;
            }
            if probe.SetText(self.ec, 0, &to_wide(replacement)).is_err() {
                return false;
            }
            select_end_of(self.context, self.ec, &probe).is_ok()
        }
    }
}

impl ComposingEffectExecutor for CompositionEditor<'_> {
    fn execute(&mut self, effect: &Effect) {
        // SAFETY: every call runs under the session's cookie on the live
        // context; the composition handle is this editor's own.
        let outcome = unsafe {
            match effect {
                Effect::UpdatePreedit(text) => self.set_preedit(text),
                Effect::ClearPreeditWithoutCommit => self.end_with(""),
                Effect::CommitTextReplacingPreedit(text) => self.end_with(text),
                // NAMED DIVERGENCE (as macOS `ClientEffectExecutor.swift:56-63`):
                // the preedit only ever lived in the composition, so deleting
                // a document character would eat a real host character.
                Effect::DeleteBackwardFromDocument => Ok(()),
                // No autocomplete surface; the learning handshakes never reach
                // an executor (`ComposingManager` routes them to the learner).
                Effect::ResetAutocomplete
                | Effect::PerformAutocomplete
                | Effect::ResetAutocompleteContext
                | Effect::NextWordUpdateLastSelectedWord { .. }
                | Effect::NextWordWordSelected { .. }
                | Effect::NextWordClearForNewComposing => Ok(()),
            }
        };
        self.record(outcome);
    }
}

/// An executor that writes nothing: for a composition the HOST already
/// ended (`OnCompositionTerminated`) — the text stays as the host left it
/// and only the engine is reset.
pub struct NullExecutor;

impl ComposingEffectExecutor for NullExecutor {
    fn execute(&mut self, _effect: &Effect) {}
}
