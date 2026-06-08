// 中文: PrefHelper — 同時實作 EngineSettings / EngineSettingsProvider 兩介面,作為 IME 設定的 single live-read entry。
// 中文: 後端用 androidx.datastore.preferences,cache + collector 為 process-wide companion state;
// 中文: Application.onCreate 呼叫 warmUp() 之後,後續任何 ctor (ViewModel / Activity) 都共用同一份已 warmed cache,
// 中文: 不會再撞到 Main thread runBlocking fallback。
// 中文: Engine 端永遠 live-read,不做快照(對齊 iOS SharedSettings.swift)。

package com.siansiansu.taigikeyboard.ime.core

import android.content.Context
import android.content.SharedPreferences
import androidx.datastore.preferences.core.MutablePreferences
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.preference.PreferenceManager
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettings
import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettingsProvider
import com.siansiansu.taigikeyboard.ime.core.settings.ToneToggles
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.retryWhen
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlin.properties.ReadWriteProperty
import kotlin.reflect.KProperty

class PrefHelper(
    private val context: Context,
) : EngineSettings,
    EngineSettingsProvider {
    companion object {
        private const val TAG = "PrefHelper"

        // The DataStore backend at `Context.preferencesDataStore` is already a
        // process-wide singleton, so multiple `PrefHelper(ctx)` instances all
        // read the same persistent store. We mirror that by hoisting the
        // in-memory cache + pending-write overlay + collector launch into the
        // companion. Result: Application.onCreate calls `prefs.warmUp()` once,
        // and every later ctor (Settings activities, dictionary ViewModels)
        // sees the already-populated cache — the `cached()` runBlocking
        // fallback never fires on the Main thread in production.

        @Volatile
        private var cachedPrefs: Preferences? = null

        // Pending-keys guard: tracks keys written to cache but not yet confirmed by DataStore.
        // Prevents the collector from reverting local writes with stale DataStore snapshots.
        private val lock = Any()
        private val pendingKeys = mutableMapOf<Preferences.Key<*>, Any?>()

        // Guards warmUp() — synchronous monitor (not AtomicBoolean) so the
        // initial DataStore read completes before any other caller observes
        // `collectorStarted = true`. Without this gate, a second caller could
        // see "warmed" while `cachedPrefs` is still null and fall through to
        // the Main-thread runBlocking fallback.
        private val warmUpLock = Any()
        private var collectorStarted = false

        // SupervisorJob: one child failure (e.g. an unexpected DataStore I/O
        // error in the collector) does not cancel the whole scope, so write
        // launches keep working even if the read collector dies.
        private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

        // Collector retry budget: surface DataStore I/O failures via the
        // injected `LoggerBackend.e` on each attempt, give up after this many
        // consecutive failures so the log does not spam forever on a
        // permanently-broken DataStore.
        private const val MAX_COLLECTOR_RETRIES = 3L
        private const val COLLECTOR_RETRY_DELAY_MS = 1_000L

        // Appearance size defaults are owned by ThemeAppearance (the model); referenced here for the settings keys.
        const val DEFAULT_KEY_HEIGHT_SCALE = ThemeAppearance.DEFAULT_KEY_HEIGHT_SCALE
        const val DEFAULT_KEY_FONT_SIZE_SCALE = ThemeAppearance.DEFAULT_KEY_FONT_SIZE_SCALE
        const val DEFAULT_CANDIDATE_TEXT_SIZE_SCALE = ThemeAppearance.DEFAULT_CANDIDATE_TEXT_SIZE_SCALE
        const val DEFAULT_KEY_CORNER_RADIUS = ThemeAppearance.DEFAULT_KEY_CORNER_RADIUS
        const val DEFAULT_KEY_BORDER_WIDTH = ThemeAppearance.DEFAULT_KEY_BORDER_WIDTH
        const val DEFAULT_FONT_TYPE = "openHuninn"
        const val DEFAULT_COLOR_SETTINGS = "{}"

        // Theme defaults (v3.6.2) — "default" = legacy free-pick buffer; empty user-theme list
        const val DEFAULT_SELECTED_THEME_ID = ThemeId.DEFAULT
        const val DEFAULT_USER_THEMES = "[]"
    }

    // Always resolve via the application context: `Context.preferencesDataStore`
    // returns the same process-wide singleton DataStore regardless of receiver,
    // but using `applicationContext` here removes any risk of a non-Application
    // first-warm-up path capturing an Activity context indirectly.
    private val dataStore = context.applicationContext.preferencesDataStore

    /**
     * Property delegate: a `var foo: T by preference(KEY, default)` desugars to
     * a `get()` that calls [cached] (so the live-read contract holds — each
     * access re-reads the cached snapshot) and a `set()` that calls
     * [updateCacheAndPersist]. Replaces ~5 lines of boilerplate per property
     * with a 1-line declaration.
     *
     * Pattern-C properties ([inputMode], [keyboardLayoutType]) keep
     * hand-written setters because they cascade across keys (TPS state
     * machine).
     */
    private inner class PreferenceProperty<T>(
        private val key: Preferences.Key<T>,
        private val default: T,
    ) : ReadWriteProperty<PrefHelper, T> {
        override fun getValue(
            thisRef: PrefHelper,
            property: KProperty<*>,
        ): T = cached(key, default)

        override fun setValue(
            thisRef: PrefHelper,
            property: KProperty<*>,
            value: T,
        ) {
            updateCacheAndPersist(key, value)
        }
    }

    private fun <T> preference(
        key: Preferences.Key<T>,
        default: T,
    ): PreferenceProperty<T> = PreferenceProperty(key, default)

    /**
     * Load all preferences into the process-wide cache with a single DataStore read,
     * and start the collector that keeps the cache in sync.
     *
     * Idempotent: only the first caller does the synchronous read + launches the
     * collector; subsequent callers return immediately AFTER the cache is populated
     * (the synchronized block makes the initial load happen-before any other thread
     * observes `collectorStarted = true`).
     *
     * Called by `TaigiKeyboardApplication.onCreate()` once at process start.
     */
    fun warmUp() {
        synchronized(warmUpLock) {
            if (collectorStarted) return
            // Localize the DataStore reference into the launched lambda so the
            // process-lifetime collector does not retain `this` (the PrefHelper
            // instance) beyond the warm-up call. The logger is captured the
            // same way — the backend is process-wide and stateless, so this
            // does NOT re-leak `this` via `context`.
            val ds = dataStore
            val logger = CompositionRoot.shared(context).logger
            cachedPrefs = runBlocking { ds.data.first() }
            scope.launch {
                ds.data
                    .retryWhen { cause, attempt ->
                        // Process-wide collector: surface the failure, then retry
                        // with backoff. After MAX_COLLECTOR_RETRIES exhausted,
                        // give up and clear `collectorStarted` so a future
                        // `warmUp()` can re-arm. Cache stays populated with the
                        // last-good snapshot until then.
                        logger.e(TAG, "DataStore collector failed (attempt=${attempt + 1})", cause)
                        if (attempt >= MAX_COLLECTOR_RETRIES) {
                            synchronized(warmUpLock) { collectorStarted = false }
                            false
                        } else {
                            delay(COLLECTOR_RETRY_DELAY_MS)
                            true
                        }
                    }.collect { prefs ->
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
            collectorStarted = true
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
     * Atomically applies a multi-key write plan: updates the cache + pending
     * overlay synchronously, then persists the entire batch in a single
     * DataStore transaction.
     *
     * Used by Pattern-C state-machine methods ([applyInputMode] /
     * [applyKeyboardLayoutType]) that need to commit cascade writes
     * atomically — pre-refactor each cascade fired 2-3 separate
     * `dataStore.edit { }` calls, allowing partial-write race windows.
     */
    private fun updateCacheBatchAndPersist(
        updates: Map<Preferences.Key<*>, Any>,
        afterPersist: (suspend () -> Unit)? = null,
    ) {
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
        scope.launch {
            dataStore.edit { prefs ->
                for ((key, value) in updates) {
                    @Suppress("UNCHECKED_CAST")
                    prefs[key as Preferences.Key<Any>] = value
                }
            }
            afterPersist?.invoke()
        }
    }

    // Drop every entry in the per-key overlay; intended for callers that wrote
    // DataStore directly (bypassing `updateCacheAndPersist`) and now need the
    // collector's next snapshot to install cleanly without stale per-key
    // masking. See [migrateFromSharedPreferences] / [resetToDefaults].
    private fun clearPendingOverlay() {
        synchronized(lock) { pendingKeys.clear() }
    }

    private fun <T> cached(
        key: Preferences.Key<T>,
        default: T,
    ): T {
        val snapshot = cachedPrefs
        return if (snapshot != null) {
            snapshot[key] ?: default
        } else {
            CompositionRoot.shared(context).logger.w(
                TAG,
                "cached() reached pre-warm fallback for key=${key.name}; check Application.onCreate ordering",
            )
            runBlocking { dataStore.data.map { it[key] ?: default }.first() }
        }
    }

    // Advanced settings
    var settingsTheme: String by preference(PreferenceKeys.SETTINGS_THEME, "auto")
        private set

    var showAppIcon: Boolean by preference(PreferenceKeys.SHOW_APP_ICON, true)
        private set

    // Correction settings
    var doubleSpacePeriod: Boolean by preference(PreferenceKeys.DOUBLE_SPACE_PERIOD, true)
        private set

    // Internal settings
    var versionOnInstall: String by preference(PreferenceKeys.VERSION_ON_INSTALL, AppVersionTracker.DEFAULT_VERSION_RAW)

    var versionLastUse: String by preference(PreferenceKeys.VERSION_LAST_USE, AppVersionTracker.DEFAULT_VERSION_RAW)

    // Keyboard settings
    var activeSubtypeId: Int by preference(PreferenceKeys.ACTIVE_SUBTYPE_ID, -1)

    var subtypes: String by preference(PreferenceKeys.SUBTYPES, "")

    // Looknfeel settings
    var heightFactor: String by preference(PreferenceKeys.HEIGHT_FACTOR, "normal")
        private set

    var longPressDelay: Int by preference(PreferenceKeys.LONG_PRESS_DELAY, 300)
        private set

    // Language settings
    //
    // Pattern-C: cross-key TPS state machine. Setter forwards to
    // [applyInputMode] which delegates the cascade plan to [TpsCascade] and
    // commits it in a single DataStore transaction. Cannot use the
    // `preference` delegate.
    override var inputMode: String
        get() = cached(PreferenceKeys.INPUT_MODE, "tl")
        set(value) = applyInputMode(value)

    override var isTranslateSwapped: Boolean by preference(PreferenceKeys.IS_TRANSLATE_SWAPPED, false)

    var outputBothScripts: Boolean by preference(PreferenceKeys.OUTPUT_BOTH_SCRIPTS, false)

    // §34/S22 — 顯示羅馬字 toggle. Default true (always-on legacy behaviour).
    var literalRomanCandidateEnabled: Boolean by preference(PreferenceKeys.LITERAL_ROMAN_CANDIDATE, true)

    // Taigi-specific settings
    var enableDoubleTapOO: Boolean by preference(PreferenceKeys.ENABLE_DOUBLE_TAP_OO, true)

    var enableDoubleTapNN: Boolean by preference(PreferenceKeys.ENABLE_DOUBLE_TAP_NN, true)

    var autoCapitalizationEnabled: Boolean by preference(PreferenceKeys.AUTO_CAPITALIZATION_ENABLED, true)

    var isAutoSpaceEnabled: Boolean by preference(PreferenceKeys.AUTO_SPACE_ENABLED, false)

    var isToolbarAutoCollapse: Boolean by preference(PreferenceKeys.TOOLBAR_AUTO_COLLAPSE, true)

    var isGlobeKeyEnabled: Boolean by preference(PreferenceKeys.GLOBE_KEY_ENABLED, false)

    var isSoundFeedbackEnabled: Boolean by preference(PreferenceKeys.SOUND_FEEDBACK_ENABLED, true)

    var isVibrationFeedbackEnabled: Boolean by preference(PreferenceKeys.VIBRATION_FEEDBACK_ENABLED, true)

    var fontType: String by preference(PreferenceKeys.FONT_TYPE, DEFAULT_FONT_TYPE)

    var phahTaigiLayoutEnabled: Boolean by preference(PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED, true)

    // 鍵盤佈局類型：phahTaigi, qwerty, moe1, moe2, tps
    //
    // Pattern-C: cross-key TPS state machine. Setter forwards to
    // [applyKeyboardLayoutType]. See [TpsCascade] for asymmetry vs
    // [applyInputMode]. Cannot use the `preference` delegate.
    var keyboardLayoutType: String
        get() = cached(PreferenceKeys.KEYBOARD_LAYOUT_TYPE, "phahTaigi")
        set(value) = applyKeyboardLayoutType(value)

    // Stores the inputMode before switching to TPS, so it can be restored when leaving TPS.
    //
    // Intentionally NOT a `var by preference(...)` delegate: writes route through the unified
    // batch in [updateCacheBatchAndPersist] (so the save + cascade + main value land in a single
    // DataStore transaction). Exposing a delegate setter would invite cascade-bypassing direct
    // writes that desync the in-memory cache from the persisted store.
    private val inputModeBeforeTps: String
        get() = cached(PreferenceKeys.INPUT_MODE_BEFORE_TPS, "tl")

    // Stores the layout before switching to TPS, so it can be restored when leaving TPS.
    // Same delegate-free rationale as [inputModeBeforeTps] — written only via the unified batch.
    private val layoutBeforeTps: String
        get() = cached(PreferenceKeys.LAYOUT_BEFORE_TPS, "phahTaigi")

    /**
     * Writes [value] to `INPUT_MODE` and applies the TPS ↔ layout 1:1
     * cascade. Cascade plan computed by [TpsCascade.forInputMode]; the entire
     * plan is committed in a single DataStore transaction via
     * [updateCacheBatchAndPersist].
     *
     * - Entering `.tps`: saves the current `keyboardLayoutType` into
     *   `LAYOUT_BEFORE_TPS` (skipped if the layout is already `.tps`) and
     *   flips the layout to `.tps`.
     * - Leaving `.tps`: restores the layout from `LAYOUT_BEFORE_TPS`, but only
     *   when the live layout is still `.tps` — a manual layout change
     *   earlier in the same flow is preserved (GUARDED restore).
     *
     * Mirrors iOS `SharedSettings.setInputMode(_:)` (PR #319 / 1cd6cbfd).
     */
    private fun applyInputMode(value: String) {
        val writes =
            TpsCascade.forInputMode(
                newValue = value,
                oldValue = inputMode,
                currentLayout = keyboardLayoutType,
                layoutBeforeTps = layoutBeforeTps,
            )
        updateCacheBatchAndPersist(writes)
    }

    /**
     * Writes [value] to `KEYBOARD_LAYOUT_TYPE` (+ paired
     * `PHAH_TAIGI_LAYOUT_ENABLED`) and applies the TPS ↔ inputMode 1:1
     * cascade. Cascade plan computed by [TpsCascade.forKeyboardLayoutType];
     * committed in a single DataStore transaction.
     *
     * **Deliberate asymmetry vs [applyInputMode]:** leaving `.tps` on the
     * layout side **unconditionally** restores `INPUT_MODE` from
     * `INPUT_MODE_BEFORE_TPS` (no guard), whereas the input-side exit guards
     * on `keyboardLayoutType == "tps"` before restoring layout. Mirrors
     * HEAD~1 + iOS PR-1 byte-for-byte. Any future symmetrization belongs in
     * a separate slice — do NOT collapse the two sides during incidental
     * cleanup.
     */
    private fun applyKeyboardLayoutType(value: String) {
        val writes =
            TpsCascade.forKeyboardLayoutType(
                newValue = value,
                oldValue = keyboardLayoutType,
                currentInputMode = inputMode,
                inputModeBeforeTps = inputModeBeforeTps,
            )
        // Logger fires on the IO coroutine after the batch commits.
        // HEAD~1 fired the same log inside the `dataStore.edit { }` transform
        // (during persist); the new placement fires immediately after the
        // edit completes — observationally indistinguishable for a debug log.
        updateCacheBatchAndPersist(writes) {
            CompositionRoot.shared(context).logger.debug(TAG) { "[PREF] KeyboardLayoutType set to: $value" }
        }
    }

    // TPS settings
    var tpsOrMapsToER: Boolean by preference(PreferenceKeys.TPS_OR_MAPS_TO_ER, true)

    // 詞頻紀錄開關（預設開啟）
    var frequencyRecordingEnabled: Boolean by preference(PreferenceKeys.FREQUENCY_RECORDING_ENABLED, true)

    // 詞關聯紀錄開關（預設開啟）
    var associationRecordingEnabled: Boolean by preference(PreferenceKeys.ASSOCIATION_RECORDING_ENABLED, true)

    // 自訂詞庫開關（預設開啟）
    var customDictEnabled: Boolean by preference(PreferenceKeys.CUSTOM_DICT_ENABLED, true)

    // 詞庫開關設定
    // 教育部臺灣台語常用詞辭典（kautian）
    var moeDictEnabled: Boolean by preference(PreferenceKeys.MOE_DICT_ENABLED, true)

    // 台語新詞辭庫（taigitv）
    var newwordDictEnabled: Boolean by preference(PreferenceKeys.NEWWORD_DICT_ENABLED, true)

    // iTaigi 華台對照典（itaigi）- 預設關閉
    var itaigiDictEnabled: Boolean by preference(PreferenceKeys.ITAIGI_DICT_ENABLED, false)

    // 台灣植物名彙（sitbut）
    var taiwanPlantDictEnabled: Boolean by preference(PreferenceKeys.SITBUT_DICT_ENABLED, false)

    // 台華線頂對照典（taihoa）
    var taiHuaDictEnabled: Boolean by preference(PreferenceKeys.TAIHOA_DICT_ENABLED, false)

    // 台日大辭典（taijit）
    var taiwanJapanDictEnabled: Boolean by preference(PreferenceKeys.TAIJIT_DICT_ENABLED, false)

    // 台語工藝詞庫（kungge）
    var kunggeDictEnabled: Boolean by preference(PreferenceKeys.KUNGGE_DICT_ENABLED, true)

    // 學科術語辭典（stti）
    var sttiDictEnabled: Boolean by preference(PreferenceKeys.STTI_DICT_ENABLED, true)

    // 腔口補充資料（khpoo）
    var khpooDictEnabled: Boolean by preference(PreferenceKeys.KHPOO_DICT_ENABLED, true)

    // LKK漢羅合用建議用字（預設開啟）
    var lkkDictEnabled: Boolean by preference(PreferenceKeys.LKK_DICT_ENABLED, true)

    // 開發者補充辭典（詞庫增補檔案，預設開啟）
    var devDictEnabled: Boolean by preference(PreferenceKeys.DEV_DICT_ENABLED, true)

    // 教育部辭典子集（腔調 + 姓名附錄，巢狀於 MOE master 下）— 預設全開（DD5 opt-out）。
    // 腔調順序對齊 config.yaml dialect_columns；bit 佈局由 Rust compute_filters 持有。
    var kautianAccentLukangEnabled: Boolean by preference(PreferenceKeys.KAUTIAN_ACCENT_LUKANG_ENABLED, true)

    var kautianAccentSansiaEnabled: Boolean by preference(PreferenceKeys.KAUTIAN_ACCENT_SANSIA_ENABLED, true)

    var kautianAccentTaipakEnabled: Boolean by preference(PreferenceKeys.KAUTIAN_ACCENT_TAIPAK_ENABLED, true)

    var kautianAccentGilanEnabled: Boolean by preference(PreferenceKeys.KAUTIAN_ACCENT_GILAN_ENABLED, true)

    var kautianAccentTainanEnabled: Boolean by preference(PreferenceKeys.KAUTIAN_ACCENT_TAINAN_ENABLED, true)

    var kautianAccentKaohsiungEnabled: Boolean by preference(PreferenceKeys.KAUTIAN_ACCENT_KAOHSIUNG_ENABLED, true)

    var kautianAccentKinmenEnabled: Boolean by preference(PreferenceKeys.KAUTIAN_ACCENT_KINMEN_ENABLED, true)

    var kautianAccentMakungEnabled: Boolean by preference(PreferenceKeys.KAUTIAN_ACCENT_MAKUNG_ENABLED, true)

    var kautianAccentSintikEnabled: Boolean by preference(PreferenceKeys.KAUTIAN_ACCENT_SINTIK_ENABLED, true)

    var kautianAccentTaichungEnabled: Boolean by preference(PreferenceKeys.KAUTIAN_ACCENT_TAICHUNG_ENABLED, true)

    // 姓名附錄預設開 (opt-out;CROSS-PLATFORM mirrors iOS SharedSettings.swift isKautianNameAppendixEnabledKey default true)
    var kautianNameAppendixEnabled: Boolean by preference(PreferenceKeys.KAUTIAN_NAME_APPENDIX_ENABLED, true)

    // Appearance settings
    var keyHeightScale: Float by preference(PreferenceKeys.KEY_HEIGHT_SCALE, DEFAULT_KEY_HEIGHT_SCALE)

    var keyFontSizeScale: Float by preference(PreferenceKeys.KEY_FONT_SIZE_SCALE, DEFAULT_KEY_FONT_SIZE_SCALE)

    var candidateTextSizeScale: Float by preference(PreferenceKeys.CANDIDATE_TEXT_SIZE_SCALE, DEFAULT_CANDIDATE_TEXT_SIZE_SCALE)

    var keyCornerRadius: Float by preference(PreferenceKeys.KEY_CORNER_RADIUS, DEFAULT_KEY_CORNER_RADIUS)

    var keyBorderWidth: Float by preference(PreferenceKeys.KEY_BORDER_WIDTH, DEFAULT_KEY_BORDER_WIDTH)

    var colorSettings: String by preference(PreferenceKeys.COLOR_SETTINGS, DEFAULT_COLOR_SETTINGS)

    // Theme settings (v3.6.2)
    var selectedThemeId: String by preference(PreferenceKeys.SELECTED_THEME_ID, DEFAULT_SELECTED_THEME_ID)

    var userThemes: String by preference(PreferenceKeys.USER_THEMES, DEFAULT_USER_THEMES)

    /** Loads the persisted user themes. */
    fun loadUserThemes(): List<UserTheme> = UserTheme.decodeList(userThemes)

    /**
     * The legacy free-pick appearance = the current global color settings + the
     * five size scalars (flat shadow). This is the `"default"` theme's appearance;
     * existing customized users keep their look here with no migration.
     */
    val legacyAppearance: ThemeAppearance
        get() =
            ThemeAppearance(
                colors = KeyboardColorSettings.fromJson(colorSettings),
                keyShadowIntensity = ThemeAppearance.DEFAULT_KEY_SHADOW_INTENSITY,
                keyHeightScale = keyHeightScale,
                keyFontSizeScale = keyFontSizeScale,
                candidateTextSizeScale = candidateTextSizeScale,
                keyCornerRadius = keyCornerRadius,
                keyBorderWidth = keyBorderWidth,
            )

    /**
     * Resolves the active theme into the appearance the renderer consumes.
     *
     * Convenience form — re-parses the colorSettings + userThemes JSON on every
     * call. NOT for the per-keystroke render path: P2 routes that through
     * KeyboardAppearanceResolver's string-equality parse cache instead.
     */
    fun resolvedAppearance(isDark: Boolean): ThemeAppearance =
        ThemeResolver.resolved(selectedThemeId, isDark, legacyAppearance, loadUserThemes())

    // 異用字開關（預設關閉）
    var variantEnabled: Boolean by preference(PreferenceKeys.VARIANT_DICT_ENABLED, false)

    // 在來字開關（預設關閉）
    var khiin: Boolean by preference(PreferenceKeys.KHIIN_ENABLED, false)

    // ------------------------------------------------------------------ //
    // EngineSettings / EngineSettingsProvider conformance
    //
    // Bridge properties that rename Android-side preferences to the
    // iOS-aligned `is*` naming expected by engine-facing code. Each getter
    // delegates to the existing Android property so live-read semantics
    // (re-read `cachedPrefs` per access) are inherited without duplicate
    // logic. See `ime/core/settings/EngineSettings.kt` for the contract.
    // ------------------------------------------------------------------ //

    override val isAutoCap: Boolean
        get() = autoCapitalizationEnabled

    // v3.5.8 §10.2: engine-facing alias for the Android `outputBothScripts`
    // pref (kept un-renamed because the settings UI / smartbar read it
    // directly). The continuous word-boundary-spacing predicate consumes
    // this via EngineSettings.
    override val isOutputBothScripts: Boolean
        get() = outputBothScripts

    // §34/S22: engine-facing alias for the Android `literalRomanCandidateEnabled`
    // pref (kept un-renamed because the settings UI reads it directly).
    // `ComposingManager` inverts it into `FetchAtPos.literalRomanCandidateDisabled`.
    override val isLiteralRomanCandidateEnabled: Boolean
        get() = literalRomanCandidateEnabled

    override val isAssociationRecordingEnabled: Boolean
        get() = associationRecordingEnabled

    override val toneToggles: ToneToggles
        get() = ToneToggles(enableDoubleTapOO, enableDoubleTapNN)

    override val isCustomDictEnabled: Boolean
        get() = customDictEnabled

    override val isTpsOrMappedToER: Boolean
        get() = tpsOrMapsToER

    override val isMoeDictEnabled: Boolean
        get() = moeDictEnabled

    override val isNewwordDictEnabled: Boolean
        get() = newwordDictEnabled

    override val isKunggeDictEnabled: Boolean
        get() = kunggeDictEnabled

    override val isITaigiDictEnabled: Boolean
        get() = itaigiDictEnabled

    override val isTaiwanJapanDictEnabled: Boolean
        get() = taiwanJapanDictEnabled

    override val isTaiHuaDictEnabled: Boolean
        get() = taiHuaDictEnabled

    override val isTaiwanPlantDictEnabled: Boolean
        get() = taiwanPlantDictEnabled

    override val isSttiDictEnabled: Boolean
        get() = sttiDictEnabled

    override val isKhpooDictEnabled: Boolean
        get() = khpooDictEnabled

    override val isVariantEnabled: Boolean
        get() = variantEnabled

    override val isKhiinEnabled: Boolean
        get() = khiin

    override val isLkkDictEnabled: Boolean
        get() = lkkDictEnabled

    override val isDevDictEnabled: Boolean
        get() = devDictEnabled

    // kautian subcollections (nested under the MOE master, default all on per DD5)
    override val isKautianAccentLukangEnabled: Boolean
        get() = kautianAccentLukangEnabled

    override val isKautianAccentSansiaEnabled: Boolean
        get() = kautianAccentSansiaEnabled

    override val isKautianAccentTaipakEnabled: Boolean
        get() = kautianAccentTaipakEnabled

    override val isKautianAccentGilanEnabled: Boolean
        get() = kautianAccentGilanEnabled

    override val isKautianAccentTainanEnabled: Boolean
        get() = kautianAccentTainanEnabled

    override val isKautianAccentKaohsiungEnabled: Boolean
        get() = kautianAccentKaohsiungEnabled

    override val isKautianAccentKinmenEnabled: Boolean
        get() = kautianAccentKinmenEnabled

    override val isKautianAccentMakungEnabled: Boolean
        get() = kautianAccentMakungEnabled

    override val isKautianAccentSintikEnabled: Boolean
        get() = kautianAccentSintikEnabled

    override val isKautianAccentTaichungEnabled: Boolean
        get() = kautianAccentTaichungEnabled

    override val isKautianNameAppendixEnabled: Boolean
        get() = kautianNameAppendixEnabled

    /**
     * Returns `this` as [EngineSettings]. Each property access on the
     * returned value re-reads the DataStore cache, so the engine always
     * sees the most recent values — critical for settings-screen changes
     * to propagate without rebuilding the composition root.
     */
    override val current: EngineSettings
        get() = this

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
     *
     * Writes DataStore directly (bypassing `updateCacheAndPersist`), so finishes
     * by calling [clearPendingOverlay] — see that helper for rationale.
     */
    suspend fun migrateFromSharedPreferences() {
        val sharedPrefs: SharedPreferences = PreferenceManager.getDefaultSharedPreferences(context)
        val logger = CompositionRoot.shared(context).logger

        dataStore.edit { prefs ->
            // Only migrate if DataStore is empty
            if (prefs.asMap().isEmpty()) {
                logger.debug(TAG) { "Migrating from SharedPreferences to DataStore" }

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
                    sharedPrefs.getString("internal__version_on_install", AppVersionTracker.DEFAULT_VERSION_RAW)
                        ?: AppVersionTracker.DEFAULT_VERSION_RAW
                prefs[PreferenceKeys.VERSION_LAST_USE] =
                    sharedPrefs.getString("internal__version_last_use", AppVersionTracker.DEFAULT_VERSION_RAW)
                        ?: AppVersionTracker.DEFAULT_VERSION_RAW

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

                logger.debug(TAG) { "Migration completed successfully" }
            } else {
                logger.debug(TAG) { "DataStore already has data, skipping migration" }
            }
        }
        clearPendingOverlay()
    }

    /**
     * 重置所有設定為預設值
     * 保留內部設定（版本資訊）
     *
     * Direct-write counterpart to [migrateFromSharedPreferences]; finishes
     * by calling [clearPendingOverlay] for the same reason.
     */
    suspend fun resetToDefaults() {
        dataStore.edit { prefs ->
            // 保存需要保留的值
            val versionOnInstall = prefs[PreferenceKeys.VERSION_ON_INSTALL]
            val versionLastUse = prefs[PreferenceKeys.VERSION_LAST_USE]
            // User-created themes are user content (like the SQLite user DBs) — preserved across a full settings reset.
            val userThemes = prefs[PreferenceKeys.USER_THEMES]

            // 清除所有偏好設定
            prefs.clear()

            // 恢復需要保留的值
            versionOnInstall?.let { prefs[PreferenceKeys.VERSION_ON_INSTALL] = it }
            versionLastUse?.let { prefs[PreferenceKeys.VERSION_LAST_USE] = it }
            userThemes?.let { prefs[PreferenceKeys.USER_THEMES] = it }

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
            prefs[PreferenceKeys.SELECTED_THEME_ID] = DEFAULT_SELECTED_THEME_ID

            CompositionRoot.shared(context).logger.debug(TAG) { "All preferences reset to defaults" }
        }
        clearPendingOverlay()
    }
}
