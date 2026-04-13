package com.siansiansu.taigikeyboard.ui.tabs.tab3

import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTapGestures
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
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.filled.Clear
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LargeTopAppBar
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.dictionary.DictionarySearchResult
import com.siansiansu.taigikeyboard.localization.CommonTexts
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.Tab3Texts
import com.siansiansu.taigikeyboard.ui.components.ActionRow
import com.siansiansu.taigikeyboard.ui.components.SettingInfoButton
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.components.SwitchRow
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import com.siansiansu.taigikeyboard.ui.theme.SectionHeader

// Dictionary info data model
private data class DictionaryInfo(
    val description: String,
    val websiteURL: String? = null,
)

// Static dictionary info (used by DictionaryInfoSwitch rows)
private object DictionaryInfoData {
    val iTaigi =
        DictionaryInfo(
            description = "一个群眾編輯ê開放台語辭典",
            websiteURL = "https://itaigi.tw/",
        )
    val taiwanJapan =
        DictionaryInfo(
            description = "日本時代小川尚義編纂ê台語辭典。",
            websiteURL = "http://taigi.fhl.net/dict/",
        )
    val taiHua =
        DictionaryInfo(
            description = "「台華線頂辭典」是鄭良偉教授提供資料、楊允言教授編修",
            websiteURL = null,
        )
    val taiwanPlant =
        DictionaryInfo(
            description = "日本時代佐佐木舜一整理ê台灣植物台語名。",
            websiteURL = "https://tai2.ntu.edu.tw/ebooks/ListPlFormosSasaki/0/106",
        )
    val variant =
        DictionaryInfo(
            description = "依據教典資料標示台語異用字。",
            websiteURL = null,
        )
    val khpoo =
        DictionaryInfo(
            description = "補充在地腔口差異",
            websiteURL = null,
        )
    val khiin =
        DictionaryInfo(
            description = "「水台文」、「台字田」用字",
            websiteURL = null,
        )
}

@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class)
@Composable
fun DictionarySettingsScreen(
    languageManager: LanguageManager,
    prefs: PrefHelper,
    onCustomDictionary: () -> Unit,
    onNavigateToFrequency: () -> Unit,
    onNavigateToAssociation: () -> Unit,
    onBackupRestore: () -> Unit,
    searchViewModel: DictionarySearchViewModel? = null,
) {
    // Observe language changes for reactive text updates
    val language by languageManager.currentLanguageFlow.collectAsState()
    val focusManager = LocalFocusManager.current

    // Search state
    val searchText by searchViewModel?.searchText?.collectAsState() ?: remember { mutableStateOf("") }
    val searchResults by searchViewModel?.results?.collectAsState() ?: remember { mutableStateOf(emptyList()) }
    val isSearching by searchViewModel?.isSearching?.collectAsState() ?: remember { mutableStateOf(false) }
    var selectedResultIndex by remember { mutableStateOf<Int?>(null) }
    LaunchedEffect(searchText) { selectedResultIndex = null }

    val scrollBehavior = TopAppBarDefaults.exitUntilCollapsedScrollBehavior()

    Scaffold(
        modifier = Modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
        containerColor = MaterialTheme.colorScheme.surfaceContainer,
        topBar = {
            LargeTopAppBar(
                title = {
                    Text(
                        text = languageManager.text(Tab3Texts.tabTitle),
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
        Column(modifier = Modifier.fillMaxSize().padding(innerPadding)) {
            // Scrollable content
            Column(
                modifier =
                    Modifier
                        .weight(1f)
                        .verticalScroll(rememberScrollState())
                        .pointerInput(Unit) { detectTapGestures { focusManager.clearFocus() } }
                        .padding(horizontal = 20.dp)
                        .padding(bottom = 40.dp),
            ) {
                // Data management
                SectionHeader(languageManager.text(Tab3Texts.dataManagement))

                SettingsCard {
                    ActionRow(
                        label = languageManager.text(Tab3Texts.customDictionary),
                        onClick = onCustomDictionary,
                        trailingIcon = Icons.AutoMirrored.Filled.KeyboardArrowRight,
                    )
                    SettingsDivider()
                    ActionRow(
                        label = languageManager.text(Tab3Texts.frequencyManagement),
                        onClick = onNavigateToFrequency,
                        trailingIcon = Icons.AutoMirrored.Filled.KeyboardArrowRight,
                    )
                    SettingsDivider()
                    ActionRow(
                        label = languageManager.text(Tab3Texts.associationManagement),
                        onClick = onNavigateToAssociation,
                        trailingIcon = Icons.AutoMirrored.Filled.KeyboardArrowRight,
                    )
                    SettingsDivider()
                    ActionRow(
                        label = languageManager.text(Tab3Texts.backupRestore),
                        onClick = onBackupRestore,
                        trailingIcon = Icons.AutoMirrored.Filled.KeyboardArrowRight,
                    )
                }

                Spacer(Modifier.height(24.dp))

                // MOE dictionaries (教育部)
                SectionHeader(languageManager.text(Tab3Texts.moeSectionTitle))

                SettingsCard {
                    DictRowWithDescription(
                        label = languageManager.text(CommonTexts.moeDict),
                        checked = prefs.moeDictEnabled,
                        description = "提供臺灣台語搜尋及華語搜尋，可聆聽詞目和例句發音，方便學習。附有分類索引、部首筆劃索引及附錄。",
                        url = "https://sutian.moe.edu.tw/",
                        onCheckedChange = { prefs.moeDictEnabled = it },
                    )
                    SettingsDivider()
                    DictRowWithDescription(
                        label = languageManager.text(CommonTexts.newwordDict),
                        checked = prefs.newwordDictEnabled,
                        description = "台語台邀請專家學者，定期召開會議，討論新興詞彙的適當台語講法，建立詞庫予民眾查詢使用。",
                        url = "https://www.taigitv.org.tw/taigi-words",
                        onCheckedChange = { prefs.newwordDictEnabled = it },
                    )
                    SettingsDivider()
                    DictRowWithDescription(
                        label = languageManager.text(CommonTexts.sttiDict),
                        checked = prefs.sttiDictEnabled,
                        description = "於106 年起進行語文、數學、社會、自然科學、藝術、綜合活動、科技、健康與體育等8大領域學科術語之台語編譯。",
                        url = "https://stti.moe.edu.tw/index.html?lang=sutgi",
                        onCheckedChange = { prefs.sttiDictEnabled = it },
                    )
                    SettingsDivider()
                    DictRowWithDescription(
                        label = languageManager.text(CommonTexts.kunggeDict),
                        checked = prefs.kunggeDictEnabled,
                        description = "收錄多達一千兩百組關鍵台語工藝詞彙，涵蓋陶瓷、木藝、金工、竹藤、纖維、玻璃、漆藝、石藝、皮革、紙藝等十一項。",
                        url = "https://kanggesu.ntcri.org.tw",
                        onCheckedChange = { prefs.kunggeDictEnabled = it },
                    )
                }

                Spacer(Modifier.height(24.dp))

                // Other dictionaries
                SectionHeader(languageManager.text(Tab3Texts.otherSectionTitle))

                SettingsCard {
                    DictionaryInfoSwitch(
                        languageManager.text(CommonTexts.iTaigiDict),
                        prefs.itaigiDictEnabled,
                        DictionaryInfoData.iTaigi,
                        languageManager,
                    ) {
                        prefs.itaigiDictEnabled = it
                    }
                    SettingsDivider()
                    DictionaryInfoSwitch(
                        languageManager.text(CommonTexts.taiwanJapanDict),
                        prefs.taiwanJapanDictEnabled,
                        DictionaryInfoData.taiwanJapan,
                        languageManager,
                    ) {
                        prefs.taiwanJapanDictEnabled = it
                    }
                    SettingsDivider()
                    DictionaryInfoSwitch(
                        languageManager.text(CommonTexts.taiHuaDict),
                        prefs.taiHuaDictEnabled,
                        DictionaryInfoData.taiHua,
                        languageManager,
                    ) {
                        prefs.taiHuaDictEnabled = it
                    }
                    SettingsDivider()
                    DictionaryInfoSwitch(
                        languageManager.text(CommonTexts.taiwanPlantDict),
                        prefs.taiwanPlantDictEnabled,
                        DictionaryInfoData.taiwanPlant,
                        languageManager,
                    ) {
                        prefs.taiwanPlantDictEnabled = it
                    }
                }

                Spacer(Modifier.height(24.dp))

                // Supplementary data
                SectionHeader(languageManager.text(Tab3Texts.supplementSectionTitle))

                SettingsCard {
                    DictionaryInfoSwitch(
                        languageManager.text(Tab3Texts.variantDictionary),
                        prefs.variantEnabled,
                        DictionaryInfoData.variant,
                        languageManager,
                    ) {
                        prefs.variantEnabled = it
                    }
                    SettingsDivider()
                    DictionaryInfoSwitch(languageManager.text(Tab3Texts.khiin), prefs.khiin, DictionaryInfoData.khiin, languageManager) {
                        prefs.khiin = it
                    }
                    SettingsDivider()
                    DictionaryInfoSwitch(
                        languageManager.text(CommonTexts.khpooDict),
                        prefs.khpooDictEnabled,
                        DictionaryInfoData.khpoo,
                        languageManager,
                    ) {
                        prefs.khpooDictEnabled = it
                    }
                    SettingsDivider()
                    DictRowWithDescription(
                        label = languageManager.text(Tab3Texts.lkkDict),
                        checked = prefs.lkkDictEnabled,
                        description = "李江却台語文教基金會漢羅合用建議用字。",
                        url = "https://docs.google.com/spreadsheets/d/1ICPcP3PuEdLirax-HBLtewiOz53KzAfpme9sjmoIO-w/edit?usp=sharing",
                        onCheckedChange = { prefs.lkkDictEnabled = it },
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
                                fontSize = AppStyle.bodyFontSize,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier =
                                    Modifier
                                        .padding(horizontal = 20.dp)
                                        .padding(bottom = 8.dp),
                            )
                        } else if (searchResults.isNotEmpty()) {
                            SettingsCard(
                                modifier =
                                    Modifier
                                        .padding(horizontal = 20.dp)
                                        .padding(bottom = 8.dp)
                                        .heightIn(max = 200.dp),
                            ) {
                                val visible = searchResults.take(5)
                                LazyColumn {
                                    itemsIndexed(visible) { index, result ->
                                        SearchResultRow(
                                            result = result,
                                            isExpanded = selectedResultIndex == index,
                                            onToggle = {
                                                selectedResultIndex = if (selectedResultIndex == index) null else index
                                            },
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
                    SettingsCard(modifier = Modifier.padding(horizontal = 20.dp).padding(top = 8.dp, bottom = 8.dp)) {
                        OutlinedTextField(
                            value = searchText,
                            onValueChange = { searchViewModel.updateSearchText(it) },
                            modifier = Modifier.fillMaxWidth(),
                            placeholder = {
                                Text(languageManager.text(Tab3Texts.searchPlaceholder))
                            },
                            leadingIcon = {
                                Icon(
                                    Icons.Default.Search,
                                    contentDescription = null,
                                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
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
                                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                                        )
                                    }
                                }
                            },
                            colors =
                                OutlinedTextFieldDefaults.colors(
                                    focusedBorderColor = MaterialTheme.colorScheme.primary,
                                    unfocusedBorderColor = Color.Transparent,
                                    focusedContainerColor = MaterialTheme.colorScheme.surface,
                                    unfocusedContainerColor = MaterialTheme.colorScheme.surface,
                                ),
                            shape = RoundedCornerShape(16.dp),
                            singleLine = true,
                        )
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun SearchResultRow(
    result: DictionarySearchResult,
    isExpanded: Boolean,
    onToggle: () -> Unit,
) {
    val context = LocalContext.current
    val languageManager = LanguageManager.getInstance(context)

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
                fontSize = AppStyle.bodyFontSize,
                color = MaterialTheme.colorScheme.onSurface,
            )
            if (!result.hanzi.isNullOrEmpty()) {
                Spacer(Modifier.width(8.dp))
                Text(
                    text = result.hanzi,
                    fontSize = AppStyle.bodyFontSize,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
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
                        fontSize = AppStyle.captionFontSize,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(horizontal = 5.dp, vertical = 2.dp),
                    )
                }
            }
            Spacer(Modifier.weight(1f))
            if (!isExpanded) {
                Icon(
                    painter = painterResource(id = R.drawable.ic_open_in_new),
                    contentDescription = "Open",
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.size(16.dp),
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
                            modifier = Modifier.size(16.dp),
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
                        } catch (_: Exception) {
                        }
                    }) {
                        Icon(
                            painter = painterResource(id = R.drawable.ic_open_in_new),
                            contentDescription = null,
                            modifier = Modifier.size(16.dp),
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
    onCheckedChange: (Boolean) -> Unit,
) {
    var isChecked by remember(checked) { mutableStateOf(checked) }
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
            fontSize = AppStyle.bodyFontSize,
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = contentAlpha),
        )
        Spacer(modifier = Modifier.width(6.dp))
        SettingInfoButton(description = info.description)
        Spacer(modifier = Modifier.weight(1f))
        Switch(
            checked = isChecked,
            enabled = enabled,
            onCheckedChange = {
                isChecked = it
                onCheckedChange(it)
            },
            colors = AppStyle.switchColors(),
        )
    }
}

@Composable
private fun DictRowWithDescription(
    label: String,
    checked: Boolean,
    description: String,
    url: String,
    onCheckedChange: (Boolean) -> Unit,
) {
    var isChecked by remember(checked) { mutableStateOf(checked) }
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
                val linkBlue = MaterialTheme.colorScheme.primary
                Icon(
                    painter = painterResource(id = R.drawable.ic_open_in_new),
                    contentDescription = null,
                    modifier = Modifier.size(16.dp),
                    tint = linkBlue,
                )
                Spacer(Modifier.width(4.dp))
                Text(
                    text = label,
                    fontSize = AppStyle.bodyFontSize,
                    color = linkBlue,
                )
            }
            Spacer(modifier = Modifier.weight(1f))
            Switch(
                checked = isChecked,
                onCheckedChange = {
                    isChecked = it
                    onCheckedChange(it)
                },
                colors = AppStyle.switchColors(),
            )
        }
        Text(
            text = description,
            fontSize = AppStyle.bodyFontSize,
            color = MaterialTheme.colorScheme.onSurface,
        )
    }
}
