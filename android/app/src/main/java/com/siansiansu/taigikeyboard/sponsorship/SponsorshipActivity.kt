package com.siansiansu.taigikeyboard.sponsorship

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.lifecycle.lifecycleScope
import com.siansiansu.taigikeyboard.billing.BillingManager
import com.siansiansu.taigikeyboard.util.setupEdgeToEdge
import kotlinx.coroutines.launch

/**
 * 贊助頁面 Activity
 */
class SponsorshipActivity : ComponentActivity() {

    private lateinit var billingManager: BillingManager
    private lateinit var viewModel: SponsorshipViewModel

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // 設定 Edge-to-Edge 顯示模式
        setupEdgeToEdge()

        // 初始化 BillingManager 和 ViewModel
        billingManager = BillingManager(applicationContext)
        viewModel = SponsorshipViewModel(billingManager)

        setContent {
            SponsorshipTheme {
                SponsorshipScreen(
                    viewModel = viewModel,
                    onDismiss = { finish() }
                )
            }
        }
    }

    override fun onResume() {
        super.onResume()
        // 每次返回時重新查詢購買狀態
        lifecycleScope.launch {
            billingManager.queryPurchases()
        }
    }
}

/**
 * 贊助頁面主題 - 強制使用 Light Mode
 * SPY×FAMILY 復古主題僅支援 Light Mode
 */
@Composable
fun SponsorshipTheme(
    darkTheme: Boolean = false,  // 強制使用 Light Mode，不跟隨系統設定
    content: @Composable () -> Unit
) {
    val colorScheme = if (darkTheme) {
        // SPY×FAMILY Dark Theme (未啟用，保留供未來使用)
        darkColorScheme(
            primary = Color(0xFFa8c0b5),           // 淺化青灰綠
            secondary = Color(0xFFa8666b),         // 淺化深紅
            tertiary = Color(0xFFfab3ad),          // Anya's Warm Pink
            background = Color(0xFF2B3B54),        // 深藍灰（間諜場景）
            surface = Color(0xFF3C4C64),           // 深青藍灰
            surfaceVariant = Color(0xFF4A5A74),    // 稍亮的深青藍灰
            onSurface = Color(0xFFE8E3DB),         // 淺米色文字
            onSurfaceVariant = Color(0xFFB0B0B0)   // 灰色文字
        )
    } else {
        // SPY×FAMILY Light Theme - 1960s 歐洲復古美學
        lightColorScheme(
            primary = Color(0xFF8da99b),           // Loid's Teal Gray Green
            secondary = Color(0xFF610a10),         // Yor's Deep Red
            tertiary = Color(0xFFfab3ad),          // Anya's Warm Pink
            background = Color(0xFFf5f0e8),        // 溫暖米色背景
            surface = Color(0xFFFBF9F5),           // 卡片背景（溫暖白色）
            surfaceVariant = Color(0xFFf0ebe3),    // 稍深的米色
            onSurface = Color(0xFF2c2827),         // 主要文字（深棕灰）
            onSurfaceVariant = Color(0xFF57675c)   // 次要文字（深青灰）
        )
    }

    MaterialTheme(
        colorScheme = colorScheme,
        content = content
    )
}
