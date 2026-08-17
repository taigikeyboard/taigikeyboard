// Which dictionary a result came from.

import Foundation

/// One dictionary a word can be found in.
///
/// The bitmask layout these decode from is owned by
/// `dictionary/common/source_bits.py` and read by
/// `engine/lexicon/src/dictionary_reader.rs`; the case order here is not
/// load-bearing. `custom` has no bit — it marks a row that came from the
/// user's own dictionary rather than from `dictionary.bin`.
///
/// Badge labels live where the badges are drawn, not here: several sources
/// share one label, and which ones is a display decision.
enum DictionarySource: String, CaseIterable, Sendable {
    case kautian // 教育部臺灣台語常用詞辭典
    case taigitv // 台語新詞辭庫
    case itaigi // iTaigi 華台對照典
    case sitbut // 台灣植物名彙
    case taihoa // 台華線頂對照典
    case taijit // 台日大辭典
    case kungge // 台語工藝詞庫
    case stti // 學科術語辭典
    case khpoo // 腔口補充資料
    case khiin // 在來字
    case lkk // LKK 漢羅合用建議用字
    case dev // 詞庫增補檔案
    case custom // 自訂詞庫
}

/// Reads the per-record source bitmask the engine returns with a search row.
enum LexiconBitmask {
    /// CROSS-PLATFORM INVARIANT — bit positions mirror
    /// `dictionary/common/source_bits.py` (SOURCE_BITS),
    /// `engine/lexicon/src/dictionary_reader.rs`,
    /// ios/Sources/TaigiKeyboard/Lexicon/Utils/LexiconBitmask.swift:20-27 and
    /// the Android `LexiconBitmask`. Drift silently mislabels every result.
    ///
    /// Bit 12 (異用字) is deliberately absent: it marks a record as a variant
    /// spelling rather than naming a dictionary, and it has never been a badge.
    /// Bits above it are the kautian subcollection wire region, which describes
    /// the QUERY rather than the row, and are ignored here for the same reason.
    private static let sourceBits: [(bit: UInt32, source: DictionarySource)] = [
        (1 << 0, .kautian),
        (1 << 1, .taigitv),
        (1 << 2, .itaigi),
        (1 << 3, .sitbut),
        (1 << 4, .taihoa),
        (1 << 5, .taijit),
        (1 << 6, .kungge),
        (1 << 7, .stti),
        (1 << 8, .khpoo),
        (1 << 9, .khiin),
        (1 << 10, .dev),
        (1 << 11, .lkk),
    ]

    /// The sources a record belongs to, in bit order — which is the order the
    /// badges are drawn in.
    static func sources(from bitmask: UInt32) -> [DictionarySource] {
        sourceBits.compactMap { bitmask & $0.bit != 0 ? $0.source : nil }
    }
}
