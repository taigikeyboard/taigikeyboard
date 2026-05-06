package com.siansiansu.taigikeyboard.ui

import android.content.res.Configuration
import androidx.activity.ComponentActivity
import androidx.activity.enableEdgeToEdge
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.WindowCompat

// Edge-to-edge display setup extensions for Activity classes
fun AppCompatActivity.setupEdgeToEdge() {
    enableEdgeToEdge()
    WindowCompat.getInsetsController(window, window.decorView).apply {
        isAppearanceLightStatusBars = !isDarkMode()
    }
}

fun ComponentActivity.setupEdgeToEdge() {
    enableEdgeToEdge()
}

private fun AppCompatActivity.isDarkMode(): Boolean {
    val nightModeFlags = resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK
    return nightModeFlags == Configuration.UI_MODE_NIGHT_YES
}
