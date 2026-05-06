// 中文: Taigi 鍵盤所有版面的資料定義 — 每張版面都是 [[KeyDef]]。
// 中文: _withGlobe 變體由 withGlobeKey 在底列 index 1 插入 .globe 自動衍生;
// 中文: .char(half, fullWidth:) 預設顯示半形,isTranslateSwapped 切到全形(TPS 永遠全形)。

/// Taigi keyboard layout definitions — each layout is [[KeyDef]]
///
/// _withGlobe variants are derived by inserting .globe at bottom-row index 1.
/// .char(half, fullWidth: full) shows half-width by default,
/// full-width when isTranslateSwapped = true (or always for TPS layout).
// 中文: Taigi 鍵盤所有版面的資料命名空間 — Alphabetic / Numeric / Symbolic 三大類。
enum TaigiLayouts {
    /// Derives a withGlobe variant by inserting .globe at index 1 of the bottom row
    // 中文: 衍生 withGlobe 變體的工具 — 在底列 index 1 插入 .globe。
    private static func withGlobeKey(_ layout: [[KeyDef]]) -> [[KeyDef]] {
        var result = layout
        result[result.count - 1].insert(.globe, at: 1)
        return result
    }

    // MARK: - Alphabetic Keyboards

    // 中文: 字母鍵盤命名空間 — 涵蓋 PhahTaigi / QWERTY(TL/POJ/英文)/ TPS / MOE1 / MOE2。
    enum Alphabetic {
        // MARK: PhahTaigi Layout

        /// PhahTaigi - iPhone (no globe key)
        static let phahTaigi_iPhone: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("!", fullWidth: "！"), .char("?", fullWidth: "？"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l"), .char("-")],
            [.shift, .char(",", fullWidth: "，"), .char(".", fullWidth: "。"), .char("c"), .char("v"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .emoji, .space, .translate, .return],
        ]

        /// PhahTaigi - iPhone SE / iPad (with globe key)
        static let phahTaigi_withGlobe: [[KeyDef]] = TaigiLayouts.withGlobeKey(phahTaigi_iPhone)

        // MARK: QWERTY Layout (TL mode)

        /// QWERTY TL - iPhone (no globe key)
        static let qwerty_TL_iPhone: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("q"), .char("w"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l")],
            [.shift, .char("z"), .char("x"), .char("c"), .char("v"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .emoji, .char(",", fullWidth: "，"), .space, .char("-"), .translate, .return],
        ]

        /// QWERTY TL - iPhone SE / iPad (with globe key)
        static let qwerty_TL_withGlobe: [[KeyDef]] = TaigiLayouts.withGlobeKey(qwerty_TL_iPhone)

        // MARK: QWERTY Layout (POJ mode)

        /// QWERTY POJ - iPhone (no globe key)
        static let qwerty_POJ_iPhone: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("q"), .char("w"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l"), .char("o͘")],
            [.shift, .char("z"), .char("x"), .char("c"), .char("v"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .emoji, .char(",", fullWidth: "，"), .space, .char("-"), .translate, .return],
        ]

        /// QWERTY POJ - iPhone SE / iPad (with globe key)
        static let qwerty_POJ_withGlobe: [[KeyDef]] = TaigiLayouts.withGlobeKey(qwerty_POJ_iPhone)

        // MARK: QWERTY Layout (English mode)

        /// QWERTY English - iPhone (no globe key)
        static let qwerty_English_iPhone: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("q"), .char("w"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l")],
            [.shift, .char("z"), .char("x"), .char("c"), .char("v"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .emoji, .space, .return],
        ]

        /// QWERTY English - iPhone SE / iPad (with globe key)
        static let qwerty_English_withGlobe: [[KeyDef]] = TaigiLayouts.withGlobeKey(qwerty_English_iPhone)

        // MARK: TPS Layout (Taiwanese Phonetic Symbols / 方音符號)

        // Row 1: Voiced initials + tones + nasalized vowels
        // Row 2: Unaspirated stops + vowels + nasalized vowels
        // Row 3: Aspirated stops + vowels + nasalized vowels
        // Row 4: Nasals + affricates + vowels + punctuation + backspace
        // Row 5: Function + consonants + space + vowels + enter

        /// TPS - iPhone (no globe key)
        static let tps_iPhone: [[KeyDef]] = [
            // Row 1: ㆠ, ˋ, ˪, ㆣ, ˊ, ˇ, ˫, ˙, ㆪ, ㆩ
            [.char("ㆠ"), .char("ˋ"), .char("˪"), .char("ㆣ"), .char("ˊ"), .char("ˇ"), .char("˫"), .char("˙"), .char("ㆪ"), .char("ㆩ")],

            // Row 2: ㄅ(ㆴ), ㄉ(ㆵ), ㄍ(ㆻ), ㄏ(ㆷ), ㄧ, ㄚ, ㄞ, ㄤ, ㆫ, ㆧ
            [.char("ㄅ"), .char("ㄉ"), .char("ㄍ"), .char("ㄏ"), .char("ㄧ"), .char("ㄚ"), .char("ㄞ"), .char("ㄤ"), .char("ㆫ"), .char("ㆧ")],

            // Row 3: ㄆ, ㄊ, ㄎ, ㄗ(ㄐ), ㄨ, ㄛ, ㄠ, ㆲ, ㆥ, ㆮ
            [.char("ㄆ"), .char("ㄊ"), .char("ㄎ"), .char("ㄗ"), .char("ㄨ"), .char("ㄛ"), .char("ㄠ"), .char("ㆲ"), .char("ㆥ"), .char("ㆮ")],

            // Row 4: ㄇ(ㆬ), ㄋ, ㄫ(ㆭ,ㄙ), ㄘ(ㄑ), ㄜ, ㆦ, ㄢ, ㆰ(ㆱ), ,(。), backspace
            [.char("ㄇ"), .char("ㄋ"), .char("ㄫ"), .char("ㄘ"), .char("ㄜ"), .char("ㆦ"), .char("ㄢ"), .char("ㆰ"), .char(",", fullWidth: "，"), .backspace],

            // Row 5: ?123, ㄌ, ㆡ(ㆢ), ㄙ(ㄒ), ㆨ, ㆤ(ㄝ), space, emoji, enter
            [.numeric, .char("ㄌ"), .char("ㆡ"), .char("ㄙ"), .char("ㆨ"), .char("ㆤ"), .space, .emoji, .return],
        ]

        /// TPS - iPhone SE / iPad (with globe key)
        static let tps_withGlobe: [[KeyDef]] = TaigiLayouts.withGlobeKey(tps_iPhone)

        // MARK: MOE Layout 1 (教育部輸入法佈局1) - TL version

        /// MOE1 TL - iPhone (no globe key)
        static let moe1_TL_iPhone: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("*"), .char("?", fullWidth: "？"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l"), .char("-")],
            [.shift, .char(",", fullWidth: "，"), .char(".", fullWidth: "。"), .char("("), .char(")"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .emoji, .space, .translate, .return],
        ]

        /// MOE1 TL - iPhone SE / iPad (with globe key)
        static let moe1_TL_withGlobe: [[KeyDef]] = TaigiLayouts.withGlobeKey(moe1_TL_iPhone)

        // MARK: MOE Layout 1 (教育部輸入法佈局1) - POJ version

        /// MOE1 POJ - iPhone (no globe key)
        static let moe1_POJ_iPhone: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("*"), .char("?", fullWidth: "？"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l"), .char("-")],
            [.shift, .char(",", fullWidth: "，"), .char(".", fullWidth: "。"), .char("c"), .char("v"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .emoji, .space, .translate, .return],
        ]

        /// MOE1 POJ - iPhone SE / iPad (with globe key)
        static let moe1_POJ_withGlobe: [[KeyDef]] = TaigiLayouts.withGlobeKey(moe1_POJ_iPhone)

        // MARK: MOE Layout 2 (教育部輸入法佈局2) - TL version

        /// MOE2 TL - iPhone (no globe key)
        static let moe2_TL_iPhone: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("p"), .char("ph"), .char("m"), .char("b"), .char("ts"), .char("tsh"), .char("a"), .char("i"), .char("o"), .char("oo")],
            [.char("t"), .char("th"), .char("n"), .char("l"), .char("s"), .char("j"), .char("u"), .char("e"), .char("r"), .char("-")],
            [.shift, .char("k"), .char("kh"), .char("ng"), .char("g"), .char("h"), .char("nn"), .char("?", fullWidth: "？"), .backspace],
            [.numeric, .emoji, .char(",", fullWidth: "，"), .space, .char(".", fullWidth: "。"), .translate, .return],
        ]

        /// MOE2 TL - iPhone SE / iPad (with globe key)
        static let moe2_TL_withGlobe: [[KeyDef]] = TaigiLayouts.withGlobeKey(moe2_TL_iPhone)

        // MARK: MOE Layout 2 (教育部輸入法佈局2) - POJ version

        /// MOE2 POJ - iPhone (no globe key)
        static let moe2_POJ_iPhone: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("p"), .char("ph"), .char("m"), .char("b"), .char("ch"), .char("chh"), .char("a"), .char("i"), .char("o"), .char("o\u{0358}")],
            [.char("t"), .char("th"), .char("n"), .char("l"), .char("s"), .char("j"), .char("u"), .char("e"), .char("r"), .char("-")],
            [.shift, .char("k"), .char("kh"), .char("ng"), .char("g"), .char("h"), .char("nn"), .char("?", fullWidth: "？"), .backspace],
            [.numeric, .emoji, .char(",", fullWidth: "，"), .space, .char(".", fullWidth: "。"), .translate, .return],
        ]

        /// MOE2 POJ - iPhone SE / iPad (with globe key)
        static let moe2_POJ_withGlobe: [[KeyDef]] = TaigiLayouts.withGlobeKey(moe2_POJ_iPhone)
    }

    // MARK: - Numeric Keyboard (Common Symbols)

    // 5-row design: High-frequency symbols + Quotation marks & brackets + Punctuation + Common symbols + Bottom row

    // 中文: 數字鍵盤命名空間 — 高頻符號、引號括號、標點、常用符號等五列。
    enum Numeric {
        /// Numeric - iPhone (no globe key)
        static let iPhone: [[KeyDef]] = [
            // Row 1: High-frequency symbols (numbers available via alphabetic keyboard)
            [.char("~", fullWidth: "～"), .char("/"),
             .char("=", fullWidth: "＝"), .char("_", fullWidth: "＿"),
             .char("《"), .char("》"),
             .char("〈"), .char("〉"),
             .char("|", fullWidth: "｜"), .char("‧", fullWidth: "·")],

            // Row 2: Quotation marks and CJK brackets (half-width uses curly quotes)
            [.char("\u{201C}", fullWidth: "「"), .char("\u{201D}", fullWidth: "」"),
             .char("\u{2018}", fullWidth: "『"), .char("\u{2019}", fullWidth: "』"),
             .char("(", fullWidth: "（"), .char(")", fullWidth: "）"),
             .char("【"), .char("】"),
             .char("["), .char("]")],

            // Row 3: Punctuation
            [.char(":", fullWidth: "："), .char(";", fullWidth: "；"),
             .char("'", fullWidth: "、"), .char("-"), .char("—"),
             .char("...", fullWidth: "⋯"),
             .char("$"), .char("%"), .char("#"), .char("&")],

            // Row 4: Most common punctuation (natural finger position)
            [.symbolic,
             .char(".", fullWidth: "。"), .char(",", fullWidth: "，"),
             .char("?", fullWidth: "？"), .char("!", fullWidth: "！"),
             .char("*"), .char("+"), .char("@"), .backspace],

            // Row 5: Bottom row
            [.alphabetic, .emoji, .space, .translate, .return],
        ]

        /// Numeric - iPhone SE / iPad (with globe key)
        static let withGlobe: [[KeyDef]] = TaigiLayouts.withGlobeKey(iPhone)
    }

    // MARK: - Symbolic Keyboard (Advanced Symbols)

    // 5-row design: Programming brackets + Arrows & special symbols + Currency + Math + Bottom row
    // Does not overlap with Numeric keyboard

    // 中文: 進階符號鍵盤命名空間 — 程式括號、箭頭、貨幣、數學符號等;與 Numeric 不重複。
    enum Symbolic {
        /// Symbolic - iPhone (no globe key)
        static let iPhone: [[KeyDef]] = [
            // Row 1: Programming brackets (half-width, ｛｝ have full-width versions)
            [.char("〔"), .char("〕"),
             .char("{", fullWidth: "｛"), .char("}", fullWidth: "｝"),
             .char("«"), .char("»"),
             .char("<"), .char(">"),
             .char("^"), .char("※")],

            // Row 2: Arrows and special symbols
            [.char("\\", fullWidth: "＼"),
             .char("←"), .char("→"), .char("↑"), .char("↓"),
             .char("§"), .char("†"), .char("¶"), .char("‰"), .char("℉")],

            // Row 3: Currency and special symbols
            [.char("€"), .char("£"), .char("¥"), .char("¢"),
             .char("•", fullWidth: "·"), .char("°"),
             .char("©"), .char("®"), .char("™"), .char("℃")],

            // Row 4: Math symbols (natural finger position)
            [.numeric,
             .char("±"), .char("×"), .char("÷"),
             .char("≠"), .char("≈"), .char("∞"), .char("√"), .backspace],

            // Row 5: Bottom row
            [.alphabetic, .emoji, .space, .translate, .return],
        ]

        /// Symbolic - iPhone SE / iPad (with globe key)
        static let withGlobe: [[KeyDef]] = TaigiLayouts.withGlobeKey(iPhone)
    }
}
