// Which dictionaries the user has switched on, as one snapshot value.

import Foundation

/// The user's dictionary source preferences, in the shape the engine's
/// `compute_filters` op reads them.
///
/// Field order mirrors `engine/protos/proto/lexicon.proto::DictionaryToggles`
/// (`:438-455`) so the bridge's assignment reads as a straight transcription.
/// The 11 kautian subcollection flags are nested rather than flat because the
/// wire has them nested too, and because they are only meaningful while
/// `kautian` is on.
///
/// Carried inside `EngineSettings` so one `settingsProvider.current` answers
/// for both the composing fetch and (from PR13) the dictionary search — reading
/// the toggles from a second place is what splits the snapshot and lets a
/// mid-keystroke settings change render half a composition under each rule.
///
/// The fields are `var` so a caller that varies one toggle can start from
/// `.defaults` and say only what differs; the snapshot is still immutable where
/// it matters, because `EngineSettings` holds this value in a `let`.
struct DictionarySourceToggles: Equatable, Sendable {
    /// 教育部臺灣台語常用詞辭典. Named `kautian` rather than after the iOS
    /// settings key (`moeDictEnabled`) because the engine's vocabulary is what
    /// this value is transcribed into.
    var kautian: Bool
    /// 台語新詞辭庫.
    var taigitv: Bool
    /// iTaigi 華台對照典.
    var itaigi: Bool
    /// 台灣植物名彙.
    var sitbut: Bool
    /// 台華線頂對照典.
    var taihoa: Bool
    /// 台日大辭典.
    var taijit: Bool
    /// 台語工藝詞庫.
    var kungge: Bool
    /// 學科術語辭典.
    var stti: Bool
    /// 腔口補充資料.
    var khpoo: Bool
    /// 異用字.
    var variant: Bool
    /// 在來字.
    var khiin: Bool
    /// LKK 漢羅合用建議用字.
    var lkk: Bool
    /// 詞庫增補檔案.
    var dev: Bool

    /// Always populated, never absent: the engine reads an absent
    /// subcollection message as "legacy all-on" and skips the gate entirely
    /// (`lexicon.proto:452-454`). macOS ships all eleven toggles, so it must
    /// always ask for the gate to run.
    var kautianSubcollections: KautianSubcollections

    /// The per-subcollection state of the kautian source. Order mirrors
    /// `KautianSubcollToggles` (`lexicon.proto:466-478`), which is the
    /// `config.yaml` `dialect_columns` order.
    struct KautianSubcollections: Equatable, Sendable {
        var accentLukang: Bool
        var accentSansia: Bool
        var accentTaipak: Bool
        var accentGilan: Bool
        var accentTainan: Bool
        var accentKaohsiung: Bool
        var accentKinmen: Bool
        var accentMakung: Bool
        var accentSintik: Bool
        var accentTaichung: Bool
        var nameAppendix: Bool

        /// CROSS-PLATFORM INVARIANT — every subcollection defaults ON, mirroring
        /// ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift:74-84.
        /// Drift changes which 腔口 a fresh install offers.
        static let defaults = KautianSubcollections(
            accentLukang: true,
            accentSansia: true,
            accentTaipak: true,
            accentGilan: true,
            accentTainan: true,
            accentKaohsiung: true,
            accentKinmen: true,
            accentMakung: true,
            accentSintik: true,
            accentTaichung: true,
            nameAppendix: true,
        )
    }

    /// CROSS-PLATFORM INVARIANT — mirrors
    /// ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift:53-66.
    /// Drift changes which dictionaries a fresh install searches, which is
    /// visible in the very first candidate list.
    static let defaults = DictionarySourceToggles(
        kautian: true,
        taigitv: true,
        itaigi: false,
        sitbut: false,
        taihoa: false,
        taijit: false,
        kungge: true,
        stti: true,
        khpoo: true,
        variant: false,
        khiin: false,
        lkk: true,
        dev: true,
        kautianSubcollections: .defaults,
    )
}
