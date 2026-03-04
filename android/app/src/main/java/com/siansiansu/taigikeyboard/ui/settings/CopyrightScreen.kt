package com.siansiansu.taigikeyboard.ui.settings

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.OpenInNew
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.Tab1Texts
import com.siansiansu.taigikeyboard.model.CopyrightButton
import com.siansiansu.taigikeyboard.model.CopyrightPage
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CopyrightScreen(
    copyrightPages: List<CopyrightPage>,
    languageManager: LanguageManager,
    fontFamily: FontFamily,
    onButtonClick: (String) -> Unit,
    onNavigateBack: () -> Unit
) {
    val language by languageManager.currentLanguageFlow.collectAsState()

    Scaffold(
        containerColor = MaterialTheme.colorScheme.surfaceContainerLow,
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = languageManager.text(Tab1Texts.copyrightNotice),
                        color = MaterialTheme.colorScheme.onSurface
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = "Back",
                            tint = MaterialTheme.colorScheme.onSurface
                        )
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surfaceContainerLow
                )
            )
        }
    ) { innerPadding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding),
            contentPadding = PaddingValues(
                start = 20.dp,
                end = 20.dp,
                top = 16.dp,
                bottom = 40.dp
            )
        ) {
            items(copyrightPages, key = { it.id }) { page ->
                CopyrightCard(
                    page = page,
                    languageManager = languageManager,
                    fontFamily = fontFamily,
                    onButtonClick = onButtonClick
                )
                Spacer(Modifier.height(16.dp))
            }
        }
    }
}

@Composable
private fun CopyrightCard(
    page: CopyrightPage,
    languageManager: LanguageManager,
    fontFamily: FontFamily,
    onButtonClick: (String) -> Unit
) {
    SettingsCard {
        Column(modifier = Modifier.padding(20.dp)) {
            // Title
            Text(
                text = languageManager.text(page.title),
                fontSize = 16.sp,
                fontWeight = FontWeight.Bold,
                fontFamily = fontFamily,
                color = MaterialTheme.colorScheme.onSurface
            )

            Spacer(Modifier.height(4.dp))

            // Description (copyright holder)
            Text(
                text = languageManager.text(page.description),
                fontSize = 14.sp,
                fontFamily = fontFamily,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )

            Spacer(Modifier.height(8.dp))

            // License
            Text(
                text = languageManager.text(page.license),
                fontSize = 14.sp,
                fontWeight = FontWeight.Bold,
                fontFamily = fontFamily,
                color = MaterialTheme.colorScheme.primary
            )
        }

        if (page.buttons.isNotEmpty()) {
            SettingsDivider()

            page.buttons.forEachIndexed { index, button ->
                CopyrightActionButton(
                    button = button,
                    languageManager = languageManager,
                    fontFamily = fontFamily,
                    onClick = { onButtonClick(button.url) }
                )
                if (index < page.buttons.size - 1) {
                    SettingsDivider()
                }
            }
        }
    }
}

@Composable
private fun CopyrightActionButton(
    button: CopyrightButton,
    languageManager: LanguageManager,
    fontFamily: FontFamily,
    onClick: () -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick)
            .padding(horizontal = 20.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(
            text = languageManager.text(button.text),
            modifier = Modifier.weight(1f),
            fontSize = 14.sp,
            fontFamily = fontFamily,
            color = MaterialTheme.colorScheme.primary
        )

        Spacer(Modifier.width(8.dp))

        Icon(
            imageVector = Icons.AutoMirrored.Outlined.OpenInNew,
            contentDescription = null,
            modifier = Modifier.size(16.dp),
            tint = MaterialTheme.colorScheme.primary
        )
    }
}
