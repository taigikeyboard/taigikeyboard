// Which dictionaries a fresh install searches.

/// The default dictionary source preferences, in the shape `FetchAtPos`
/// carries them.
///
/// Field order mirrors `engine/protos/proto/lexicon.proto::DictionarySourceToggles`
/// and desktop-core's `DictionarySourceToggles` (`settings/engine_settings.rs`).
/// The 11 kautian subcollection flags are nested rather than flat because the
/// wire has them nested too, and because they are only meaningful while
/// `kautian` is on.
///
/// Part of the `EngineSettings.defaults` table: `SettingsStore.Keys` reads each
/// toggle's default from `.defaults`.
struct DictionarySourceToggles: Sendable {
    /// 教育部臺灣台語常用詞辭典. Named `kautian` rather than after the iOS
    /// settings key (`moeDictEnabled`) because the engine's vocabulary is what
    /// this value is transcribed into.
    let kautian: Bool
    /// 台語新詞辭庫.
    let taigitv: Bool
    /// iTaigi 華台對照典.
    let itaigi: Bool
    /// 台灣植物名彙.
    let sitbut: Bool
    /// 台華線頂對照典.
    let taihoa: Bool
    /// 台日大辭典.
    let taijit: Bool
    /// 台語工藝詞庫.
    let kungge: Bool
    /// 學科術語辭典.
    let stti: Bool
    /// Accent Variations.
    let khpoo: Bool
    /// Variant characters.
    let variant: Bool
    /// 在來字.
    let khiin: Bool
    /// LKK 漢羅合用建議用字.
    let lkk: Bool
    /// Supplementary Word List.
    let dev: Bool

    /// Always populated, never absent: the engine reads an absent
    /// subcollection message as "legacy all-on" and skips the gate entirely
    /// (`lexicon.proto:452-454`). macOS ships all twelve toggles, so it must
    /// always ask for the gate to run.
    let kautianSubcollections: KautianSubcollections

    /// The per-subcollection state of the kautian source. Order mirrors
    /// `KautianSubcollectionToggles` (`lexicon.proto`), which is the
    /// `config.yaml` `dialect_columns` order.
    struct KautianSubcollections: Sendable {
        let accentLukang: Bool
        let accentSansia: Bool
        let accentTaipak: Bool
        let accentGilan: Bool
        let accentTainan: Bool
        let accentKaohsiung: Bool
        let accentKinmen: Bool
        let accentMakung: Bool
        let accentSintik: Bool
        let accentTaichung: Bool
        let nameAppendix: Bool
        let altReading: Bool

        /// CROSS-PLATFORM INVARIANT — every subcollection defaults ON, mirroring
        /// ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift:74-84.
        /// Drift changes which accents a fresh install offers.
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
            altReading: true,
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
