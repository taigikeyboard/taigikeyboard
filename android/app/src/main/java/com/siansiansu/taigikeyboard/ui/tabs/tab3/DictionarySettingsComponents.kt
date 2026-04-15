package com.siansiansu.taigikeyboard.ui.tabs.tab3

import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
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
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.dictionary.DictionarySearchResult
import com.siansiansu.taigikeyboard.localization.Tab3Texts
import com.siansiansu.taigikeyboard.ui.components.SettingInfoButton
import com.siansiansu.taigikeyboard.ui.theme.AppStyle

// Helper composables and data for DictionarySettingsScreen

internal data class DictionaryInfo(
    val description: String,
)

internal object DictionaryInfoData {
    val iTaigi = DictionaryInfo(description = "一个群眾編輯ê開放台語辭典")
    val taiwanJapan = DictionaryInfo(description = "日本時代小川尚義編纂ê台語辭典。")
    val taiHua = DictionaryInfo(description = "「台華線頂辭典」是鄭良偉教授提供資料、楊允言教授編修")
    val taiwanPlant = DictionaryInfo(description = "日本時代佐佐木舜一整理ê台灣植物台語名。")
    val variant = DictionaryInfo(description = "依據教典資料標示台語異用字。")
    val khpoo = DictionaryInfo(description = "補充在地腔口差異")
    val khiin = DictionaryInfo(description = "「水台文」、「台字田」用字")
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
            result.sources.map { it.displayName }.filter { it.isNotEmpty() }.distinct().forEach { tag ->
                Spacer(Modifier.width(4.dp))
                Surface(
                    shape = RoundedCornerShape(4.dp),
                    color = MaterialTheme.colorScheme.surfaceVariant,
                ) {
                    Text(
                        text = tag,
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
                            context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(moeUrl)))
                        } catch (_: Exception) {
                        }
                    }) {
                        Icon(
                            painter = painterResource(id = R.drawable.ic_open_in_new),
                            contentDescription = null,
                            modifier = Modifier.size(AppStyle.smallIconSize),
                        )
                        Spacer(Modifier.width(4.dp))
                        Text(Tab3Texts.lookupMoe)
                    }
                }
                if (chhoeUrl != null) {
                    TextButton(onClick = {
                        onToggle()
                        try {
                            context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(chhoeUrl)))
                        } catch (_: Exception) {
                        }
                    }) {
                        Icon(
                            painter = painterResource(id = R.drawable.ic_open_in_new),
                            contentDescription = null,
                            modifier = Modifier.size(AppStyle.smallIconSize),
                        )
                        Spacer(Modifier.width(4.dp))
                        Text(Tab3Texts.lookupChhoe)
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
        Text(
            text = label,
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = contentAlpha),
            style = MaterialTheme.typography.bodyLarge,
        )
        Spacer(modifier = Modifier.width(6.dp))
        SettingInfoButton(description = info.description)
        Spacer(modifier = Modifier.weight(1f))
        Switch(
            checked = checked,
            enabled = enabled,
            onCheckedChange = onCheckedChange,
            colors = AppStyle.switchColors(),
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
            Row(
                verticalAlignment = Alignment.CenterVertically,
                modifier =
                    Modifier.clickable {
                        try {
                            context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)))
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
            Spacer(modifier = Modifier.weight(1f))
            Switch(
                checked = checked,
                onCheckedChange = onCheckedChange,
                colors = AppStyle.switchColors(),
            )
        }
        Text(
            text = description,
            color = MaterialTheme.colorScheme.onSurface,
            style = MaterialTheme.typography.bodyLarge,
        )
    }
}
