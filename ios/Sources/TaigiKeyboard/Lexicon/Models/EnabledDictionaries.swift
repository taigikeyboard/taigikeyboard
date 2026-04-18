import Foundation

// NOTE: Not yet shared-core — mechanically Foundation-only, but the bitmask
// layout and `enabledSources` enumeration are tightly coupled to
// `DictionarySource` and the iOS `dictionary.bin` binary format. Android
// still computes enabled sources inline in its VM; extraction is blocked on
// Android aligning to this centralised form (tracked in Phase 10 follow-ups).
///
/// Previously `EnabledDictionaries.fromSettings()` reached into
/// `SharedSettings.shared`; this form accepts any `EngineSettings`
/// provider so callers (engine services with an injected provider,
/// tests with a stub) fully own the source of truth.
struct EnabledDictionaries {
    let kautian: Bool // 教育部臺灣台語常用詞辭典
    let taigitv: Bool // 台語新詞辭庫
    let kungge: Bool // 台語工藝詞庫
    let itaigi: Bool // iTaigi 華台對照典
    let taijit: Bool // 台日大辭典
    let taihoa: Bool // 台華線頂對照典
    let sitbut: Bool // 台灣植物名彙
    let stti: Bool // 學科術語辭典
    let khpoo: Bool // 腔口補充資料
    let variant: Bool // 異用字
    let khiin: Bool // 在來字
    let lkk: Bool // LKK漢羅合用建議用字

    /// Build from an `EngineSettings` snapshot.
    init(from settings: EngineSettings) {
        kautian = settings.isMoeDictEnabled
        taigitv = settings.isNewwordDictEnabled
        kungge = settings.isKunggeDictEnabled
        itaigi = settings.isITaigiDictEnabled
        taijit = settings.isTaiwanJapanDictEnabled
        taihoa = settings.isTaiHuaDictEnabled
        sitbut = settings.isTaiwanPlantDictEnabled
        stti = settings.isSttiDictEnabled
        khpoo = settings.isKhpooDictEnabled
        variant = settings.isVariantEnabled
        khiin = settings.isKhiinEnabled
        lkk = settings.isLkkDictEnabled
    }

    /// 是否全部開啟（9 個主要來源 + lkk）
    var allEnabled: Bool {
        kautian && taigitv && kungge && itaigi && taijit && taihoa && sitbut && stti && khpoo && lkk
    }

    /// 轉換為 dictionary bitmask（bits 0-11）
    /// Bit layout 必須與 dictionary.bin 一致
    func sourceBitmask() -> UInt16 {
        var mask: UInt16 = 0
        if kautian { mask |= 1 << 0 }
        if taigitv { mask |= 1 << 1 }
        if itaigi { mask |= 1 << 2 }
        if sitbut { mask |= 1 << 3 }
        if taihoa { mask |= 1 << 4 }
        if taijit { mask |= 1 << 5 }
        if kungge { mask |= 1 << 6 }
        if stti { mask |= 1 << 7 }
        if khpoo { mask |= 1 << 8 }
        // khiin = bit 9 (handled separately in filter)
        // dev = bit 10 (always included)
        if lkk { mask |= 1 << 11 }
        return mask
    }

    /// 轉換為 association bitmask（bits 0-8，對應 association.bin 的 9 個來源）
    func associationBitmask() -> UInt16 {
        sourceBitmask() & 0x1FF
    }

    /// association 的 9 個來源是否全部開啟
    var allAssociationSourcesEnabled: Bool {
        kautian && taigitv && itaigi && sitbut && taihoa && taijit && kungge && stti && khpoo
    }

    /// Enabled sources as a `Set<DictionarySource>`, suitable for filtering
    /// `DictionarySearchResult.sources` badges. `.dev` and `.custom` are always
    /// included (non-toggleable); `.variant` is not a `DictionarySource` case.
    var enabledSources: Set<DictionarySource> {
        var set: Set<DictionarySource> = [.dev, .custom]
        if kautian { set.insert(.kautian) }
        if taigitv { set.insert(.taigitv) }
        if kungge { set.insert(.kungge) }
        if itaigi { set.insert(.itaigi) }
        if taijit { set.insert(.taijit) }
        if taihoa { set.insert(.taihoa) }
        if sitbut { set.insert(.sitbut) }
        if stti { set.insert(.stti) }
        if khpoo { set.insert(.khpoo) }
        if khiin { set.insert(.khiin) }
        if lkk { set.insert(.lkk) }
        return set
    }
}
