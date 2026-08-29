//! Decides which input context is allowed to drive the one composing engine.
//! Port of `ComposingSessionCoordinator.swift`, keyed by CONTEXT rather than
//! session: one host process may hold several TSF thread managers /
//! document managers / contexts (Office, browsers), and the Rust composing
//! state is one per process (`engine/composing/src/handle.rs:21-56`).
//! Without an owner, a context still alive in a background window would
//! append to the composition the user is typing in the foreground one.
//!
//! What the TSF shell (PR5b) MUST add on top of this model — `claim` alone
//! only drops the outgoing composition, it cannot write it anywhere:
//!
//! 1. Map each live `ITfContext` COM identity to ONE token for its whole
//!    life, and drop the mapping at teardown; never use a raw pointer address
//!    as a long-lived identity.
//! 2. On a key from context B while A owns the engine: reach A's live
//!    `ITfContext`, commit / end A's composition inside A's OWN synchronous
//!    read-write edit session, and only when both the call `HRESULT` and
//!    `phrSession` succeeded hand ownership to B. If A is unreachable,
//!    read-only or torn down, reset with a fresh generation and never run A's
//!    effects against B.
//! 3. After the handover, re-validate the incoming token and generation —
//!    COM re-entrancy may have moved the focus again.
//! 4. Every queued `OnSetFocus` / WndProc message carries token + generation
//!    and is dropped when either no longer matches.
//! 5. A key from a stale context goes back to the host unconsumed; it never
//!    calls `claim` to grab ownership.
//! 6. Never hold a `RefCell` borrow across a COM call: take the data, drop the
//!    borrow, call, re-borrow and re-check the generation.
//! 7. The key sink's own `ITfContext` argument is the authority on which
//!    context is typing; a late `OnSetFocus` is a hint, not the truth.

// 中文: 決定哪個 context 可驅動唯一的組字引擎;所有權以 context token 而非 process 為鍵。

use super::manager::ComposingManager;

/// Identity of one input context. Allocated by the shell (a counter, never a
/// COM pointer address: a torn-down context leaves its ownership behind and
/// the allocator can hand the same address to the next one, which would then
/// inherit the dead one's half-typed composition).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ContextToken(pub u64);

pub struct ComposingSessionCoordinator {
    manager: ComposingManager,
    current_owner: Option<ContextToken>,
    next_token: u64,
}

impl ComposingSessionCoordinator {
    pub fn new(manager: ComposingManager) -> Self {
        Self {
            manager,
            current_owner: None,
            next_token: 1,
        }
    }

    /// A fresh, never-used token for a context the shell just met. `0` is
    /// never handed out (it reads as "no token" in queued messages), and the
    /// counter cannot silently wrap onto a live value: at `u64::MAX` — which
    /// no process reaches — it stops advancing and the panic is caught at the
    /// COM boundary like any other.
    pub fn allocate_token(&mut self) -> ContextToken {
        let value = self.next_token;
        assert!(
            value != 0 && value != u64::MAX,
            "context token space exhausted"
        );
        self.next_token += 1;
        ContextToken(value)
    }

    /// Makes `owner` the context that drives the engine and returns the
    /// manager it should drive. Taking ownership from another context starts
    /// a fresh engine session — whatever the previous one was composing
    /// belongs to a document this one cannot write to. Re-claiming an
    /// ownership this context already holds leaves the composition alone.
    pub fn claim(&mut self, owner: ContextToken) -> &mut ComposingManager {
        if self.current_owner != Some(owner) {
            log::debug!("context ownership changed");
            self.manager.start_new_session();
            self.current_owner = Some(owner);
        }
        &mut self.manager
    }

    /// The manager, or `None` when `owner` is not the focused context —
    /// the answer that keeps a stale context from writing into the live
    /// composition; callers treat it as "this key is not mine".
    pub fn manager(&mut self, owner: ContextToken) -> Option<&mut ComposingManager> {
        (self.current_owner == Some(owner)).then_some(&mut self.manager)
    }

    /// Read-only view for a context that owns the engine.
    pub fn manager_ref(&self, owner: ContextToken) -> Option<&ComposingManager> {
        (self.current_owner == Some(owner)).then_some(&self.manager)
    }

    pub fn current_owner(&self) -> Option<ContextToken> {
        self.current_owner
    }

    /// Gives up ownership when a context ends. A context that is no longer
    /// the owner has already been superseded by `claim`, so releasing it must
    /// not disturb the composition that took its place.
    pub fn release(&mut self, owner: ContextToken) {
        if self.current_owner != Some(owner) {
            return;
        }
        log::debug!("context ownership released");
        self.manager.start_new_session();
        self.current_owner = None;
    }
}
