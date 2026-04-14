
package com.siansiansu.taigikeyboard.ime.core

import android.content.Context
import android.content.SharedPreferences
import android.util.Log
import androidx.datastore.preferences.core.MutablePreferences
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.preference.PreferenceManager
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.util.AppVersionUtils
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking

class PrefHelper(
    private val context: Context,
) {
    companion object {
        private const val TAG = "PrefHelper"

        // Appearance default values — single source of truth for getters, reset, and UI
        const val DEFAULT_KEY_HEIGHT_SCALE = 1.0f
        const val DEFAULT_KEY_FONT_SIZE_SCALE = 1.0f
        const val DEFAULT_CANDIDATE_TEXT_SIZE_SCALE = 1.0f
        const val DEFAULT_KEY_CORNER_RADIUS = 6.0f
        const val DEFAULT_KEY_BORDER_WIDTH = 0.0f
        const val DEFAULT_FONT_TYPE = "openHuninn"
        const val DEFAULT_COLOR_SETTINGS = "{}"
    }

    private val dataStore = context.preferencesDataStore
    private val scope = CoroutineScope(Dispatchers.IO)

    @Volatile
    private var cachedPrefs: Preferences? = null

    // Pending-keys guard: tracks keys written to cache but not yet confirmed by DataStore.
    // Prevents the collector from reverting local writes with stale DataStore snapshots.
    private val lock = Any()
    private val pendingKeys = mutableMapOf<Preferences.Key<*>, Any?>()

    /**
     * Load all preferences into memory cache with a single DataStore read.
     * Call once during TaigiKeyboard.onCreate() to replace multiple runBlocking calls.
     */
    fun warmUp() {
        cachedPrefs = runBlocking { dataStore.data.first() }
        // Keep cache in sync when preferences change
        scope.launch {
            dataStore.data.collect { prefs ->
                synchronized(lock) {
                    if (pendingKeys.isEmpty()) {
                        // Fast path: no pending writes, accept DataStore snapshot as-is
                        cachedPrefs = prefs
                    } else {
                        // Merge: start from DataStore snapshot, overlay pending values
                        val mutable = prefs.toMutablePreferences()
                        for ((key, value) in pendingKeys) {
                            @Suppress("UNCHECKED_CAST")
                            if (value != null) {
                                (mutable as MutablePreferences)[key as Preferences.Key<Any>] = value
                            }
                        }
                        // Prune pending keys that DataStore has caught up to
                        val iter = pendingKeys.iterator()
                        while (iter.hasNext()) {
                            val (key, pendingValue) = iter.next()
                            if (prefs[key] == pendingValue) {
                                iter.remove()
                            }
                        }
                        cachedPrefs = mutable.toPreferences()
                    }
                }
            }
        }
    }

    private fun <T> updateCacheAndPersist(
        key: Preferences.Key<T>,
        value: T,
    ) {
        synchronized(lock) {
            pendingKeys[key] = value
            cachedPrefs?.toMutablePreferences()?.let { mutable ->
                mutable[key] = value
                cachedPrefs = mutable.toPreferences()
            }
        }
        scope.launch {
            dataStore.edit { prefs ->
                prefs[key] = value
            }
        }
    }

    /**
     * Atomically update multiple keys in the cache and register them as pending.
     * Used by Pattern C setters (inputMode/keyboardLayoutType) that batch-update multiple keys.
     */
    private fun updateCacheBatch(updates: Map<Preferences.Key<*>, Any>) {
        synchronized(lock) {
            for ((key, value) in updates) {
                pendingKeys[key] = value
            }
            cachedPrefs?.toMutablePreferences()?.let { mutable ->
                for ((key, value) in updates) {
                    @Suppress("UNCHECKED_CAST")
                    (mutable as MutablePreferences)[key as Preferences.Key<Any>] = value
                }
                cachedPrefs = mutable.toPreferences()
            }
        }
    }

    private fun <T> cached(
        key: Preferences.Key<T>,
        default: T,
    ): T {
        val snapshot = cachedPrefs
        return if (snapshot != null) {
            snapshot[key] ?: default
        } else {
            runBlocking { dataStore.data.map { it[key] ?: default }.first() }
        }
    }

    // Advanced settings
    var settingsTheme: String
        get() = cached(PreferenceKeys.SETTINGS_THEME, "auto")
        private set(value) {
            updateCacheAndPersist(PreferenceKeys.SETTINGS_THEME, value)
        }

    var showAppIcon: Boolean
        get() = cached(PreferenceKeys.SHOW_APP_ICON, true)
        private set(value) {
            updateCacheAndPersist(PreferenceKeys.SHOW_APP_ICON, value)
        }

    // Correction settings
    var doubleSpacePeriod: Boolean
        get() = cached(PreferenceKeys.DOUBLE_SPACE_PERIOD, true)
        private set(value) {
            updateCacheAndPersist(PreferenceKeys.DOUBLE_SPACE_PERIOD, value)
        }

    // Internal settings
    var versionOnInstall: String
        get() = cached(PreferenceKeys.VERSION_ON_INSTALL, AppVersionUtils.DEFAULT_VERSION_RAW)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.VERSION_ON_INSTALL, value)
        }

    var versionLastUse: String
        get() = cached(PreferenceKeys.VERSION_LAST_USE, AppVersionUtils.DEFAULT_VERSION_RAW)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.VERSION_LAST_USE, value)
        }

    // Keyboard settings
    var activeSubtypeId: Int
        get() = cached(PreferenceKeys.ACTIVE_SUBTYPE_ID, -1)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.ACTIVE_SUBTYPE_ID, value)
        }

    var subtypes: String
        get() = cached(PreferenceKeys.SUBTYPES, "")
        set(value) {
            updateCacheAndPersist(PreferenceKeys.SUBTYPES, value)
        }

    // Looknfeel settings
    var heightFactor: String
        get() = cached(PreferenceKeys.HEIGHT_FACTOR, "normal")
        private set(value) {
            updateCacheAndPersist(PreferenceKeys.HEIGHT_FACTOR, value)
        }

    var longPressDelay: Int
        get() = cached(PreferenceKeys.LONG_PRESS_DELAY, 300)
        private set(value) {
            updateCacheAndPersist(PreferenceKeys.LONG_PRESS_DELAY, value)
        }

    // Language settings
    var inputMode: String
        get() = cached(PreferenceKeys.INPUT_MODE, "tl")
        set(value) {
            val oldValue = inputMode

            // TPS ↔ layout 1:1 sync (reverse direction: inputMode → layout)
            // Write directly to cache/DataStore to avoid recursion with keyboardLayoutType setter
            if (value == "tps" && oldValue != "tps") {
                if (keyboardLayoutType != "tps") {
                    layoutBeforeTps = keyboardLayoutType
                    updateCacheBatch(
                        mapOf<Preferences.Key<*>, Any>(
                            Pair(PreferenceKeys.KEYBOARD_LAYOUT_TYPE, "tps"),
                            Pair(PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED, false),
                        ),
                    )
                    scope.launch {
                        dataStore.edit { prefs ->
                            prefs[PreferenceKeys.KEYBOARD_LAYOUT_TYPE] = "tps"
                            prefs[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED] = false
                        }
                    }
                }
            } else if (value != "tps" && oldValue == "tps") {
                if (keyboardLayoutType == "tps") {
                    val restored = layoutBeforeTps
                    updateCacheBatch(
                        mapOf<Preferences.Key<*>, Any>(
                            Pair(PreferenceKeys.KEYBOARD_LAYOUT_TYPE, restored),
                            Pair(PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED, restored == "phahTaigi"),
                        ),
                    )
                    scope.launch {
                        dataStore.edit { prefs ->
                            prefs[PreferenceKeys.KEYBOARD_LAYOUT_TYPE] = restored
                            prefs[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED] = (restored == "phahTaigi")
                        }
                    }
                }
            }

            // Sync cache + persist inputMode
            updateCacheBatch(mapOf<Preferences.Key<*>, Any>(Pair(PreferenceKeys.INPUT_MODE, value)))
            scope.launch {
                dataStore.edit { prefs ->
                    prefs[PreferenceKeys.INPUT_MODE] = value
                }
            }
        }

    var isTranslateSwapped: Boolean
        get() = cached(PreferenceKeys.IS_TRANSLATE_SWAPPED, false)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.IS_TRANSLATE_SWAPPED, value)
        }

    var outputBothScripts: Boolean
        get() = cached(PreferenceKeys.OUTPUT_BOTH_SCRIPTS, false)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.OUTPUT_BOTH_SCRIPTS, value)
        }

    // Taigi-specific settings
    var enableDoubleTapOO: Boolean
        get() = cached(PreferenceKeys.ENABLE_DOUBLE_TAP_OO, true)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.ENABLE_DOUBLE_TAP_OO, value)
        }

    var enableDoubleTapNN: Boolean
        get() = cached(PreferenceKeys.ENABLE_DOUBLE_TAP_NN, true)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.ENABLE_DOUBLE_TAP_NN, value)
        }

    var autoCapitalizationEnabled: Boolean
        get() = cached(PreferenceKeys.AUTO_CAPITALIZATION_ENABLED, true)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.AUTO_CAPITALIZATION_ENABLED, value)
        }

    var isAutoSpaceEnabled: Boolean
        get() = cached(PreferenceKeys.AUTO_SPACE_ENABLED, false)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.AUTO_SPACE_ENABLED, value)
        }

    var isToolbarAutoCollapse: Boolean
        get() = cached(PreferenceKeys.TOOLBAR_AUTO_COLLAPSE, true)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.TOOLBAR_AUTO_COLLAPSE, value)
        }

    var isGlobeKeyEnabled: Boolean
        get() = cached(PreferenceKeys.GLOBE_KEY_ENABLED, true)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.GLOBE_KEY_ENABLED, value)
        }

    var isSoundFeedbackEnabled: Boolean
        get() = cached(PreferenceKeys.SOUND_FEEDBACK_ENABLED, true)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.SOUND_FEEDBACK_ENABLED, value)
        }

    var isVibrationFeedbackEnabled: Boolean
        get() = cached(PreferenceKeys.VIBRATION_FEEDBACK_ENABLED, true)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.VIBRATION_FEEDBACK_ENABLED, value)
        }

    var fontType: String
        get() = cached(PreferenceKeys.FONT_TYPE, DEFAULT_FONT_TYPE)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.FONT_TYPE, value)
        }

    var phahTaigiLayoutEnabled: Boolean
        get() = cached(PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED, true)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED, value)
        }

    // 鍵盤佈局類型：phahTaigi, qwerty, moe1, moe2, tps
    var keyboardLayoutType: String
        get() = cached(PreferenceKeys.KEYBOARD_LAYOUT_TYPE, "phahTaigi")
        set(value) {
            val oldValue = keyboardLayoutType
            // TPS ↔ inputMode 1:1 sync (forward direction: layout → inputMode)
            // Write inputMode directly to cache/DataStore to avoid recursion with inputMode setter
            if (value == "tps" && oldValue != "tps") {
                val currentInputMode = inputMode
                if (currentInputMode != "tps") {
                    inputModeBeforeTps = currentInputMode
                }
                updateCacheBatch(mapOf<Preferences.Key<*>, Any>(Pair(PreferenceKeys.INPUT_MODE, "tps")))
                scope.launch {
                    dataStore.edit { prefs ->
                        prefs[PreferenceKeys.INPUT_MODE] = "tps"
                    }
                }
            } else if (value != "tps" && oldValue == "tps") {
                val restored = inputModeBeforeTps
                updateCacheBatch(mapOf<Preferences.Key<*>, Any>(Pair(PreferenceKeys.INPUT_MODE, restored)))
                scope.launch {
                    dataStore.edit { prefs ->
                        prefs[PreferenceKeys.INPUT_MODE] = restored
                    }
                }
            }
            // Sync cache + persist layout type
            updateCacheBatch(
                mapOf<Preferences.Key<*>, Any>(
                    Pair(PreferenceKeys.KEYBOARD_LAYOUT_TYPE, value),
                    Pair(PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED, value == "phahTaigi"),
                ),
            )
            scope.launch {
                dataStore.edit { prefs ->
                    prefs[PreferenceKeys.KEYBOARD_LAYOUT_TYPE] = value
                    prefs[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED] = (value == "phahTaigi")
                    if (BuildConfig.DEBUG) Log.d(TAG, "[PREF] KeyboardLayoutType set to: $value")
                }
            }
        }

    // Stores the inputMode before switching to TPS, so it can be restored when leaving TPS
    private var inputModeBeforeTps: String
        get() = cached(PreferenceKeys.INPUT_MODE_BEFORE_TPS, "tl")
        set(value) {
            updateCacheAndPersist(PreferenceKeys.INPUT_MODE_BEFORE_TPS, value)
        }

    // Stores the layout before switching to TPS, so it can be restored when leaving TPS
    private var layoutBeforeTps: String
        get() = cached(PreferenceKeys.LAYOUT_BEFORE_TPS, "phahTaigi")
        set(value) {
            updateCacheAndPersist(PreferenceKeys.LAYOUT_BEFORE_TPS, value)
        }

    // TPS settings
    var tpsOrMapsToER: Boolean
        get() = cached(PreferenceKeys.TPS_OR_MAPS_TO_ER, true)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.TPS_OR_MAPS_TO_ER, value)
        }

    // 詞頻紀錄開關（預設開啟）
    var frequencyRecordingEnabled: Boolean
        get() = cached(PreferenceKeys.FREQUENCY_RECORDING_ENABLED, true)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.FREQUENCY_RECORDING_ENABLED, value)
        }

    // 詞關聯紀錄開關（預設開啟）
    var associationRecordingEnabled: Boolean
        get() = cached(PreferenceKeys.ASSOCIATION_RECORDING_ENABLED, true)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.ASSOCIATION_RECORDING_ENABLED, value)
        }

    // 自訂詞庫開關（預設開啟）
    var customDictEnabled: Boolean
        get() = cached(PreferenceKeys.CUSTOM_DICT_ENABLED, true)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.CUSTOM_DICT_ENABLED, value)
        }

    // 詞庫開關設定
    // 教育部臺灣台語常用詞辭典（kautian）
    var moeDictEnabled: Boolean
        get() = cached(PreferenceKeys.MOE_DICT_ENABLED, true)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.MOE_DICT_ENABLED, value)
        }

    // 台語新詞辭庫（taigitv）
    var newwordDictEnabled: Boolean
        get() = cached(PreferenceKeys.NEWWORD_DICT_ENABLED, true)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.NEWWORD_DICT_ENABLED, value)
        }

    // iTaigi 華台對照典（itaigi）- 預設關閉
    var itaigiDictEnabled: Boolean
        get() = cached(PreferenceKeys.ITAIGI_DICT_ENABLED, false)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.ITAIGI_DICT_ENABLED, value)
        }

    // 台灣植物名彙（sitbut）
    var taiwanPlantDictEnabled: Boolean
        get() = cached(PreferenceKeys.SITBUT_DICT_ENABLED, false)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.SITBUT_DICT_ENABLED, value)
        }

    // 台華線頂對照典（taihoa）
    var taiHuaDictEnabled: Boolean
        get() = cached(PreferenceKeys.TAIHOA_DICT_ENABLED, false)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.TAIHOA_DICT_ENABLED, value)
        }

    // 台日大辭典（taijit）
    var taiwanJapanDictEnabled: Boolean
        get() = cached(PreferenceKeys.TAIJIT_DICT_ENABLED, false)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.TAIJIT_DICT_ENABLED, value)
        }

    // 台語工藝詞庫（kungge）
    var kunggeDictEnabled: Boolean
        get() = cached(PreferenceKeys.KUNGGE_DICT_ENABLED, true)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.KUNGGE_DICT_ENABLED, value)
        }

    // 學科術語辭典（stti）
    var sttiDictEnabled: Boolean
        get() = cached(PreferenceKeys.STTI_DICT_ENABLED, true)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.STTI_DICT_ENABLED, value)
        }

    // 腔口補充資料（khpoo）
    var khpooDictEnabled: Boolean
        get() = cached(PreferenceKeys.KHPOO_DICT_ENABLED, true)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.KHPOO_DICT_ENABLED, value)
        }

    // LKK漢羅合用建議用字（預設開啟）
    var lkkDictEnabled: Boolean
        get() = cached(PreferenceKeys.LKK_DICT_ENABLED, true)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.LKK_DICT_ENABLED, value)
        }

    // Appearance settings
    var keyHeightScale: Float
        get() = cached(PreferenceKeys.KEY_HEIGHT_SCALE, DEFAULT_KEY_HEIGHT_SCALE)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.KEY_HEIGHT_SCALE, value)
        }

    var keyFontSizeScale: Float
        get() = cached(PreferenceKeys.KEY_FONT_SIZE_SCALE, DEFAULT_KEY_FONT_SIZE_SCALE)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.KEY_FONT_SIZE_SCALE, value)
        }

    var candidateTextSizeScale: Float
        get() = cached(PreferenceKeys.CANDIDATE_TEXT_SIZE_SCALE, DEFAULT_CANDIDATE_TEXT_SIZE_SCALE)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.CANDIDATE_TEXT_SIZE_SCALE, value)
        }

    var keyCornerRadius: Float
        get() = cached(PreferenceKeys.KEY_CORNER_RADIUS, DEFAULT_KEY_CORNER_RADIUS)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.KEY_CORNER_RADIUS, value)
        }

    var keyBorderWidth: Float
        get() = cached(PreferenceKeys.KEY_BORDER_WIDTH, DEFAULT_KEY_BORDER_WIDTH)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.KEY_BORDER_WIDTH, value)
        }

    var colorSettings: String
        get() = cached(PreferenceKeys.COLOR_SETTINGS, DEFAULT_COLOR_SETTINGS)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.COLOR_SETTINGS, value)
        }

    // 異用字開關（預設關閉）
    var variantEnabled: Boolean
        get() = cached(PreferenceKeys.VARIANT_DICT_ENABLED, false)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.VARIANT_DICT_ENABLED, value)
        }

    // 在來字開關（預設關閉）
    var khiin: Boolean
        get() = cached(PreferenceKeys.KHIIN_ENABLED, false)
        set(value) {
            updateCacheAndPersist(PreferenceKeys.KHIIN_ENABLED, value)
        }

    /**
     * Snapshot of all dictionary-enabled flags, captured atomically from a single
     * cached preferences read to avoid torn reads across multiple getters.
     */
    data class DictEnabledSnapshot(
        val moe: Boolean,
        val newword: Boolean,
        val itaigi: Boolean,
        val taiwanPlant: Boolean,
        val taiHua: Boolean,
        val taiwanJapan: Boolean,
        val kungge: Boolean,
        val stti: Boolean,
        val khpoo: Boolean,
        val variant: Boolean,
        val khiin: Boolean,
        val lkk: Boolean,
    )

    fun snapshotEnabledDictionaries(): DictEnabledSnapshot {
        val snapshot = cachedPrefs
        return if (snapshot != null) {
            DictEnabledSnapshot(
                moe = snapshot[PreferenceKeys.MOE_DICT_ENABLED] ?: true,
                newword = snapshot[PreferenceKeys.NEWWORD_DICT_ENABLED] ?: true,
                itaigi = snapshot[PreferenceKeys.ITAIGI_DICT_ENABLED] ?: false,
                taiwanPlant = snapshot[PreferenceKeys.SITBUT_DICT_ENABLED] ?: false,
                taiHua = snapshot[PreferenceKeys.TAIHOA_DICT_ENABLED] ?: false,
                taiwanJapan = snapshot[PreferenceKeys.TAIJIT_DICT_ENABLED] ?: false,
                kungge = snapshot[PreferenceKeys.KUNGGE_DICT_ENABLED] ?: true,
                stti = snapshot[PreferenceKeys.STTI_DICT_ENABLED] ?: true,
                khpoo = snapshot[PreferenceKeys.KHPOO_DICT_ENABLED] ?: true,
                variant = snapshot[PreferenceKeys.VARIANT_DICT_ENABLED] ?: false,
                khiin = snapshot[PreferenceKeys.KHIIN_ENABLED] ?: false,
                lkk = snapshot[PreferenceKeys.LKK_DICT_ENABLED] ?: true,
            )
        } else {
            DictEnabledSnapshot(
                moe = moeDictEnabled,
                newword = newwordDictEnabled,
                itaigi = itaigiDictEnabled,
                taiwanPlant = taiwanPlantDictEnabled,
                taiHua = taiHuaDictEnabled,
                taiwanJapan = taiwanJapanDictEnabled,
                kungge = kunggeDictEnabled,
                stti = sttiDictEnabled,
                khpoo = khpooDictEnabled,
                variant = variantEnabled,
                khiin = khiin,
                lkk = lkkDictEnabled,
            )
        }
    }

    // Flow-based API for reactive observations

    /**
     * Observes input mode changes as a Flow.
     * Emits "tl" or "poj" whenever the value changes in DataStore.
     */
    fun observeInputMode(): Flow<String> =
        dataStore.data
            .map { prefs ->
                prefs[PreferenceKeys.INPUT_MODE] ?: "tl"
            }.distinctUntilChanged()

    /**
     * Observes keyboardLayoutType changes as a Flow.
     * Emits the layout type string whenever the value changes in DataStore.
     */
    fun observeKeyboardLayoutType(): Flow<String> =
        dataStore.data
            .map { prefs ->
                prefs[PreferenceKeys.KEYBOARD_LAYOUT_TYPE] ?: "phahTaigi"
            }.distinctUntilChanged()

    /**
     * Migrates data from SharedPreferences to DataStore.
     * This is called once during the first app launch after update.
     */
    suspend fun migrateFromSharedPreferences() {
        val sharedPrefs: SharedPreferences = PreferenceManager.getDefaultSharedPreferences(context)

        dataStore.edit { prefs ->
            // Only migrate if DataStore is empty
            if (prefs.asMap().isEmpty()) {
                if (BuildConfig.DEBUG) {
                    Log.d("PrefHelper", "Migrating from SharedPreferences to DataStore")
                }

                // Advanced settings
                prefs[PreferenceKeys.SETTINGS_THEME] =
                    sharedPrefs.getString("advanced__settings_theme", "auto") ?: "auto"
                prefs[PreferenceKeys.SHOW_APP_ICON] =
                    sharedPrefs.getBoolean("advanced__show_app_icon", true)

                // Correction settings
                prefs[PreferenceKeys.DOUBLE_SPACE_PERIOD] =
                    sharedPrefs.getBoolean("correction__double_space_period", true)

                // Internal settings
                prefs[PreferenceKeys.VERSION_ON_INSTALL] =
                    sharedPrefs.getString("internal__version_on_install", AppVersionUtils.DEFAULT_VERSION_RAW)
                        ?: AppVersionUtils.DEFAULT_VERSION_RAW
                prefs[PreferenceKeys.VERSION_LAST_USE] =
                    sharedPrefs.getString("internal__version_last_use", AppVersionUtils.DEFAULT_VERSION_RAW)
                        ?: AppVersionUtils.DEFAULT_VERSION_RAW

                // Keyboard settings
                prefs[PreferenceKeys.ACTIVE_SUBTYPE_ID] =
                    sharedPrefs.getInt("keyboard__active_subtype_id", -1)
                prefs[PreferenceKeys.SUBTYPES] =
                    sharedPrefs.getString("keyboard__subtypes", "") ?: ""
                prefs[PreferenceKeys.INPUT_MODE] =
                    sharedPrefs.getString("keyboard__input_mode", "tl") ?: "tl"

                // Taigi-specific settings
                prefs[PreferenceKeys.ENABLE_DOUBLE_TAP_OO] =
                    sharedPrefs.getBoolean("taigi__enable_double_tap_oo", true)
                prefs[PreferenceKeys.ENABLE_DOUBLE_TAP_NN] =
                    sharedPrefs.getBoolean("taigi__enable_double_tap_nn", true)
                prefs[PreferenceKeys.AUTO_CAPITALIZATION_ENABLED] =
                    sharedPrefs.getBoolean("taigi__auto_capitalization_enabled", true)
                prefs[PreferenceKeys.AUTO_SPACE_ENABLED] =
                    sharedPrefs.getBoolean("taigi__auto_space_enabled", false)
                prefs[PreferenceKeys.FONT_TYPE] = DEFAULT_FONT_TYPE
                prefs[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED] =
                    sharedPrefs.getBoolean("keyboard__phah_taigi_layout_enabled", true)

                // Looknfeel settings
                prefs[PreferenceKeys.HEIGHT_FACTOR] =
                    sharedPrefs.getString("looknfeel__height_factor", "normal") ?: "normal"
                prefs[PreferenceKeys.LONG_PRESS_DELAY] =
                    sharedPrefs.getInt("looknfeel__long_press_delay", 300)

                if (BuildConfig.DEBUG) {
                    Log.d("PrefHelper", "Migration completed successfully")
                }
            } else {
                if (BuildConfig.DEBUG) {
                    Log.d("PrefHelper", "DataStore already has data, skipping migration")
                }
            }
        }
    }

    /**
     * 重置所有設定為預設值
     * 保留內部設定（版本資訊）
     */
    suspend fun resetToDefaults() {
        dataStore.edit { prefs ->
            // 保存需要保留的值
            val versionOnInstall = prefs[PreferenceKeys.VERSION_ON_INSTALL]
            val versionLastUse = prefs[PreferenceKeys.VERSION_LAST_USE]

            // 清除所有偏好設定
            prefs.clear()

            // 恢復需要保留的值
            versionOnInstall?.let { prefs[PreferenceKeys.VERSION_ON_INSTALL] = it }
            versionLastUse?.let { prefs[PreferenceKeys.VERSION_LAST_USE] = it }

            // 設定預設值（明確寫入，確保一致性）
            prefs[PreferenceKeys.SETTINGS_THEME] = "auto"
            prefs[PreferenceKeys.SHOW_APP_ICON] = true
            prefs[PreferenceKeys.DOUBLE_SPACE_PERIOD] = true
            prefs[PreferenceKeys.ACTIVE_SUBTYPE_ID] = -1
            prefs[PreferenceKeys.SUBTYPES] = ""
            prefs[PreferenceKeys.INPUT_MODE] = "tl"
            prefs[PreferenceKeys.IS_TRANSLATE_SWAPPED] = false
            prefs[PreferenceKeys.OUTPUT_BOTH_SCRIPTS] = false
            prefs[PreferenceKeys.ENABLE_DOUBLE_TAP_OO] = true
            prefs[PreferenceKeys.ENABLE_DOUBLE_TAP_NN] = true
            prefs[PreferenceKeys.AUTO_CAPITALIZATION_ENABLED] = true
            prefs[PreferenceKeys.AUTO_SPACE_ENABLED] = false
            prefs[PreferenceKeys.FONT_TYPE] = DEFAULT_FONT_TYPE
            prefs[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED] = true
            prefs[PreferenceKeys.KEYBOARD_LAYOUT_TYPE] = "phahTaigi"
            prefs[PreferenceKeys.HEIGHT_FACTOR] = "normal"
            prefs[PreferenceKeys.LONG_PRESS_DELAY] = 300
            prefs[PreferenceKeys.KEY_HEIGHT_SCALE] = DEFAULT_KEY_HEIGHT_SCALE
            prefs[PreferenceKeys.KEY_FONT_SIZE_SCALE] = DEFAULT_KEY_FONT_SIZE_SCALE
            prefs[PreferenceKeys.CANDIDATE_TEXT_SIZE_SCALE] = DEFAULT_CANDIDATE_TEXT_SIZE_SCALE
            prefs[PreferenceKeys.KEY_CORNER_RADIUS] = DEFAULT_KEY_CORNER_RADIUS
            prefs[PreferenceKeys.KEY_BORDER_WIDTH] = DEFAULT_KEY_BORDER_WIDTH
            prefs[PreferenceKeys.TPS_OR_MAPS_TO_ER] = true
            prefs[PreferenceKeys.TOOLBAR_AUTO_COLLAPSE] = true
            prefs[PreferenceKeys.GLOBE_KEY_ENABLED] = true
            prefs[PreferenceKeys.SOUND_FEEDBACK_ENABLED] = true
            prefs[PreferenceKeys.VIBRATION_FEEDBACK_ENABLED] = true
            prefs[PreferenceKeys.FREQUENCY_RECORDING_ENABLED] = true
            prefs[PreferenceKeys.ASSOCIATION_RECORDING_ENABLED] = true
            prefs[PreferenceKeys.CUSTOM_DICT_ENABLED] = true
            prefs.remove(PreferenceKeys.COLOR_SETTINGS)

            if (BuildConfig.DEBUG) {
                Log.d("PrefHelper", "All preferences reset to defaults")
            }
        }
    }
}
