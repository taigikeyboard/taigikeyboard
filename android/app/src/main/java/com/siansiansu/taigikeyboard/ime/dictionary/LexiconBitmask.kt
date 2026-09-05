package com.siansiansu.taigikeyboard.ime.dictionary

/**
 * `LexiconBitmask` — bitmask → `List<DictionarySource>` decoder.
 *
 * CROSS-PLATFORM INVARIANT — bit positions mirror
 * `dictionary/common/source_bits.py` (SOURCE_BITS + IS_VARIANT_BIT at
 * bit 12), `engine/lexicon/src/dictionary_reader.rs` constants, and iOS
 * `LexiconBitmask`. Drift causes silent filter + ranking divergence.
 */
object LexiconBitmask {
    fun sourcesFromBitmask(bitmask: Int): List<DictionarySource> {
        val pairs = listOf(
            (1 shl 0) to DictionarySource.KAUTIAN,
            (1 shl 1) to DictionarySource.TAIGITV,
            (1 shl 2) to DictionarySource.ITAIGI,
            (1 shl 3) to DictionarySource.SITBUT,
            (1 shl 4) to DictionarySource.TAIHOA,
            (1 shl 5) to DictionarySource.TAIJIT,
            (1 shl 6) to DictionarySource.KUNGGE,
            (1 shl 7) to DictionarySource.STTI,
            (1 shl 8) to DictionarySource.KHPOO,
            (1 shl 9) to DictionarySource.KHIIN,
            (1 shl 10) to DictionarySource.DEV,
            (1 shl 11) to DictionarySource.LKK,
        )
        return pairs.mapNotNull { (bit, source) ->
            if (bitmask and bit != 0) source else null
        }
    }
}
