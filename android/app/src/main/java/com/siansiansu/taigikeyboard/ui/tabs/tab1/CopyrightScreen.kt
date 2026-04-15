package com.siansiansu.taigikeyboard.ui.tabs.tab1

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
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.content.CopyrightButton
import com.siansiansu.taigikeyboard.content.CopyrightPage
import com.siansiansu.taigikeyboard.localization.Tab1Texts
import com.siansiansu.taigikeyboard.ui.components.OpenInNew
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.theme.AppStyle

// Tab1 copyright screen: displays open-source licenses and attribution
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CopyrightScreen(
    copyrightPages: List<CopyrightPage>,
    fontFamily: FontFamily,
    onButtonClick: (String) -> Unit,
    onNavigateBack: () -> Unit,
) {
    Scaffold(
        containerColor = MaterialTheme.colorScheme.surfaceContainer,
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = Tab1Texts.copyrightNotice,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = "Back",
                            tint = MaterialTheme.colorScheme.onSurface,
                        )
                    }
                },
                colors =
                    TopAppBarDefaults.topAppBarColors(
                        containerColor = MaterialTheme.colorScheme.surfaceContainer,
                    ),
            )
        },
    ) { innerPadding ->
        LazyColumn(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(innerPadding),
            contentPadding =
                PaddingValues(
                    start = 20.dp,
                    end = 20.dp,
                    top = 16.dp,
                    bottom = AppStyle.scrollContentBottomPadding,
                ),
        ) {
            items(copyrightPages, key = { it.id }) { page ->
                CopyrightCard(
                    page = page,
                    fontFamily = fontFamily,
                    onButtonClick = onButtonClick,
                )
                Spacer(Modifier.height(16.dp))
            }
        }
    }
}

@Composable
private fun CopyrightCard(
    page: CopyrightPage,
    fontFamily: FontFamily,
    onButtonClick: (String) -> Unit,
) {
    SettingsCard {
        Column(modifier = Modifier.padding(20.dp)) {
            Text(
                text = page.title,
                fontWeight = FontWeight.Bold,
                fontFamily = fontFamily,
                color = MaterialTheme.colorScheme.onSurface,
                style = MaterialTheme.typography.bodyLarge,
            )

            Spacer(Modifier.height(4.dp))

            Text(
                text = page.description,
                fontFamily = fontFamily,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                style = MaterialTheme.typography.bodyLarge,
            )

            Spacer(Modifier.height(8.dp))

            Text(
                text = page.license,
                fontWeight = FontWeight.Bold,
                fontFamily = fontFamily,
                color = MaterialTheme.colorScheme.primary,
                style = MaterialTheme.typography.labelLarge,
            )
        }

        if (page.buttons.isNotEmpty()) {
            SettingsDivider()

            page.buttons.forEachIndexed { index, button ->
                CopyrightActionButton(
                    button = button,
                    fontFamily = fontFamily,
                    onClick = { onButtonClick(button.url) },
                )
                if (index < page.buttons.lastIndex) {
                    SettingsDivider()
                }
            }
        }
    }
}

@Composable
private fun CopyrightActionButton(
    button: CopyrightButton,
    fontFamily: FontFamily,
    onClick: () -> Unit,
) {
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .clickable(onClick = onClick)
                .padding(horizontal = 20.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = button.text,
            modifier = Modifier.weight(1f),
            fontFamily = fontFamily,
            color = MaterialTheme.colorScheme.primary,
            style = MaterialTheme.typography.labelLarge,
        )

        Spacer(Modifier.width(8.dp))

        Icon(
            imageVector = Icons.AutoMirrored.Outlined.OpenInNew,
            contentDescription = null,
            modifier = Modifier.size(AppStyle.smallIconSize),
            tint = MaterialTheme.colorScheme.primary,
        )
    }
}
