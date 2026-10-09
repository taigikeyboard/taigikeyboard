package com.siansiansu.taigikeyboard.ime.settings

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.floatPreferencesKey
import androidx.datastore.preferences.core.intPreferencesKey
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore

/**
 * DataStore instance for TaigiKeyboard preferences.
 * Singleton pattern using extension property.
 */
val Context.preferencesDataStore: DataStore<Preferences> by preferencesDataStore(
    name = "taigi_keyboard_prefs",
)

/**
 * Preference keys for DataStore.
 * Centralized key definitions to avoid string duplication.
 */
object PreferenceKeys {
    /**
     * Keys retired 2026-09-30. No settings UI in this repo ever wrote them —
     * only the SharedPreferences migration (pre-repo values), the layout-type
     * cascade (the paired phah flag) and the subtype manager — so
     * [PrefHelper.migrateFromSharedPreferences] removes them on every start:
     * a stale value must not shadow today's default. Spellings stay reserved.
     */
    val RETIRED: List<Preferences.Key<*>> =
        listOf(
            stringPreferencesKey("advanced__settings_theme"),
            booleanPreferencesKey("advanced__show_app_icon"),
            booleanPreferencesKey("correction__double_space_period"),
            intPreferencesKey("keyboard__active_subtype_id"),
            stringPreferencesKey("keyboard__subtypes"),
            booleanPreferencesKey("keyboard__phah_taigi_layout_enabled"),
            stringPreferencesKey("looknfeel__height_factor"),
            intPreferencesKey("looknfeel__long_press_delay"),
        )

    /** Set once the launcher alias has been restored to its manifest default (see `TaigiKeyboardApplication`). */
    val LAUNCHER_ALIAS_RESTORED = booleanPreferencesKey("internal__launcher_alias_restored")

    // App UI display language (i18n). Tag of DisplayLanguage; default "system" (Automatic) = follow device OS locale.
    val DISPLAY_LANGUAGE = stringPreferencesKey("app__display_language")

    // Internal settings
    val VERSION_ON_INSTALL = stringPreferencesKey("internal__version_on_install")
    val VERSION_LAST_USE = stringPreferencesKey("internal__version_last_use")

    // Keyboard settings
    val INPUT_MODE = stringPreferencesKey("keyboard__input_mode")
    val IS_HANJI_FIRST = booleanPreferencesKey("keyboard__is_translate_swapped")
    val OUTPUT_BOTH_SCRIPTS = booleanPreferencesKey("keyboard__output_both_scripts")

    // Raw value = CandidateDisplayMode.storageValue ("sideBySide" / "romanOnly" / "combined").
    val CANDIDATE_DISPLAY_MODE = stringPreferencesKey("keyboard__candidate_display_mode")
    val LITERAL_ROMAN_CANDIDATE = booleanPreferencesKey("keyboard__literal_roman_candidate")

    // Raw value = SyllableSeparator.storageValue ("hyphen" / "space" / "none").
    val SYLLABLE_SEPARATOR = stringPreferencesKey("keyboard__syllable_separator")

    // Retired 2026-10-06: the No Hyphens switch, carried over by [carryOverHyphenlessRoman].
    val RETIRED_HYPHENLESS_ROMAN = booleanPreferencesKey("keyboard__hyphenless_roman")
    val KEYBOARD_LAYOUT_TYPE = stringPreferencesKey("keyboard__layout_type")
    val INPUT_MODE_BEFORE_TPS = stringPreferencesKey("keyboard__input_mode_before_tps")
    val LAYOUT_BEFORE_TPS = stringPreferencesKey("keyboard__layout_before_tps")
    val TOOLBAR_AUTO_COLLAPSE = booleanPreferencesKey("keyboard__toolbar_auto_collapse")

    // Raw values = OneHandedMode / KeyboardToolbarAction storageValue (shared with iOS).
    val ONE_HANDED_MODE = stringPreferencesKey("keyboard__one_handed_mode")
    val KEYBOARD_TOOLBAR_ACTION = stringPreferencesKey("keyboard__toolbar_keyboard_action")
    val GLOBE_KEY_ENABLED = booleanPreferencesKey("keyboard__globe_key_enabled")
    val SOUND_FEEDBACK_ENABLED = booleanPreferencesKey("keyboard__sound_feedback_enabled")
    val VIBRATION_FEEDBACK_ENABLED = booleanPreferencesKey("keyboard__vibration_feedback_enabled")

    // Taigi-specific keys
    val ENABLE_DOUBLE_TAP_OO = booleanPreferencesKey("taigi__enable_double_tap_oo")
    val ENABLE_DOUBLE_TAP_NN = booleanPreferencesKey("taigi__enable_double_tap_nn")
    val NASAL_MARKER_UPPERCASE = booleanPreferencesKey("taigi__nasal_marker_uppercase")
    val AUTO_CAPITALIZATION_ENABLED = booleanPreferencesKey("taigi__auto_capitalization_enabled")
    val AUTO_SPACE_ENABLED = booleanPreferencesKey("taigi__auto_space_enabled")
    val FONT_TYPE = stringPreferencesKey("taigi__font_type")

    // Dictionary toggles. VARIANT = Variant Characters, KHIIN = Conventional Characters,
    // LKK = LKK漢羅合用建議用字, DEV = 開發者補充辭典.
    val CUSTOM_DICT_ENABLED = booleanPreferencesKey("dictionary__custom_dict_enabled")
    val MOE_DICT_ENABLED = booleanPreferencesKey("dictionary__moe_dict_enabled")
    val NEWWORD_DICT_ENABLED = booleanPreferencesKey("dictionary__newword_dict_enabled")
    val ITAIGI_DICT_ENABLED = booleanPreferencesKey("dictionary__itaigi_dict_enabled")
    val SITBUT_DICT_ENABLED = booleanPreferencesKey("dictionary__sitbut_dict_enabled")
    val TAIHOA_DICT_ENABLED = booleanPreferencesKey("dictionary__taihoa_dict_enabled")
    val TAIJIT_DICT_ENABLED = booleanPreferencesKey("dictionary__taijit_dict_enabled")
    val KUNGGE_DICT_ENABLED = booleanPreferencesKey("dictionary__kungge_dict_enabled")
    val STTI_DICT_ENABLED = booleanPreferencesKey("dictionary__stti_dict_enabled")
    val KHPOO_DICT_ENABLED = booleanPreferencesKey("dictionary__khpoo_dict_enabled")
    val VARIANT_DICT_ENABLED = booleanPreferencesKey("dictionary__variant_enabled")
    val KHIIN_ENABLED = booleanPreferencesKey("dictionary__khiin_enabled")
    val LKK_DICT_ENABLED = booleanPreferencesKey("dictionary__lkk_dict_enabled")
    val DEV_DICT_ENABLED = booleanPreferencesKey("dictionary__dev_dict_enabled")

    // MOE subsets (accents + name appendix), nested under the MOE master; both default ON (opt-out).
    // Accent order matches config.yaml dialect_columns (= subtag bit - 1); bit layout lives in Rust compute_filters.
    val KAUTIAN_ACCENT_LUKANG_ENABLED = booleanPreferencesKey("dictionary__kautian_accent_lukang_enabled")
    val KAUTIAN_ACCENT_SANSIA_ENABLED = booleanPreferencesKey("dictionary__kautian_accent_sansia_enabled")
    val KAUTIAN_ACCENT_TAIPAK_ENABLED = booleanPreferencesKey("dictionary__kautian_accent_taipak_enabled")
    val KAUTIAN_ACCENT_GILAN_ENABLED = booleanPreferencesKey("dictionary__kautian_accent_gilan_enabled")
    val KAUTIAN_ACCENT_TAINAN_ENABLED = booleanPreferencesKey("dictionary__kautian_accent_tainan_enabled")
    val KAUTIAN_ACCENT_KAOHSIUNG_ENABLED = booleanPreferencesKey("dictionary__kautian_accent_kaohsiung_enabled")
    val KAUTIAN_ACCENT_KINMEN_ENABLED = booleanPreferencesKey("dictionary__kautian_accent_kinmen_enabled")
    val KAUTIAN_ACCENT_MAKUNG_ENABLED = booleanPreferencesKey("dictionary__kautian_accent_makung_enabled")
    val KAUTIAN_ACCENT_SINTIK_ENABLED = booleanPreferencesKey("dictionary__kautian_accent_sintik_enabled")
    val KAUTIAN_ACCENT_TAICHUNG_ENABLED = booleanPreferencesKey("dictionary__kautian_accent_taichung_enabled")
    val KAUTIAN_NAME_APPENDIX_ENABLED = booleanPreferencesKey("dictionary__kautian_name_appendix_enabled")
    val KAUTIAN_ALT_READING_ENABLED = booleanPreferencesKey("dictionary__kautian_alt_reading_enabled")

    // TPS settings
    val TPS_OR_MAPS_TO_ER = booleanPreferencesKey("tps__or_maps_to_er")

    // Retired: the pre-theme global appearance, written by the appearance screen the theme editor
    // replaced. [LegacyAppearance.retire] carries a customized one into a user theme, then removes them.
    val KEY_HEIGHT_SCALE = floatPreferencesKey("appearance__key_height_scale")
    val KEY_FONT_SIZE_SCALE = floatPreferencesKey("appearance__key_font_size_scale")
    val CANDIDATE_TEXT_SIZE_SCALE = floatPreferencesKey("appearance__candidate_text_size_scale")
    val KEY_CORNER_RADIUS = floatPreferencesKey("appearance__key_corner_radius")
    val KEY_BORDER_WIDTH = floatPreferencesKey("appearance__key_border_width")
    val COLOR_SETTINGS = stringPreferencesKey("appearance__color_settings")

    // Theme settings (v3.6.2)
    val SELECTED_THEME_ID = stringPreferencesKey("appearance__selected_theme_id")
    val USER_THEMES = stringPreferencesKey("appearance__user_themes")
}
