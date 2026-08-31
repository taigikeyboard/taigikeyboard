//! 詞庫來源: which dictionaries the engine draws from, in three sections —
//! 教育部, the others, the supplements — with 教典's eleven subcollections
//! always visible under it, greyed while 教典 is off. Port of
//! `DictionaryTogglesView.swift`. Every toggle is read live by the engine
//! bridge on the next fetch.

// 中文: 詞庫來源頁 — 三個區段的開關;教典子集恆展開在教典下方,教典關閉時停用;恢復預設。

use crate::winui::cards;
use crate::winui::window::{Message, ResetScope, SettingsWindow};
use taigi_windows_core::settings::{keys, SettingsKey};
use taigi_windows_core::strings::{StringKey, StringResolver};
use windows_reactor::*;

/// The 教典 subcollections, in `DictionaryTogglesView`'s order.
const KAUTIAN_SUBCOLLECTIONS: [(SettingsKey<bool>, StringKey); 11] = [
    (
        keys::IS_KAUTIAN_ACCENT_LUKANG_ENABLED,
        StringKey::DictionaryKautianAccentLukang,
    ),
    (
        keys::IS_KAUTIAN_ACCENT_SANSIA_ENABLED,
        StringKey::DictionaryKautianAccentSansia,
    ),
    (
        keys::IS_KAUTIAN_ACCENT_TAIPAK_ENABLED,
        StringKey::DictionaryKautianAccentTaipak,
    ),
    (
        keys::IS_KAUTIAN_ACCENT_GILAN_ENABLED,
        StringKey::DictionaryKautianAccentGilan,
    ),
    (
        keys::IS_KAUTIAN_ACCENT_TAINAN_ENABLED,
        StringKey::DictionaryKautianAccentTainan,
    ),
    (
        keys::IS_KAUTIAN_ACCENT_KAOHSIUNG_ENABLED,
        StringKey::DictionaryKautianAccentKaohsiung,
    ),
    (
        keys::IS_KAUTIAN_ACCENT_KINMEN_ENABLED,
        StringKey::DictionaryKautianAccentKinmen,
    ),
    (
        keys::IS_KAUTIAN_ACCENT_MAKUNG_ENABLED,
        StringKey::DictionaryKautianAccentMakung,
    ),
    (
        keys::IS_KAUTIAN_ACCENT_SINTIK_ENABLED,
        StringKey::DictionaryKautianAccentSintik,
    ),
    (
        keys::IS_KAUTIAN_ACCENT_TAICHUNG_ENABLED,
        StringKey::DictionaryKautianAccentTaichung,
    ),
    (
        keys::IS_KAUTIAN_NAME_APPENDIX_ENABLED,
        StringKey::DictionaryKautianNameAppendix,
    ),
];

const MOE_OTHERS: [(SettingsKey<bool>, StringKey); 3] = [
    (keys::IS_TAIGITV_ENABLED, StringKey::CommonNewwordDict),
    (keys::IS_KUNGGE_ENABLED, StringKey::CommonKunggeDict),
    (keys::IS_STTI_ENABLED, StringKey::CommonSttiDict),
];

const OTHERS: [(SettingsKey<bool>, StringKey); 4] = [
    (keys::IS_ITAIGI_ENABLED, StringKey::CommonITaigiDict),
    (keys::IS_TAIJIT_ENABLED, StringKey::CommonTaiwanJapanDict),
    (keys::IS_TAIHOA_ENABLED, StringKey::CommonTaiHuaDict),
    (keys::IS_SITBUT_ENABLED, StringKey::CommonTaiwanPlantDict),
];

const SUPPLEMENTS: [(SettingsKey<bool>, StringKey); 5] = [
    (
        keys::IS_VARIANT_ENABLED,
        StringKey::DictionaryVariantDictionary,
    ),
    (keys::IS_KHIIN_ENABLED, StringKey::DictionaryKhiin),
    (keys::IS_KHPOO_ENABLED, StringKey::CommonAccentDict),
    (keys::IS_LKK_ENABLED, StringKey::DictionaryLkkDict),
    (keys::IS_DEV_ENABLED, StringKey::DictionaryDevSupplementDict),
];

pub fn view(
    window: &SettingsWindow,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
) -> View {
    let is_kautian_enabled = window.document().bool(&keys::IS_KAUTIAN_ENABLED);
    View::fragment((
        cards::section_title(strings.resolve(StringKey::DictionaryMoeSectionTitle)),
        // 教典 and its eleven subcollections are ONE always-open card: the
        // parent source is the group's first line, the subcollections step
        // in under it and stay browsable — they are read far more often
        // than they are changed, and a collapsed group hides which 腔口 the
        // engine is currently drawing from. Why this is a card and not an
        // `Expander` that starts open is written out on `cards::group`.
        // Disabled, not cleared, while 教典 is off: the choices come back
        // with it. `DictionaryTogglesView.swift` indents the same eleven
        // under the same master toggle.
        cards::group(
            cards::line(
                strings.resolve(StringKey::CommonMoeDict),
                cards::switch(
                    is_kautian_enabled,
                    true,
                    context.callback(|is_on| Message::SetSwitch(keys::IS_KAUTIAN_ENABLED, is_on)),
                ),
            ),
            KAUTIAN_SUBCOLLECTIONS.map(|(key, label)| {
                (
                    key.name,
                    cards::sub_row(
                        strings.resolve(label),
                        is_kautian_enabled,
                        cards::switch(
                            window.document().bool(&key),
                            is_kautian_enabled,
                            context.callback(move |is_on| Message::SetSwitch(key, is_on)),
                        ),
                    ),
                )
            }),
        ),
        source_rows(window, strings, context, &MOE_OTHERS),
        cards::section_title(strings.resolve(StringKey::DictionaryOtherSectionTitle)),
        source_rows(window, strings, context, &OTHERS),
        cards::section_title(strings.resolve(StringKey::DictionarySupplementSectionTitle)),
        source_rows(window, strings, context, &SUPPLEMENTS),
        cards::section_gap(),
        cards::action_row(
            strings.resolve(StringKey::ThemeEditorResetAll),
            strings.resolve(StringKey::SettingsReset),
            false,
            true,
            context.callback(|()| Message::Reset(ResetScope::DictionarySources)),
        ),
    ))
}

/// One card per source, each bound to its own settings key.
fn source_rows(
    window: &SettingsWindow,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
    sources: &'static [(SettingsKey<bool>, StringKey)],
) -> View {
    View::keyed_fragment(sources.iter().map(|(key, label)| {
        let key = *key;
        (
            key.name,
            cards::switch_row(
                strings.resolve(*label),
                window.document().bool(&key),
                true,
                context.callback(move |is_on| Message::SetSwitch(key, is_on)),
            ),
        )
    }))
}
