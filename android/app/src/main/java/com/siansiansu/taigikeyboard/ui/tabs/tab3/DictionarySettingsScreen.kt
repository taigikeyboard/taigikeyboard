package com.siansiansu.taigikeyboard.ui.tabs.tab3

import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Column
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
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LargeTopAppBar
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.dictionary.DictionarySearchResult
import com.siansiansu.taigikeyboard.localization.CommonTexts
import com.siansiansu.taigikeyboard.localization.Tab3Texts
import com.siansiansu.taigikeyboard.ui.components.ActionRow
import com.siansiansu.taigikeyboard.ui.components.FilterSearchBar
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import com.siansiansu.taigikeyboard.ui.theme.SectionHeader

private const val MAX_VISIBLE_SEARCH_RESULTS = 5
private val SEARCH_RESULTS_MAX_HEIGHT = 200.dp

// Dictionary settings screen — dictionary toggles, search, data management navigation
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DictionarySettingsScreen(
    prefs: PrefHelper,
    onCustomDictionary: () -> Unit,
    onNavigateToFrequency: () -> Unit,
    onNavigateToAssociation: () -> Unit,
    onBackupRestore: () -> Unit,
    searchViewModel: DictionarySearchViewModel? = null,
    resetCounter: Int = 0,
) {
    val focusManager = LocalFocusManager.current

    // Search state
    val searchText by searchViewModel?.searchText?.collectAsStateWithLifecycle() ?: remember { mutableStateOf("") }
    val searchResults by searchViewModel?.results?.collectAsStateWithLifecycle() ?: remember { mutableStateOf(emptyList()) }
    val isSearching by searchViewModel?.isSearching?.collectAsStateWithLifecycle() ?: remember { mutableStateOf(false) }
    var selectedResultIndex by remember { mutableStateOf<Int?>(null) }
    LaunchedEffect(searchText) { selectedResultIndex = null }

    // Each toggle re-reads from prefs when resetCounter changes (after global Tab4 reset).
    // Mirrors InputSettingsScreen pattern; iOS uses @State for the same effect.
    var moeEnabled by remember(resetCounter) { mutableStateOf(prefs.moeDictEnabled) }
    var newwordEnabled by remember(resetCounter) { mutableStateOf(prefs.newwordDictEnabled) }
    var sttiEnabled by remember(resetCounter) { mutableStateOf(prefs.sttiDictEnabled) }
    var kunggeEnabled by remember(resetCounter) { mutableStateOf(prefs.kunggeDictEnabled) }
    var itaigiEnabled by remember(resetCounter) { mutableStateOf(prefs.itaigiDictEnabled) }
    var taiwanJapanEnabled by remember(resetCounter) { mutableStateOf(prefs.taiwanJapanDictEnabled) }
    var taiHuaEnabled by remember(resetCounter) { mutableStateOf(prefs.taiHuaDictEnabled) }
    var taiwanPlantEnabled by remember(resetCounter) { mutableStateOf(prefs.taiwanPlantDictEnabled) }
    var variantEnabled by remember(resetCounter) { mutableStateOf(prefs.variantEnabled) }
    var khiinEnabled by remember(resetCounter) { mutableStateOf(prefs.khiin) }
    var khpooEnabled by remember(resetCounter) { mutableStateOf(prefs.khpooDictEnabled) }
    var lkkEnabled by remember(resetCounter) { mutableStateOf(prefs.lkkDictEnabled) }

    val scrollBehavior = TopAppBarDefaults.exitUntilCollapsedScrollBehavior()

    Scaffold(
        modifier = Modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
        containerColor = MaterialTheme.colorScheme.surfaceContainer,
        topBar = {
            LargeTopAppBar(
                title = {
                    Text(
                        text = Tab3Texts.tabTitle,
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
        Column(modifier = Modifier.fillMaxSize().padding(innerPadding)) {
            // Scrollable content
            Column(
                modifier =
                    Modifier
                        .weight(1f)
                        .verticalScroll(rememberScrollState())
                        .pointerInput(Unit) { detectTapGestures { focusManager.clearFocus() } }
                        .padding(horizontal = 20.dp)
                        .padding(bottom = AppStyle.scrollContentBottomPadding),
            ) {
                // Data management
                SectionHeader(Tab3Texts.dataManagement)

                SettingsCard {
                    ActionRow(
                        label = Tab3Texts.customDictionary,
                        onClick = onCustomDictionary,
                        trailingIcon = Icons.AutoMirrored.Filled.KeyboardArrowRight,
                    )
                    SettingsDivider()
                    ActionRow(
                        label = Tab3Texts.frequencyManagement,
                        onClick = onNavigateToFrequency,
                        trailingIcon = Icons.AutoMirrored.Filled.KeyboardArrowRight,
                    )
                    SettingsDivider()
                    ActionRow(
                        label = Tab3Texts.associationManagement,
                        onClick = onNavigateToAssociation,
                        trailingIcon = Icons.AutoMirrored.Filled.KeyboardArrowRight,
                    )
                    SettingsDivider()
                    ActionRow(
                        label = Tab3Texts.backupRestore,
                        onClick = onBackupRestore,
                        trailingIcon = Icons.AutoMirrored.Filled.KeyboardArrowRight,
                    )
                }

                Spacer(Modifier.height(24.dp))

                // MOE dictionaries (教育部)
                SectionHeader(Tab3Texts.moeSectionTitle)

                SettingsCard {
                    DictionaryRowWithDescription(
                        label = CommonTexts.moeDict,
                        checked = moeEnabled,
                        description = "提供臺灣台語搜尋及華語搜尋，可聆聽詞目和例句發音，方便學習。附有分類索引、部首筆劃索引及附錄。",
                        url = "https://sutian.moe.edu.tw/",
                        onCheckedChange = {
                            moeEnabled = it
                            prefs.moeDictEnabled = it
                        },
                    )
                    SettingsDivider()
                    DictionaryRowWithDescription(
                        label = CommonTexts.newwordDict,
                        checked = newwordEnabled,
                        description = "台語台邀請專家學者，定期召開會議，討論新興詞彙的適當台語講法，建立詞庫予民眾查詢使用。",
                        url = "https://www.taigitv.org.tw/taigi-words",
                        onCheckedChange = {
                            newwordEnabled = it
                            prefs.newwordDictEnabled = it
                        },
                    )
                    SettingsDivider()
                    DictionaryRowWithDescription(
                        label = CommonTexts.sttiDict,
                        checked = sttiEnabled,
                        description = "於106 年起進行語文、數學、社會、自然科學、藝術、綜合活動、科技、健康與體育等8大領域學科術語之台語編譯。",
                        url = "https://stti.moe.edu.tw/index.html?lang=sutgi",
                        onCheckedChange = {
                            sttiEnabled = it
                            prefs.sttiDictEnabled = it
                        },
                    )
                    SettingsDivider()
                    DictionaryRowWithDescription(
                        label = CommonTexts.kunggeDict,
                        checked = kunggeEnabled,
                        description = "收錄多達一千兩百組關鍵台語工藝詞彙，涵蓋陶瓷、木藝、金工、竹藤、纖維、玻璃、漆藝、石藝、皮革、紙藝等十一項。",
                        url = "https://kanggesu.ntcri.org.tw",
                        onCheckedChange = {
                            kunggeEnabled = it
                            prefs.kunggeDictEnabled = it
                        },
                    )
                }

                Spacer(Modifier.height(24.dp))

                // Other dictionaries
                SectionHeader(Tab3Texts.otherSectionTitle)

                SettingsCard {
                    DictionaryInfoSwitch(
                        CommonTexts.iTaigiDict,
                        itaigiEnabled,
                        DictionaryInfoData.iTaigi,
                    ) {
                        itaigiEnabled = it
                        prefs.itaigiDictEnabled = it
                    }
                    SettingsDivider()
                    DictionaryInfoSwitch(
                        CommonTexts.taiwanJapanDict,
                        taiwanJapanEnabled,
                        DictionaryInfoData.taiwanJapan,
                    ) {
                        taiwanJapanEnabled = it
                        prefs.taiwanJapanDictEnabled = it
                    }
                    SettingsDivider()
                    DictionaryInfoSwitch(
                        CommonTexts.taiHuaDict,
                        taiHuaEnabled,
                        DictionaryInfoData.taiHua,
                    ) {
                        taiHuaEnabled = it
                        prefs.taiHuaDictEnabled = it
                    }
                    SettingsDivider()
                    DictionaryInfoSwitch(
                        CommonTexts.taiwanPlantDict,
                        taiwanPlantEnabled,
                        DictionaryInfoData.taiwanPlant,
                    ) {
                        taiwanPlantEnabled = it
                        prefs.taiwanPlantDictEnabled = it
                    }
                }

                Spacer(Modifier.height(24.dp))

                // Supplementary data
                SectionHeader(Tab3Texts.supplementSectionTitle)

                SettingsCard {
                    DictionaryInfoSwitch(
                        Tab3Texts.variantDictionary,
                        variantEnabled,
                        DictionaryInfoData.variant,
                    ) {
                        variantEnabled = it
                        prefs.variantEnabled = it
                    }
                    SettingsDivider()
                    DictionaryInfoSwitch(Tab3Texts.khiin, khiinEnabled, DictionaryInfoData.khiin) {
                        khiinEnabled = it
                        prefs.khiin = it
                    }
                    SettingsDivider()
                    DictionaryInfoSwitch(
                        CommonTexts.khpooDict,
                        khpooEnabled,
                        DictionaryInfoData.khpoo,
                    ) {
                        khpooEnabled = it
                        prefs.khpooDictEnabled = it
                    }
                    SettingsDivider()
                    DictionaryRowWithDescription(
                        label = Tab3Texts.lkkDict,
                        checked = lkkEnabled,
                        description = "李江却台語文教基金會漢羅合用建議用字。",
                        url = "https://docs.google.com/spreadsheets/d/1ICPcP3PuEdLirax-HBLtewiOz53KzAfpme9sjmoIO-w/edit?usp=sharing",
                        onCheckedChange = {
                            lkkEnabled = it
                            prefs.lkkDictEnabled = it
                        },
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
                                text = Tab3Texts.noResults,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                style = MaterialTheme.typography.bodyLarge,
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
                                        .heightIn(max = SEARCH_RESULTS_MAX_HEIGHT),
                            ) {
                                val visible = searchResults.take(MAX_VISIBLE_SEARCH_RESULTS)
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
                    FilterSearchBar(
                        value = searchText,
                        onValueChange = { searchViewModel.updateSearchText(it) },
                        placeholder = Tab3Texts.searchPlaceholder,
                        onClear = { focusManager.clearFocus() },
                    )
                }
            }
        }
    }
}
