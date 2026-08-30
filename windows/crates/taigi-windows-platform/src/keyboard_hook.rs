//! The thread-scoped keyboard hook the shortcut recorder records through
//! (roadmap W17-B1). `windows-reactor` exposes no keyboard events — only
//! static accelerators — so while a row is recording, a `WH_KEYBOARD` hook
//! on the UI thread takes the presses before the XAML tree sees them.
//!
//! THREAD-SCOPED, never global: the hook is installed with the calling
//! thread's id, so it sees this window's keys and no one else's. The
//! callback does bounded work only — decode, translate, hand the press to
//! the delivery closure — and every path that is not a press the recorder
//! wants falls through to `CallNextHookEx`.
//!
//! Two rules the callback owes the window it hides keys from:
//!
//! * a key it swallows on the way DOWN is swallowed again on the way UP,
//!   so XAML never sees a lone key-up (WinUI invokes a focused `Button` on
//!   the key-UP of Space — a released recording would fire it);
//! * Tab is reported but NOT swallowed. `evaluate_press` answers Tab with
//!   `PassThrough`: recording ends AND the key goes on to walk the form,
//!   which it cannot do if the hook ate it.
//!
//! The decisions live in [`Recording`], which knows nothing of Win32 and
//! is tested on the macOS host; the `#[cfg(windows)]` half below is the
//! API calls and the thread-local that holds them.

// 中文: 錄製快捷鍵用的執行緒鍵盤 hook(W17-B1)。Reactor 不給鍵盤事件,錄製中就用 WH_KEYBOARD 在 XAML 之前攔下按鍵。吞掉的鍵 down/up 都要吞(否則 XAML 會收到孤兒 key-up,WinUI 的 Button 是在 key-up 觸發);Tab 只回報不吞,因為它要繼續走 focus。判斷邏輯在純資料的 Recording,Win32 只負責呼叫。

use taigi_windows_core::keys::RecordedPress;

/// What the window did with a press the hook offered it. The callback
/// cannot block, so the answer decides on the spot whether the key is
/// hidden from XAML or let through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delivery {
    /// Queued for the recorder: hide the key.
    Accepted,
    /// The recorder's queue is full. STILL hidden — a key lost is better
    /// than a key typed into whatever the form has focused.
    Full,
    /// Nobody is listening any more (the row stopped recording between
    /// the press and this call): let the key through.
    Gone,
}

impl Delivery {
    fn is_wanted(self) -> bool {
        matches!(self, Self::Accepted | Self::Full)
    }
}

/// What the callback does with one message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Decision {
    /// Let the window have it.
    PassThrough,
    /// Hide it from the window.
    Swallow,
    /// Hide it — and the hook has nothing left to protect, so it goes.
    SwallowAndRelease,
}

/// The keys hidden on the way down, so their key-ups are hidden too. A
/// `Vec` rather than a set: at most a handful of keys are held at once,
/// and a linear scan of four beats hashing them.
#[derive(Default)]
struct SwallowedKeys(Vec<u16>);

impl SwallowedKeys {
    fn press(&mut self, virtual_key: u16) {
        if !self.0.contains(&virtual_key) {
            self.0.push(virtual_key);
        }
    }

    /// Answers whether this key-up belongs to a hidden key-down, and
    /// forgets it.
    fn release(&mut self, virtual_key: u16) -> bool {
        let Some(index) = self.0.iter().position(|held| *held == virtual_key) else {
            return false;
        };
        self.0.swap_remove(index);
        true
    }

    fn contains(&self, virtual_key: u16) -> bool {
        self.0.contains(&virtual_key)
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The hook's state machine: which keys it has hidden, and whether a row
/// is still listening. Outlives any one recording — the keys a stopped
/// row hid are still owed their key-ups, so a new row takes the same
/// hook over rather than starting a fresh one.
#[derive(Default)]
struct Recording {
    swallowed: SwallowedKeys,
    is_listening: bool,
}

impl Recording {
    fn listening() -> Self {
        Self {
            swallowed: SwallowedKeys::default(),
            is_listening: true,
        }
    }

    /// A row starts recording. The hidden keys are KEPT: their key-ups are
    /// still owed to the window, whichever row is listening by then.
    fn start(&mut self) {
        self.is_listening = true;
    }

    /// A row stops. Answers whether the hook can go now — it cannot while
    /// a key it hid is still held.
    fn stop(&mut self) -> bool {
        self.is_listening = false;
        self.swallowed.is_empty()
    }

    /// `delivery` is what the window did with the press, or `None` for a
    /// key that carries no press (a modifier) or a window that is gone.
    /// `is_pass_through_key` is Tab, the one key reported but not hidden.
    fn on_key_down(
        &mut self,
        virtual_key: u16,
        delivery: Option<Delivery>,
        is_pass_through_key: bool,
    ) -> Decision {
        // A repeat of a key already hidden stays hidden, whatever the
        // delivery says: letting the repeats through mid-hold would type
        // into the form.
        let is_held = self.swallowed.contains(virtual_key);
        if !delivery.is_some_and(Delivery::is_wanted) && !is_held {
            return Decision::PassThrough;
        }
        if is_pass_through_key {
            // Never remembered as hidden, so its key-up goes to the
            // window too.
            return Decision::PassThrough;
        }
        self.swallowed.press(virtual_key);
        Decision::Swallow
    }

    fn on_key_up(&mut self, virtual_key: u16) -> Decision {
        if !self.swallowed.release(virtual_key) {
            return Decision::PassThrough;
        }
        if !self.is_listening && self.swallowed.is_empty() {
            return Decision::SwallowAndRelease;
        }
        Decision::Swallow
    }
}

/// Everything the callback needs, on the thread that installed the hook.
/// One per thread: at most one row records at a time, and a later row
/// takes this over rather than installing a second hook.
#[cfg(windows)]
struct HookState {
    hook: windows::Win32::UI::WindowsAndMessaging::HHOOK,
    /// `None` once the owning guard is dropped: the hook stays installed
    /// until the keys it hid are released, but reports nothing more.
    /// `Rc` so the callback can hold it across the delivery call without
    /// holding the thread-local's borrow — and without taking it out,
    /// which would lose it if the closure ever panicked.
    deliver: Option<std::rc::Rc<dyn Fn(RecordedPress) -> Delivery>>,
    recording: Recording,
    /// Which guard owns the current installation, so an older guard's
    /// `Drop` cannot stop a newer row's recording.
    owner: u64,
}

#[cfg(windows)]
thread_local! {
    static STATE: std::cell::RefCell<Option<HookState>> = const { std::cell::RefCell::new(None) };
    static NEXT_OWNER: std::cell::Cell<u64> = const { std::cell::Cell::new(1) };
}

/// The hook, for as long as a row is recording. Dropping it stops the
/// reporting; the hook itself outlives the drop until every key it hid has
/// been released, or until the next row takes it over.
#[cfg(windows)]
pub struct KeyboardHook {
    owner: u64,
    /// The hook belongs to the thread that installed it — and so does the
    /// thread-local it reads. A guard sent elsewhere would unhook nothing.
    _not_send: std::marker::PhantomData<*const ()>,
}

#[cfg(windows)]
impl KeyboardHook {
    /// Starts recording on the CALLING thread, reporting each press to
    /// `deliver`. A hook still draining an earlier row is taken over, keys
    /// and all; otherwise one is installed. `None` when it could not be.
    ///
    /// `deliver` runs inside the hook callback: it must do bounded,
    /// non-blocking work (queue the press and wake the UI), never call
    /// back into the hook, and never block.
    pub fn install(deliver: impl Fn(RecordedPress) -> Delivery + 'static) -> Option<Self> {
        use windows::Win32::System::Threading::GetCurrentThreadId;
        use windows::Win32::UI::WindowsAndMessaging::{SetWindowsHookExW, WH_KEYBOARD};
        let deliver: std::rc::Rc<dyn Fn(RecordedPress) -> Delivery> = std::rc::Rc::new(deliver);
        let owner = NEXT_OWNER.with(|next| {
            let owner = next.get();
            next.set(owner.wrapping_add(1));
            owner
        });
        let is_taken_over = with_state(|state| {
            state.deliver = Some(std::rc::Rc::clone(&deliver));
            state.recording.start();
            state.owner = owner;
        })
        .is_some();
        if !is_taken_over {
            // SAFETY: a thread-scoped hook (a real thread id, no module
            // handle) with a static callback; the handle is stored and
            // unhooked by `release_hook`.
            let hook = unsafe {
                SetWindowsHookExW(WH_KEYBOARD, Some(hook_proc), None, GetCurrentThreadId())
            }
            .ok()?;
            STATE.with(|state| {
                *state.borrow_mut() = Some(HookState {
                    hook,
                    deliver: Some(deliver),
                    recording: Recording::listening(),
                    owner,
                });
            });
        }
        Some(Self {
            owner,
            _not_send: std::marker::PhantomData,
        })
    }
}

#[cfg(windows)]
impl Drop for KeyboardHook {
    fn drop(&mut self) {
        let is_drained = with_state(|state| {
            // A newer row took the hook over: this guard owns nothing.
            if state.owner != self.owner {
                return false;
            }
            state.deliver = None;
            state.recording.stop()
        })
        .unwrap_or(false);
        if is_drained {
            release_hook();
        }
    }
}

/// Unhooks the thread's hook and forgets it. A FAILED unhook keeps the
/// state: the handle is the only way to try again, and dropping it would
/// leave a hook installed that the next `install` would stack a second
/// one on top of.
#[cfg(windows)]
fn release_hook() {
    use windows::Win32::UI::WindowsAndMessaging::UnhookWindowsHookEx;
    let Some(hook) = with_state(|state| state.hook) else {
        return;
    };
    // SAFETY: a handle this module installed and has not yet unhooked.
    // Called with no borrow of `STATE` held; `UnhookWindowsHookEx` is
    // documented as safe to call from inside the hook procedure.
    if let Err(error) = unsafe { UnhookWindowsHookEx(hook) } {
        log::error!("recorder.unhook_failed error={error}");
        return;
    }
    STATE
        .try_with(|state| {
            if let Ok(mut state) = state.try_borrow_mut() {
                *state = None;
            }
        })
        .ok();
}

/// The hook procedure. `nCode < 0` and anything the recorder does not want
/// falls through to the rest of the chain; a panic is caught and falls
/// through too — a hook must never unwind into Windows.
#[cfg(windows)]
unsafe extern "system" fn hook_proc(
    code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::LRESULT;
    use windows::Win32::UI::WindowsAndMessaging::CallNextHookEx;
    if code >= 0 {
        let is_swallowed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            handle((wparam.0 & 0xFFFF) as u16, HookMessage::decode(lparam.0))
        }))
        .unwrap_or(false);
        if is_swallowed {
            // Never falls through after a release: the hook this call is
            // running in may no longer exist.
            return LRESULT(1);
        }
    }
    // SAFETY: the parameters are the ones Windows passed; `None` asks it
    // for the next hook in this thread's chain.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// Answers whether this key is hidden from the window. Never holds the
/// thread-local's borrow across the delivery closure or across Win32.
#[cfg(windows)]
fn handle(virtual_key: u16, message: HookMessage) -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::VK_TAB;
    let decision = if message.is_key_up {
        with_state(|state| state.recording.on_key_up(virtual_key))
    } else {
        // Modifier and synthetic keys carry no press of their own, and the
        // window needs them: `recorded_press` answers `None` for both.
        let delivery = crate::key_translation::recorded_press(
            virtual_key,
            message.scan_code,
            message.is_repeat,
        )
        .and_then(|press| with_deliver(|deliver| deliver(press)));
        with_state(|state| {
            state
                .recording
                .on_key_down(virtual_key, delivery, virtual_key == VK_TAB.0)
        })
    };
    match decision.unwrap_or(Decision::PassThrough) {
        Decision::PassThrough => false,
        Decision::Swallow => true,
        Decision::SwallowAndRelease => {
            release_hook();
            true
        }
    }
}

/// Runs `act` on the thread's hook state. `None` when there is no hook,
/// when the state is already borrowed (a re-entrant callback passes the
/// key through rather than panicking), or at thread exit.
#[cfg(windows)]
fn with_state<T>(act: impl FnOnce(&mut HookState) -> T) -> Option<T> {
    STATE
        .try_with(|state| {
            let mut state = state.try_borrow_mut().ok()?;
            state.as_mut().map(act)
        })
        .ok()?
}

/// Hands the press to the delivery closure with NO borrow of the state
/// held: the closure queues into the window, which must be free to touch
/// anything but this hook.
#[cfg(windows)]
fn with_deliver<T>(act: impl FnOnce(&dyn Fn(RecordedPress) -> Delivery) -> T) -> Option<T> {
    let deliver = with_state(|state| state.deliver.clone())??;
    Some(act(deliver.as_ref()))
}

/// The host build has no Win32 to hook; the recorder is Windows-only, so
/// this exists to keep the crate compiling and testable on macOS.
#[cfg(not(windows))]
pub struct KeyboardHook;

#[cfg(not(windows))]
impl KeyboardHook {
    pub fn install(_deliver: impl Fn(RecordedPress) -> Delivery + 'static) -> Option<Self> {
        log::info!("recorder.hook_stub");
        None
    }
}

/// One `WH_KEYBOARD` message, decoded from its `lParam`
/// (<https://learn.microsoft.com/windows/win32/winmsg/keyboardproc>).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HookMessage {
    is_key_up: bool,
    /// The low 8 bits only — the extended-key flag is deliberately NOT
    /// folded in, so the shared translation gets exactly what the DLL's
    /// key sink has always passed it.
    scan_code: u32,
    is_repeat: bool,
}

impl HookMessage {
    /// Bit 31 = transition state (set on release), bit 30 = previous key
    /// state (set while the key was already down: a repeat), bits 16–23 =
    /// the scan code.
    fn decode(lparam: isize) -> Self {
        let bits = lparam as u32;
        Self {
            is_key_up: bits & (1 << 31) != 0,
            scan_code: (bits >> 16) & 0xFF,
            is_repeat: bits & (1 << 30) != 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Not Tab: every test key below is an ordinary one.
    const A: u16 = 0x41;
    const B: u16 = 0x42;

    fn down(recording: &mut Recording, key: u16, delivery: Delivery) -> Decision {
        recording.on_key_down(key, Some(delivery), false)
    }

    #[test]
    fn a_hook_message_reads_its_transition_repeat_and_scan_code() {
        // trace: a fresh key-down of scan code 0x1E — no transition bit,
        // no previous-state bit.
        let down = HookMessage::decode(0x001E_0001);
        assert_eq!(
            down,
            HookMessage {
                is_key_up: false,
                scan_code: 0x1E,
                is_repeat: false,
            }
        );
        // The same key held: bit 30 set.
        assert!(HookMessage::decode(0x401E_0001u32 as i32 as isize).is_repeat);
        // Released: bits 31 and 30 both set, as Windows sends them.
        let up = HookMessage::decode(0xC01E_0001u32 as i32 as isize);
        assert!(up.is_key_up);
        assert_eq!(up.scan_code, 0x1E);
    }

    #[test]
    fn a_hidden_key_is_hidden_again_on_release_and_the_hook_goes_with_the_last_one() {
        let mut recording = Recording::listening();
        assert_eq!(
            down(&mut recording, A, Delivery::Accepted),
            Decision::Swallow
        );
        assert_eq!(
            down(&mut recording, B, Delivery::Accepted),
            Decision::Swallow
        );
        // Still listening: releasing a key does not retire the hook.
        assert_eq!(recording.on_key_up(A), Decision::Swallow);
        assert_eq!(
            recording.on_key_up(A),
            Decision::PassThrough,
            "the up is claimed once"
        );
        // The row stops with B still held, so the hook stays for its up.
        assert!(!recording.stop(), "a held key is still owed its key-up");
        assert_eq!(recording.on_key_up(B), Decision::SwallowAndRelease);
    }

    #[test]
    fn a_row_that_stops_with_nothing_held_retires_the_hook_at_once() {
        let mut recording = Recording::listening();
        assert_eq!(
            down(&mut recording, A, Delivery::Accepted),
            Decision::Swallow
        );
        assert_eq!(recording.on_key_up(A), Decision::Swallow);
        assert!(recording.stop());
    }

    #[test]
    fn a_repeat_of_a_hidden_key_stays_hidden_even_after_the_row_stopped() {
        // trace: the guard is dropped mid-hold, so `deliver` is gone and
        // the delivery reads `None` — but the key is already hidden, and
        // letting its repeats through would type them into the form.
        let mut recording = Recording::listening();
        down(&mut recording, A, Delivery::Accepted);
        assert!(!recording.stop());
        assert_eq!(
            recording.on_key_down(A, None, false),
            Decision::Swallow,
            "a repeat mid-hold cannot reach the window"
        );
        assert_eq!(recording.on_key_up(A), Decision::SwallowAndRelease);
    }

    #[test]
    fn a_new_row_takes_the_hook_over_and_still_owes_the_old_rows_key_ups() {
        // trace: the BLOCK a force-release would reintroduce — the user
        // stopped one row while A was held and started another; A's key-up
        // must still not reach the window alone.
        let mut recording = Recording::listening();
        down(&mut recording, A, Delivery::Accepted);
        assert!(!recording.stop());
        recording.start();
        assert_eq!(recording.on_key_up(A), Decision::Swallow);
        assert!(recording.stop(), "nothing else was held");
    }

    #[test]
    fn a_full_queue_still_hides_the_key_but_a_gone_one_does_not() {
        let mut recording = Recording::listening();
        assert_eq!(
            down(&mut recording, A, Delivery::Full),
            Decision::Swallow,
            "a lost key beats a key typed into the form"
        );
        assert_eq!(recording.on_key_up(A), Decision::Swallow);
        // A key the window never wanted is never hidden, and neither is
        // its release.
        assert_eq!(
            down(&mut recording, B, Delivery::Gone),
            Decision::PassThrough
        );
        assert_eq!(recording.on_key_up(B), Decision::PassThrough);
    }

    #[test]
    fn tab_is_reported_but_reaches_the_form_and_so_does_its_release() {
        // trace: `evaluate_press` answers a bare Tab with `PassThrough` —
        // recording ends AND the focus moves on, which it cannot do if the
        // hook ate the key.
        let mut recording = Recording::listening();
        assert_eq!(
            recording.on_key_down(A, Some(Delivery::Accepted), true),
            Decision::PassThrough
        );
        assert_eq!(
            recording.on_key_up(A),
            Decision::PassThrough,
            "never remembered as hidden, so the up goes too"
        );
    }

    #[test]
    fn a_key_the_hook_never_took_is_not_its_to_release() {
        let mut recording = Recording::listening();
        assert_eq!(recording.on_key_up(A), Decision::PassThrough);
        assert!(recording.stop());
    }
}
