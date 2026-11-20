package com.siansiansu.taigikeyboard.ime.text.candidates

import org.junit.Assert.assertEquals
import org.junit.Test

// Pins the English current-word rule; iOS twin: EnglishAutocompleteServiceTests.
class EnglishCurrentWordTest {
    private fun word(text: String) = EnglishAutocompleteService.extractCurrentWord(text)

    @Test
    fun wordAfterLastSeparator() {
        assertEquals("hel", word("say hel"))
        assertEquals("hello", word("hello"))
    }

    @Test
    fun punctuationSplits() {
        assertEquals("", word("hi,"))
        assertEquals("the", word("hi,the"))
        assertEquals("Wor", word("end. Wor"))
        assertEquals("wor", word("line\nwor"))
    }

    @Test
    fun apostropheAndHyphenStayInWord() {
        assertEquals("don't", word("I don't"))
        assertEquals("well-known", word("a well-known"))
    }

    @Test
    fun trailingWhitespaceDropped() {
        assertEquals("hello", word("hello "))
        assertEquals("", word(""))
        assertEquals("", word("   "))
    }
}
