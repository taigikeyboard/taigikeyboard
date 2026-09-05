package com.siansiansu.taigikeyboard.ui.tabs.dictionary

import android.content.Intent
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import androidx.core.net.toUri
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.i18n.generated.StringKey
import com.siansiansu.taigikeyboard.i18n.stringRes
import com.siansiansu.taigikeyboard.ime.dictionary.DictionarySearchResult
import com.siansiansu.taigikeyboard.ime.dictionary.DictionarySource
import com.siansiansu.taigikeyboard.ui.components.SettingInfoButton
import com.siansiansu.taigikeyboard.ui.theme.AppStyle

// Helper composables and data for DictionarySettingsScreen

// Carries the i18n key for a source's description; the call site resolves it
// via the active display language (`stringRes(info.descriptionKey)`).
internal data class DictionaryInfo(
    val descriptionKey: StringKey,
)

internal object DictionaryInfoData {
    val iTaigi = DictionaryInfo(descriptionKey = StringKey.DICTIONARY_I_TAIGI_DESCRIPTION)
    val taiwanJapan = DictionaryInfo(descriptionKey = StringKey.DICTIONARY_TAIWAN_JAPAN_DESCRIPTION)
    val taiHua = DictionaryInfo(descriptionKey = StringKey.DICTIONARY_TAI_HUA_DESCRIPTION)
    val taiwanPlant = DictionaryInfo(descriptionKey = StringKey.DICTIONARY_TAIWAN_PLANT_DESCRIPTION)
    val variant = DictionaryInfo(descriptionKey = StringKey.DICTIONARY_VARIANT_DESCRIPTION)
    val khpoo = DictionaryInfo(descriptionKey = StringKey.DICTIONARY_KHPOO_DESCRIPTION)
    val khiin = DictionaryInfo(descriptionKey = StringKey.DICTIONARY_KHIIN_DESCRIPTION)
}

// i18n key for a dictionary source's compact badge label, resolved at the call
// site via the active display language. Kept here (UI layer) so the shared-core
// `DictionarySource` enum stays free of app i18n types.
// 來源 badge 短標籤的 i18n key — 放 UI 層映射,讓 shared-core enum 不依賴 i18n 型別。
private fun DictionarySource.tagKey(): StringKey =
    when (this) {
        DictionarySource.KAUTIAN -> StringKey.DICTIONARY_KAUTIAN_TAG
        DictionarySource.TAIGITV -> StringKey.DICTIONARY_TAIGITV_TAG
        DictionarySource.ITAIGI -> StringKey.DICTIONARY_I_TAIGI_TAG
        DictionarySource.SITBUT -> StringKey.DICTIONARY_SITBUT_TAG
        DictionarySource.TAIHOA -> StringKey.DICTIONARY_TAIHOA_TAG
        DictionarySource.TAIJIT -> StringKey.DICTIONARY_TAIJIT_TAG
        DictionarySource.KUNGGE -> StringKey.DICTIONARY_KUNGGE_TAG
        DictionarySource.STTI -> StringKey.DICTIONARY_STTI_TAG
        DictionarySource.LKK -> StringKey.DICTIONARY_LKK_TAG
        // khpoo/khiin/dev/custom collapse to one "補充資料" badge (reuses the section-title key).
        DictionarySource.KHPOO,
        DictionarySource.KHIIN,
        DictionarySource.DEV,
        DictionarySource.CUSTOM,
        -> StringKey.DICTIONARY_SUPPLEMENT_SECTION_TITLE
    }

@Composable
internal fun SearchResultRow(
    result: DictionarySearchResult,
    isExpanded: Boolean,
    onToggle: () -> Unit,
) {
    val context = LocalContext.current

    Column {
        Row(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .background(
                        if (isExpanded) {
                            MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f)
                        } else {
                            Color.Transparent
                        },
                    ).pointerInput(Unit) { detectTapGestures { onToggle() } }
                    .padding(horizontal = 20.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = result.roman,
                color = MaterialTheme.colorScheme.onSurface,
                style = MaterialTheme.typography.bodyLarge,
            )
            if (!result.hanzi.isNullOrEmpty()) {
                Spacer(Modifier.width(8.dp))
                Text(
                    text = result.hanzi,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    style = MaterialTheme.typography.bodyLarge,
                )
            }
            // Dedup by key (not resolved string) so the 4 supplementary sources
            // collapse to one badge regardless of the active display language.
            result.sources.map { it.tagKey() }.distinct().forEach { tagKey ->
                Spacer(Modifier.width(4.dp))
                Surface(
                    shape = RoundedCornerShape(4.dp),
                    color = MaterialTheme.colorScheme.surfaceVariant,
                ) {
                    Text(
                        text = stringRes(tagKey),
                        style = MaterialTheme.typography.labelLarge,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(horizontal = 5.dp, vertical = 2.dp),
                    )
                }
            }
            Spacer(Modifier.weight(1f))
            if (!isExpanded) {
                Icon(
                    painter = painterResource(id = R.drawable.ic_open_in_new),
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.size(AppStyle.smallIconSize),
                )
            }
        }

        if (isExpanded) {
            val chhoeUrl = result.chhoeUrl()
            val moeUrl = result.moeUrl()
            Row(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 20.dp)
                        .padding(bottom = 8.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                if (moeUrl != null) {
                    TextButton(onClick = {
                        onToggle()
                        try {
                            context.startActivity(Intent(Intent.ACTION_VIEW, moeUrl.toUri()))
                        } catch (_: Exception) {
                        }
                    }) {
                        Icon(
                            painter = painterResource(id = R.drawable.ic_open_in_new),
                            contentDescription = null,
                            modifier = Modifier.size(AppStyle.smallIconSize),
                        )
                        Spacer(Modifier.width(4.dp))
                        Text(L10n.dictionaryLookupMoe)
                    }
                }
                if (chhoeUrl != null) {
                    TextButton(onClick = {
                        onToggle()
                        try {
                            context.startActivity(Intent(Intent.ACTION_VIEW, chhoeUrl.toUri()))
                        } catch (_: Exception) {
                        }
                    }) {
                        Icon(
                            painter = painterResource(id = R.drawable.ic_open_in_new),
                            contentDescription = null,
                            modifier = Modifier.size(AppStyle.smallIconSize),
                        )
                        Spacer(Modifier.width(4.dp))
                        Text(L10n.dictionaryLookupChhoe)
                    }
                }
            }
        }
    }
}

@Composable
internal fun DictionaryInfoSwitch(
    label: String,
    checked: Boolean,
    info: DictionaryInfo,
    enabled: Boolean = true,
    onCheckedChange: (Boolean) -> Unit,
) {
    val contentAlpha = if (enabled) 1f else 0.38f

    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = 48.dp)
                .padding(horizontal = 20.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        // Measure the trailing switch first; localized labels wrap within the remaining width.
        Row(
            modifier = Modifier.weight(1f),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = label,
                modifier = Modifier.weight(1f, fill = false),
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = contentAlpha),
                style = MaterialTheme.typography.bodyLarge,
            )
            Spacer(modifier = Modifier.width(6.dp))
            SettingInfoButton(description = stringRes(info.descriptionKey))
        }
        Switch(
            checked = checked,
            enabled = enabled,
            onCheckedChange = onCheckedChange,
        )
    }
}

// Title-only switch row for nested subcollection toggles (no info button).
// Used for the kautian 腔調 / 姓名附錄 rows nested under the MOE master toggle;
// `enabled = false` greys the label + switch (DD7), `modifier` carries the
// nested indent. Mirrors iOS DictionaryTab.kautianSubcollToggle.
@Composable
internal fun DictionarySubToggleRow(
    label: String,
    checked: Boolean,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    onCheckedChange: (Boolean) -> Unit,
) {
    val contentAlpha = if (enabled) 1f else 0.38f

    Row(
        modifier =
            modifier
                .fillMaxWidth()
                .heightIn(min = 48.dp)
                .padding(horizontal = 20.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        // Weight the label instead of a trailing spacer so it cannot displace the switch.
        Text(
            text = label,
            modifier = Modifier.weight(1f),
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = contentAlpha),
            style = MaterialTheme.typography.bodyLarge,
        )
        Switch(
            checked = checked,
            enabled = enabled,
            onCheckedChange = onCheckedChange,
        )
    }
}

@Composable
internal fun DictionaryRowWithDescription(
    label: String,
    checked: Boolean,
    description: String,
    url: String,
    onCheckedChange: (Boolean) -> Unit,
) {
    val context = LocalContext.current

    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(horizontal = 20.dp, vertical = 12.dp),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            // Bound the link content without expanding its clickable area into the empty space.
            Box(modifier = Modifier.weight(1f)) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier =
                        Modifier.clickable {
                            try {
                                context.startActivity(Intent(Intent.ACTION_VIEW, url.toUri()))
                            } catch (_: Exception) {
                            }
                        },
                ) {
                    val linkColor = MaterialTheme.colorScheme.primary
                    Icon(
                        painter = painterResource(id = R.drawable.ic_open_in_new),
                        contentDescription = null,
                        modifier = Modifier.size(AppStyle.smallIconSize),
                        tint = linkColor,
                    )
                    Spacer(Modifier.width(4.dp))
                    Text(
                        text = label,
                        color = linkColor,
                        style = MaterialTheme.typography.bodyLarge,
                    )
                }
            }
            Switch(
                checked = checked,
                onCheckedChange = onCheckedChange,
            )
        }
        Text(
            text = description,
            color = MaterialTheme.colorScheme.onSurface,
            style = MaterialTheme.typography.bodyLarge,
        )
    }
}
