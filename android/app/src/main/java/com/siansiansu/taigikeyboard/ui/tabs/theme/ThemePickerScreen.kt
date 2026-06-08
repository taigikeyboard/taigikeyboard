package com.siansiansu.taigikeyboard.ui.tabs.theme

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
import androidx.compose.foundation.layout.aspectRatio
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
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.BuiltInTheme
import com.siansiansu.taigikeyboard.ime.core.BuiltInThemes
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.localization.ThemeTexts
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import com.siansiansu.taigikeyboard.ui.theme.SectionHeader

// Theme tab main screen: built-in theme gallery, one horizontal shelf per family
// (經典 / Swifty / Minimal). Selecting a card writes selectedThemeId, which wakes the
// P2 render seam (gradient background + key shadow) for the gradient themes. The
// custom-theme shelf + editor land in P4 — P3 has no theme creation path yet.

// Card metrics — 200.dp matches LayoutCard so the theme tab lines up with the layout
// tab in-app (intentional divergence from iOS 240pt). The 585/369 aspect matches the
// layout_*_preview assets so future theme screenshots render at the same size.
private const val THEME_CARD_WIDTH_DP = 200
private const val THEME_PREVIEW_ASPECT = 585f / 369f
private val THEME_CARD_SPACING = 12.dp

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ThemePickerScreen(prefs: PrefHelper) {
    val selectedThemeId by prefs
        .observeSelectedThemeId()
        .collectAsStateWithLifecycle(initialValue = prefs.selectedThemeId)

    val scrollBehavior = TopAppBarDefaults.exitUntilCollapsedScrollBehavior()

    Scaffold(
        modifier = Modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
        containerColor = MaterialTheme.colorScheme.surfaceContainer,
        topBar = {
            LargeTopAppBar(
                title = {
                    Text(
                        text = ThemeTexts.tabTitle,
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
            BuiltInThemes.families.forEachIndexed { index, family ->
                ThemeShelf(
                    title = family.title,
                    themes = family.themes,
                    selectedThemeId = selectedThemeId,
                    onThemeSelected = { id ->
                        if (selectedThemeId != id) prefs.selectedThemeId = id
                    },
                )
                if (index < BuiltInThemes.families.size - 1) {
                    Spacer(Modifier.height(AppStyle.sectionSpacing))
                }
            }
        }
    }
}

@Composable
private fun ThemeShelf(
    title: String,
    themes: List<BuiltInTheme>,
    selectedThemeId: String,
    onThemeSelected: (String) -> Unit,
) {
    SectionHeader(title)
    Row(
        modifier =
            Modifier
                .horizontalScroll(rememberScrollState())
                .padding(horizontal = 20.dp),
    ) {
        themes.forEachIndexed { index, theme ->
            ThemeCard(
                title = theme.displayName,
                // Built-in preview screenshots (585x369) wire in here once supplied —
                // map theme.previewImageName to an explicit R.drawable.* (NEVER
                // resources.getIdentifier, which the resource shrinker can't track).
                // None bundled yet, so every built-in card shows the neutral placeholder.
                previewRes = null,
                isSelected = selectedThemeId == theme.id,
                onClick = { onThemeSelected(theme.id) },
            )
            if (index < themes.size - 1) {
                Spacer(Modifier.width(THEME_CARD_SPACING))
            }
        }
    }
}

@Composable
private fun ThemeCard(
    title: String,
    @DrawableRes previewRes: Int?,
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
                .width(THEME_CARD_WIDTH_DP.dp)
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
            Box(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .aspectRatio(THEME_PREVIEW_ASPECT),
            ) {
                if (previewRes != null) {
                    Image(
                        painter = painterResource(previewRes),
                        contentDescription = title,
                        modifier = Modifier.fillMaxSize(),
                        contentScale = ContentScale.Crop,
                    )
                } else {
                    ThemePreviewPlaceholder(title)
                }

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
            text = title,
            fontWeight = FontWeight.SemiBold,
            color = MaterialTheme.colorScheme.onSurface,
            textAlign = TextAlign.Center,
            style = MaterialTheme.typography.labelLarge,
        )
    }
}

// Neutral fallback shown when a built-in card has no bundled screenshot (the current
// state for every built-in — the 585x369 preview drawables are supplied later). Fixed
// to the card aspect so cards never change height once screenshots land.
@Composable
private fun ThemePreviewPlaceholder(title: String) {
    Box(
        modifier =
            Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.surfaceContainerHighest),
        contentAlignment = Alignment.Center,
    ) {
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Icon(
                painter = painterResource(R.drawable.keyboard_24),
                contentDescription = null,
                modifier = Modifier.size(28.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(Modifier.height(6.dp))
            Text(
                text = title,
                style = MaterialTheme.typography.labelLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                maxLines = 1,
            )
        }
    }
}

