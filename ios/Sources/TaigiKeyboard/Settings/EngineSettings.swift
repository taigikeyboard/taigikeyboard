// 引擎層 (lexicon / composing / ranking) 需要的唯讀設定 protocol。
// Foundation-only,可作為 shared-core 擴出候選。實作端為 SharedSettings。

import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Read-only view of the settings the lexicon / input engine needs to
/// make decisions during candidate search, composing, and suggestion
/// ranking. Supplied to engine services via `EngineSettingsProvider`.
///
/// Kept Foundation-only (no UIKit, KeyboardKit, SwiftUI) so engine-layer
/// files can depend on this protocol without pulling platform frameworks.
// 引擎層讀取設定的 protocol。所有 dictionary 開關、輸入模式、自動大寫等都從這裡讀。
// 嚴禁 import UIKit / KeyboardKit / SwiftUI,以保留 shared-core 抽出可能性。
protocol EngineSettings {
    // 目前選用的輸入模式 (POJ / TL / English / TPS)。
    var inputMode: InputMode { get }
    // 是否啟用自動大寫 (來自 KeyboardKit 的 isAutocapitalizationEnabled)。
    var isAutoCap: Bool { get }
    // 翻譯方向是否反轉 (台↔英 切換)。
    var isTranslateSwapped: Bool { get }
    // 是否同時輸出漢字 + 羅馬字 (雙腳本)。連續輸入 §10.2 字界空格判斷需要它
    // 區分「漢字優先」(無空格) 與「雙腳本」(`hit (彼)` 要空格) — 兩者 isTranslateSwapped 都為 true。
    /// Output both hanji + roman ("both-scripts"). The continuous-input
    /// §10.2 word-boundary-spacing predicate needs this to tell
    /// hanji-first (no space) from both-scripts (`hit (彼)` — space
    /// wanted); `isTranslateSwapped` is `true` for both.
    // CROSS-PLATFORM INVARIANT — mirrors android/app/src/main/java/com/siansiansu/taigikeyboard/ime/core/settings/EngineSettings.kt:isOutputBothScripts.
    // Drift causes silent divergence (hanji-first spurious word-boundary spaces).
    var isOutputBothScripts: Bool { get }

    /// Candidate cell rendering mode (漢羅並排 / 羅馬字). Under `.romanOnly`
    /// the two flags above read `false` regardless of their stored values —
    /// they are derived, never overwritten — so auto-space, the commit
    /// formatter, and the engine `AppConfig` all take the roman-first arms.
    /// The bridge forwards this as `AppConfig.candidate_display_mode` so the
    /// engine collapses same-roman rows for display.
    // 候選詞顯示模式。羅馬字模式下 isTranslateSwapped / isOutputBothScripts 讀成 false(推導,不覆寫儲存值)。
    // CROSS-PLATFORM INVARIANT — mirrors android/app/src/main/java/com/siansiansu/taigikeyboard/ime/core/settings/EngineSettings.kt:candidateDisplayMode.
    // Drift causes silent divergence (one platform still shows hanji / swaps scripts under 羅馬字).
    var candidateDisplayMode: CandidateDisplayMode { get }
    // 是否記錄使用者選字的關聯資料,供 NextWord 推薦使用。
    var isAssociationRecordingEnabled: Bool { get }

    // 顯示當咧拍的字開關 (§34/S22)。TL/POJ 組字時是否在候選列首位顯示字面 roman
    // 候選 (= preedit WYSIWYG),讓漢羅一鍵上屏免切 文/A。預設 true。只關 §34
    // 強制 prepend,不影響 assemble_candidates 自然產生的 roman 候選。
    /// Literal-roman candidate toggle (§34/S22). When on (default), TL/POJ
    /// composing surfaces the preedit literal (`derived_display`) as the
    /// index-0 candidate so 漢羅 mixing commits the romanization in one tap.
    /// The bridge inverts this into `FetchAtPos.literal_roman_candidate_disabled`.
    // CROSS-PLATFORM INVARIANT — mirrors android/app/src/main/java/com/siansiansu/taigikeyboard/ime/core/settings/EngineSettings.kt:isLiteralRomanCandidateEnabled.
    // Drift causes silent divergence (one platform shows the §34 candidate, the other does not).
    var isLiteralRomanCandidateEnabled: Bool { get }

    /// POJ preprocessing toggles bundled as a live-read value so
    /// `ComposingState` / `ToneConverter` can stay Foundation-pure.
    // POJ 雙擊 OO / NN 預處理開關打包,讓 ComposingState / ToneConverter 不需直接讀設定。
    var toneToggles: ToneToggles { get }

    // 自訂詞庫開關。
    var isCustomDictEnabled: Bool { get }
    // TPS 注音中 "or" 是否映射到ㄜ;false 時映射到ㄛ。
    var isTpsOrMappedToER: Bool { get }

    // 教育部詞典 (MOE) 開關。
    var isMoeDictEnabled: Bool { get }
    // 新詞詞典開關。
    var isNewwordDictEnabled: Bool { get }
    // 公語詞典 (kungge) 開關。
    var isKunggeDictEnabled: Bool { get }
    // iTaigi 詞典開關。
    var isITaigiDictEnabled: Bool { get }
    // 台日大辭典開關。
    var isTaiwanJapanDictEnabled: Bool { get }
    // 台華對照辭典開關。
    var isTaiHuaDictEnabled: Bool { get }
    // 台灣植物名彙開關。
    var isTaiwanPlantDictEnabled: Bool { get }
    // 教育部臺灣台語常用詞辭典 (sttj/STTI) 開關。
    var isSttiDictEnabled: Bool { get }
    // 教育部閩南語推薦用字 (khpoo) 開關。
    var isKhpooDictEnabled: Bool { get }
    // 異體字候選開關。
    var isVariantEnabled: Bool { get }
    // Khiin 補充資料開關。
    var isKhiinEnabled: Bool { get }
    // LKK 漢羅混寫候選開關。
    var isLkkDictEnabled: Bool { get }
    // 開發者補充辭典 (詞庫增補檔案) 開關。
    var isDevDictEnabled: Bool { get }

    // kautian subcollection toggles (nested under the kautian master). Order
    // mirrors config.yaml dialect_columns; the bridge packs these into the
    // KautianSubcollToggles proto and `compute_filters` owns the subtag bit
    // layout. Absent gate ⇒ engine keeps legacy all-on (DD5).
    // kautian subcollection 子開關 — 10 腔調 + 姓名附錄。bridge 打包成 proto,bit 佈局由 Rust compute_filters 持有。
    var isKautianAccentLukangEnabled: Bool { get }
    var isKautianAccentSansiaEnabled: Bool { get }
    var isKautianAccentTaipakEnabled: Bool { get }
    var isKautianAccentGilanEnabled: Bool { get }
    var isKautianAccentTainanEnabled: Bool { get }
    var isKautianAccentKaohsiungEnabled: Bool { get }
    var isKautianAccentKinmenEnabled: Bool { get }
    var isKautianAccentMakungEnabled: Bool { get }
    var isKautianAccentSintikEnabled: Bool { get }
    var isKautianAccentTaichungEnabled: Bool { get }
    var isKautianNameAppendixEnabled: Bool { get }
}
