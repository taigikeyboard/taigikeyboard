package com.siansiansu.taigikeyboard.util

import android.content.res.Configuration
import androidx.activity.ComponentActivity
import androidx.activity.enableEdgeToEdge
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsControllerCompat

/**
 * 為 AppCompatActivity 設定 Edge-to-Edge 顯示模式
 * 遵循 Android 15+ 的最佳實踐
 */
fun AppCompatActivity.setupEdgeToEdge() {
    // 使用 Android 官方推薦的 enableEdgeToEdge API
    enableEdgeToEdge()

    // 設定狀態列圖示顏色（根據目前主題）
    WindowCompat.getInsetsController(window, window.decorView).apply {
        isAppearanceLightStatusBars = !isDarkMode()
    }
}

/**
 * 為 ComponentActivity 設定 Edge-to-Edge 顯示模式
 * 適用於 Compose Activity
 */
fun ComponentActivity.setupEdgeToEdge() {
    // 使用 Android 官方推薦的 enableEdgeToEdge API
    enableEdgeToEdge()
}

/**
 * 檢查目前是否為深色模式
 */
private fun AppCompatActivity.isDarkMode(): Boolean {
    val nightModeFlags = resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK
    return nightModeFlags == Configuration.UI_MODE_NIGHT_YES
}
