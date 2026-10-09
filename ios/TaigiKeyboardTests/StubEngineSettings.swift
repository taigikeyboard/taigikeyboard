@testable import TaigiKeyboard

// Fixed-value `EngineSettings` for bridge / manager tests — every field is a plain
// stored property so a test states exactly the snapshot a request is rendered under.
struct StubEngineSettings: EngineSettings {
    var inputMode: InputMode = .tl
    var isHanjiFirst = false
    var isOutputBothScripts = false
    var candidateDisplayMode: CandidateDisplayMode = .sideBySide
    var isLiteralRomanCandidateEnabled = true
    var syllableSeparator: SyllableSeparator = .hyphen
    var pojMarkerOptions = PojMarkerOptions(
        isDoubleTapOOEnabled: false,
        isDoubleTapNNEnabled: false,
        isNasalMarkerUppercaseEnabled: true,
    )
    var isCustomDictEnabled = true
    var isTpsOrMappedToER = false
    var isMoeDictEnabled = true
    var isNewwordDictEnabled = true
    var isKunggeDictEnabled = true
    var isITaigiDictEnabled = true
    var isTaiwanJapanDictEnabled = true
    var isTaiHuaDictEnabled = true
    var isTaiwanPlantDictEnabled = true
    var isSttiDictEnabled = true
    var isKhpooDictEnabled = true
    var isVariantEnabled = true
    var isKhiinEnabled = true
    var isLkkDictEnabled = true
    var isDevDictEnabled = true
    var isKautianAccentLukangEnabled = true
    var isKautianAccentSansiaEnabled = true
    var isKautianAccentTaipakEnabled = true
    var isKautianAccentGilanEnabled = true
    var isKautianAccentTainanEnabled = true
    var isKautianAccentKaohsiungEnabled = true
    var isKautianAccentKinmenEnabled = true
    var isKautianAccentMakungEnabled = true
    var isKautianAccentSintikEnabled = true
    var isKautianAccentTaichungEnabled = true
    var isKautianNameAppendixEnabled = true
    var isKautianAltReadingEnabled = true
}

/// A provider whose `current` is the one stub it was built with. Mirrors Android
/// `StubEngineSettingsProvider`.
final class StubEngineSettingsProvider: EngineSettingsProvider {
    let current: EngineSettings

    init(_ settings: EngineSettings) {
        current = settings
    }
}
