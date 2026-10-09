package com.siansiansu.taigikeyboard.ui.tabs.dictionary

import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.ime.settings.PrefHelper
import com.siansiansu.taigikeyboard.ui.components.FilterSearchBar
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import com.siansiansu.taigikeyboard.ui.theme.SectionHeader

// Horizontal indent for kautian subcollection rows nested under the MOE master (DD7).
private val NESTED_TOGGLE_INDENT = 16.dp

private const val MAX_VISIBLE_SEARCH_RESULTS = 5
private val SEARCH_RESULTS_MAX_HEIGHT = 200.dp

// Manage Dictionaries — the dictionary-source toggles (MOE characters with nested kautian
// subcollections, other dictionaries, supplementary data) plus dictionary search pinned at
// the bottom. Desktop twin: the Manage Dictionaries settings pane. Each toggle reads its
// pref when the page opens.
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DictionarySourcesScreen(
    prefs: PrefHelper,
    searchViewModel: DictionarySearchViewModel,
    onNavigateBack: () -> Unit,
) {
    val focusManager = LocalFocusManager.current

    var moeEnabled by remember { mutableStateOf(prefs.moeDictEnabled) }
    var newwordEnabled by remember { mutableStateOf(prefs.newwordDictEnabled) }
    var sttiEnabled by remember { mutableStateOf(prefs.sttiDictEnabled) }
    var kunggeEnabled by remember { mutableStateOf(prefs.kunggeDictEnabled) }
    var itaigiEnabled by remember { mutableStateOf(prefs.itaigiDictEnabled) }
    var taiwanJapanEnabled by remember { mutableStateOf(prefs.taiwanJapanDictEnabled) }
    var taiHuaEnabled by remember { mutableStateOf(prefs.taiHuaDictEnabled) }
    var taiwanPlantEnabled by remember { mutableStateOf(prefs.taiwanPlantDictEnabled) }
    var variantEnabled by remember { mutableStateOf(prefs.variantEnabled) }
    var khiinEnabled by remember { mutableStateOf(prefs.khiin) }
    var khpooEnabled by remember { mutableStateOf(prefs.khpooDictEnabled) }
    var lkkEnabled by remember { mutableStateOf(prefs.lkkDictEnabled) }
    var devEnabled by remember { mutableStateOf(prefs.devDictEnabled) }

    // kautian subcollections (nested under MOE master, greyed when MOE off — DD7).
    // Alternative Readings first, then the accents in config.yaml dialect_columns order.
    var kautianAltReadingEnabled by remember { mutableStateOf(prefs.kautianAltReadingEnabled) }
    var kautianLukangEnabled by remember { mutableStateOf(prefs.kautianAccentLukangEnabled) }
    var kautianSansiaEnabled by remember { mutableStateOf(prefs.kautianAccentSansiaEnabled) }
    var kautianTaipakEnabled by remember { mutableStateOf(prefs.kautianAccentTaipakEnabled) }
    var kautianGilanEnabled by remember { mutableStateOf(prefs.kautianAccentGilanEnabled) }
    var kautianTainanEnabled by remember { mutableStateOf(prefs.kautianAccentTainanEnabled) }
    var kautianKaohsiungEnabled by remember { mutableStateOf(prefs.kautianAccentKaohsiungEnabled) }
    var kautianKinmenEnabled by remember { mutableStateOf(prefs.kautianAccentKinmenEnabled) }
    var kautianMakungEnabled by remember { mutableStateOf(prefs.kautianAccentMakungEnabled) }
    var kautianSintikEnabled by remember { mutableStateOf(prefs.kautianAccentSintikEnabled) }
    var kautianTaichungEnabled by remember { mutableStateOf(prefs.kautianAccentTaichungEnabled) }
    var kautianNameAppendixEnabled by remember { mutableStateOf(prefs.kautianNameAppendixEnabled) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = L10n.desktopDictionarySourcesLink,
                        fontWeight = FontWeight.Bold,
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = L10n.commonBack)
                    }
                },
                colors =
                    TopAppBarDefaults.topAppBarColors(
                        containerColor = MaterialTheme.colorScheme.surfaceContainer,
                    ),
            )
        },
        containerColor = MaterialTheme.colorScheme.surfaceContainer,
    ) { padding ->
        Column(modifier = Modifier.fillMaxSize().padding(padding)) {
            Column(
                modifier =
                    Modifier
                        .weight(1f)
                        .verticalScroll(rememberScrollState())
                        .pointerInput(Unit) { detectTapGestures { focusManager.clearFocus() } }
                        .padding(horizontal = 20.dp)
                        .padding(bottom = AppStyle.scrollContentBottomPadding),
            ) {
                Spacer(Modifier.height(8.dp))
                // MOE dictionaries
                SectionHeader(L10n.dictionaryMoeSectionTitle)

                SettingsCard {
                    DictionaryRowWithDescription(
                        label = L10n.commonMoeDict,
                        checked = moeEnabled,
                        description = L10n.dictionaryMoeDescription,
                        url = "https://sutian.moe.edu.tw/",
                        onCheckedChange = {
                            moeEnabled = it
                            prefs.moeDictEnabled = it
                        },
                    )
                    // kautian subcollections — nested under the MOE master,
                    // greyed when the master is off (DD7). Alternative Readings
                    // first, then the accents in config.yaml dialect_columns
                    // order. Title-only rows via
                    // DictionarySubToggleRow; each writes its own pref + state.
                    val kautianSubcollRows: List<Triple<String, Boolean, (Boolean) -> Unit>> =
                        listOf(
                            Triple(L10n.dictionaryKautianAltReading, kautianAltReadingEnabled) { on ->
                                kautianAltReadingEnabled = on
                                prefs.kautianAltReadingEnabled = on
                            },
                            Triple(L10n.dictionaryKautianAccentLukang, kautianLukangEnabled) { on ->
                                kautianLukangEnabled = on
                                prefs.kautianAccentLukangEnabled = on
                            },
                            Triple(L10n.dictionaryKautianAccentSansia, kautianSansiaEnabled) { on ->
                                kautianSansiaEnabled = on
                                prefs.kautianAccentSansiaEnabled = on
                            },
                            Triple(L10n.dictionaryKautianAccentTaipak, kautianTaipakEnabled) { on ->
                                kautianTaipakEnabled = on
                                prefs.kautianAccentTaipakEnabled = on
                            },
                            Triple(L10n.dictionaryKautianAccentGilan, kautianGilanEnabled) { on ->
                                kautianGilanEnabled = on
                                prefs.kautianAccentGilanEnabled = on
                            },
                            Triple(L10n.dictionaryKautianAccentTainan, kautianTainanEnabled) { on ->
                                kautianTainanEnabled = on
                                prefs.kautianAccentTainanEnabled = on
                            },
                            Triple(L10n.dictionaryKautianAccentKaohsiung, kautianKaohsiungEnabled) { on ->
                                kautianKaohsiungEnabled = on
                                prefs.kautianAccentKaohsiungEnabled = on
                            },
                            Triple(L10n.dictionaryKautianAccentKinmen, kautianKinmenEnabled) { on ->
                                kautianKinmenEnabled = on
                                prefs.kautianAccentKinmenEnabled = on
                            },
                            Triple(L10n.dictionaryKautianAccentMakung, kautianMakungEnabled) { on ->
                                kautianMakungEnabled = on
                                prefs.kautianAccentMakungEnabled = on
                            },
                            Triple(L10n.dictionaryKautianAccentSintik, kautianSintikEnabled) { on ->
                                kautianSintikEnabled = on
                                prefs.kautianAccentSintikEnabled = on
                            },
                            Triple(L10n.dictionaryKautianAccentTaichung, kautianTaichungEnabled) { on ->
                                kautianTaichungEnabled = on
                                prefs.kautianAccentTaichungEnabled = on
                            },
                            Triple(L10n.dictionaryKautianNameAppendix, kautianNameAppendixEnabled) { on ->
                                kautianNameAppendixEnabled = on
                                prefs.kautianNameAppendixEnabled = on
                            },
                        )
                    kautianSubcollRows.forEach { (label, checked, onChange) ->
                        SettingsDivider()
                        DictionarySubToggleRow(
                            label = label,
                            checked = checked,
                            enabled = moeEnabled,
                            modifier = Modifier.padding(start = NESTED_TOGGLE_INDENT),
                            onCheckedChange = onChange,
                        )
                    }
                    SettingsDivider()
                    DictionaryRowWithDescription(
                        label = L10n.commonNewwordDict,
                        checked = newwordEnabled,
                        description = L10n.dictionaryNewwordDescription,
                        url = "https://www.taigitv.org.tw/taigi-words",
                        onCheckedChange = {
                            newwordEnabled = it
                            prefs.newwordDictEnabled = it
                        },
                    )
                    SettingsDivider()
                    DictionaryRowWithDescription(
                        label = L10n.commonSttiDict,
                        checked = sttiEnabled,
                        description = L10n.dictionarySttiDescription,
                        url = "https://stti.moe.edu.tw/index.html?lang=sutgi",
                        onCheckedChange = {
                            sttiEnabled = it
                            prefs.sttiDictEnabled = it
                        },
                    )
                    SettingsDivider()
                    DictionaryRowWithDescription(
                        label = L10n.commonKunggeDict,
                        checked = kunggeEnabled,
                        description = L10n.dictionaryKunggeDescription,
                        url = "https://kanggesu.ntcri.gov.tw",
                        onCheckedChange = {
                            kunggeEnabled = it
                            prefs.kunggeDictEnabled = it
                        },
                    )
                }

                Spacer(Modifier.height(24.dp))

                // Other dictionaries
                SectionHeader(L10n.dictionaryOtherSectionTitle)

                SettingsCard {
                    DictionaryInfoSwitch(
                        L10n.commonITaigiDict,
                        itaigiEnabled,
                        DictionaryInfoData.iTaigi,
                    ) {
                        itaigiEnabled = it
                        prefs.itaigiDictEnabled = it
                    }
                    SettingsDivider()
                    DictionaryInfoSwitch(
                        L10n.commonTaiwanJapanDict,
                        taiwanJapanEnabled,
                        DictionaryInfoData.taiwanJapan,
                    ) {
                        taiwanJapanEnabled = it
                        prefs.taiwanJapanDictEnabled = it
                    }
                    SettingsDivider()
                    DictionaryInfoSwitch(
                        L10n.commonTaiHuaDict,
                        taiHuaEnabled,
                        DictionaryInfoData.taiHua,
                    ) {
                        taiHuaEnabled = it
                        prefs.taiHuaDictEnabled = it
                    }
                    SettingsDivider()
                    DictionaryInfoSwitch(
                        L10n.commonTaiwanPlantDict,
                        taiwanPlantEnabled,
                        DictionaryInfoData.taiwanPlant,
                    ) {
                        taiwanPlantEnabled = it
                        prefs.taiwanPlantDictEnabled = it
                    }
                }

                Spacer(Modifier.height(24.dp))

                // Supplementary data
                SectionHeader(L10n.dictionarySupplementSectionTitle)

                SettingsCard {
                    DictionaryInfoSwitch(
                        L10n.dictionaryVariantDictionary,
                        variantEnabled,
                        DictionaryInfoData.variant,
                    ) {
                        variantEnabled = it
                        prefs.variantEnabled = it
                    }
                    SettingsDivider()
                    DictionaryInfoSwitch(L10n.dictionaryKhiin, khiinEnabled, DictionaryInfoData.khiin) {
                        khiinEnabled = it
                        prefs.khiin = it
                    }
                    SettingsDivider()
                    DictionaryInfoSwitch(
                        L10n.commonAccentDict,
                        khpooEnabled,
                        DictionaryInfoData.khpoo,
                    ) {
                        khpooEnabled = it
                        prefs.khpooDictEnabled = it
                    }
                    SettingsDivider()
                    DictionaryRowWithDescription(
                        label = L10n.dictionaryLkkDict,
                        checked = lkkEnabled,
                        description = L10n.dictionaryLkkDescription,
                        url = "https://docs.google.com/spreadsheets/d/1ICPcP3PuEdLirax-HBLtewiOz53KzAfpme9sjmoIO-w/edit?usp=sharing",
                        onCheckedChange = {
                            lkkEnabled = it
                            prefs.lkkDictEnabled = it
                        },
                    )
                    SettingsDivider()
                    DictionaryRowWithDescription(
                        label = L10n.dictionaryDevSupplementDict,
                        checked = devEnabled,
                        description = L10n.dictionaryDevDescription,
                        url = "https://github.com/luke871016/Taigi-Input-method-dictionary-supplement",
                        onCheckedChange = {
                            devEnabled = it
                            prefs.devDictEnabled = it
                        },
                    )
                }
            }

            DictionarySearchPanel(searchViewModel)
        }
    }
}

// Dictionary search: results pop up above the search bar, at most MAX_VISIBLE_SEARCH_RESULTS rows.
@Composable
private fun DictionarySearchPanel(searchViewModel: DictionarySearchViewModel) {
    val focusManager = LocalFocusManager.current
    val searchText by searchViewModel.searchText.collectAsStateWithLifecycle()
    val searchResults by searchViewModel.results.collectAsStateWithLifecycle()
    val isSearching by searchViewModel.isSearching.collectAsStateWithLifecycle()
    var selectedResultIndex by remember { mutableStateOf<Int?>(null) }
    LaunchedEffect(searchText) { selectedResultIndex = null }

    Column {
        if (searchText.isNotEmpty()) {
            if (searchResults.isEmpty() && !isSearching) {
                Text(
                    text = L10n.dictionaryNoResults,
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

        FilterSearchBar(
            value = searchText,
            onValueChange = { searchViewModel.updateSearchText(it) },
            placeholder = L10n.dictionarySearchPlaceholder,
            onClear = { focusManager.clearFocus() },
        )
    }
}
