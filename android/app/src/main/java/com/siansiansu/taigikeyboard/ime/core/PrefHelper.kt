
package com.siansiansu.taigikeyboard.ime.core

import android.content.Context
import android.content.SharedPreferences
import androidx.datastore.preferences.core.edit
import androidx.preference.PreferenceManager
import com.siansiansu.taigikeyboard.util.VersionName
import android.util.Log
import com.siansiansu.taigikeyboard.BuildConfig
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking

class PrefHelper(
    private val context: Context
) {
    private val dataStore = context.preferencesDataStore
    private val scope = CoroutineScope(Dispatchers.IO)

    // Advanced settings
    var settingsTheme: String
        get() = runBlocking {
            dataStore.data.map { prefs ->
                prefs[PreferenceKeys.SETTINGS_THEME] ?: "auto"
            }.first()
        }
        private set(value) {
            scope.launch {
                dataStore.edit { prefs ->
                    prefs[PreferenceKeys.SETTINGS_THEME] = value
                }
            }
        }

    var showAppIcon: Boolean
        get() = runBlocking {
            dataStore.data.map { prefs ->
                prefs[PreferenceKeys.SHOW_APP_ICON] ?: true
            }.first()
        }
        private set(value) {
            scope.launch {
                dataStore.edit { prefs ->
                    prefs[PreferenceKeys.SHOW_APP_ICON] = value
                }
            }
        }

    // Correction settings
    var doubleSpacePeriod: Boolean
        get() = runBlocking {
            dataStore.data.map { prefs ->
                prefs[PreferenceKeys.DOUBLE_SPACE_PERIOD] ?: true
            }.first()
        }
        private set(value) {
            scope.launch {
                dataStore.edit { prefs ->
                    prefs[PreferenceKeys.DOUBLE_SPACE_PERIOD] = value
                }
            }
        }

    // Internal settings
    var versionOnInstall: String
        get() = runBlocking {
            dataStore.data.map { prefs ->
                prefs[PreferenceKeys.VERSION_ON_INSTALL] ?: VersionName.DEFAULT_RAW
            }.first()
        }
        set(value) {
            scope.launch {
                dataStore.edit { prefs ->
                    prefs[PreferenceKeys.VERSION_ON_INSTALL] = value
                }
            }
        }

    var versionLastUse: String
        get() = runBlocking {
            dataStore.data.map { prefs ->
                prefs[PreferenceKeys.VERSION_LAST_USE] ?: VersionName.DEFAULT_RAW
            }.first()
        }
        set(value) {
            scope.launch {
                dataStore.edit { prefs ->
                    prefs[PreferenceKeys.VERSION_LAST_USE] = value
                }
            }
        }

    // Keyboard settings
    var activeSubtypeId: Int
        get() = runBlocking {
            dataStore.data.map { prefs ->
                prefs[PreferenceKeys.ACTIVE_SUBTYPE_ID] ?: -1
            }.first()
        }
        set(value) {
            scope.launch {
                dataStore.edit { prefs ->
                    prefs[PreferenceKeys.ACTIVE_SUBTYPE_ID] = value
                }
            }
        }

    var subtypes: String
        get() = runBlocking {
            dataStore.data.map { prefs ->
                prefs[PreferenceKeys.SUBTYPES] ?: ""
            }.first()
        }
        set(value) {
            scope.launch {
                dataStore.edit { prefs ->
                    prefs[PreferenceKeys.SUBTYPES] = value
                }
            }
        }

    // Looknfeel settings
    var heightFactor: String
        get() = runBlocking {
            dataStore.data.map { prefs ->
                prefs[PreferenceKeys.HEIGHT_FACTOR] ?: "normal"
            }.first()
        }
        private set(value) {
            scope.launch {
                dataStore.edit { prefs ->
                    prefs[PreferenceKeys.HEIGHT_FACTOR] = value
                }
            }
        }

    var longPressDelay: Int
        get() = runBlocking {
            dataStore.data.map { prefs ->
                prefs[PreferenceKeys.LONG_PRESS_DELAY] ?: 300
            }.first()
        }
        private set(value) {
            scope.launch {
                dataStore.edit { prefs ->
                    prefs[PreferenceKeys.LONG_PRESS_DELAY] = value
                }
            }
        }

    // Suggestion settings
    var suggestionEnabled: Boolean
        get() = runBlocking {
            dataStore.data.map { prefs ->
                prefs[PreferenceKeys.SUGGESTION_ENABLED] ?: true
            }.first()
        }
        private set(value) {
            scope.launch {
                dataStore.edit { prefs ->
                    prefs[PreferenceKeys.SUGGESTION_ENABLED] = value
                }
            }
        }

    // Language settings
    var inputMode: String
        get() = runBlocking {
            dataStore.data.map { prefs ->
                prefs[PreferenceKeys.INPUT_MODE] ?: "tl"
            }.first()
        }
        set(value) {
            scope.launch {
                dataStore.edit { prefs ->
                    prefs[PreferenceKeys.INPUT_MODE] = value
                }
            }
        }

    var showHanjiMode: Boolean
        get() = runBlocking {
            dataStore.data.map { prefs ->
                prefs[PreferenceKeys.SHOW_HANJI_MODE] ?: true
            }.first()
        }
        set(value) {
            scope.launch {
                dataStore.edit { prefs ->
                    prefs[PreferenceKeys.SHOW_HANJI_MODE] = value
                }
            }
        }

    var isTranslateSwapped: Boolean
        get() = runBlocking {
            dataStore.data.map { prefs ->
                prefs[PreferenceKeys.IS_TRANSLATE_SWAPPED] ?: false
            }.first()
        }
        set(value) {
            scope.launch {
                dataStore.edit { prefs ->
                    prefs[PreferenceKeys.IS_TRANSLATE_SWAPPED] = value
                }
            }
        }

    var outputBothScripts: Boolean
        get() = runBlocking {
            dataStore.data.map { prefs ->
                prefs[PreferenceKeys.OUTPUT_BOTH_SCRIPTS] ?: false
            }.first()
        }
        set(value) {
            scope.launch {
                dataStore.edit { prefs ->
                    prefs[PreferenceKeys.OUTPUT_BOTH_SCRIPTS] = value
                }
            }
        }

    // Taigi-specific settings
    var enableDoubleTapOO: Boolean
        get() = runBlocking {
            dataStore.data.map { prefs ->
                prefs[PreferenceKeys.ENABLE_DOUBLE_TAP_OO] ?: true
            }.first()
        }
        set(value) {
            scope.launch {
                dataStore.edit { prefs ->
                    prefs[PreferenceKeys.ENABLE_DOUBLE_TAP_OO] = value
                }
            }
        }

    var enableDoubleTapNN: Boolean
        get() = runBlocking {
            dataStore.data.map { prefs ->
                prefs[PreferenceKeys.ENABLE_DOUBLE_TAP_NN] ?: true
            }.first()
        }
        set(value) {
            scope.launch {
                dataStore.edit { prefs ->
                    prefs[PreferenceKeys.ENABLE_DOUBLE_TAP_NN] = value
                }
            }
        }

    var autoCapitalizationEnabled: Boolean
        get() = runBlocking {
            dataStore.data.map { prefs ->
                prefs[PreferenceKeys.AUTO_CAPITALIZATION_ENABLED] ?: true
            }.first()
        }
        set(value) {
            scope.launch {
                dataStore.edit { prefs ->
                    prefs[PreferenceKeys.AUTO_CAPITALIZATION_ENABLED] = value
                }
            }
        }

    var autoSpaceEnabled: Boolean
        get() = runBlocking {
            dataStore.data.map { prefs ->
                prefs[PreferenceKeys.AUTO_SPACE_ENABLED] ?: false
            }.first()
        }
        set(value) {
            scope.launch {
                dataStore.edit { prefs ->
                    prefs[PreferenceKeys.AUTO_SPACE_ENABLED] = value
                }
            }
        }

    var customFontEnabled: Boolean
        get() = runBlocking {
            dataStore.data.map { prefs ->
                prefs[PreferenceKeys.CUSTOM_FONT_ENABLED] ?: true
            }.first()
        }
        set(value) {
            scope.launch {
                dataStore.edit { prefs ->
                    prefs[PreferenceKeys.CUSTOM_FONT_ENABLED] = value
                }
            }
        }

    var phahTaigiLayoutEnabled: Boolean
        get() = runBlocking {
            dataStore.data.map { prefs ->
                prefs[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED] ?: true
            }.first()
        }
        set(value) {
            scope.launch {
                dataStore.edit { prefs ->
                    prefs[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED] = value
                    android.util.Log.d("PrefHelper", "PhahTaigiLayoutEnabled set to: $value")
                }
            }
        }

    // Onboarding settings
    var hasSeenOnboarding: Boolean
        get() = runBlocking {
            dataStore.data.map { prefs ->
                prefs[PreferenceKeys.HAS_SEEN_ONBOARDING] ?: false
            }.first()
        }
        private set(value) {
            scope.launch {
                dataStore.edit { prefs ->
                    prefs[PreferenceKeys.HAS_SEEN_ONBOARDING] = value
                }
            }
        }

    /**
     * 設定已完成 onboarding（suspend 版本）
     * 確保寫入完成後再繼續執行
     */
    suspend fun setHasSeenOnboarding(value: Boolean) {
        dataStore.edit { prefs ->
            prefs[PreferenceKeys.HAS_SEEN_ONBOARDING] = value
        }
    }

    // Flow-based API for reactive observations
    /**
     * Observes input mode changes as a Flow.
     * Emits "tl" or "poj" whenever the value changes in DataStore.
     */
    fun observeInputMode(): Flow<String> =
        dataStore.data.map { prefs ->
            prefs[PreferenceKeys.INPUT_MODE] ?: "tl"
        }

    /**
     * Observes showHanjiMode changes as a Flow.
     * Emits true/false whenever the value changes in DataStore.
     */
    fun observeShowHanjiMode(): Flow<Boolean> =
        dataStore.data.map { prefs ->
            prefs[PreferenceKeys.SHOW_HANJI_MODE] ?: true
        }

    /**
     * Observes phahTaigiLayoutEnabled changes as a Flow.
     * Emits true/false whenever the value changes in DataStore.
     */
    fun observePhahTaigiLayoutEnabled(): Flow<Boolean> =
        dataStore.data.map { prefs ->
            prefs[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED] ?: false
        }

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
                    sharedPrefs.getString("internal__version_on_install", VersionName.DEFAULT_RAW)
                    ?: VersionName.DEFAULT_RAW
                prefs[PreferenceKeys.VERSION_LAST_USE] =
                    sharedPrefs.getString("internal__version_last_use", VersionName.DEFAULT_RAW)
                    ?: VersionName.DEFAULT_RAW

                // Keyboard settings
                prefs[PreferenceKeys.ACTIVE_SUBTYPE_ID] =
                    sharedPrefs.getInt("keyboard__active_subtype_id", -1)
                prefs[PreferenceKeys.SUBTYPES] =
                    sharedPrefs.getString("keyboard__subtypes", "") ?: ""
                prefs[PreferenceKeys.INPUT_MODE] =
                    sharedPrefs.getString("keyboard__input_mode", "tl") ?: "tl"
                prefs[PreferenceKeys.SHOW_HANJI_MODE] =
                    sharedPrefs.getBoolean("keyboard__show_hanji_mode", true)

                // Taigi-specific settings
                prefs[PreferenceKeys.ENABLE_DOUBLE_TAP_OO] =
                    sharedPrefs.getBoolean("taigi__enable_double_tap_oo", true)
                prefs[PreferenceKeys.ENABLE_DOUBLE_TAP_NN] =
                    sharedPrefs.getBoolean("taigi__enable_double_tap_nn", true)
                prefs[PreferenceKeys.AUTO_CAPITALIZATION_ENABLED] =
                    sharedPrefs.getBoolean("taigi__auto_capitalization_enabled", true)
                prefs[PreferenceKeys.AUTO_SPACE_ENABLED] =
                    sharedPrefs.getBoolean("taigi__auto_space_enabled", false)
                prefs[PreferenceKeys.CUSTOM_FONT_ENABLED] =
                    sharedPrefs.getBoolean("taigi__custom_font_enabled", true)
                prefs[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED] =
                    sharedPrefs.getBoolean("keyboard__phah_taigi_layout_enabled", true)

                // Looknfeel settings
                prefs[PreferenceKeys.HEIGHT_FACTOR] =
                    sharedPrefs.getString("looknfeel__height_factor", "normal") ?: "normal"
                prefs[PreferenceKeys.LONG_PRESS_DELAY] =
                    sharedPrefs.getInt("looknfeel__long_press_delay", 300)

                // Suggestion settings
                prefs[PreferenceKeys.SUGGESTION_ENABLED] =
                    sharedPrefs.getBoolean("suggestion__enabled", true)

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
     * Placeholder for backward compatibility.
     * DataStore doesn't require explicit initialization.
     */
    fun initDefaultPreferences() {
        // No-op: DataStore handles defaults in getters
    }

    /**
     * 重置所有設定為預設值
     * 保留內部設定（版本資訊、onboarding 狀態）
     */
    suspend fun resetToDefaults() {
        dataStore.edit { prefs ->
            // 保存需要保留的值
            val versionOnInstall = prefs[PreferenceKeys.VERSION_ON_INSTALL]
            val versionLastUse = prefs[PreferenceKeys.VERSION_LAST_USE]
            val hasSeenOnboarding = prefs[PreferenceKeys.HAS_SEEN_ONBOARDING]

            // 清除所有偏好設定
            prefs.clear()

            // 恢復需要保留的值
            versionOnInstall?.let { prefs[PreferenceKeys.VERSION_ON_INSTALL] = it }
            versionLastUse?.let { prefs[PreferenceKeys.VERSION_LAST_USE] = it }
            hasSeenOnboarding?.let { prefs[PreferenceKeys.HAS_SEEN_ONBOARDING] = it }

            // 設定預設值（明確寫入，確保一致性）
            prefs[PreferenceKeys.SETTINGS_THEME] = "auto"
            prefs[PreferenceKeys.SHOW_APP_ICON] = true
            prefs[PreferenceKeys.DOUBLE_SPACE_PERIOD] = true
            prefs[PreferenceKeys.ACTIVE_SUBTYPE_ID] = -1
            prefs[PreferenceKeys.SUBTYPES] = ""
            prefs[PreferenceKeys.INPUT_MODE] = "tl"
            prefs[PreferenceKeys.SHOW_HANJI_MODE] = true
            prefs[PreferenceKeys.IS_TRANSLATE_SWAPPED] = false
            prefs[PreferenceKeys.OUTPUT_BOTH_SCRIPTS] = false
            prefs[PreferenceKeys.ENABLE_DOUBLE_TAP_OO] = true
            prefs[PreferenceKeys.ENABLE_DOUBLE_TAP_NN] = true
            prefs[PreferenceKeys.AUTO_CAPITALIZATION_ENABLED] = true
            prefs[PreferenceKeys.AUTO_SPACE_ENABLED] = false
            prefs[PreferenceKeys.CUSTOM_FONT_ENABLED] = true
            prefs[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED] = true
            prefs[PreferenceKeys.HEIGHT_FACTOR] = "normal"
            prefs[PreferenceKeys.LONG_PRESS_DELAY] = 300
            prefs[PreferenceKeys.SUGGESTION_ENABLED] = true

            if (BuildConfig.DEBUG) {
                Log.d("PrefHelper", "All preferences reset to defaults")
            }
        }
    }
}
