package com.siansiansu.taigikeyboard.ime.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pure-JVM tests for the shared-core candidate [Outcome] sum type.
 *
 * Asserts Success/Failure construction, `when` exhaustiveness, and
 * reference equality of payload values — no `inline` or extension helpers
 * are exercised because those are forbidden in shared-core candidate files
 * (`rules/android-guidelines.md` §1).
 */
class OutcomeTest {
    @Test
    fun `Success carries typed value payload`() {
        val outcome: Outcome<Int, String> = Outcome.Success(42)

        assertTrue(outcome is Outcome.Success)
        assertEquals(42, (outcome as Outcome.Success).value)
    }

    @Test
    fun `Failure carries typed error payload`() {
        val outcome: Outcome<Int, String> = Outcome.Failure("boom")

        assertTrue(outcome is Outcome.Failure)
        assertEquals("boom", (outcome as Outcome.Failure).error)
    }

    @Test
    fun `when-branches destructure Success path`() {
        val outcome: Outcome<List<String>, DummyError> = Outcome.Success(listOf("a", "b"))

        val unwrapped =
            when (outcome) {
                is Outcome.Success -> outcome.value
                is Outcome.Failure -> emptyList()
            }

        assertEquals(listOf("a", "b"), unwrapped)
    }

    @Test
    fun `when-branches destructure Failure path`() {
        val outcome: Outcome<List<String>, DummyError> = Outcome.Failure(DummyError.NotFound)

        val unwrapped =
            when (outcome) {
                is Outcome.Success -> outcome.value
                is Outcome.Failure -> emptyList()
            }

        assertEquals(emptyList<String>(), unwrapped)
    }

    private sealed class DummyError {
        object NotFound : DummyError()
    }
}
