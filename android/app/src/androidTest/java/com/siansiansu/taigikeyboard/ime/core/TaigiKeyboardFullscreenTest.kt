package com.siansiansu.taigikeyboard.ime.core

import android.content.Context
import android.content.ContextWrapper
import android.content.res.Configuration
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertFalse
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Layout audit A6: in landscape the framework's default fullscreen mode showed a transparent
 * extract editor over the status bar and the app. The keyboard never goes fullscreen.
 */
@RunWith(AndroidJUnit4::class)
class TaigiKeyboardFullscreenTest {
    @Test
    fun landscape_doesNotEvaluateToFullscreen() {
        val target = InstrumentationRegistry.getInstrumentation().targetContext
        val landscape =
            target.createConfigurationContext(
                Configuration(target.resources.configuration).apply {
                    orientation = Configuration.ORIENTATION_LANDSCAPE
                },
            )
        var result = true
        InstrumentationRegistry.getInstrumentation().runOnMainSync {
            val service = TaigiKeyboard()
            // The framework default answers true for landscape with no editor flags; attach a
            // landscape base context so that default would be reached without the override.
            ContextWrapper::class.java
                .getDeclaredMethod("attachBaseContext", Context::class.java)
                .apply { isAccessible = true }
                .invoke(service, landscape)
            result = service.onEvaluateFullscreenMode()
        }
        assertFalse(result)
    }
}
