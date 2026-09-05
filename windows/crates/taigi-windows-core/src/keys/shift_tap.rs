//! Recognising a Shift TAP — one Shift pressed on its own and released with
//! no other key in between — which is a common Windows CJK convention for
//! switching 中/英: the USER confirmed it on 微軟注音 (2026-09-04), and
//! 新酷音 ships it on by default. Pure state, no Win32: the TSF key sink
//! reports presses and releases, this decides.
//!
//! The rule is 新酷音's (`references/PIME/python/input_methods/chewing/
//! chewing_ime.py:701-724`): the last key pressed must be Shift, the key
//! released must be that same Shift, and the press must be shorter than half
//! a second. Any other key pressed in between disqualifies the release, which
//! is what keeps Shift+A a capital A rather than a mode switch.
//!
//! One rule is ours rather than 新酷音's: a Shift pressed while another
//! modifier is already down never arms. Ctrl+Shift is the OS's own
//! keyboard-layout switch, and its Shift release must not also switch our
//! mode; the other Shift key counts too, so holding one and tapping the other
//! is a chord, not a tap. The two Shift keys share `VK_SHIFT` and are told
//! apart by scan code.

// 判斷「單獨短按 Shift」——按下到放開之間無別的鍵、且短於 0.5 秒。純狀態,無 Win32。
// 加一條咱家己的:別粒 modifier 咧壓的時陣按 Shift 無算(Ctrl+Shift 是系統換鍵盤配置;
// 另外彼粒 Shift 嘛算)。兩粒 Shift 用 scan code 分。

/// `VK_SHIFT`. Windows reports both Shift keys as this generic code in a key
/// message's `wParam`; the scan code is what says which one, and either may
/// tap.
pub const VK_SHIFT_CODE: u16 = 0x10;

/// The scan codes Windows gives the two Shift keys — the only thing that
/// tells them apart, since `wParam` folds both to [`VK_SHIFT_CODE`]. Here
/// rather than in the shell so the tracker's tests and the keyboard-state
/// read that feeds it cannot disagree about which key is which.
pub const LEFT_SHIFT_SCAN_CODE: u32 = 0x2A;
pub const RIGHT_SHIFT_SCAN_CODE: u32 = 0x36;

/// How long Shift may be held and still count as a tap. 新酷音's 0.5 s
/// (`chewing_ime.py:722`): long enough for a deliberate tap, short enough
/// that holding Shift to type capitals never switches the mode.
pub const SHIFT_TAP_MAX_MILLISECONDS: u64 = 500;

/// One physical key: the virtual-key code names it, the scan code says which
/// of a pair it is. Left and right Shift share `VK_SHIFT` and differ only
/// here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PhysicalKey {
    virtual_key: u16,
    scan_code: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PendingPress {
    key: PhysicalKey,
    pressed_at_milliseconds: u64,
}

/// The one press that could still become a tap: a Shift pressed with no
/// chord held, not yet released and not yet disqualified.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShiftTapTracker {
    pending: Option<PendingPress>,
}

impl ShiftTapTracker {
    /// A key went down.
    ///
    /// `is_repeat` is the message's previous-key-state bit: a held key's
    /// auto-repeat is NOT a new press, so it must neither restart the clock
    /// nor disqualify the press it repeats.
    ///
    /// `is_other_modifier_held` says whether Ctrl, Alt, Win or the OTHER Shift
    /// key was already down as this key went down — the shell reads it from
    /// the keyboard state, and only for a Shift press. Such a press arms
    /// nothing: it is half a chord, not a key pressed on its own.
    ///
    /// Called from both `OnTestKeyDown` and `OnKeyDown`, which fire back to
    /// back for one physical press; the second call rewrites the timestamp
    /// with a value microseconds newer, which no threshold can see. Recording
    /// in both is what makes the tracker survive a host that skips the test
    /// callback (terminals do — roadmap W5).
    pub fn observe_key_down(
        &mut self,
        virtual_key: u16,
        scan_code: u32,
        is_repeat: bool,
        is_other_modifier_held: bool,
        now_milliseconds: u64,
    ) {
        if is_repeat {
            return;
        }
        let key = PhysicalKey {
            virtual_key,
            scan_code,
        };
        if virtual_key == VK_SHIFT_CODE
            && !is_other_modifier_held
            && !self.is_other_shift_armed(key)
        {
            self.pending = Some(PendingPress {
                key,
                pressed_at_milliseconds: now_milliseconds,
            });
            return;
        }
        // Anything else going down ends the tap the pending press could have
        // been — the other Shift key included, since the armed one is then
        // still held and neither was pressed on its own.
        self.pending = None;
    }

    /// Whether a DIFFERENT Shift key is the one currently armed. The shell's
    /// keyboard-state read answers the same question, and this is what still
    /// answers it when that read is unavailable.
    fn is_other_shift_armed(&self, key: PhysicalKey) -> bool {
        self.pending.is_some_and(|pending| pending.key != key)
    }

    /// Whether releasing this key right now completes a Shift tap.
    /// Read-only: `OnTestKeyUp` asks this to answer "I may handle it" without
    /// spending the press, because the switch itself belongs in the delivery.
    pub fn is_tap_on_release(
        &self,
        virtual_key: u16,
        scan_code: u32,
        now_milliseconds: u64,
    ) -> bool {
        let key = PhysicalKey {
            virtual_key,
            scan_code,
        };
        self.pending.is_some_and(|pending| {
            pending.key == key
                && now_milliseconds.saturating_sub(pending.pressed_at_milliseconds)
                    < SHIFT_TAP_MAX_MILLISECONDS
        })
    }

    /// The delivered release: the same answer, and the press is spent either
    /// way — a release always ends the press it belongs to, so a second
    /// callback for the same release can never switch the mode twice.
    pub fn take_tap_on_release(
        &mut self,
        virtual_key: u16,
        scan_code: u32,
        now_milliseconds: u64,
    ) -> bool {
        let is_tap = self.is_tap_on_release(virtual_key, scan_code, now_milliseconds);
        self.pending = None;
        is_tap
    }

    /// Focus left, or the service is going away: a Shift still held belongs
    /// to whatever has the keyboard now, and its release must not tap here.
    pub fn clear(&mut self) {
        self.pending = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VK_A: u16 = 0x41;
    const VK_CONTROL: u16 = 0x11;
    const LEFT_SHIFT_SCAN: u32 = LEFT_SHIFT_SCAN_CODE;
    const RIGHT_SHIFT_SCAN: u32 = RIGHT_SHIFT_SCAN_CODE;
    const SCAN_A: u32 = 0x1E;
    const SCAN_CONTROL: u32 = 0x1D;

    fn tracker_with_left_shift_down(at: u64) -> ShiftTapTracker {
        let mut tracker = ShiftTapTracker::default();
        tracker.observe_key_down(VK_SHIFT_CODE, LEFT_SHIFT_SCAN, false, false, at);
        tracker
    }

    #[test]
    fn press_and_release_shift_inside_the_window_is_a_tap() {
        let mut tracker = tracker_with_left_shift_down(1_000);
        assert!(tracker.take_tap_on_release(VK_SHIFT_CODE, LEFT_SHIFT_SCAN, 1_120));
    }

    #[test]
    fn either_shift_key_taps() {
        let mut tracker = ShiftTapTracker::default();
        tracker.observe_key_down(VK_SHIFT_CODE, RIGHT_SHIFT_SCAN, false, false, 1_000);
        assert!(tracker.take_tap_on_release(VK_SHIFT_CODE, RIGHT_SHIFT_SCAN, 1_050));
    }

    #[test]
    fn holding_shift_past_the_window_is_not_a_tap() {
        let mut tracker = tracker_with_left_shift_down(1_000);
        assert!(!tracker.take_tap_on_release(
            VK_SHIFT_CODE,
            LEFT_SHIFT_SCAN,
            1_000 + SHIFT_TAP_MAX_MILLISECONDS
        ));
    }

    #[test]
    fn a_key_pressed_while_shift_is_held_disqualifies_the_release() {
        let mut tracker = tracker_with_left_shift_down(1_000);
        tracker.observe_key_down(VK_A, SCAN_A, false, false, 1_050);
        assert!(!tracker.take_tap_on_release(VK_SHIFT_CODE, LEFT_SHIFT_SCAN, 1_100));
    }

    #[test]
    fn shift_pressed_under_a_chord_never_arms() {
        // Ctrl+Shift is the OS keyboard-layout switch: releasing its Shift
        // must not also switch our mode.
        let mut tracker = ShiftTapTracker::default();
        tracker.observe_key_down(VK_CONTROL, SCAN_CONTROL, false, false, 1_000);
        tracker.observe_key_down(VK_SHIFT_CODE, LEFT_SHIFT_SCAN, false, true, 1_010);
        assert!(!tracker.take_tap_on_release(VK_SHIFT_CODE, LEFT_SHIFT_SCAN, 1_060));
    }

    #[test]
    fn the_second_shift_disarms_the_first_instead_of_re_arming() {
        // Left held, right tapped: the left one is still down, so nothing was
        // pressed "on its own" and neither release taps. Answered from the
        // tracker's own state, with the shell reporting no other modifier —
        // this is the case that must not depend on the keyboard-state read.
        let mut tracker = tracker_with_left_shift_down(1_000);
        tracker.observe_key_down(VK_SHIFT_CODE, RIGHT_SHIFT_SCAN, false, false, 1_020);
        assert!(!tracker.take_tap_on_release(VK_SHIFT_CODE, RIGHT_SHIFT_SCAN, 1_050));
        assert!(!tracker.take_tap_on_release(VK_SHIFT_CODE, LEFT_SHIFT_SCAN, 1_080));
    }

    #[test]
    fn a_shift_pressed_while_the_other_is_held_never_arms_even_after_a_letter() {
        // The letter clears what the tracker knew about the held left Shift,
        // so the shell's keyboard-state read is what disqualifies the right
        // one here.
        let mut tracker = tracker_with_left_shift_down(1_000);
        tracker.observe_key_down(VK_A, SCAN_A, false, false, 1_020);
        tracker.observe_key_down(VK_SHIFT_CODE, RIGHT_SHIFT_SCAN, false, true, 1_040);
        assert!(!tracker.take_tap_on_release(VK_SHIFT_CODE, RIGHT_SHIFT_SCAN, 1_070));
    }

    #[test]
    fn auto_repeat_does_not_restart_the_clock() {
        let mut tracker = tracker_with_left_shift_down(1_000);
        for repeat_at in [1_500, 1_530, 1_560] {
            tracker.observe_key_down(VK_SHIFT_CODE, LEFT_SHIFT_SCAN, true, false, repeat_at);
        }
        assert!(!tracker.take_tap_on_release(VK_SHIFT_CODE, LEFT_SHIFT_SCAN, 1_580));
    }

    #[test]
    fn the_auto_repeat_of_another_held_key_does_not_disarm() {
        // A letter held down while Shift is tapped already disarmed the tap
        // on its FIRST press; its repeats must not be read as fresh presses
        // that could disarm a later, legitimate one.
        let mut tracker = ShiftTapTracker::default();
        tracker.observe_key_down(VK_A, SCAN_A, false, false, 1_000);
        tracker.observe_key_down(VK_SHIFT_CODE, LEFT_SHIFT_SCAN, false, false, 1_100);
        tracker.observe_key_down(VK_A, SCAN_A, true, false, 1_150);
        assert!(tracker.take_tap_on_release(VK_SHIFT_CODE, LEFT_SHIFT_SCAN, 1_200));
    }

    #[test]
    fn the_test_and_delivery_callbacks_of_one_press_agree() {
        // Both key-down callbacks fire for one physical press; the second
        // must not turn a hold into a tap.
        let mut tracker = ShiftTapTracker::default();
        tracker.observe_key_down(VK_SHIFT_CODE, LEFT_SHIFT_SCAN, false, false, 1_000);
        tracker.observe_key_down(VK_SHIFT_CODE, LEFT_SHIFT_SCAN, false, false, 1_001);
        assert!(tracker.is_tap_on_release(VK_SHIFT_CODE, LEFT_SHIFT_SCAN, 1_100));
        assert!(tracker.take_tap_on_release(VK_SHIFT_CODE, LEFT_SHIFT_SCAN, 1_100));
    }

    #[test]
    fn only_the_delivered_release_spends_the_press() {
        let mut tracker = tracker_with_left_shift_down(1_000);
        assert!(tracker.is_tap_on_release(VK_SHIFT_CODE, LEFT_SHIFT_SCAN, 1_050));
        assert!(tracker.is_tap_on_release(VK_SHIFT_CODE, LEFT_SHIFT_SCAN, 1_060));
        assert!(tracker.take_tap_on_release(VK_SHIFT_CODE, LEFT_SHIFT_SCAN, 1_070));
        assert!(!tracker.take_tap_on_release(VK_SHIFT_CODE, LEFT_SHIFT_SCAN, 1_080));
    }

    #[test]
    fn releasing_another_key_spends_the_press_without_tapping() {
        let mut tracker = tracker_with_left_shift_down(1_000);
        assert!(!tracker.take_tap_on_release(VK_A, SCAN_A, 1_050));
        assert!(!tracker.take_tap_on_release(VK_SHIFT_CODE, LEFT_SHIFT_SCAN, 1_060));
    }

    #[test]
    fn a_release_with_no_press_behind_it_is_not_a_tap() {
        let mut tracker = ShiftTapTracker::default();
        assert!(!tracker.take_tap_on_release(VK_SHIFT_CODE, LEFT_SHIFT_SCAN, 1_000));
    }

    #[test]
    fn focus_loss_disarms_a_shift_held_across_the_switch() {
        let mut tracker = tracker_with_left_shift_down(1_000);
        tracker.clear();
        assert!(!tracker.take_tap_on_release(VK_SHIFT_CODE, LEFT_SHIFT_SCAN, 1_050));
    }

    #[test]
    fn a_tick_source_that_did_not_advance_still_taps() {
        // `GetTickCount64` has ~15 ms resolution: a fast tap can press and
        // release inside one tick, and zero elapsed is the shortest tap
        // there is, not a missing one.
        let mut tracker = tracker_with_left_shift_down(1_000);
        assert!(tracker.take_tap_on_release(VK_SHIFT_CODE, LEFT_SHIFT_SCAN, 1_000));
    }
}
