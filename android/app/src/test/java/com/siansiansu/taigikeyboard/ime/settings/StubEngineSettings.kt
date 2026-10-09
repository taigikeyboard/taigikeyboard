package com.siansiansu.taigikeyboard.ime.settings

// EngineSettings stand-in for JVM tests: every field is a plain `var`, so a test states exactly the
// snapshot a request runs under and can mutate it in place for live-read pins. Mirrors iOS `StubEngineSettings`.
internal data class StubEngineSettings(
    override var inputMode: String = "tl",
    override var candidateDisplayMode: CandidateDisplayMode = CandidateDisplayMode.SIDE_BY_SIDE,
    override var isHanjiFirst: Boolean = false,
    override var isOutputBothScripts: Boolean = false,
    override var isLiteralRomanCandidateEnabled: Boolean = true,
    override var syllableSeparator: SyllableSeparator = SyllableSeparator.HYPHEN,
    override var pojMarkerOptions: PojMarkerOptions =
        PojMarkerOptions(
            isDoubleTapOOEnabled = false,
            isDoubleTapNNEnabled = false,
            isNasalMarkerUppercaseEnabled = true,
        ),
    override var isCustomDictEnabled: Boolean = true,
    override var isTpsOrMappedToER: Boolean = false,
    override var isMoeDictEnabled: Boolean = true,
    override var isNewwordDictEnabled: Boolean = true,
    override var isKunggeDictEnabled: Boolean = true,
    override var isITaigiDictEnabled: Boolean = true,
    override var isTaiwanJapanDictEnabled: Boolean = true,
    override var isTaiHuaDictEnabled: Boolean = true,
    override var isTaiwanPlantDictEnabled: Boolean = true,
    override var isSttiDictEnabled: Boolean = true,
    override var isKhpooDictEnabled: Boolean = true,
    override var isVariantEnabled: Boolean = true,
    override var isKhiinEnabled: Boolean = true,
    override var isLkkDictEnabled: Boolean = true,
    override var isDevDictEnabled: Boolean = true,
    override var isKautianAccentLukangEnabled: Boolean = true,
    override var isKautianAccentSansiaEnabled: Boolean = true,
    override var isKautianAccentTaipakEnabled: Boolean = true,
    override var isKautianAccentGilanEnabled: Boolean = true,
    override var isKautianAccentTainanEnabled: Boolean = true,
    override var isKautianAccentKaohsiungEnabled: Boolean = true,
    override var isKautianAccentKinmenEnabled: Boolean = true,
    override var isKautianAccentMakungEnabled: Boolean = true,
    override var isKautianAccentSintikEnabled: Boolean = true,
    override var isKautianAccentTaichungEnabled: Boolean = true,
    override var isKautianNameAppendixEnabled: Boolean = true,
    override var isKautianAltReadingEnabled: Boolean = true,
) : EngineSettings

/** A provider whose `current` is the one stub it was built with. */
internal class StubEngineSettingsProvider(
    override val current: EngineSettings,
) : EngineSettingsProvider
