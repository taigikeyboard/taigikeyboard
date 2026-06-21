package com.siansiansu.taigikeyboard.ui.tabs.home

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.theme.AppStyle

// Home tab setup guide: step-by-step keyboard activation instructions
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SetupGuideScreen(
    isFullScreen: Boolean,
    onGoToSettings: () -> Unit,
    onClose: () -> Unit,
    onNavigateBack: () -> Unit,
) {
    Scaffold(
        containerColor = MaterialTheme.colorScheme.surfaceContainer,
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = L10n.homeSetupGuide,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                },
                navigationIcon = {
                    if (!isFullScreen) {
                        IconButton(onClick = onNavigateBack) {
                            Icon(
                                imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                                contentDescription = L10n.commonBack,
                                tint = MaterialTheme.colorScheme.onSurface,
                            )
                        }
                    }
                },
                colors =
                    TopAppBarDefaults.topAppBarColors(
                        containerColor = MaterialTheme.colorScheme.surfaceContainer,
                    ),
            )
        },
    ) { innerPadding ->
        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(innerPadding)
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = 24.dp)
                    .padding(top = 16.dp, bottom = AppStyle.scrollContentBottomPadding),
        ) {
            Text(
                text = L10n.homeSetupGuideDescription,
                lineHeight = 22.sp,
                color = MaterialTheme.colorScheme.onSurface,
                style = MaterialTheme.typography.bodyLarge,
            )

            Spacer(Modifier.height(24.dp))

            StepCard(
                stepNumber = 1,
                title = L10n.homeSetupGuideStep1Settings,
                imageResId = R.drawable.setup_step1,
            )

            Spacer(Modifier.height(16.dp))

            StepCard(
                stepNumber = 2,
                title = L10n.homeSetupGuideStep2AddKeyboard,
                imageResId = R.drawable.setup_step2,
            )

            Spacer(Modifier.height(16.dp))

            Text(
                text = L10n.homeSetupGuideCompletedMessage,
                lineHeight = 22.sp,
                color = MaterialTheme.colorScheme.onSurface,
                style = MaterialTheme.typography.bodyLarge,
            )

            Spacer(Modifier.height(16.dp))

            Button(
                onClick = onGoToSettings,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text(
                    text = L10n.homeSetupGuideGoToSettings,
                    fontWeight = FontWeight.Bold,
                    style = MaterialTheme.typography.bodyLarge,
                )
            }

            Spacer(Modifier.height(16.dp))

            WarningRow(L10n.homeSetupInfoMessage)

            Spacer(Modifier.height(12.dp))

            WarningRow(L10n.homeSetupBrandWarning)

            if (isFullScreen) {
                Spacer(Modifier.height(24.dp))
                Button(
                    onClick = onClose,
                    modifier = Modifier.align(Alignment.CenterHorizontally),
                    colors =
                        ButtonDefaults.buttonColors(
                            containerColor = MaterialTheme.colorScheme.error,
                        ),
                ) {
                    Icon(
                        painter = painterResource(R.drawable.ic_close),
                        contentDescription = null,
                        modifier = Modifier.size(18.dp),
                        tint = MaterialTheme.colorScheme.onError,
                    )
                    Spacer(Modifier.width(8.dp))
                    Text(
                        text = L10n.homeSetupGuideCloseButton,
                        color = MaterialTheme.colorScheme.onError,
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
    imageResId: Int,
) {
    SettingsCard {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(
                modifier = Modifier.padding(bottom = 12.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Box(
                    modifier =
                        Modifier
                            .size(22.dp)
                            .background(
                                color = MaterialTheme.colorScheme.primary,
                                shape = CircleShape,
                            ),
                    contentAlignment = Alignment.Center,
                ) {
                    Text(
                        text = "$stepNumber",
                        fontWeight = FontWeight.Bold,
                        color = Color.White,
                        textAlign = TextAlign.Center,
                        style = MaterialTheme.typography.labelLarge,
                    )
                }
                Spacer(Modifier.width(12.dp))
                Text(
                    text = title,
                    color = MaterialTheme.colorScheme.onSurface,
                    style = MaterialTheme.typography.bodyLarge,
                )
            }

            Image(
                painter = painterResource(imageResId),
                contentDescription = null,
                modifier = Modifier.fillMaxWidth(),
                contentScale = ContentScale.FillWidth,
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
            modifier =
                Modifier
                    .size(AppStyle.smallIconSize)
                    .padding(top = 2.dp),
            tint = AppStyle.warningOrange(),
        )
        Spacer(Modifier.width(8.dp))
        Text(
            text = text,
            lineHeight = 22.sp,
            color = MaterialTheme.colorScheme.onSurface,
            style = MaterialTheme.typography.bodyLarge,
        )
    }
}
