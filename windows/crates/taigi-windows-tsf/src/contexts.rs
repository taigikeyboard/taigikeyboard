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

// 中文: 每個活著的 ITfContext 對應一個 token;COM 呼叫都在借用之外,這裡只做 map 操作。

use std::collections::HashMap;
use taigi_windows_core::composing::ContextToken;
use windows::core::{IUnknown, Interface};
use windows::Win32::UI::TextServices::ITfContext;

#[derive(Default)]
pub struct ContextRegistry {
    entries: HashMap<usize, (ITfContext, ContextToken)>,
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
        if let Some((_, token)) = self.entries.get(&identity) {
            return (*token, Some(owned));
        }
        let token = allocate();
        self.entries.insert(identity, (owned, token));
        (token, None)
    }

    /// Drops the mapping at teardown and hands back the reference and the
    /// token, so the caller releases the reference outside its borrow.
    pub fn forget(&mut self, identity: usize) -> Option<(ITfContext, ContextToken)> {
        self.entries.remove(&identity)
    }

    /// Every mapping, moved out (deactivation): the caller drops the
    /// references outside its borrow.
    pub fn into_tokens(self) -> Vec<ContextToken> {
        self.entries.into_values().map(|(_, token)| token).collect()
    }
}
