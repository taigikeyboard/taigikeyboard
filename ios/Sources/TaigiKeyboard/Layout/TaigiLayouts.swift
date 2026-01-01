/// 台語鍵盤佈局定義
///
/// 所有鍵盤佈局都在此處完整定義，方便一眼看到全貌。
/// 每個佈局都是 `[[KeyDef]]`，代表完整的鍵盤。
///
/// 命名規則：
/// - `_iPhone`: 無 globe 鍵（一般 iPhone）
/// - `_withGlobe`: 有 globe 鍵在最左邊（iPhone SE、iPad）
///
/// 全形/半形對應：
/// - 預設顯示半形（全羅文章用）
/// - isTranslateSwapped = true 時顯示全形（漢羅文章用）
enum TaigiLayouts {

    // ========================================
    // MARK: - Alphabetic 鍵盤
    // ========================================

    enum Alphabetic {

        // MARK: PhahTaigi 佈局

        /// PhahTaigi - iPhone（無 globe）
        static let phahTaigi_iPhone: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("!", fullWidth: "！"), .char("?", fullWidth: "？"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l"), .char("-")],
            [.shift, .char(",", fullWidth: "，"), .char(".", fullWidth: "。"), .char("c"), .char("v"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .emoji, .space, .translate, .return]
        ]

        /// PhahTaigi - iPhone SE / iPad（有 globe）
        static let phahTaigi_withGlobe: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("!", fullWidth: "！"), .char("?", fullWidth: "？"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l"), .char("-")],
            [.shift, .char(",", fullWidth: "，"), .char(".", fullWidth: "。"), .char("c"), .char("v"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .globe, .emoji, .space, .translate, .return]
        ]

        // MARK: QWERTY 佈局（TL 模式）

        /// QWERTY TL - iPhone（無 globe）
        static let qwerty_TL_iPhone: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("q"), .char("w"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l")],
            [.shift, .char("z"), .char("x"), .char("c"), .char("v"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .emoji, .char(","), .space, .char("-"), .translate, .return]
        ]

        /// QWERTY TL - iPhone SE / iPad（有 globe）
        static let qwerty_TL_withGlobe: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("q"), .char("w"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l")],
            [.shift, .char("z"), .char("x"), .char("c"), .char("v"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .globe, .emoji, .char(","), .space, .char("-"), .translate, .return]
        ]

        // MARK: QWERTY 佈局（POJ 模式）

        /// QWERTY POJ - iPhone（無 globe）
        static let qwerty_POJ_iPhone: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("q"), .char("w"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l"), .char("o͘")],
            [.shift, .char("z"), .char("x"), .char("c"), .char("v"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .emoji, .char(","), .space, .char("-"), .translate, .return]
        ]

        /// QWERTY POJ - iPhone SE / iPad（有 globe）
        static let qwerty_POJ_withGlobe: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("q"), .char("w"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l"), .char("o͘")],
            [.shift, .char("z"), .char("x"), .char("c"), .char("v"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .globe, .emoji, .char(","), .space, .char("-"), .translate, .return]
        ]

        // MARK: QWERTY 佈局（English 模式）
        // Apple 標準英文鍵盤：4 列，無數字列

        /// QWERTY English - iPhone（無 globe）
        static let qwerty_English_iPhone: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("q"), .char("w"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l")],
            [.shift, .char("z"), .char("x"), .char("c"), .char("v"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .emoji, .space, .return]
        ]

        /// QWERTY English - iPhone SE / iPad（有 globe）
        static let qwerty_English_withGlobe: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("q"), .char("w"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l")],
            [.shift, .char("z"), .char("x"), .char("c"), .char("v"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .globe, .emoji, .space, .return]
        ]

        // MARK: TPS 佈局（台灣注音/方音符號）
        //
        // 佈局設計參考 images.png，基於 QWERTY 風格
        // Row 1: 聲調符號 + 入聲韻尾
        // Row 2: 聲母（雙唇、舌尖、舌根）
        // Row 3: 聲母 + 基本韻母
        // Row 4: 韻母 + 功能鍵
        // Row 5: 功能列

        /// TPS - iPhone（無 globe）
        static let tps_iPhone: [[KeyDef]] = [
            // Row 1: ㄅ, ㄉ, ˇ, ˋ, ㄓ, ˊ, ˙, ㄚ, ㄞ, ㄢ (對應圖片第一列)
            [.char("ㄅ"), .char("ㄉ"), .char("ˇ"), .char("ˋ"), .char("ㄓ"), .char("ˊ"), .char("˙"), .char("ㄚ"), .char("ㄞ"), .char("ㄢ")],

            // Row 2: ㄆ, ㄊ, ㄍ, ㄐ, ㄑ, ㄒ, ㄧ, ㄛ, ㄣ, ㄦ (對應圖片第二列)
            [.char("ㄆ"), .char("ㄊ"), .char("ㄍ"), .char("ㄐ"), .char("ㄑ"), .char("ㄒ"), .char("ㄧ"), .char("ㄛ"), .char("ㄣ"), .char("ㄦ")],

            // Row 3: ㄇ, ㄋ, ㄎ, ㄏ, ㄫ, ㄬ, ㄨ, ㄜ, ㄤ, ㄠ (對應圖片第三列)
            [.char("ㄇ"), .char("ㄋ"), .char("ㄎ"), .char("ㄏ"), .char("ㄫ"), .char("ㄬ"), .char("ㄨ"), .char("ㄜ"), .char("ㄤ"), .char("ㄠ")],

            // Row 4: ㄈ, ㄌ, ㄯ, ㄙ, ㄗ, ㄘ, ㄙ, ㄩ, ㄝ, ㄥ (對應圖片第四列)
            // 註：圖片中 Z、X 位置使用了方音符號的變體，此處依視覺匹配
            [.char("ㆠ"), .char("ㄌ"), .char("ㄯ"), .char("ㄒ"), .char("ㄗ"), .char("ㄘ"), .char("ㄙ"), .char("ㄩ"), .char("ㄝ"), .char("ㄥ")],
            // Row 5: 功能列
            [.numeric, .emoji, .space, .translate, .return]
        ]

        /// TPS - iPhone SE / iPad（有 globe）
        static let tps_withGlobe: [[KeyDef]] = [
            // Row 1: ㄅ, ㄉ, ˇ, ˋ, ㄓ, ˊ, ˙, ㄚ, ㄞ, ㄢ (對應圖片第一列)
            [.char("ㄅ"), .char("ㄉ"), .char("ˇ"), .char("ˋ"), .char("ㄓ"), .char("ˊ"), .char("˙"), .char("ㄚ"), .char("ㄞ"), .char("ㄢ")],

            // Row 2: ㄆ, ㄊ, ㄍ, ㄐ, ㄑ, ㄒ, ㄧ, ㄛ, ㄣ, ㄦ (對應圖片第二列)
            [.char("ㄆ"), .char("ㄊ"), .char("ㄍ"), .char("ㄐ"), .char("ㄑ"), .char("ㄒ"), .char("ㄧ"), .char("ㄛ"), .char("ㄣ"), .char("ㄦ")],

            // Row 3: ㄇ, ㄋ, ㄎ, ㄏ, ㄫ, ㄬ, ㄨ, ㄜ, ㄤ, ㄠ (對應圖片第三列)
            [.char("ㄇ"), .char("ㄋ"), .char("ㄎ"), .char("ㄏ"), .char("ㄫ"), .char("ㄬ"), .char("ㄨ"), .char("ㄜ"), .char("ㄤ"), .char("ㄠ")],

            // Row 4: ㄈ, ㄌ, ㄯ, ㄙ, ㄗ, ㄘ, ㄙ, ㄩ, ㄝ, ㄥ (對應圖片第四列)
            // 註：圖片中 Z、X 位置使用了方音符號的變體，此處依視覺匹配
            [.char("ㄈ"), .char("ㄌ"), .char("ㄯ"), .char("ㄒ"), .char("ㄗ"), .char("ㄘ"), .char("ㄙ"), .char("ㄩ"), .char("ㄝ"), .char("ㄥ")],

            // Row 5: 功能列 (依照圖片底部配置)
            [.numeric, .globe, .emoji, .space, .translate, .return]
        ]
    }

    // ========================================
    // MARK: - Numeric 鍵盤（常用符號）
    // ========================================
    // 5 列設計：數字 + 基本標點 + 常用括號 + 功能列 + 底部列

    enum Numeric {

        /// Numeric - iPhone（無 globe）
        static let iPhone: [[KeyDef]] = [
            // 第1列：數字
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"),
             .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],

            // 第2列：引號和中文括號（半形用 curly quotes）
            [.char("\u{201C}", fullWidth: "「"), .char("\u{201D}", fullWidth: "」"),
             .char("\u{2018}", fullWidth: "『"), .char("\u{2019}", fullWidth: "』"),
             .char("（"), .char("）"),
             .char("【"), .char("】"),
             .char("["), .char("]")],

            // 第3列：標點符號
            [.char(":", fullWidth: "："), .char(";", fullWidth: "；"),
             .char("'", fullWidth: "、"), .char("-"), .char("—"),
             .char("...", fullWidth: "⋯"),
             .char("$"), .char("%"), .char("#"), .char("&")],

            // 第4列：最常用標點（手指自然位置）
            [.symbolic,
             .char(".", fullWidth: "。"), .char(",", fullWidth: "，"),
             .char("?", fullWidth: "？"), .char("!", fullWidth: "！"),
             .char("*"), .char("+"), .char("@"), .backspace],

            // 第5列：底部列
            [.alphabetic, .emoji, .space, .translate, .return]
        ]

        /// Numeric - iPhone SE / iPad（有 globe）
        static let withGlobe: [[KeyDef]] = [
            // 第1列：數字
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"),
             .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],

            // 第2列：引號和中文括號（半形用 curly quotes）
            [.char("\u{201C}", fullWidth: "「"), .char("\u{201D}", fullWidth: "」"),
             .char("\u{2018}", fullWidth: "『"), .char("\u{2019}", fullWidth: "』"),
             .char("（"), .char("）"),
             .char("【"), .char("】"),
             .char("["), .char("]")],

            // 第3列：標點符號
            [.char(":", fullWidth: "："), .char(";", fullWidth: "；"),
             .char("'", fullWidth: "、"), .char("-"), .char("—"),
             .char("...", fullWidth: "⋯"),
             .char("$"), .char("%"), .char("#"), .char("&")],

            // 第4列：最常用標點（手指自然位置）
            [.symbolic,
             .char(".", fullWidth: "。"), .char(",", fullWidth: "，"),
             .char("?", fullWidth: "？"), .char("!", fullWidth: "！"),
             .char("*"), .char("+"), .char("@"), .backspace],

            // 第5列：底部列
            [.alphabetic, .globe, .emoji, .space, .translate, .return]
        ]
    }

    // ========================================
    // MARK: - Symbolic 鍵盤（進階符號）
    // ========================================
    // 5 列設計：程式符號 + 貨幣符號 + 書名號 + 功能列 + 底部列
    // 注意：不與 Numeric 重複

    enum Symbolic {

        /// Symbolic - iPhone（無 globe）
        static let iPhone: [[KeyDef]] = [
            // 第1列：程式括號（半形，｛｝有全形版本）
            [.char("〔"), .char("〕"),
             .char("{", fullWidth: "｛"), .char("}", fullWidth: "｝"),
             .char("«"), .char("»"),
             .char("<"), .char(">"),
             .char("^"), .char("※")],

            // 第2列：書名號（永遠全形）
            [.char("〈"), .char("〉"),
             .char("《"), .char("》"),
             .char("|"), .char("~"),
             .char("\\"), .char("/"),
             .char("_"), .char("=")],

            // 第3列：貨幣和特殊符號
            [.char("€"), .char("£"), .char("¥"), .char("¢"),
             .char("•", fullWidth: "·"), .char("°"),
             .char("©"), .char("®"), .char("™"), .char("℃")],

            // 第4列：數學符號（手指自然位置）
            [.numeric,
             .char("±"), .char("×"), .char("÷"),
             .char("≠"), .char("≈"), .char("∞"), .char("√"), .backspace],

            // 第5列：底部列
            [.alphabetic, .emoji, .space, .translate, .return]
        ]

        /// Symbolic - iPhone SE / iPad（有 globe）
        static let withGlobe: [[KeyDef]] = [
            // 第1列：程式括號（半形，｛｝有全形版本）
            [.char("〔"), .char("〕"),
             .char("{", fullWidth: "｛"), .char("}", fullWidth: "｝"),
             .char("«"), .char("»"),
             .char("<"), .char(">"),
             .char("^"), .char("※")],

            // 第2列：書名號（永遠全形）
            [.char("〈"), .char("〉"),
             .char("《"), .char("》"),
             .char("|"), .char("~"),
             .char("\\"), .char("/"),
             .char("_"), .char("=")],

            // 第3列：貨幣和特殊符號
            [.char("€"), .char("£"), .char("¥"), .char("¢"),
             .char("•", fullWidth: "·"), .char("°"),
             .char("©"), .char("®"), .char("™"), .char("℃")],

            // 第4列：數學符號（手指自然位置）
            [.numeric,
             .char("±"), .char("×"), .char("÷"),
             .char("≠"), .char("≈"), .char("∞"), .char("√"), .backspace],

            // 第5列：底部列
            [.alphabetic, .globe, .emoji, .space, .translate, .return]
        ]
    }
}
