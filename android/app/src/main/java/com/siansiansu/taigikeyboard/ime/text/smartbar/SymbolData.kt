package com.siansiansu.taigikeyboard.ime.text.smartbar

/**
 * Symbol categories for the symbol selection overlay.
 */
enum class SymbolCategory(
    val label: String,
    val columnCount: Int = 6,
    val fontSize: Float = 20f,
) {
    FULL_WIDTH("全形"),
    HALF_WIDTH("半形"),
    HIRAGANA("平仮名"),
    KATAKANA("片仮名"),
    KAOMOJI("顏文字", columnCount = 2, fontSize = 14f),
}

/**
 * Symbol data for the symbol selection overlay.
 *
 * Provides symbol rows for each category in the symbol panel.
 */
object SymbolData {
    /** Full-width symbols (14 rows x 6 cols) */
    val fullWidthRows: List<List<String>> = listOf(
        listOf("，", "。", "！", "？", "；", "："),
        listOf("、", "．", "‧", "…", "～", "·"),
        listOf("—", "–", "＿", "－", "﹏", "＝"),
        listOf("「", "」", "『", "』", "（", "）"),
        listOf("《", "》", "〈", "〉", "【", "】"),
        listOf("﹁", "﹂", "﹃", "﹄", "〔", "〕"),
        listOf("［", "］", "｛", "｝", "＂", "＇"),
        listOf("＠", "＃", "＄", "％", "＆", "＊"),
        listOf("＋", "＜", "＞", "／", "＼", "｜"),
        listOf("￠", "￡", "￥", "￦", "＾", "｀"),
        listOf("❬", "❭", "❰", "❱", "⟨", "⟩"),
        listOf("⟪", "⟫", "⌈", "⌉", "⌊", "⌋"),
        listOf("←", "↑", "→", "↓", "⇐", "⇒"),
        listOf("⇑", "⇓", "⇔", "⇕", "⏎", "↵"),
    )

    /** Half-width symbols (8 rows x 6 cols) */
    val halfWidthRows: List<List<String>> = listOf(
        listOf(",", ".", "!", "?", ";", ":"),
        listOf("\"", "'", "-", "~", "`", "_"),
        listOf("(", ")", "[", "]", "{", "}"),
        listOf("<", ">", "/", "\\", "^", "|"),
        listOf("@", "#", "$", "%", "&", "*"),
        listOf("+", "=", "¬", "¦", "¯", "·"),
        listOf("₹", "€", "£", "¥", "₩", "¢"),
        listOf("\u201C", "\u201D", "\u2018", "\u2019", "«", "»"),
    )

    /** Hiragana (8 rows x 6 cols) */
    val hiraganaRows: List<List<String>> = listOf(
        listOf("あ", "い", "う", "え", "お", "か"),
        listOf("き", "く", "け", "こ", "さ", "し"),
        listOf("す", "せ", "そ", "た", "ち", "つ"),
        listOf("て", "と", "な", "に", "ぬ", "ね"),
        listOf("の", "は", "ひ", "ふ", "へ", "ほ"),
        listOf("ま", "み", "む", "め", "も", "や"),
        listOf("ゆ", "よ", "ら", "り", "る", "れ"),
        listOf("ろ", "わ", "を", "ん", "っ", "ー"),
    )

    /** Katakana (8 rows x 6 cols) */
    val katakanaRows: List<List<String>> = listOf(
        listOf("ア", "イ", "ウ", "エ", "オ", "カ"),
        listOf("キ", "ク", "ケ", "コ", "サ", "シ"),
        listOf("ス", "セ", "ソ", "タ", "チ", "ツ"),
        listOf("テ", "ト", "ナ", "ニ", "ヌ", "ネ"),
        listOf("ノ", "ハ", "ヒ", "フ", "ヘ", "ホ"),
        listOf("マ", "ミ", "ム", "メ", "モ", "ヤ"),
        listOf("ユ", "ヨ", "ラ", "リ", "ル", "レ"),
        listOf("ロ", "ワ", "ヲ", "ン", "ッ", "ー"),
    )

    /** Kaomoji (20 rows x 2 cols) */
    val kaomojiRows: List<List<String>> = listOf(
        listOf("( ˶'ᵕ'˶)", "(´・ω・`)"),
        listOf("(◕‿◕)", "(≧▽≦)"),
        listOf("(ﾉ◕ヮ◕)ﾉ*:・ﾟ✧", "(╯°□°)╯︵ ┻━┻"),
        listOf("┬─┬ノ( º _ ºノ)", "¯\\_(ツ)_/¯"),
        listOf("(ง •_•)ง", "(っ˘ω˘ς)"),
        listOf("(つ≧▽≦)つ", "(>_<)"),
        listOf("(T_T)", "(;_;)"),
        listOf("( ˘ω˘ )", "(⌐■_■)"),
        listOf("(╥﹏╥)", "(ﾉ´ヮ`)ﾉ*: ・ﾟ"),
        listOf("(◠‿◠)", "ʕ•ᴥ•ʔ"),
        listOf("(=^・^=)", "(ΦωΦ)"),
        listOf("(◕ᴗ◕✿)", "(✿◠‿◠)"),
        listOf("(≖_≖)", "(⊙_⊙)"),
        listOf("(☞ﾟヮﾟ)☞", "☜(ﾟヮﾟ☜)"),
        listOf("(ノಠ益ಠ)ノ", "(´;ω;`)"),
        listOf("(⌒▽⌒)", "(*^▽^*)"),
        listOf("(•̀ᴗ•́)و", "(ᵔᴥᵔ)"),
        listOf("(¬‿¬)", "(✧ω✧)"),
        listOf("(◔‿◔)", "(ᗒᗣᗕ)"),
        listOf("(。♥‿♥。)", "(*≧ω≦)"),
    )

    /** Returns rows for the given category */
    fun rows(category: SymbolCategory): List<List<String>> = when (category) {
        SymbolCategory.FULL_WIDTH -> fullWidthRows
        SymbolCategory.HALF_WIDTH -> halfWidthRows
        SymbolCategory.HIRAGANA -> hiraganaRows
        SymbolCategory.KATAKANA -> katakanaRows
        SymbolCategory.KAOMOJI -> kaomojiRows
    }
}
