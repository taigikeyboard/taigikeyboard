package com.siansiansu.taigikeyboard.ui.tabs.dictionary

import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material3.ExperimentalMaterial3Api
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
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ui.components.ActionRow
import com.siansiansu.taigikeyboard.ui.components.FilterSearchBar
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import com.siansiansu.taigikeyboard.ui.theme.SectionHeader

private const val MAX_VISIBLE_SEARCH_RESULTS = 5
private val SEARCH_RESULTS_MAX_HEIGHT = 200.dp

// Horizontal indent for kautian subcollection rows nested under the MOE master (DD7).
private val NESTED_TOGGLE_INDENT = 16.dp

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

    // Each toggle re-reads from prefs when resetCounter changes (after global Settings tab reset).
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
    var devEnabled by remember(resetCounter) { mutableStateOf(prefs.devDictEnabled) }

    // kautian subcollections (nested under MOE master, greyed when MOE off — DD7).
    // Order mirrors config.yaml dialect_columns.
    var kautianLukangEnabled by remember(resetCounter) { mutableStateOf(prefs.kautianAccentLukangEnabled) }
    var kautianSansiaEnabled by remember(resetCounter) { mutableStateOf(prefs.kautianAccentSansiaEnabled) }
    var kautianTaipakEnabled by remember(resetCounter) { mutableStateOf(prefs.kautianAccentTaipakEnabled) }
    var kautianGilanEnabled by remember(resetCounter) { mutableStateOf(prefs.kautianAccentGilanEnabled) }
    var kautianTainanEnabled by remember(resetCounter) { mutableStateOf(prefs.kautianAccentTainanEnabled) }
    var kautianKaohsiungEnabled by remember(resetCounter) { mutableStateOf(prefs.kautianAccentKaohsiungEnabled) }
    var kautianKinmenEnabled by remember(resetCounter) { mutableStateOf(prefs.kautianAccentKinmenEnabled) }
    var kautianMakungEnabled by remember(resetCounter) { mutableStateOf(prefs.kautianAccentMakungEnabled) }
    var kautianSintikEnabled by remember(resetCounter) { mutableStateOf(prefs.kautianAccentSintikEnabled) }
    var kautianTaichungEnabled by remember(resetCounter) { mutableStateOf(prefs.kautianAccentTaichungEnabled) }
    var kautianNameAppendixEnabled by remember(resetCounter) { mutableStateOf(prefs.kautianNameAppendixEnabled) }

    val scrollBehavior = TopAppBarDefaults.exitUntilCollapsedScrollBehavior()

    Scaffold(
        modifier = Modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
        containerColor = MaterialTheme.colorScheme.surfaceContainer,
        topBar = {
            LargeTopAppBar(
                title = {
                    Text(
                        text = L10n.navTabDictionary,
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
                SectionHeader(L10n.dictionaryDataManagement)

                SettingsCard {
                    ActionRow(
                        label = L10n.dictionaryCustomDictionary,
                        onClick = onCustomDictionary,
                        trailingIcon = Icons.AutoMirrored.Filled.KeyboardArrowRight,
                    )
                    SettingsDivider()
                    ActionRow(
                        label = L10n.dictionaryFrequencyManagement,
                        onClick = onNavigateToFrequency,
                        trailingIcon = Icons.AutoMirrored.Filled.KeyboardArrowRight,
                    )
                    SettingsDivider()
                    ActionRow(
                        label = L10n.dictionaryAssociationManagement,
                        onClick = onNavigateToAssociation,
                        trailingIcon = Icons.AutoMirrored.Filled.KeyboardArrowRight,
                    )
                    SettingsDivider()
                    ActionRow(
                        label = L10n.dictionaryBackupRestore,
                        onClick = onBackupRestore,
                        trailingIcon = Icons.AutoMirrored.Filled.KeyboardArrowRight,
                    )
                }

                Spacer(Modifier.height(24.dp))

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
                    // greyed when the master is off (DD7). Order mirrors
                    // config.yaml dialect_columns. Title-only rows via
                    // DictionarySubToggleRow; each writes its own pref + state.
                    val kautianSubcollRows: List<Triple<String, Boolean, (Boolean) -> Unit>> =
                        listOf(
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

            // Search bar with popup results (pinned at bottom)
            if (searchViewModel != null) {
                Column {
                    // Search results popup above search bar
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

                    // Search bar
                    FilterSearchBar(
                        value = searchText,
                        onValueChange = { searchViewModel.updateSearchText(it) },
                        placeholder = L10n.dictionarySearchPlaceholder,
                        onClear = { focusManager.clearFocus() },
                    )
                }
            }
        }
    }
}
