package com.siansiansu.taigikeyboard.ui.tabs.theme

import androidx.annotation.DrawableRes
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LargeTopAppBar
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.i18n.stringRes
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.settings.PrefHelper
import com.siansiansu.taigikeyboard.ime.theme.BuiltInTheme
import com.siansiansu.taigikeyboard.ime.theme.BuiltInThemes
import com.siansiansu.taigikeyboard.ime.theme.ThemeAppearance
import com.siansiansu.taigikeyboard.ime.theme.ThemeId
import com.siansiansu.taigikeyboard.ime.theme.ThemeImageVariant
import com.siansiansu.taigikeyboard.ime.theme.UserTheme
import com.siansiansu.taigikeyboard.ime.theme.UserThemeSeed
import com.siansiansu.taigikeyboard.ime.theme.UserThemeStore
import com.siansiansu.taigikeyboard.ime.theme.themeBackground
import com.siansiansu.taigikeyboard.settings.ThemeEditorActivity
import com.siansiansu.taigikeyboard.ui.components.GalleryCard
import com.siansiansu.taigikeyboard.ui.components.GalleryCreateCard
import com.siansiansu.taigikeyboard.ui.components.GalleryScreenshot
import com.siansiansu.taigikeyboard.ui.components.GalleryShelf
import com.siansiansu.taigikeyboard.ui.theme.AppStyle

// Theme tab main screen: a custom-theme shelf (Create New + saved themes with an
// apply/edit/delete menu) above one built-in shelf per key-style family (Filled /
// Outlined / Borderless — same 7 colors, different key style). Selecting any card writes
// selectedThemeId, which wakes the P2 render seam (gradient background + key shadow
// + key border). The editor lives in ThemeEditorActivity; add/edit/delete refresh
// this shelf reactively via observeUserThemes().

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
    val userThemeStore = remember(prefs, context) { CompositionRoot.shared(context).userThemeStore(prefs) }

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
                        text = L10n.navTabTheme,
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
            verticalArrangement = Arrangement.spacedBy(AppStyle.sectionSpacing),
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

            BuiltInThemes.families.forEach { family ->
                BuiltInThemeShelf(
                    title = stringRes(family.titleKey),
                    themes = family.themes,
                    selectedThemeId = selectedThemeId,
                    onThemeSelected = applyTheme,
                )
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
    GalleryShelf(L10n.themeCustomThemesSection) {
        // Create-new card leads the shelf, hidden once the cap is reached (mirrors iOS).
        if (userThemes.size < UserThemeStore.MAX_USER_THEMES) {
            GalleryCreateCard(title = L10n.themeCreateNewTheme, onClick = onCreateNew)
        }
        userThemes.forEach { theme ->
            GalleryCard(
                title = theme.name,
                isSelected = selectedThemeId == theme.id,
                onClick = { onApply(theme.id) },
                preview = { CustomThemeBackgroundPreview(theme.appearance) },
                accessory = {
                    ThemeCardMenu(
                        listOf(
                            ThemeCardAction(L10n.themeCardMenuApply) { onApply(theme.id) },
                            ThemeCardAction(L10n.commonEdit) { onEdit(theme) },
                            ThemeCardAction(L10n.commonDelete, isDestructive = true) { onDelete(theme) },
                        ),
                    )
                },
            )
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
    GalleryShelf(title) {
        themes.forEach { theme ->
            // Resolve once for both the card title and the preview's contentDescription.
            val title = stringRes(theme.displayNameKey)
            GalleryCard(
                title = title,
                isSelected = selectedThemeId == theme.id,
                onClick = { onThemeSelected(theme.id) },
                preview = {
                    // Maps theme.previewImageName to an explicit R.drawable.* (NEVER
                    // resources.getIdentifier, which the resource shrinker can't track).
                    // An unmapped theme falls through to null → neutral placeholder. Mirrors
                    // iOS UIImage(named:).
                    GalleryScreenshot(
                        previewRes = builtInThemePreviewRes(theme.previewImageName),
                        title = title,
                    )
                },
            )
        }
    }
}

/** One trailing-menu action on a custom theme card (apply / edit / delete). */
private data class ThemeCardAction(
    val title: String,
    val isDestructive: Boolean = false,
    val onClick: () -> Unit,
)

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

// A custom-theme card preview: the theme background alone (solid, gradient or photo, same
// surface painting as the keyboard) — no sample key, so the selection checkmark sits on the
// bare surface like a built-in card's (USER 2026-09-26). User themes are seeded at decode, so
// a null background only occurs for a malformed entry and falls back to the seed. Mirrors iOS
// CustomThemeBackgroundPreview.
@Composable
private fun CustomThemeBackgroundPreview(appearance: ThemeAppearance) {
    Box(
        modifier =
            Modifier
                .fillMaxSize()
                .themeBackground(appearance.colors.surface, fallback = Color(UserThemeSeed.SOLID_COLOR), photoVariant = ThemeImageVariant.THUMBNAIL),
    )
}

// Maps a built-in theme's previewImageName to its bundled screenshot drawable, or
// null when none ships (scaffold → neutral placeholder). Explicit when — never
// resources.getIdentifier, which the resource shrinker can't track. The Default
// (adaptive) reuses the phahtaigi layout screenshot (light + night buckets), so its
// card adapts to dark mode like the theme does. The 5 gradient themes (Sakura/Gold/Sea Breeze/Jade/
// Wisteria) are light-only, so a single drawable-xxxhdpi asset serves both light and dark.
// Mirrors iOS UIImage(named: previewImageName) in GalleryScreenshot (GalleryCard.swift) — except iOS ships a
// name-keyed theme_standard_preview imageset, while this map points Default straight at
// R.drawable.layout_phahtaigi_preview. PreviewAssetGeneratorTest renders every drawable this
// map names.
// Outlined / Borderless families ship their own screenshots: theme_framed_preview /
// theme_clean_preview carry light + night buckets (adaptive Default); the 5 gradient
// theme_framed*_preview / theme_clean*_preview are light-only single bucket.
@DrawableRes
internal fun builtInThemePreviewRes(previewImageName: String?): Int? =
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
