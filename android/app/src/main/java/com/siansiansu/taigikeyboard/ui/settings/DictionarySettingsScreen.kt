package com.siansiansu.taigikeyboard.ui.settings

import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowForward
import androidx.compose.material.icons.filled.Clear
import androidx.compose.material.icons.filled.Search
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.dictionary.DictionarySearchResult
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.Tab3Texts
import com.siansiansu.taigikeyboard.ui.components.ActionRow
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.components.SwitchRow
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.runtime.LaunchedEffect

// Dictionary info data model
private data class DictionaryInfo(
    val description: String,
    val websiteURL: String? = null
)

// Static dictionary info
private object DictionaryInfoData {
    val moe = DictionaryInfo(
        description = "教育部編纂，收錄台語常用詞。",
        websiteURL = "https://sutian.moe.edu.tw/"
    )
    val stti = DictionaryInfo(
        description = "教育部提供逐學科專業術語ê台語對譯。",
        websiteURL = "https://stti.moe.edu.tw/"
    )
    val newword = DictionaryInfo(
        description = "公視台語台整理ê台語新詞。",
        websiteURL = "https://www.taigitv.org.tw/taigi-words"
    )
    val kungge = DictionaryInfo(
        description = "國立臺灣工藝研究發展中心收錄ê台語工藝相關台語詞。",
        websiteURL = "https://kanggesu.ntcri.org.tw/NTCRI_TaigiWebSite"
    )
    val iTaigi = DictionaryInfo(
        description = "一个群眾編輯ê開放台語辭典",
        websiteURL = "https://itaigi.tw/"
    )
    val taiwanJapan = DictionaryInfo(
        description = "日本時代小川尚義編纂ê台語辭典。",
        websiteURL = "http://taigi.fhl.net/dict/"
    )
    val taiHua = DictionaryInfo(
        description = "「台華線頂辭典」是鄭良偉教授提供資料、楊允言教授編修",
        websiteURL = null
    )
    val taiwanPlant = DictionaryInfo(
        description = "日本時代佐佐木舜一整理ê台灣植物台語名。",
        websiteURL = "https://tai2.ntu.edu.tw/ebooks/ListPlFormosSasaki/0/106"
    )
    val variant = DictionaryInfo(
        description = "依據教典資料標示台語異用字。",
        websiteURL = null
    )
    val khpoo = DictionaryInfo(
        description = "補充在地腔口差異",
        websiteURL = null
    )
    val khiin = DictionaryInfo(
        description = "「水台文」、「台字田」用字",
        websiteURL = null
    )
    val lkk = DictionaryInfo(
        description = "李江却台語文教基金會漢羅合用建議用字。",
        websiteURL = null
    )
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun DictionarySettingsScreen(
    languageManager: LanguageManager,
    prefs: PrefHelper,
    onClearCache: () -> Unit,
    onCustomDictionary: () -> Unit,
    searchViewModel: DictionarySearchViewModel? = null
) {
    // Observe language changes for reactive text updates
    val language by languageManager.currentLanguageFlow.collectAsState()
    var showClearDialog by remember { mutableStateOf(false) }
    val focusManager = LocalFocusManager.current

    // Search state
    val searchText by searchViewModel?.searchText?.collectAsState() ?: remember { mutableStateOf("") }
    val searchResults by searchViewModel?.results?.collectAsState() ?: remember { mutableStateOf(emptyList()) }
    val isSearching by searchViewModel?.isSearching?.collectAsState() ?: remember { mutableStateOf(false) }
    var selectedResultIndex by remember { mutableStateOf<Int?>(null) }
    LaunchedEffect(searchText) { selectedResultIndex = null }

    Surface(
        modifier = Modifier.fillMaxSize(),
        color = MaterialTheme.colorScheme.surfaceContainer
    ) {
        Column(modifier = Modifier.fillMaxSize()) {
            // Page title (pinned)
            Text(
                text = languageManager.text(Tab3Texts.tabTitle),
                modifier = Modifier
                    .padding(horizontal = 20.dp)
                    .padding(top = 80.dp),
                fontSize = 34.sp,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onSurface
            )

            Spacer(Modifier.height(16.dp))

            // Scrollable content
            Column(
                modifier = Modifier
                    .weight(1f)
                    .verticalScroll(rememberScrollState())
                    .pointerInput(Unit) { detectTapGestures { focusManager.clearFocus() } }
                    .padding(horizontal = 20.dp)
                    .padding(bottom = 40.dp)
            ) {

            // Custom dictionary card (at top)
            SettingsCard {
                ActionRow(
                    label = languageManager.text(Tab3Texts.customDictionary),
                    onClick = onCustomDictionary,
                    trailingIcon = Icons.AutoMirrored.Filled.ArrowForward
                )
            }

            Spacer(Modifier.height(24.dp))

            // 教育部用字
            Text(
                text = languageManager.text(Tab3Texts.moeSectionTitle),
                fontSize = 18.sp,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(start = 16.dp, bottom = 6.dp)
            )

            SettingsCard {
                DictionaryInfoSwitch(languageManager.text(Tab3Texts.moeDict), prefs.moeDictEnabled, DictionaryInfoData.moe, languageManager) {
                    prefs.moeDictEnabled = it
                }
                SettingsDivider()
                DictionaryInfoSwitch(languageManager.text(Tab3Texts.newwordDict), prefs.newwordDictEnabled, DictionaryInfoData.newword, languageManager) {
                    prefs.newwordDictEnabled = it
                }
                SettingsDivider()
                DictionaryInfoSwitch(languageManager.text(Tab3Texts.sttiDict), prefs.sttiDictEnabled, DictionaryInfoData.stti, languageManager) {
                    prefs.sttiDictEnabled = it
                }
                SettingsDivider()
                DictionaryInfoSwitch(languageManager.text(Tab3Texts.kunggeDict), prefs.kunggeDictEnabled, DictionaryInfoData.kungge, languageManager) {
                    prefs.kunggeDictEnabled = it
                }
            }

            Spacer(Modifier.height(24.dp))

            // 其他辭典
            Text(
                text = languageManager.text(Tab3Texts.otherSectionTitle),
                fontSize = 18.sp,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(start = 16.dp, bottom = 6.dp)
            )

            SettingsCard {
                DictionaryInfoSwitch(languageManager.text(Tab3Texts.iTaigiDict), prefs.itaigiDictEnabled, DictionaryInfoData.iTaigi, languageManager) {
                    prefs.itaigiDictEnabled = it
                }
                SettingsDivider()
                DictionaryInfoSwitch(languageManager.text(Tab3Texts.taiwanJapanDict), prefs.taiwanJapanDictEnabled, DictionaryInfoData.taiwanJapan, languageManager) {
                    prefs.taiwanJapanDictEnabled = it
                }
                SettingsDivider()
                DictionaryInfoSwitch(languageManager.text(Tab3Texts.taiHuaDict), prefs.taiHuaDictEnabled, DictionaryInfoData.taiHua, languageManager) {
                    prefs.taiHuaDictEnabled = it
                }
                SettingsDivider()
                DictionaryInfoSwitch(languageManager.text(Tab3Texts.taiwanPlantDict), prefs.taiwanPlantDictEnabled, DictionaryInfoData.taiwanPlant, languageManager) {
                    prefs.taiwanPlantDictEnabled = it
                }
            }

            Spacer(Modifier.height(24.dp))

            // 補充資料
            Text(
                text = languageManager.text(Tab3Texts.supplementSectionTitle),
                fontSize = 18.sp,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(start = 16.dp, bottom = 6.dp)
            )

            SettingsCard {
                DictionaryInfoSwitch(languageManager.text(Tab3Texts.variantDictionary), prefs.variantEnabled, DictionaryInfoData.variant, languageManager) {
                    prefs.variantEnabled = it
                }
                SettingsDivider()
                DictionaryInfoSwitch(languageManager.text(Tab3Texts.khiin), prefs.khiin, DictionaryInfoData.khiin, languageManager) {
                    prefs.khiin = it
                }
                SettingsDivider()
                DictionaryInfoSwitch(languageManager.text(Tab3Texts.khpooDict), prefs.khpooDictEnabled, DictionaryInfoData.khpoo, languageManager) {
                    prefs.khpooDictEnabled = it
                }
                SettingsDivider()
                DictionaryInfoSwitch(languageManager.text(Tab3Texts.lkkDict), prefs.lkkDictEnabled, DictionaryInfoData.lkk, languageManager) {
                    prefs.lkkDictEnabled = it
                }
            }

            Spacer(Modifier.height(24.dp))

            // Clear cache card
            SettingsCard {
                ActionRow(
                    label = languageManager.text(Tab3Texts.clearCache),
                    onClick = { showClearDialog = true },
                    textColor = MaterialTheme.colorScheme.error
                )
            }
            }

            // Search bar with popup results (pinned at bottom)
            if (searchViewModel != null) {
                Column {
                    // Search results popup above search bar
                    if (searchText.isNotEmpty()) {
                        if (searchResults.isEmpty() && !isSearching) {
                            Text(
                                text = languageManager.text(Tab3Texts.noResults),
                                fontSize = 14.sp,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier
                                    .padding(horizontal = 20.dp)
                                    .padding(bottom = 8.dp)
                            )
                        } else if (searchResults.isNotEmpty()) {
                            SettingsCard(
                                modifier = Modifier
                                    .padding(horizontal = 20.dp)
                                    .padding(bottom = 8.dp)
                                    .heightIn(max = 200.dp)
                            ) {
                                val visible = searchResults.take(5)
                                LazyColumn {
                                    itemsIndexed(visible) { index, result ->
                                        SearchResultRow(
                                            result = result,
                                            isExpanded = selectedResultIndex == index,
                                            onToggle = {
                                                selectedResultIndex = if (selectedResultIndex == index) null else index
                                            }
                                        )
                                        if (index < visible.size - 1) {
                                            SettingsDivider()
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Search bar
                    OutlinedTextField(
                        value = searchText,
                        onValueChange = { searchViewModel.updateSearchText(it) },
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 20.dp)
                            .padding(bottom = 16.dp),
                        placeholder = {
                            Text(languageManager.text(Tab3Texts.searchPlaceholder))
                        },
                        leadingIcon = {
                            Icon(
                                Icons.Default.Search,
                                contentDescription = null,
                                tint = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        },
                        trailingIcon = {
                            if (searchText.isNotEmpty()) {
                                IconButton(onClick = {
                                    searchViewModel.updateSearchText("")
                                    focusManager.clearFocus()
                                }) {
                                    Icon(
                                        Icons.Default.Clear,
                                        contentDescription = "Clear",
                                        tint = MaterialTheme.colorScheme.onSurfaceVariant
                                    )
                                }
                            }
                        },
                        colors = OutlinedTextFieldDefaults.colors(
                            focusedBorderColor = MaterialTheme.colorScheme.primary,
                            unfocusedBorderColor = Color.Transparent,
                            focusedContainerColor = MaterialTheme.colorScheme.background,
                            unfocusedContainerColor = MaterialTheme.colorScheme.background
                        ),
                        shape = RoundedCornerShape(16.dp),
                        singleLine = true
                    )
                }
            }
        }
    }

    if (showClearDialog) {
        AlertDialog(
            onDismissRequest = { showClearDialog = false },
            title = { Text(languageManager.text(Tab3Texts.clearCache)) },
            text = { Text(languageManager.text(Tab3Texts.clearCacheMessage)) },
            confirmButton = {
                TextButton(onClick = {
                    showClearDialog = false
                    onClearCache()
                }) {
                    Text(languageManager.text(Tab3Texts.clear))
                }
            },
            dismissButton = {
                TextButton(onClick = { showClearDialog = false }) {
                    Text(languageManager.text(Tab3Texts.cancel))
                }
            }
        )
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun SearchResultRow(
    result: DictionarySearchResult,
    isExpanded: Boolean,
    onToggle: () -> Unit
) {
    val context = LocalContext.current
    val languageManager = LanguageManager.getInstance(context)

    Column {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .background(
                    if (isExpanded) MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f)
                    else Color.Transparent
                )
                .pointerInput(Unit) { detectTapGestures { onToggle() } }
                .padding(horizontal = 20.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = result.roman,
                fontSize = 16.sp,
                color = MaterialTheme.colorScheme.onSurface
            )
            if (!result.hanzi.isNullOrEmpty()) {
                Spacer(Modifier.width(8.dp))
                Text(
                    text = result.hanzi,
                    fontSize = 16.sp,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            result.sources.map { it.displayName }.filter { it.isNotEmpty() }.distinct().forEach { tag ->
                Spacer(Modifier.width(4.dp))
                Surface(
                    shape = RoundedCornerShape(4.dp),
                    color = MaterialTheme.colorScheme.surfaceVariant
                ) {
                    Text(
                        text = tag,
                        fontSize = 10.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(horizontal = 5.dp, vertical = 2.dp)
                    )
                }
            }
            Spacer(Modifier.weight(1f))
            if (!isExpanded) {
                Icon(
                    painter = painterResource(id = R.drawable.ic_open_in_new),
                    contentDescription = "Open",
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.size(16.dp)
                )
            }
        }

        if (isExpanded) {
            val chhoeUrl = result.chhoeUrl()
            val moeUrl = result.moeUrl()
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 20.dp)
                    .padding(bottom = 8.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                if (moeUrl != null) {
                    TextButton(onClick = {
                        onToggle()
                        try {
                            context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(moeUrl)))
                        } catch (_: Exception) {}
                    }) {
                        Icon(
                            painter = painterResource(id = R.drawable.ic_open_in_new),
                            contentDescription = null,
                            modifier = Modifier.size(16.dp)
                        )
                        Spacer(Modifier.width(4.dp))
                        Text(languageManager.text(Tab3Texts.lookupMoe))
                    }
                }
                if (chhoeUrl != null) {
                    TextButton(onClick = {
                        onToggle()
                        try {
                            context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(chhoeUrl)))
                        } catch (_: Exception) {}
                    }) {
                        Icon(
                            painter = painterResource(id = R.drawable.ic_open_in_new),
                            contentDescription = null,
                            modifier = Modifier.size(16.dp)
                        )
                        Spacer(Modifier.width(4.dp))
                        Text(languageManager.text(Tab3Texts.lookupChhoe))
                    }
                }
            }
        }
    }
}

@Composable
private fun DictionaryInfoSwitch(
    label: String,
    checked: Boolean,
    info: DictionaryInfo,
    languageManager: LanguageManager,
    enabled: Boolean = true,
    onCheckedChange: (Boolean) -> Unit
) {
    var isChecked by remember(checked) { mutableStateOf(checked) }
    var showInfoDialog by remember { mutableStateOf(false) }
    val context = LocalContext.current
    val contentAlpha = if (enabled) 1f else 0.38f

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(min = 48.dp)
            .padding(horizontal = 20.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(
            text = label,
            fontSize = 16.sp,
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = contentAlpha)
        )
        IconButton(
            onClick = { showInfoDialog = true },
            modifier = Modifier.size(32.dp)
        ) {
            Icon(
                painter = painterResource(id = R.drawable.ic_help),
                contentDescription = "Info",
                tint = Color(0xFF007AFF),
                modifier = Modifier.size(18.dp)
            )
        }
        Spacer(modifier = Modifier.weight(1f))
        Switch(
            checked = isChecked,
            enabled = enabled,
            onCheckedChange = {
                isChecked = it
                onCheckedChange(it)
            }
        )
    }

    if (showInfoDialog) {
        AlertDialog(
            onDismissRequest = { showInfoDialog = false },
            title = null,
            text = {
                Column(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalAlignment = Alignment.CenterHorizontally
                ) {
                    Spacer(Modifier.height(8.dp))

                    Text(
                        text = info.description,
                        fontSize = 18.sp,
                        color = MaterialTheme.colorScheme.onSurface,
                        lineHeight = 26.sp
                    )
                }
            },
            confirmButton = {
                Button(
                    onClick = { showInfoDialog = false },
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp)
                        .padding(bottom = 8.dp),
                    shape = RoundedCornerShape(12.dp),
                    colors = ButtonDefaults.buttonColors(
                        containerColor = MaterialTheme.colorScheme.surfaceVariant,
                        contentColor = MaterialTheme.colorScheme.onSurface
                    )
                ) {
                    Text(
                        text = "OK",
                        fontSize = 16.sp,
                        fontWeight = FontWeight.SemiBold,
                        modifier = Modifier.padding(vertical = 4.dp)
                    )
                }
            }
        )
    }
}
