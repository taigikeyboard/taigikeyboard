// Which keys type a tone — and, by the same choice, which keys pick a candidate.

/// Which keys type a tone — the half of the key contract that decides, in
/// turn, which keys pick a candidate.
///
/// Standard and Telex swap eight letters and the nine digits (USER
/// 2026-09-08). Under `standard` a digit is the TL/POJ tone marker (`tai5`),
/// so the letters no syllable spells are free to pick candidates; under
/// `telex` those letters carry the tones, so the digits are free to pick.
/// Neither half is chosen on its own: the slot key set is derived here
/// (`slotKeySet`) rather than stored, so the two can never disagree about who
/// owns `v` or `3`. Telex also claims `c` and `h`, which pick nothing under
/// Standard and stay plain letters there.
///
/// The letter table is the kahiok scheme (madmaxieee/taigi-telex) with `c` as
/// the unaspirated affricate rather than its `c` → `tsh`, which leaves `z` to
/// write tone 6 (USER 2026-11-05). The semantics — which tone each key writes,
/// how `c` resolves by input mode, what a repeated key does — live in the
/// engine (`engine/composing/src/telex.rs`); this side only decides which keys
/// reach it.
enum ToneInputScheme: String, CaseIterable, Sendable {
    /// Digits type tones; `q w d f z x v y ;` pick candidates. The shipped
    /// default, and today's behaviour before the scheme existed.
    case standard
    /// Letters type tones; `1`…`9` pick candidates.
    case telex

    /// The keys that pick a candidate under this scheme.
    var slotKeySet: CandidateSlotKeySet {
        self == .telex ? .digits : .bareKeys
    }
}

/// Which keys the candidate window draws beside its nine slots.
///
/// The window's LABELS only: which key picks which slot is desktop-core's
/// (`keys/slot_key_set.rs`), and the core reads it from the same tone scheme
/// (`ToneInputScheme.slotKeySet`), so a key drawn beside a candidate is the
/// key that picks it — held equal by `CandidateSlotKeyTests`, against the
/// Shortcuts pane's slot row the core draws. The letters and the digits are
/// the same keys under both schemes, with the two jobs swapped: whichever
/// keys type the tones leaves the others free to pick.
enum CandidateSlotKeySet: CaseIterable, Sendable {
    /// Nine bare keys, one per slot — `q w d f z x v y ;`: the eight letters
    /// no TL or POJ syllable spells, and `;`. The set under `standard`
    /// (USER 2026-08-28).
    case bareKeys
    /// Bare `1`…`9`, the set under `telex`, where the letters above type the
    /// tones.
    case digits
    /// `1`…`9` of the number row and the keypad, the set under TPS
    /// (`SettingsStore.candidateSlotKeySet`). The number row picks only once
    /// the window is up, which under TPS opens on demand (desktop TPS
    /// roadmap D7). Drawn as the digits, as desktop-core labels it.
    case tpsDigits

    /// The keys `bareKeys` puts on slots 0…8, in slot order. Lowercase, as a
    /// bare key types its lowercase form.
    static let bareKeyRow = ["q", "w", "d", "f", "z", "x", "v", "y", ";"]

    /// The key this set gives the candidate in `slot`, for the nine slots a
    /// page holds (`CandidateIndexLabel`, which owns which key is drawn).
    func label(forSlot slot: Int) -> String {
        switch self {
        case .bareKeys: Self.bareKeyRow[slot]
        case .digits, .tpsDigits: String(slot + 1)
        }
    }
}
