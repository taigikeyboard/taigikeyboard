package com.siansiansu.taigikeyboard.ui.tabs.layout

import androidx.annotation.DrawableRes
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Image
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.localization.LayoutTexts
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import com.siansiansu.taigikeyboard.ui.theme.SectionHeader

// Layout tab main screen: keyboard layout selection

private data class LayoutOption(
    val key: String,
    val label: String,
    @param:DrawableRes val previewRes: Int,
)

private val romanizationLayouts =
    listOf(
        LayoutOption("phahTaigi", LayoutTexts.phahTaigiLayout, R.drawable.layout_phahtaigi_preview),
        LayoutOption("qwerty", LayoutTexts.standardLayout, R.drawable.layout_standard_preview),
        LayoutOption("moe1", LayoutTexts.moe1Layout, R.drawable.layout_moe1_preview),
        LayoutOption("moe2", LayoutTexts.moe2Layout, R.drawable.layout_moe2_preview),
    )

private val phoneticLayouts =
    listOf(
        LayoutOption("tps", LayoutTexts.tpsLayout, R.drawable.layout_tps_preview),
    )

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LayoutScreen(prefs: PrefHelper) {
    val selectedLayout by prefs
        .observeKeyboardLayoutType()
        .collectAsState(initial = prefs.keyboardLayoutType)

    val scrollBehavior = TopAppBarDefaults.exitUntilCollapsedScrollBehavior()

    Scaffold(
        modifier = Modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
        containerColor = MaterialTheme.colorScheme.surfaceContainer,
        topBar = {
            LargeTopAppBar(
                title = {
                    Text(
                        text = LayoutTexts.tabTitle,
                        style = MaterialTheme.typography.headlineLarge,
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
                    .padding(bottom = AppStyle.scrollContentBottomPadding),
        ) {
            LayoutSection(
                title = LayoutTexts.romanizationKeyboard,
                layouts = romanizationLayouts,
                selectedLayout = selectedLayout,
                onLayoutSelected = { prefs.keyboardLayoutType = it },
                horizontalScroll = true,
            )

            Spacer(Modifier.height(24.dp))

            LayoutSection(
                title = LayoutTexts.taigiPhonetic,
                layouts = phoneticLayouts,
                selectedLayout = selectedLayout,
                onLayoutSelected = { prefs.keyboardLayoutType = it },
            )
        }
    }
}

@Composable
private fun LayoutSection(
    title: String,
    layouts: List<LayoutOption>,
    selectedLayout: String,
    onLayoutSelected: (String) -> Unit,
    horizontalScroll: Boolean = false,
) {
    SectionHeader(title)
    val scrollModifier =
        if (horizontalScroll) {
            Modifier.horizontalScroll(rememberScrollState())
        } else {
            Modifier
        }
    Row(
        modifier =
            scrollModifier
                .padding(horizontal = 20.dp),
    ) {
        layouts.forEachIndexed { index, layout ->
            LayoutCard(
                label = layout.label,
                previewRes = layout.previewRes,
                isSelected = selectedLayout == layout.key,
                onClick = {
                    if (selectedLayout != layout.key) {
                        onLayoutSelected(layout.key)
                    }
                },
            )
            if (index < layouts.size - 1) {
                Spacer(Modifier.width(12.dp))
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
                    BorderStroke(2.5.dp, MaterialTheme.colorScheme.primary)
                } else {
                    null
                },
        ) {
            Box {
                Image(
                    painter = painterResource(previewRes),
                    contentDescription = label,
                    modifier = Modifier.fillMaxWidth(),
                    contentScale = ContentScale.FillWidth,
                )

                if (overlayAlpha > 0f) {
                    Box(
                        modifier =
                            Modifier
                                .matchParentSize()
                                .alpha(overlayAlpha)
                                .background(MaterialTheme.colorScheme.scrim.copy(alpha = 0.25f)),
                    )
                }

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
                            modifier = Modifier.size(AppStyle.smallIconSize),
                            tint = MaterialTheme.colorScheme.onPrimary,
                        )
                    }
                }
            }
        }

        Spacer(Modifier.height(6.dp))

        Text(
            text = label,
            fontWeight = FontWeight.SemiBold,
            color = MaterialTheme.colorScheme.onSurface,
            textAlign = TextAlign.Center,
            style = MaterialTheme.typography.labelLarge,
        )
    }
}
