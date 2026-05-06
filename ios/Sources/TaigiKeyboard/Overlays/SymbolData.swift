// 中文: 符號選擇 overlay 的資料來源 — 五大分類(全形/半形/平假名/片假名/顏文字)的符號表。

import Foundation

/// Symbol category for the symbol selection overlay
// 中文: 符號 overlay 的分類 enum — 全形 / 半形 / 平仮名 / 片仮名 / 顏文字。
enum SymbolCategory: CaseIterable {
    case fullWidth
    case halfWidth
    case hiragana
    case katakana
    case kaomoji

    var label: String {
        switch self {
        case .fullWidth: return "全形"
        case .halfWidth: return "半形"
        case .hiragana: return "平仮名"
        case .katakana: return "片仮名"
        case .kaomoji: return "顏文字"
        }
    }

    var columnCount: Int {
        switch self {
        case .kaomoji: return 2
        default: return 6
        }
    }

    var fontSize: CGFloat {
        switch self {
        case .kaomoji: return 14
        default: return 20
        }
    }
}

/// Symbol data for the symbol selection overlay
// 中文: 五大分類的符號資料表;每個分類是 [[String]]。
enum SymbolData {
    // MARK: - Tab 1: Full-width (14 rows x 6 cols)

    static let fullWidthRows: [[String]] = [
        ["，", "。", "！", "？", "；", "："],
        ["、", "．", "‧", "…", "～", "·"],
        ["—", "–", "＿", "－", "﹏", "＝"],
        ["「", "」", "『", "』", "（", "）"],
        ["《", "》", "〈", "〉", "【", "】"],
        ["﹁", "﹂", "﹃", "﹄", "〔", "〕"],
        ["［", "］", "｛", "｝", "＂", "＇"],
        ["＠", "＃", "＄", "％", "＆", "＊"],
        ["＋", "＜", "＞", "/", "＼", "｜"],
        ["￠", "￡", "￥", "￦", "＾", "｀"],
        ["❬", "❭", "❰", "❱", "⟨", "⟩"],
        ["⟪", "⟫", "⌈", "⌉", "⌊", "⌋"],
        ["←", "↑", "→", "↓", "⇐", "⇒"],
        ["⇑", "⇓", "⇔", "⇕", "⏎", "↵"],
    ]

    // MARK: - Tab 2: Half-width (8 rows x 6 cols)

    static let halfWidthRows: [[String]] = [
        [",", ".", "!", "?", ";", ":"],
        ["\"", "'", "-", "~", "`", "_"],
        ["(", ")", "[", "]", "{", "}"],
        ["<", ">", "/", "\\", "^", "|"],
        ["@", "#", "$", "%", "&", "*"],
        ["+", "=", "¬", "¦", "¯", "·"],
        ["₹", "€", "£", "¥", "₩", "¢"],
        ["\u{201C}", "\u{201D}", "\u{2018}", "\u{2019}", "«", "»"],
    ]

    // MARK: - Tab 3: Hiragana (8 rows x 6 cols)

    static let hiraganaRows: [[String]] = [
        ["あ", "い", "う", "え", "お", "か"],
        ["き", "く", "け", "こ", "さ", "し"],
        ["す", "せ", "そ", "た", "ち", "つ"],
        ["て", "と", "な", "に", "ぬ", "ね"],
        ["の", "は", "ひ", "ふ", "へ", "ほ"],
        ["ま", "み", "む", "め", "も", "や"],
        ["ゆ", "よ", "ら", "り", "る", "れ"],
        ["ろ", "わ", "を", "ん", "っ", "ー"],
    ]

    // MARK: - Tab 4: Katakana (8 rows x 6 cols)

    static let katakanaRows: [[String]] = [
        ["ア", "イ", "ウ", "エ", "オ", "カ"],
        ["キ", "ク", "ケ", "コ", "サ", "シ"],
        ["ス", "セ", "ソ", "タ", "チ", "ツ"],
        ["テ", "ト", "ナ", "ニ", "ヌ", "ネ"],
        ["ノ", "ハ", "ヒ", "フ", "ヘ", "ホ"],
        ["マ", "ミ", "ム", "メ", "モ", "ヤ"],
        ["ユ", "ヨ", "ラ", "リ", "ル", "レ"],
        ["ロ", "ワ", "ヲ", "ン", "ッ", "ー"],
    ]

    // MARK: - Tab 5: Kaomoji (20 rows x 2 cols)

    static let kaomojiRows: [[String]] = [
        ["( ˶'ᵕ'˶)", "(´・ω・`)"],
        ["(◕‿◕)", "(≧▽≦)"],
        ["(ﾉ◕ヮ◕)ﾉ*:・ﾟ✧", "(╯°□°)╯︵ ┻━┻"],
        ["┬─┬ノ( º _ ºノ)", "¯\\_(ツ)_/¯"],
        ["(ง •_•)ง", "(っ˘ω˘ς)"],
        ["(つ≧▽≦)つ", "(>_<)"],
        ["(T_T)", "(;_;)"],
        ["( ˘ω˘ )", "(⌐■_■)"],
        ["(╥﹏╥)", "(ﾉ´ヮ`)ﾉ*: ・ﾟ"],
        ["(◠‿◠)", "ʕ•ᴥ•ʔ"],
        ["(=^・^=)", "(ΦωΦ)"],
        ["(◕ᴗ◕✿)", "(✿◠‿◠)"],
        ["(≖_≖)", "(⊙_⊙)"],
        ["(☞ﾟヮﾟ)☞", "☜(ﾟヮﾟ☜)"],
        ["(ノಠ益ಠ)ノ", "(´;ω;`)"],
        ["(⌒▽⌒)", "(*^▽^*)"],
        ["(•̀ᴗ•́)و", "(ᵔᴥᵔ)"],
        ["(¬‿¬)", "(✧ω✧)"],
        ["(◔‿◔)", "(ᗒᗣᗕ)"],
        ["(。♥‿♥。)", "(*≧ω≦)"],
    ]

    /// Returns rows for the given category
    // 中文: 依分類回傳對應的符號 rows。
    static func rows(for category: SymbolCategory) -> [[String]] {
        switch category {
        case .fullWidth: return fullWidthRows
        case .halfWidth: return halfWidthRows
        case .hiragana: return hiraganaRows
        case .katakana: return katakanaRows
        case .kaomoji: return kaomojiRows
        }
    }
}
