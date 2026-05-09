// 中文: 主設定畫面 — Material3 NavigationBar + Scaffold,容納 Home / Dictionary / Layout / Settings 4 個 tab。
// 中文: 由 SettingsMainActivity setContent 掛載。

package com.siansiansu.taigikeyboard.ui.tabs

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource

data class TabItem(
    val iconResId: Int,
    val label: String,
)

@Composable
fun MainSettingsScreen(
    tabs: List<TabItem>,
    initialTab: Int = 0,
    content: @Composable (selectedTab: Int) -> Unit,
) {
    var selectedTab by rememberSaveable { mutableIntStateOf(initialTab) }

    Scaffold(
        bottomBar = {
            NavigationBar {
                tabs.forEachIndexed { index, tab ->
                    NavigationBarItem(
                        icon = {
                            Icon(
                                painter = painterResource(tab.iconResId),
                                contentDescription = tab.label,
                            )
                        },
                        label = { Text(tab.label) },
                        selected = selectedTab == index,
                        onClick = { selectedTab = index },
                        colors =
                            NavigationBarItemDefaults.colors(
                                selectedIconColor = MaterialTheme.colorScheme.primary,
                                selectedTextColor = MaterialTheme.colorScheme.primary,
                                indicatorColor = MaterialTheme.colorScheme.surfaceContainer,
                            ),
                    )
                }
            }
        },
    ) { innerPadding ->
        Box(Modifier.padding(innerPadding)) {
            content(selectedTab)
        }
    }
}
