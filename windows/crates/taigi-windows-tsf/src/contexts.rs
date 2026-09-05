//! Maps each live `ITfContext` to ONE `ContextToken` for its whole life
//! (coordinator handover contract, point 1). The key is the context's
//! canonical `IUnknown` identity, and the registry HOLDS a reference to the
//! context, so the address cannot be reused while the mapping exists;
//! the mapping is dropped at `OnPopContext`.
//!
//! No COM call happens inside this type: the caller resolves the identity
//! (`identity`, a `QueryInterface`) and clones the context (an `AddRef`)
//! BEFORE taking the state borrow, and drops what `forget` / `into_tokens`
//! hand back AFTER releasing it (roadmap W3).

// 每個活著的 ITfContext 對應一個 token;COM 呼叫都在借用之外,這裡只做 map 操作。

use std::collections::HashMap;
use std::rc::Rc;
use taigi_windows_core::composing::{CandidateSource, ContextToken};
use windows::core::{IUnknown, Interface};
use windows::Win32::UI::TextServices::{ITfComposition, ITfContext, ITfRange};

/// What one context is composing right now, beside the engine's own state:
/// the TSF composition object, the candidates fetched for it, and the
/// auto-space arm. Plain data — the composition handle is only ever moved
/// in and out under the borrow, never called under it.
#[derive(Default)]
pub struct ContextState {
    pub composition: Option<ITfComposition>,
    /// The list the last fetch produced with the cells shown for it; empty
    /// = no candidates showing.
    pub candidates: CandidateSource,
    /// The highlighted candidate (PR5b: a headless list; PR6's window
    /// The caret's range when this IME left an auto space in front of it —
    /// the position the swap re-checks before it rewrites anything (the Mac's
    /// `armedAutoSpaceCaret`). Its EXISTENCE is the verdict: the arm is only
    /// ever set after a commit that wrote romanization earned its space (§23).
    pub armed_auto_space: Option<ITfRange>,
    /// The host ended this context's composition while the engine could not
    /// be reached (a callback re-entering a running session): the engine is
    /// reset at the next key instead of silently drifting.
    pub is_engine_reset_pending: bool,
}

pub struct ContextEntry {
    /// `Rc` so a second handle for a handover is a Rust clone, not an
    /// `AddRef` under the borrow; the one COM reference drops with the Rc.
    pub context: Rc<ITfContext>,
    pub token: ContextToken,
    pub state: ContextState,
}

#[derive(Default)]
pub struct ContextRegistry {
    entries: HashMap<usize, ContextEntry>,
}

impl ContextRegistry {
    /// The canonical COM identity: the `IUnknown` pointer, which COM
    /// guarantees equal for every interface of one object. A COM call —
    /// resolve it before borrowing any state.
    pub fn identity(context: &ITfContext) -> Option<usize> {
        context
            .cast::<IUnknown>()
            .ok()
            .map(|unknown| unknown.as_raw() as usize)
    }

    /// The token for the context whose identity is `identity`, allocating
    /// one through `allocate` the first time it is seen. `owned` is the
    /// caller's already-counted reference, kept only on first sight (a
    /// duplicate is handed back to be dropped outside the borrow).
    pub fn token_for(
        &mut self,
        identity: usize,
        owned: ITfContext,
        allocate: impl FnOnce() -> ContextToken,
    ) -> (ContextToken, Option<ITfContext>) {
        if let Some(entry) = self.entries.get(&identity) {
            return (entry.token, Some(owned));
        }
        let token = allocate();
        self.entries.insert(
            identity,
            ContextEntry {
                context: Rc::new(owned),
                token,
                state: ContextState::default(),
            },
        );
        (token, None)
    }

    pub fn entry_mut(&mut self, identity: usize) -> Option<&mut ContextEntry> {
        self.entries.get_mut(&identity)
    }

    /// The entry holding `token`, for the handover and the composition sink.
    pub fn entry_by_token_mut(&mut self, token: ContextToken) -> Option<&mut ContextEntry> {
        self.entries.values_mut().find(|entry| entry.token == token)
    }

    pub fn entries_mut(&mut self) -> impl Iterator<Item = &mut ContextEntry> {
        self.entries.values_mut()
    }

    /// Drops the mapping at teardown and hands back the reference and the
    /// token, so the caller releases the reference outside its borrow.
    pub fn forget(&mut self, identity: usize) -> Option<ContextEntry> {
        self.entries.remove(&identity)
    }

    /// Every entry, moved out (deactivation): the caller releases the
    /// references outside its borrow.
    pub fn into_entries(self) -> Vec<ContextEntry> {
        self.entries.into_values().collect()
    }
}
