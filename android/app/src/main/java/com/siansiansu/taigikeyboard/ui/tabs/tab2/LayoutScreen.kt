package com.siansiansu.taigikeyboard.ui.tabs.tab2

import androidx.annotation.DrawableRes
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.LargeTopAppBar
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.Tab2Texts
import com.siansiansu.taigikeyboard.ui.components.ActionRow
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import com.siansiansu.taigikeyboard.ui.theme.SectionHeader

private data class LayoutOption(
    val key: String,
    val label: @Composable () -> String,
    @param:DrawableRes val previewRes: Int,
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LayoutScreen(
    languageManager: LanguageManager,
    prefs: PrefHelper,
    onAppearanceSettings: () -> Unit,
) {
    val language by languageManager.currentLanguageFlow.collectAsState()
    var selectedLayout by remember { mutableStateOf(prefs.keyboardLayoutType) }

    val romanizationLayouts =
        remember {
            listOf(
                LayoutOption("phahTaigi", { languageManager.text(Tab2Texts.phahTaigiLayout) }, R.drawable.layout_phahtaigi_preview),
                LayoutOption("qwerty", { languageManager.text(Tab2Texts.standardLayout) }, R.drawable.layout_standard_preview),
                LayoutOption("moe1", { languageManager.text(Tab2Texts.moe1Layout) }, R.drawable.layout_moe1_preview),
                LayoutOption("moe2", { languageManager.text(Tab2Texts.moe2Layout) }, R.drawable.layout_moe2_preview),
            )
        }

    val scrollBehavior = TopAppBarDefaults.exitUntilCollapsedScrollBehavior()

    Scaffold(
        modifier = Modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
        containerColor = MaterialTheme.colorScheme.surfaceContainer,
        topBar = {
            LargeTopAppBar(
                title = {
                    Text(
                        text = languageManager.text(Tab2Texts.tabTitle),
                        fontSize = AppStyle.pageTitleFontSize,
                    )
                },
                expandedHeight = AppStyle.largeTopAppBarExpandedHeight,
                colors =
                    TopAppBarDefaults.topAppBarColors(
                        containerColor = MaterialTheme.colorScheme.surfaceContainer,
                        scrolledContainerColor = MaterialTheme.colorScheme.surfaceContainer,
                    ),
                scrollBehavior = scrollBehavior,
            )
        },
    ) { innerPadding ->
        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(innerPadding)
                    .verticalScroll(rememberScrollState())
                    .padding(bottom = 40.dp),
        ) {
            // Appearance settings card
            SettingsCard(modifier = Modifier.padding(horizontal = 20.dp)) {
                ActionRow(
                    label = languageManager.text(Tab2Texts.appearanceSettings),
                    onClick = onAppearanceSettings,
                    trailingIcon = Icons.AutoMirrored.Filled.KeyboardArrowRight,
                )
            }

            Spacer(Modifier.height(24.dp))

            // Romanization keyboard section header
            SectionHeader(languageManager.text(Tab2Texts.romanizationKeyboard))

            // Horizontal scrolling layout options
            Row(
                modifier =
                    Modifier
                        .horizontalScroll(rememberScrollState())
                        .padding(horizontal = 20.dp),
            ) {
                romanizationLayouts.forEachIndexed { index, layout ->
                    LayoutCard(
                        label = layout.label(),
                        previewRes = layout.previewRes,
                        isSelected = selectedLayout == layout.key,
                        onClick = {
                            if (selectedLayout != layout.key) {
                                selectedLayout = layout.key
                                prefs.keyboardLayoutType = layout.key
                            }
                        },
                    )
                    if (index < romanizationLayouts.size - 1) {
                        Spacer(Modifier.width(12.dp))
                    }
                }
            }

            Spacer(Modifier.height(24.dp))

            // TPS section header
            SectionHeader(languageManager.text(Tab2Texts.taigiPhonetic))

            // TPS layout option
            Row(
                modifier = Modifier.padding(horizontal = 20.dp),
            ) {
                LayoutCard(
                    label = languageManager.text(Tab2Texts.tpsLayout),
                    previewRes = R.drawable.layout_tps_preview,
                    isSelected = selectedLayout == "tps",
                    onClick = {
                        if (selectedLayout != "tps") {
                            selectedLayout = "tps"
                            prefs.keyboardLayoutType = "tps"
                        }
                    },
                )
            }
        }
    }
}

@Composable
private fun LayoutCard(
    label: String,
    @DrawableRes previewRes: Int,
    isSelected: Boolean,
    onClick: () -> Unit,
) {
    val checkmarkScale by animateFloatAsState(
        targetValue = if (isSelected) 1f else 0f,
        animationSpec =
            spring(
                dampingRatio = Spring.DampingRatioMediumBouncy,
                stiffness = Spring.StiffnessMedium,
            ),
        label = "checkmarkScale",
    )
    val overlayAlpha by animateFloatAsState(
        targetValue = if (isSelected) 1f else 0f,
        label = "overlayAlpha",
    )

    Column(
        modifier =
            Modifier
                .width(200.dp)
                .clickable(onClick = onClick),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Surface(
            shape = RoundedCornerShape(10.dp),
            color = MaterialTheme.colorScheme.surfaceContainerHigh,
            border =
                if (isSelected) {
                    androidx.compose.foundation.BorderStroke(2.5.dp, MaterialTheme.colorScheme.primary)
                } else {
                    null
                },
        ) {
            Box {
                // Preview image
                androidx.compose.foundation.Image(
                    painter = painterResource(previewRes),
                    contentDescription = label,
                    modifier = Modifier.fillMaxWidth(),
                    contentScale = ContentScale.FillWidth,
                )

                // Dark overlay
                if (overlayAlpha > 0f) {
                    Box(
                        modifier =
                            Modifier
                                .matchParentSize()
                                .alpha(overlayAlpha)
                                .background(MaterialTheme.colorScheme.scrim.copy(alpha = 0.25f)),
                    )
                }

                // Checkmark circle
                if (checkmarkScale > 0f) {
                    Box(
                        modifier =
                            Modifier
                                .align(Alignment.Center)
                                .scale(checkmarkScale)
                                .size(36.dp)
                                .clip(CircleShape)
                                .background(MaterialTheme.colorScheme.primary),
                        contentAlignment = Alignment.Center,
                    ) {
                        Icon(
                            imageVector = Icons.Default.Check,
                            contentDescription = null,
                            modifier = Modifier.size(16.dp),
                            tint = MaterialTheme.colorScheme.onPrimary,
                        )
                    }
                }
            }
        }

        Spacer(Modifier.height(6.dp))

        Text(
            text = label,
            fontSize = AppStyle.captionFontSize,
            fontWeight = FontWeight.SemiBold,
            color = MaterialTheme.colorScheme.onSurface,
            textAlign = TextAlign.Center,
        )
    }
}
