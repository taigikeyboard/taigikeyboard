import Foundation

/// 日語 Flick 鍵盤佈局（參考 azooKey）
///
/// 4 行 x 5 列的標準日語假名佈局
/// 用於測試 UI 與 azooKey 的一致性
enum FlickJapaneseLayout {

    // MARK: - 主佈局

    /// 日語假名 Flick 鍵盤
    ///
    /// 佈局結構（4行×5列，參考 azooKey）：
    /// ```
    ///          x=0       x=1      x=2      x=3      x=4
    /// y=0      ☆123      あ        か        さ        ⌫
    /// y=1      ABC       た        な        は        空白
    /// y=2      あa       ま        や        ら        改行
    /// y=3      🌐       小        わ        句読      ┘
    /// ```
    static let japanese = FlickLayout(
        identifier: "japanese_flick",
        displayName: "日本語フリック",
        rowCount: 4,
        columnCount: 5,
        keys: [
            // ========================================
            // Column 0 (x=0): Tab 切換欄
            // ========================================
            FlickKeyPosition(row: 0, column: 0): .system(.flickTabNumber),  // ☆123
            FlickKeyPosition(row: 1, column: 0): .system(.flickTabAbc),     // ABC
            FlickKeyPosition(row: 2, column: 0): .system(.flickTabTaigi),   // あa（切回假名）
            FlickKeyPosition(row: 3, column: 0): .system(.globe),           // 🌐

            // ========================================
            // Column 1 (x=1): 假名欄 1
            // ========================================
            FlickKeyPosition(row: 0, column: 1): kanaA,     // あ行
            FlickKeyPosition(row: 1, column: 1): kanaTa,    // た行
            FlickKeyPosition(row: 2, column: 1): kanaMa,    // ま行
            FlickKeyPosition(row: 3, column: 1): kanaKogaki, // 小

            // ========================================
            // Column 2 (x=2): 假名欄 2
            // ========================================
            FlickKeyPosition(row: 0, column: 2): kanaKa,    // か行
            FlickKeyPosition(row: 1, column: 2): kanaNa,    // な行
            FlickKeyPosition(row: 2, column: 2): kanaYa,    // や行
            FlickKeyPosition(row: 3, column: 2): kanaWa,    // わ行

            // ========================================
            // Column 3 (x=3): 假名欄 3
            // ========================================
            FlickKeyPosition(row: 0, column: 3): kanaSa,    // さ行
            FlickKeyPosition(row: 1, column: 3): kanaHa,    // は行
            FlickKeyPosition(row: 2, column: 3): kanaRa,    // ら行
            FlickKeyPosition(row: 3, column: 3): kanaKutoten, // 句読

            // ========================================
            // Column 4 (x=4): 系統功能欄
            // ========================================
            FlickKeyPosition(row: 0, column: 4): .system(.delete),          // ⌫
            FlickKeyPosition(row: 1, column: 4): .system(.space),           // 空白
            FlickKeyPosition(row: 2, column: 4, height: 2): .system(.enter), // 改行（跨2列）
        ]
    )

    // MARK: - 假名按鍵定義

    /// あ行：中心=あ, 左=い, 上=う, 右=え, 下=お
    static let kanaA: FlickKeyDef = .consonantGroup(
        "あ", label: "あ",
        left: "い", top: "う", right: "え", bottom: "お"
    )

    /// か行：中心=か, 左=き, 上=く, 右=け, 下=こ
    static let kanaKa: FlickKeyDef = .consonantGroup(
        "か", label: "か",
        left: "き", top: "く", right: "け", bottom: "こ"
    )

    /// さ行：中心=さ, 左=し, 上=す, 右=せ, 下=そ
    static let kanaSa: FlickKeyDef = .consonantGroup(
        "さ", label: "さ",
        left: "し", top: "す", right: "せ", bottom: "そ"
    )

    /// た行：中心=た, 左=ち, 上=つ, 右=て, 下=と
    static let kanaTa: FlickKeyDef = .consonantGroup(
        "た", label: "た",
        left: "ち", top: "つ", right: "て", bottom: "と"
    )

    /// な行：中心=な, 左=に, 上=ぬ, 右=ね, 下=の
    static let kanaNa: FlickKeyDef = .consonantGroup(
        "な", label: "な",
        left: "に", top: "ぬ", right: "ね", bottom: "の"
    )

    /// は行：中心=は, 左=ひ, 上=ふ, 右=へ, 下=ほ
    static let kanaHa: FlickKeyDef = .consonantGroup(
        "は", label: "は",
        left: "ひ", top: "ふ", right: "へ", bottom: "ほ"
    )

    /// ま行：中心=ま, 左=み, 上=む, 右=め, 下=も
    static let kanaMa: FlickKeyDef = .consonantGroup(
        "ま", label: "ま",
        left: "み", top: "む", right: "め", bottom: "も"
    )

    /// や行：中心=や, 左=「, 上=ゆ, 右=」, 下=よ
    static let kanaYa: FlickKeyDef = .consonantGroup(
        "や", label: "や",
        left: "「", top: "ゆ", right: "」", bottom: "よ"
    )

    /// ら行：中心=ら, 左=り, 上=る, 右=れ, 下=ろ
    static let kanaRa: FlickKeyDef = .consonantGroup(
        "ら", label: "ら",
        left: "り", top: "る", right: "れ", bottom: "ろ"
    )

    /// わ行：中心=わ, 左=を, 上=ん, 右=ー
    static let kanaWa: FlickKeyDef = .consonantGroup(
        "わ", label: "わ",
        left: "を", top: "ん", right: "ー"
    )

    /// 小書き：中心=小, 左=゛, 上=゜, 右=...
    static let kanaKogaki: FlickKeyDef = .symbolGroup(
        "小", label: "小",
        left: "゛", top: "゜", right: "ゝ", bottom: "ゞ"
    )

    /// 句読点：中心=、, 左=。, 上=？, 右=！
    static let kanaKutoten: FlickKeyDef = .symbolGroup(
        "、", label: "、。",
        left: "。", top: "？", right: "！", bottom: "…"
    )
}
