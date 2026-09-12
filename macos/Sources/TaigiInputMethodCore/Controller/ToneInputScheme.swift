// Which keys type a tone — and, by the same choice, which keys pick a candidate.

/// Which keys type a tone — the half of the key contract that decides, in
/// turn, which keys pick a candidate.
///
/// Standard and Telex are the same eight letters and the same nine digits,
/// swapped (USER 2026-09-08). Under `standard` a digit is the TL/POJ tone
/// marker (`tai5`), so the letters no syllable spells are free to pick
/// candidates; under `telex` those letters carry the tones, so the digits
/// are free to pick. Neither half is chosen on its own: the slot key set is
/// derived here (`slotKeySet`) rather than stored, so the two can never
/// disagree about who owns `v` or `3`.
///
/// The letter table is the kahiok scheme (madmaxieee/taigi-telex) minus its
/// `c` → `tsh` key, because POJ spells `ch` / `chh` with `c`. Tone 6 has no
/// free letter and is not offered. The semantics — which tone each key
/// writes, how `z` resolves by input mode, what a repeated key does — live in
/// the engine (`engine/composing/src/telex.rs`); this side only decides which
/// keys reach it.
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

    /// Lower-case Telex keys: `v y d w x q` for tones 2 3 5 7 8 9, `z` for
    /// the affricate initial (`ts` / `ch`; `zh` then spells `tsh` / `chh`),
    /// `f` for the hyphen. Mirrors the engine's `composing::telex::TELEX_KEYS`
    /// — the two lists must name the same keys, or a key classified as Telex
    /// here would be ignored there.
    static let telexKeys: Set<Character> = ["v", "y", "d", "w", "x", "q", "z", "f"]

    /// Whether `character` is a Telex key in either case. ASCII only: the
    /// engine reads the key as an ASCII letter, and a `v` from another script
    /// is document text.
    static func isTelexKey(_ character: Character) -> Bool {
        guard character.isASCII, let folded = character.lowercased().first else { return false }
        return telexKeys.contains(folded)
    }

    /// Whether `character` may start a composition on its own. Only `z` /
    /// `Z`: it types an initial, so it begins a syllable the way any letter
    /// does. A tone letter or `f` has nothing to attach to when idle, and
    /// passes to the host like an idle digit.
    static func startsComposition(_ character: Character) -> Bool {
        character == "z" || character == "Z"
    }
}
