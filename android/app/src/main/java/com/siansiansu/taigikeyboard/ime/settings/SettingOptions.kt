package com.siansiansu.taigikeyboard.ime.settings

// Value→label-key option lists shared by the app's settings screens and the in-keyboard settings
// overlay. Structural (no resolved strings); each list is resolved at render.

import com.siansiansu.taigikeyboard.i18n.generated.StringKey

// Mode→label-key pairs, used by CandidateDisplayModeScreen, InputSettingsScreen and the
// in-keyboard SettingsOverlayContent.
val candidateDisplayModeOptions: List<Pair<CandidateDisplayMode, StringKey>> =
    listOf(
        CandidateDisplayMode.SIDE_BY_SIDE to StringKey.SETTINGS_CANDIDATE_DISPLAY_MODE_SIDE_BY_SIDE,
        CandidateDisplayMode.COMBINED to StringKey.SETTINGS_CANDIDATE_DISPLAY_MODE_COMBINED,
        CandidateDisplayMode.ROMAN_ONLY to StringKey.SETTINGS_CANDIDATE_DISPLAY_MODE_ROMAN_ONLY,
    )

// Separator→label-key pairs, the hyphen (the default) first; used by SyllableSeparatorScreen, the
// InputSettingsScreen row and the in-keyboard SettingsOverlayContent.
val syllableSeparatorOptions: List<Pair<SyllableSeparator, StringKey>> =
    listOf(
        SyllableSeparator.HYPHEN to StringKey.DESKTOP_TELEX_GUIDE_HYPHEN,
        SyllableSeparator.SPACE to StringKey.SETTINGS_SYLLABLE_SEPARATOR_SPACE,
        SyllableSeparator.NONE to StringKey.SETTINGS_SYLLABLE_SEPARATOR_NONE,
    )

// Stored switch→label-key pairs, ᴺ (the default) first; used by NasalMarkerStyleScreen, the
// InputSettingsScreen row and the in-keyboard SettingsOverlayContent.
val nasalMarkerStyleOptions: List<Pair<Boolean, StringKey>> =
    listOf(
        true to StringKey.SETTINGS_NASAL_MARKER_UPPERCASE_CAPITAL,
        false to StringKey.SETTINGS_NASAL_MARKER_UPPERCASE_SMALL,
    )
