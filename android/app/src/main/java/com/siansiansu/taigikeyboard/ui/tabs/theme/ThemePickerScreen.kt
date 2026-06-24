package com.siansiansu.taigikeyboard.ui.tabs.theme

import android.view.ContextThemeWrapper
import androidx.annotation.DrawableRes
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LargeTopAppBar
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.i18n.stringRes
import com.siansiansu.taigikeyboard.ime.core.BuiltInTheme
import com.siansiansu.taigikeyboard.ime.core.BuiltInThemes
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.ThemeAppearance
import com.siansiansu.taigikeyboard.ime.core.ThemeId
import com.siansiansu.taigikeyboard.ime.core.UserTheme
import com.siansiansu.taigikeyboard.ime.core.UserThemeStore
import com.siansiansu.taigikeyboard.ime.theme.getColorFromAttr
import com.siansiansu.taigikeyboard.settings.ThemeEditorActivity
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import com.siansiansu.taigikeyboard.ui.theme.SectionHeader

// Theme tab main screen: a custom-theme shelf (Create New + saved themes with an
// apply/edit/delete menu) above one built-in shelf per key-style family (經典 /
// 框線 / 簡潔 — same 7 colors, different key style). Selecting any card writes
// selectedThemeId, which wakes the P2 render seam (gradient background + key shadow
// + key border). The editor lives in ThemeEditorActivity; add/edit/delete refresh
// this shelf reactively via observeUserThemes().

// Card metrics — 200.dp matches LayoutCard so the theme tab lines up with the layout
// tab in-app (intentional divergence from iOS 240pt). The 585/395 aspect matches the
// Android keyboard screenshot proportion (theme_*_preview assets), which is taller than
// iOS's 585/369 — the Android keyboard is taller, so the card follows the Android
// keyboard shape rather than the iOS card slot (intentional cross-platform divergence:
// forcing iOS 585/369 here clipped the screenshots' top tone-mark row).
private const val THEME_CARD_WIDTH_DP = 200
private const val THEME_PREVIEW_ASPECT = 585f / 395f
private val THEME_CARD_SPACING = 12.dp

/**
 * The selected-theme id after deleting [deletedId]: falls back to the default
 * theme when the deleted theme was the active one (so the keyboard never renders
 * an orphan id), otherwise the selection is unchanged. Pure for unit testing.
 */
internal fun selectionAfterDelete(
    deletedId: String,
    currentSelection: String,
): String = if (currentSelection == deletedId) ThemeId.DEFAULT else currentSelection

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ThemePickerScreen(prefs: PrefHelper) {
    val context = LocalContext.current
    val selectedThemeId by prefs
        .observeSelectedThemeId()
        .collectAsStateWithLifecycle(initialValue = prefs.selectedThemeId)
    val userThemes by prefs
        .observeUserThemes()
        .collectAsStateWithLifecycle(initialValue = prefs.loadUserThemes())

    // Delete is the only in-place mutation (add/edit go through the editor Activity).
    val userThemeStore = remember(prefs) {
        UserThemeStore(read = { prefs.userThemes }, write = { prefs.userThemes = it })
    }

    val applyTheme: (String) -> Unit = { id ->
        if (selectedThemeId != id) prefs.selectedThemeId = id
    }
    val deleteTheme: (UserTheme) -> Unit = { theme ->
        // delete() already no-ops on a missing id, and selectionAfterDelete only
        // rewrites the selection when the deleted theme was active — so both run
        // unconditionally (no redundant existence pre-check / extra JSON parse).
        userThemeStore.delete(theme.id)
        val next = selectionAfterDelete(theme.id, prefs.selectedThemeId)
        if (next != prefs.selectedThemeId) prefs.selectedThemeId = next
    }

    val scrollBehavior = TopAppBarDefaults.exitUntilCollapsedScrollBehavior()

    Scaffold(
        modifier = Modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
        containerColor = MaterialTheme.colorScheme.surfaceContainer,
        topBar = {
            LargeTopAppBar(
                title = {
                    Text(
                        text = stringResource(R.string.tab_theme),
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
            CustomThemeShelf(
                userThemes = userThemes,
                selectedThemeId = selectedThemeId,
                onCreateNew = {
                    context.startActivity(ThemeEditorActivity.createIntent(context, null))
                },
                onApply = applyTheme,
                onEdit = { theme ->
                    context.startActivity(ThemeEditorActivity.createIntent(context, theme.id))
                },
                onDelete = deleteTheme,
            )
            Spacer(Modifier.height(AppStyle.sectionSpacing))

            BuiltInThemes.families.forEachIndexed { index, family ->
                BuiltInThemeShelf(
                    title = stringRes(family.titleKey),
                    themes = family.themes,
                    selectedThemeId = selectedThemeId,
                    onThemeSelected = applyTheme,
                )
                if (index < BuiltInThemes.families.size - 1) {
                    Spacer(Modifier.height(AppStyle.sectionSpacing))
                }
            }
        }
    }
}

@Composable
private fun CustomThemeShelf(
    userThemes: List<UserTheme>,
    selectedThemeId: String,
    onCreateNew: () -> Unit,
    onApply: (String) -> Unit,
    onEdit: (UserTheme) -> Unit,
    onDelete: (UserTheme) -> Unit,
) {
    SectionHeader(L10n.themeCustomThemesSection)
    Row(
        modifier =
            Modifier
                .horizontalScroll(rememberScrollState())
                .padding(horizontal = 20.dp),
    ) {
        // Create-new card leads the shelf, hidden once the cap is reached (mirrors iOS).
        if (userThemes.size < UserThemeStore.MAX_USER_THEMES) {
            CreateNewThemeCard(onClick = onCreateNew)
            if (userThemes.isNotEmpty()) Spacer(Modifier.width(THEME_CARD_SPACING))
        }
        userThemes.forEachIndexed { index, theme ->
            ThemeCard(
                title = theme.name,
                isSelected = selectedThemeId == theme.id,
                onClick = { onApply(theme.id) },
                menuActions =
                    listOf(
                        ThemeCardAction(L10n.themeCardMenuApply) { onApply(theme.id) },
                        ThemeCardAction(L10n.themeCardMenuEdit) { onEdit(theme) },
                        ThemeCardAction(L10n.commonDelete, isDestructive = true) { onDelete(theme) },
                    ),
                preview = { CustomThemeButtonPreview(theme.appearance) },
            )
            if (index < userThemes.size - 1) {
                Spacer(Modifier.width(THEME_CARD_SPACING))
            }
        }
    }
}

@Composable
private fun BuiltInThemeShelf(
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
            // Resolve once for both the card title and the preview's contentDescription.
            val title = stringRes(theme.displayNameKey)
            ThemeCard(
                title = title,
                isSelected = selectedThemeId == theme.id,
                onClick = { onThemeSelected(theme.id) },
                preview = {
                    // Maps theme.previewImageName to an explicit R.drawable.* (NEVER
                    // resources.getIdentifier, which the resource shrinker can't track).
                    // 預設 (reuses phahtaigi) + the 5 漸層 themes ship a screenshot; the
                    // rest fall through to null → neutral placeholder. Mirrors iOS
                    // UIImage(named:).
                    BuiltInThemePreview(
                        previewRes = builtInThemePreviewRes(theme.previewImageName),
                        title = title,
                    )
                },
            )
            if (index < themes.size - 1) {
                Spacer(Modifier.width(THEME_CARD_SPACING))
            }
        }
    }
}

/** One trailing-menu action on a custom theme card (apply / edit / delete). */
private data class ThemeCardAction(
    val title: String,
    val isDestructive: Boolean = false,
    val onClick: () -> Unit,
)

// A theme card: a tappable preview (built-in screenshot/placeholder or the live
// custom-theme button preview) with the shared selection overlay, plus a title row
// carrying an optional overflow menu (custom themes only). The tap-to-apply target
// is the preview only — the title row (and its menu) is excluded, so opening the
// menu never applies the theme.
@Composable
private fun ThemeCard(
    title: String,
    isSelected: Boolean,
    onClick: () -> Unit,
    preview: @Composable () -> Unit,
    menuActions: List<ThemeCardAction> = emptyList(),
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
        modifier = Modifier.width(THEME_CARD_WIDTH_DP.dp),
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
            modifier = Modifier.clickable(onClick = onClick),
        ) {
            Box(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .aspectRatio(THEME_PREVIEW_ASPECT),
            ) {
                preview()

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

        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                text = title,
                fontWeight = FontWeight.SemiBold,
                color = MaterialTheme.colorScheme.onSurface,
                textAlign = TextAlign.Center,
                style = MaterialTheme.typography.labelLarge,
                modifier = Modifier.weight(1f),
            )
            if (menuActions.isNotEmpty()) {
                ThemeCardMenu(actions = menuActions)
            }
        }
    }
}

@Composable
private fun ThemeCardMenu(actions: List<ThemeCardAction>) {
    var expanded by remember { mutableStateOf(false) }
    Box {
        IconButton(onClick = { expanded = true }, modifier = Modifier.size(28.dp)) {
            Icon(
                imageVector = Icons.Default.MoreVert,
                contentDescription = L10n.themeCardMenu,
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            actions.forEach { action ->
                DropdownMenuItem(
                    text = {
                        Text(
                            text = action.title,
                            color =
                                if (action.isDestructive) {
                                    MaterialTheme.colorScheme.error
                                } else {
                                    MaterialTheme.colorScheme.onSurface
                                },
                        )
                    },
                    onClick = {
                        expanded = false
                        action.onClick()
                    },
                )
            }
        }
    }
}

// The leading card on the custom shelf: a neutral panel with a centered "+" tile.
@Composable
private fun CreateNewThemeCard(onClick: () -> Unit) {
    Column(
        modifier = Modifier.width(THEME_CARD_WIDTH_DP.dp),
    ) {
        Surface(
            shape = RoundedCornerShape(10.dp),
            color = MaterialTheme.colorScheme.surfaceContainerHigh,
            modifier = Modifier.clickable(onClick = onClick),
        ) {
            Box(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .aspectRatio(THEME_PREVIEW_ASPECT),
                contentAlignment = Alignment.Center,
            ) {
                Box(
                    modifier =
                        Modifier
                            .size(84.dp)
                            .clip(RoundedCornerShape(12.dp))
                            .background(MaterialTheme.colorScheme.surface),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        imageVector = Icons.Default.Add,
                        contentDescription = L10n.themeCreateNewTheme,
                        modifier = Modifier.size(28.dp),
                        tint = MaterialTheme.colorScheme.onSurface,
                    )
                }
            }
        }

        Spacer(Modifier.height(6.dp))

        Text(
            text = L10n.themeCreateNewTheme,
            fontWeight = FontWeight.SemiBold,
            color = MaterialTheme.colorScheme.onSurface,
            textAlign = TextAlign.Center,
            style = MaterialTheme.typography.labelLarge,
        )
    }
}

// A custom-theme card preview: the theme background with one large centered key
// applying the theme's full button style — fill, glyph, corner radius, border, and
// shadow — so the saved key look reads at a glance. Null color roles fall back to
// the same KeyboardTheme adaptive colors the running keyboard uses. Mirrors iOS
// CustomThemeButtonPreview.
@Composable
private fun CustomThemeButtonPreview(appearance: ThemeAppearance) {
    val context = LocalContext.current
    val themed = remember(context) { ContextThemeWrapper(context, R.style.KeyboardTheme) }
    val colors = appearance.colors
    // Resolve the adaptive fallbacks once per (theme, colors) — not every frame of
    // the shelf's selection animation.
    val resolved = remember(themed, colors) {
        ThemePreviewColors(
            background = colors.backgroundColor ?: getColorFromAttr(themed, R.attr.keyboard_bgColor),
            keyFill = colors.normalKeyFillColor ?: getColorFromAttr(themed, R.attr.key_bgColor),
            keyText = colors.keyTextColor ?: getColorFromAttr(themed, R.attr.key_fgColor),
        )
    }
    val cornerShape = RoundedCornerShape(appearance.keyCornerRadius.dp)
    val borderWidth = appearance.keyBorderWidth
    val shadow = appearance.keyShadowIntensity

    Box(
        modifier =
            Modifier
                .fillMaxSize()
                .background(Color(resolved.background)),
        contentAlignment = Alignment.Center,
    ) {
        Box(
            modifier =
                Modifier
                    .size(width = 88.dp, height = 54.dp)
                    .let { if (shadow > 0f) it.shadow(shadow.dp, cornerShape, clip = false) else it }
                    .background(Color(resolved.keyFill), cornerShape)
                    // Border follows the key text color (mirrors the real keyboard's
                    // role-first border) so the preview matches the live 框線 look.
                    .let { if (borderWidth > 0f) it.border(borderWidth.dp, Color(resolved.keyText), cornerShape) else it },
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = CUSTOM_PREVIEW_GLYPH,
                color = Color(resolved.keyText),
                fontSize = (CUSTOM_PREVIEW_GLYPH_BASE_SP * appearance.keyFontSizeScale).sp,
            )
        }
    }
}

// Resolved ARGB colors for the custom-theme preview (memoized per theme).
private data class ThemePreviewColors(
    val background: Int,
    val keyFill: Int,
    val keyText: Int,
)

// Sample glyph on the preview key — a Taigi romanization letter with a tone mark.
private const val CUSTOM_PREVIEW_GLYPH = "â"
private const val CUSTOM_PREVIEW_GLYPH_BASE_SP = 26f

// Maps a built-in theme's previewImageName to its bundled screenshot drawable, or
// null when none ships (scaffold → neutral placeholder). Explicit when — never
// resources.getIdentifier, which the resource shrinker can't track. The 預設 default
// (adaptive) reuses the phahtaigi layout screenshot (light + night buckets), so its
// card adapts to dark mode like the theme does. The 5 漸層 themes (櫻花/金煌/海風/翠青/
// 藤紫) are light-only, so a single drawable-xxhdpi asset serves both light and dark.
// Mirrors iOS UIImage(named: previewImageName) in ThemePickerView.swift — except iOS
// byte-copies phahtaigi into a name-keyed theme_standard_preview imageset, while this
// ID-keyed map points 預設 straight at R.drawable.layout_phahtaigi_preview (no copy).
// 框線 / 簡潔 families ship their own screenshots: theme_framed_preview /
// theme_clean_preview carry light + night buckets (adaptive 預設); the 5 漸層
// theme_framed*_preview / theme_clean*_preview are light-only single bucket.
@DrawableRes
private fun builtInThemePreviewRes(previewImageName: String?): Int? =
    when (previewImageName) {
        "theme_standard_preview" -> R.drawable.layout_phahtaigi_preview
        "theme_standardPink_preview" -> R.drawable.theme_standardpink_preview
        "theme_standardGold_preview" -> R.drawable.theme_standardgold_preview
        "theme_standardBlue_preview" -> R.drawable.theme_standardblue_preview
        "theme_standardGreen_preview" -> R.drawable.theme_standardgreen_preview
        "theme_standardPurple_preview" -> R.drawable.theme_standardpurple_preview
        "theme_standardCatppuccin_preview" -> R.drawable.theme_standardcatppuccin_preview
        "theme_framed_preview" -> R.drawable.theme_framed_preview
        "theme_framedPink_preview" -> R.drawable.theme_framedpink_preview
        "theme_framedGold_preview" -> R.drawable.theme_framedgold_preview
        "theme_framedBlue_preview" -> R.drawable.theme_framedblue_preview
        "theme_framedGreen_preview" -> R.drawable.theme_framedgreen_preview
        "theme_framedPurple_preview" -> R.drawable.theme_framedpurple_preview
        "theme_framedCatppuccin_preview" -> R.drawable.theme_framedcatppuccin_preview
        "theme_clean_preview" -> R.drawable.theme_clean_preview
        "theme_cleanPink_preview" -> R.drawable.theme_cleanpink_preview
        "theme_cleanGold_preview" -> R.drawable.theme_cleangold_preview
        "theme_cleanBlue_preview" -> R.drawable.theme_cleanblue_preview
        "theme_cleanGreen_preview" -> R.drawable.theme_cleangreen_preview
        "theme_cleanPurple_preview" -> R.drawable.theme_cleanpurple_preview
        "theme_cleanCatppuccin_preview" -> R.drawable.theme_cleancatppuccin_preview
        else -> null
    }

// Built-in card preview: the bundled screenshot (585x369) when supplied, else a
// neutral placeholder fixed to the card aspect so cards never change height once
// screenshots land.
@Composable
private fun BuiltInThemePreview(
    @DrawableRes previewRes: Int?,
    title: String,
) {
    if (previewRes != null) {
        Image(
            painter = painterResource(previewRes),
            contentDescription = title,
            modifier = Modifier.fillMaxSize(),
            contentScale = ContentScale.Crop,
        )
    } else {
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
}
