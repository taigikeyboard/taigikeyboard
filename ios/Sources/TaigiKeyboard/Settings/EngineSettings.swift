// 中文: 引擎層 (lexicon / composing / ranking) 需要的唯讀設定 protocol。
// 中文: Foundation-only,可作為 shared-core 擴出候選。實作端為 SharedSettings。

import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Read-only view of the settings the lexicon / input engine needs to
/// make decisions during candidate search, composing, and suggestion
/// ranking. Supplied to engine services via `EngineSettingsProvider`.
///
/// Kept Foundation-only (no UIKit, KeyboardKit, SwiftUI) so engine-layer
/// files can depend on this protocol without pulling platform frameworks.
// 中文: 引擎層讀取設定的 protocol。所有 dictionary 開關、輸入模式、自動大寫等都從這裡讀。
// 中文: 嚴禁 import UIKit / KeyboardKit / SwiftUI,以保留 shared-core 抽出可能性。
protocol EngineSettings {
    // 中文: 目前選用的輸入模式 (POJ / TL / English / TPS)。
    var inputMode: InputMode { get }
    // 中文: 是否啟用自動大寫 (來自 KeyboardKit 的 isAutocapitalizationEnabled)。
    var isAutoCap: Bool { get }
    // 中文: 翻譯方向是否反轉 (台↔英 切換)。
    var isTranslateSwapped: Bool { get }
    // 中文: 是否記錄使用者選字的關聯資料,供 NextWord 推薦使用。
    var isAssociationRecordingEnabled: Bool { get }

    /// POJ preprocessing toggles bundled as a live-read value so
    /// `ComposingState` / `ToneConverter` can stay Foundation-pure.
    // 中文: POJ 雙擊 OO / NN 預處理開關打包,讓 ComposingState / ToneConverter 不需直接讀設定。
    var toneToggles: ToneToggles { get }

    // 中文: 自訂詞庫開關。
    var isCustomDictEnabled: Bool { get }
    // 中文: TPS 注音中 "or" 是否映射到ㄜ;false 時映射到ㄛ。
    var isTpsOrMappedToER: Bool { get }

    // 中文: 教育部詞典 (MOE) 開關。
    var isMoeDictEnabled: Bool { get }
    // 中文: 新詞詞典開關。
    var isNewwordDictEnabled: Bool { get }
    // 中文: 公語詞典 (kungge) 開關。
    var isKunggeDictEnabled: Bool { get }
    // 中文: iTaigi 詞典開關。
    var isITaigiDictEnabled: Bool { get }
    // 中文: 台日大辭典開關。
    var isTaiwanJapanDictEnabled: Bool { get }
    // 中文: 台華對照辭典開關。
    var isTaiHuaDictEnabled: Bool { get }
    // 中文: 台灣植物名彙開關。
    var isTaiwanPlantDictEnabled: Bool { get }
    // 中文: 教育部臺灣台語常用詞辭典 (sttj/STTI) 開關。
    var isSttiDictEnabled: Bool { get }
    // 中文: 教育部閩南語推薦用字 (khpoo) 開關。
    var isKhpooDictEnabled: Bool { get }
    // 中文: 異體字候選開關。
    var isVariantEnabled: Bool { get }
    // 中文: Khiin 補充資料開關。
    var isKhiinEnabled: Bool { get }
    // 中文: LKK 漢羅混寫候選開關。
    var isLkkDictEnabled: Bool { get }
}
