package com.siansiansu.taigikeyboard.ui.settings

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.Tab1Texts
import com.siansiansu.taigikeyboard.ui.components.SettingsCard

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SetupGuideScreen(
    languageManager: LanguageManager,
    isFullScreen: Boolean,
    onGoToSettings: () -> Unit,
    onClose: () -> Unit,
    onNavigateBack: () -> Unit
) {
    val language by languageManager.currentLanguageFlow.collectAsState()

    Scaffold(
        containerColor = MaterialTheme.colorScheme.surfaceContainer,
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = languageManager.text(Tab1Texts.setupGuide),
                        color = MaterialTheme.colorScheme.onSurface
                    )
                },
                navigationIcon = {
                    if (!isFullScreen) {
                        IconButton(onClick = onNavigateBack) {
                            Icon(
                                imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                                contentDescription = "Back",
                                tint = MaterialTheme.colorScheme.onSurface
                            )
                        }
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surfaceContainer
                )
            )
        }
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 24.dp)
                .padding(top = 16.dp, bottom = 40.dp)
        ) {
            // Description
            Text(
                text = languageManager.text(Tab1Texts.setupGuideDescription),
                fontSize = AppStyle.bodyFontSize,
                lineHeight = 22.sp,
                color = MaterialTheme.colorScheme.onSurface
            )

            Spacer(Modifier.height(24.dp))

            // Step 1 card
            StepCard(
                stepNumber = 1,
                title = languageManager.text(Tab1Texts.setupGuideStep1Settings),
                imageResId = R.drawable.setup_step1
            )

            Spacer(Modifier.height(16.dp))

            // Step 2 card
            StepCard(
                stepNumber = 2,
                title = languageManager.text(Tab1Texts.setupGuideStep2AddKeyboard),
                imageResId = R.drawable.setup_step2
            )

            Spacer(Modifier.height(16.dp))

            // Completed message
            Text(
                text = languageManager.text(Tab1Texts.setupGuideCompletedMessage),
                fontSize = AppStyle.bodyFontSize,
                lineHeight = 22.sp,
                color = MaterialTheme.colorScheme.onSurface
            )

            Spacer(Modifier.height(16.dp))

            // Go to settings button
            Button(
                onClick = onGoToSettings,
                modifier = Modifier.fillMaxWidth()
            ) {
                Text(
                    text = languageManager.text(Tab1Texts.setupGuideGoToSettings),
                    fontSize = AppStyle.bodyFontSize,
                    fontWeight = FontWeight.Bold
                )
            }

            Spacer(Modifier.height(16.dp))

            // Warning: privacy message
            WarningRow(languageManager.text(Tab1Texts.setupInfoMessage))

            Spacer(Modifier.height(12.dp))

            // Warning: brand differences
            WarningRow(languageManager.text(Tab1Texts.setupBrandWarning))

            // Close button (full screen mode only)
            if (isFullScreen) {
                Spacer(Modifier.height(24.dp))
                Button(
                    onClick = onClose,
                    modifier = Modifier.align(Alignment.CenterHorizontally),
                    colors = ButtonDefaults.buttonColors(
                        containerColor = MaterialTheme.colorScheme.error
                    )
                ) {
                    Icon(
                        painter = painterResource(R.drawable.ic_close),
                        contentDescription = null,
                        modifier = Modifier.size(18.dp),
                        tint = MaterialTheme.colorScheme.onError
                    )
                    Spacer(Modifier.width(8.dp))
                    Text(
                        text = languageManager.text(Tab1Texts.setupGuideCloseButton),
                        color = MaterialTheme.colorScheme.onError
                    )
                }
            }
        }
    }
}

@Composable
private fun StepCard(
    stepNumber: Int,
    title: String,
    imageResId: Int
) {
    SettingsCard {
        Column(modifier = Modifier.padding(16.dp)) {
            // Step number + title
            Row(
                modifier = Modifier.padding(bottom = 12.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                // Number circle
                Box(
                    modifier = Modifier
                        .size(22.dp)
                        .background(
                            color = MaterialTheme.colorScheme.primary,
                            shape = CircleShape
                        ),
                    contentAlignment = Alignment.Center
                ) {
                    Text(
                        text = "$stepNumber",
                        fontSize = 14.sp,
                        fontWeight = FontWeight.Bold,
                        color = Color.White,
                        textAlign = TextAlign.Center
                    )
                }
                Spacer(Modifier.width(12.dp))
                Text(
                    text = title,
                    fontSize = AppStyle.bodyFontSize,
                    color = MaterialTheme.colorScheme.onSurface
                )
            }

            // Screenshot
            Image(
                painter = painterResource(imageResId),
                contentDescription = null,
                modifier = Modifier.fillMaxWidth(),
                contentScale = ContentScale.FillWidth
            )
        }
    }
}

@Composable
private fun WarningRow(text: String) {
    Row(modifier = Modifier.fillMaxWidth()) {
        Icon(
            painter = painterResource(R.drawable.ic_warning),
            contentDescription = null,
            modifier = Modifier
                .size(16.dp)
                .padding(top = 2.dp),
            tint = AppStyle.warningOrange()
        )
        Spacer(Modifier.width(8.dp))
        Text(
            text = text,
            fontSize = AppStyle.bodyFontSize,
            lineHeight = 22.sp,
            color = MaterialTheme.colorScheme.onSurface
        )
    }
}
