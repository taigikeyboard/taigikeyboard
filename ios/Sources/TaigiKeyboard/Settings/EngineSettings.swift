import Foundation

/// Read-only view of the settings the lexicon / input engine needs to
/// make decisions during candidate search, composing, and suggestion
/// ranking. Supplied to engine services via `EngineSettingsProvider`.
///
/// Kept Foundation-only (no UIKit, KeyboardKit, SwiftUI) so engine-layer
/// files can depend on this protocol without pulling platform frameworks.
protocol EngineSettings {
    var inputMode: InputMode { get }
    var isAutoCap: Bool { get }
    var isTranslateSwapped: Bool { get }
    var isAssociationRecordingEnabled: Bool { get }

    var isCustomDictEnabled: Bool { get }
    var isTpsOrMappedToER: Bool { get }

    var isMoeDictEnabled: Bool { get }
    var isNewwordDictEnabled: Bool { get }
    var isKunggeDictEnabled: Bool { get }
    var isITaigiDictEnabled: Bool { get }
    var isTaiwanJapanDictEnabled: Bool { get }
    var isTaiHuaDictEnabled: Bool { get }
    var isTaiwanPlantDictEnabled: Bool { get }
    var isSttiDictEnabled: Bool { get }
    var isKhpooDictEnabled: Bool { get }
    var isVariantEnabled: Bool { get }
    var isKhiinEnabled: Bool { get }
    var isLkkDictEnabled: Bool { get }
}
