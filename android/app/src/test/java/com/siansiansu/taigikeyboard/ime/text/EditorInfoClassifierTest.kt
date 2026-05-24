package com.siansiansu.taigikeyboard.ime.text

import android.text.InputType
import android.view.inputmethod.EditorInfo
import com.siansiansu.taigikeyboard.ime.text.key.KeyVariation
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardMode
import org.junit.Assert.assertEquals
import org.junit.Test

class EditorInfoClassifierTest {
    private fun editorInfo(inputType: Int): EditorInfo = EditorInfo().apply { this.inputType = inputType }

    @Test
    fun classify_numberClass_disablesComposing() {
        val result = EditorInfoClassifier.classify(editorInfo(InputType.TYPE_CLASS_NUMBER))
        assertEquals(KeyboardMode.NUMERIC, result.mode)
        assertEquals(KeyVariation.NORMAL, result.keyVariation)
        assertEquals(false, result.isComposingEnabled)
    }

    @Test
    fun classify_phoneClass_disablesComposing() {
        val result = EditorInfoClassifier.classify(editorInfo(InputType.TYPE_CLASS_PHONE))
        assertEquals(KeyboardMode.PHONE, result.mode)
        assertEquals(KeyVariation.NORMAL, result.keyVariation)
        assertEquals(false, result.isComposingEnabled)
    }

    @Test
    fun classify_textPasswordVariation_disablesComposing() {
        val result = EditorInfoClassifier.classify(
            editorInfo(InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD),
        )
        assertEquals(KeyboardMode.CHARACTERS, result.mode)
        assertEquals(KeyVariation.PASSWORD, result.keyVariation)
        assertEquals(false, result.isComposingEnabled)
    }

    @Test
    fun classify_textVisiblePassword_disablesComposing() {
        val result = EditorInfoClassifier.classify(
            editorInfo(InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD),
        )
        assertEquals(KeyVariation.PASSWORD, result.keyVariation)
        assertEquals(false, result.isComposingEnabled)
    }

    @Test
    fun classify_textWebPassword_disablesComposing() {
        val result = EditorInfoClassifier.classify(
            editorInfo(InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_WEB_PASSWORD),
        )
        assertEquals(KeyVariation.PASSWORD, result.keyVariation)
        assertEquals(false, result.isComposingEnabled)
    }

    @Test
    fun classify_textEmail_setsEmailVariation() {
        val result = EditorInfoClassifier.classify(
            editorInfo(InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS),
        )
        assertEquals(KeyboardMode.CHARACTERS, result.mode)
        assertEquals(KeyVariation.EMAIL_ADDRESS, result.keyVariation)
        assertEquals(true, result.isComposingEnabled)
    }

    @Test
    fun classify_textWebEmail_setsEmailVariation() {
        val result = EditorInfoClassifier.classify(
            editorInfo(InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_WEB_EMAIL_ADDRESS),
        )
        assertEquals(KeyVariation.EMAIL_ADDRESS, result.keyVariation)
        assertEquals(true, result.isComposingEnabled)
    }

    @Test
    fun classify_textUri_setsUriVariation() {
        val result = EditorInfoClassifier.classify(
            editorInfo(InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_URI),
        )
        assertEquals(KeyboardMode.CHARACTERS, result.mode)
        assertEquals(KeyVariation.URI, result.keyVariation)
        assertEquals(true, result.isComposingEnabled)
    }

    @Test
    fun classify_textNormal_charactersNormalComposing() {
        val result = EditorInfoClassifier.classify(editorInfo(InputType.TYPE_CLASS_TEXT))
        assertEquals(KeyboardMode.CHARACTERS, result.mode)
        assertEquals(KeyVariation.NORMAL, result.keyVariation)
        assertEquals(true, result.isComposingEnabled)
    }

    @Test
    fun classify_unknownClass_fallsBackToCharactersNormal() {
        val result = EditorInfoClassifier.classify(editorInfo(InputType.TYPE_CLASS_DATETIME))
        assertEquals(KeyboardMode.CHARACTERS, result.mode)
        assertEquals(KeyVariation.NORMAL, result.keyVariation)
        assertEquals(true, result.isComposingEnabled)
    }

    @Test
    fun classify_passwordWithCapWordsFlag_stillMapsToPassword() {
        val result = EditorInfoClassifier.classify(
            editorInfo(
                InputType.TYPE_CLASS_TEXT or
                    InputType.TYPE_TEXT_FLAG_CAP_WORDS or
                    InputType.TYPE_TEXT_VARIATION_PASSWORD,
            ),
        )
        assertEquals(KeyboardMode.CHARACTERS, result.mode)
        assertEquals(KeyVariation.PASSWORD, result.keyVariation)
        assertEquals(false, result.isComposingEnabled)
    }

    @Test
    fun classify_emailWithMultiLineFlag_stillMapsToEmail() {
        val result = EditorInfoClassifier.classify(
            editorInfo(
                InputType.TYPE_CLASS_TEXT or
                    InputType.TYPE_TEXT_FLAG_MULTI_LINE or
                    InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS,
            ),
        )
        assertEquals(KeyVariation.EMAIL_ADDRESS, result.keyVariation)
        assertEquals(true, result.isComposingEnabled)
    }

    @Test
    fun classify_uriWithAutoCorrectFlag_stillMapsToUri() {
        val result = EditorInfoClassifier.classify(
            editorInfo(
                InputType.TYPE_CLASS_TEXT or
                    InputType.TYPE_TEXT_FLAG_AUTO_CORRECT or
                    InputType.TYPE_TEXT_VARIATION_URI,
            ),
        )
        assertEquals(KeyVariation.URI, result.keyVariation)
        assertEquals(true, result.isComposingEnabled)
    }
}
