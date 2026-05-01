import Foundation

/// `LexiconBitmask` — bitmask → `[DictionarySource]` decoder.
///
/// CROSS-PLATFORM INVARIANT — bit positions mirror
/// `dictionary/common/source_bits.py` (SOURCE_BITS + IS_VARIANT_BIT at
/// bit 12), `engine/lexicon/src/dictionary_reader.rs` constants, and
/// Android `LexiconBitmask` equivalent. Drift causes silent filter +
/// ranking divergence.
enum LexiconBitmask {
    /// Decode a record bitmask (`UInt32` from `RustEngineBridge.LexiconRow`)
    /// into the ordered list of `DictionarySource` values. Order matches
    /// the bit position so iOS UI badges render predictably.
    static func sources(from bitmask: UInt32) -> [DictionarySource] {
        let pairs: [(bit: UInt32, source: DictionarySource)] = [
            (1 << 0, .kautian), (1 << 1, .taigitv), (1 << 2, .itaigi), (1 << 3, .sitbut),
            (1 << 4, .taihoa), (1 << 5, .taijit), (1 << 6, .kungge), (1 << 7, .stti),
            (1 << 8, .khpoo), (1 << 9, .khiin), (1 << 10, .dev), (1 << 11, .lkk),
        ]
        return pairs.compactMap { bitmask & $0.bit != 0 ? $0.source : nil }
    }
}
