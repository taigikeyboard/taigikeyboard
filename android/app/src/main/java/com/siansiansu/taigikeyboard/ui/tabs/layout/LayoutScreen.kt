package com.siansiansu.taigikeyboard.ui.tabs.layout

import androidx.annotation.DrawableRes
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.LargeTopAppBar
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.i18n.generated.StringKey
import com.siansiansu.taigikeyboard.i18n.stringRes
import com.siansiansu.taigikeyboard.ime.settings.PrefHelper
import com.siansiansu.taigikeyboard.ime.text.layout.KeyboardLayoutOption
import com.siansiansu.taigikeyboard.ime.text.layout.KeyboardLayoutOptions
import com.siansiansu.taigikeyboard.ui.components.GalleryCard
import com.siansiansu.taigikeyboard.ui.components.GalleryCreateCard
import com.siansiansu.taigikeyboard.ui.components.GalleryScreenshot
import com.siansiansu.taigikeyboard.ui.components.GalleryShelf
import com.siansiansu.taigikeyboard.ui.theme.AppStyle

// Layout tab main screen: shelves of layout cards (shared gallery chrome with the Theme tab) —
// Custom Layouts (a create card, not available yet), Universal (one key table for both scripts),
// Tâi-lô and Pe̍h-ōe-jī (the layouts whose POJ keys differ, once per script), Phonetic Symbols.
// A Tâi-lô / Pe̍h-ōe-jī card applies its layout and switches the input mode to that script.
// Mirrors iOS LayoutTab.

// The script of a Tâi-lô / Pe̍h-ōe-jī shelf card, as its stored input mode.
internal enum class LayoutScript(
    val inputMode: String,
) {
    TL("tl"),
    POJ("poj"),
}

// One Layout-tab card: a layout plus, on the Tâi-lô / Pe̍h-ōe-jī shelves, the script it applies
// (null: the card writes the layout only).
internal data class LayoutChoice(
    val option: KeyboardLayoutOption,
    val script: LayoutScript?,
) {
    @get:DrawableRes
    val previewRes: Int
        get() = option.pojPreviewRes.takeIf { script == LayoutScript.POJ } ?: option.previewRes

    // A script card is selected only in its own input mode (English or TPS selects neither);
    // a layout-only card follows the stored layout.
    fun isSelected(
        layout: String,
        inputMode: String,
    ): Boolean = layout == option.key && (script == null || inputMode == script.inputMode)
}

internal data class LayoutShelf(
    val titleKey: StringKey,
    val choices: List<LayoutChoice>,
)

internal val layoutShelves: List<LayoutShelf> =
    KeyboardLayoutOptions.romanization.partition { it.pojPreviewRes == null }.let { (universal, perScript) ->
        listOf(
            LayoutShelf(StringKey.LAYOUT_COMMON_LAYOUTS_SECTION, universal.map { LayoutChoice(it, null) }),
            LayoutShelf(StringKey.SETTINGS_TL_MODE, perScript.map { LayoutChoice(it, LayoutScript.TL) }),
            LayoutShelf(StringKey.SETTINGS_POJ_MODE, perScript.map { LayoutChoice(it, LayoutScript.POJ) }),
            LayoutShelf(StringKey.SETTINGS_TPS_MODE, KeyboardLayoutOptions.phonetic.map { LayoutChoice(it, null) }),
        )
    }

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LayoutScreen(prefs: PrefHelper) {
    val selectedLayout by prefs
        .observeKeyboardLayoutType()
        .collectAsStateWithLifecycle(initialValue = prefs.keyboardLayoutType)
    val inputMode by prefs
        .observeInputMode()
        .collectAsStateWithLifecycle(initialValue = prefs.inputMode)

    val selectChoice: (LayoutChoice) -> Unit = { choice ->
        if (!choice.isSelected(selectedLayout, inputMode)) {
            val script = choice.script
            if (script == null) {
                prefs.keyboardLayoutType = choice.option.key
            } else {
                prefs.setKeyboardLayoutAndInputMode(choice.option.key, script.inputMode)
            }
        }
    }

    val scrollBehavior = TopAppBarDefaults.exitUntilCollapsedScrollBehavior()

    Scaffold(
        modifier = Modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
        containerColor = MaterialTheme.colorScheme.surfaceContainer,
        topBar = {
            LargeTopAppBar(
                title = {
                    Text(
                        text = L10n.navTabLayout,
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
            GalleryShelf(L10n.layoutCustomLayoutsSection) {
                GalleryCreateCard(
                    title = L10n.layoutCreateNewLayout,
                    onClick = null,
                    comingSoonBadge = L10n.layoutComingSoon,
                )
            }

            layoutShelves.forEach { shelf ->
                GalleryShelf(stringRes(shelf.titleKey)) {
                    shelf.choices.forEach { choice ->
                        val title = stringRes(choice.option.labelKey)
                        GalleryCard(
                            title = title,
                            isSelected = choice.isSelected(selectedLayout, inputMode),
                            onClick = { selectChoice(choice) },
                            preview = { GalleryScreenshot(previewRes = choice.previewRes, title = title) },
                        )
                    }
                }
            }
        }
    }
}
