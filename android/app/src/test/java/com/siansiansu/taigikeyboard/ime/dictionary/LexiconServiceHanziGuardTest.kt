package com.siansiansu.taigikeyboard.ime.dictionary

import org.junit.Ignore
import org.junit.Test

/**
 * `INVARIANT_LEX_HANZI_GUARD` — D-8 parity correction toward Android
 * (v3.5.6). Stub awaiting the platform test infrastructure that lets us
 * stand up a `LexiconService` instance without Robolectric / instrumented
 * `Context` plumbing. The Rust engine-layer guard is fully tested in
 * `engine/lexicon/tests/parity.rs::invariant_lex_hanzi_guard_short_circuits`;
 * the Android service-layer guard is a 1-line check at the top of
 * `LexiconService.search()` (`if (inputType is InputType.Hanzi) return
 * Outcome.Success(emptyList())`). Adding the platform-layer parity test
 * requires Mockito/Robolectric setup which lands as a follow-up.
 *
 * Per `feedback_path_g_delete_mirrors.md`, this test stays in
 * `src/test/java` (pure JVM, no `librust_taigi.so` load); it does NOT
 * move to `androidTest`. See `docs/architecture/behavioral-invariants.md`
 * §14 implementation-status note.
 */
class LexiconServiceHanziGuardTest {
    @Test
    @Ignore("D-8 platform parity test pending — see file-level KDoc + invariants.md §14")
    fun `hanzi inputType returns empty list short-circuiting before bridge`() {
        // Implementation lands with Mockito/Robolectric infra.
    }
}
