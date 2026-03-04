/// Taigi keyboard layout definitions
///
/// All keyboard layouts are defined here for easy overview.
/// Each layout is `[[KeyDef]]` representing a complete keyboard.
///
/// Naming conventions:
/// - `_iPhone`: No globe key (standard iPhone)
/// - `_withGlobe`: Globe key on the left (iPhone SE, iPad)
///
/// Full-width/Half-width mapping:
/// - Default shows half-width (for full romanization text)
/// - When isTranslateSwapped = true, shows full-width (for Hàn-lô mixed text)
enum TaigiLayouts {

    // ========================================
    // MARK: - Alphabetic Keyboards
    // ========================================

    enum Alphabetic {

        // MARK: PhahTaigi Layout

        /// PhahTaigi - iPhone (no globe key)
        static let phahTaigi_iPhone: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("!", fullWidth: "！"), .char("?", fullWidth: "？"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l"), .char("-")],
            [.shift, .char(",", fullWidth: "，"), .char(".", fullWidth: "。"), .char("c"), .char("v"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .emoji, .space, .translate, .return]
        ]

        /// PhahTaigi - iPhone SE / iPad (with globe key)
        static let phahTaigi_withGlobe: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("!", fullWidth: "！"), .char("?", fullWidth: "？"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l"), .char("-")],
            [.shift, .char(",", fullWidth: "，"), .char(".", fullWidth: "。"), .char("c"), .char("v"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .globe, .emoji, .space, .translate, .return]
        ]

        // MARK: QWERTY Layout (TL mode)

        /// QWERTY TL - iPhone (no globe key)
        static let qwerty_TL_iPhone: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("q"), .char("w"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l")],
            [.shift, .char("z"), .char("x"), .char("c"), .char("v"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .emoji, .char(","), .space, .char("-"), .translate, .return]
        ]

        /// QWERTY TL - iPhone SE / iPad (with globe key)
        static let qwerty_TL_withGlobe: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("q"), .char("w"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l")],
            [.shift, .char("z"), .char("x"), .char("c"), .char("v"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .globe, .emoji, .char(","), .space, .char("-"), .translate, .return]
        ]

        // MARK: QWERTY Layout (POJ mode)

        /// QWERTY POJ - iPhone (no globe key)
        static let qwerty_POJ_iPhone: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("q"), .char("w"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l"), .char("o͘")],
            [.shift, .char("z"), .char("x"), .char("c"), .char("v"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .emoji, .char(","), .space, .char("-"), .translate, .return]
        ]

        /// QWERTY POJ - iPhone SE / iPad (with globe key)
        static let qwerty_POJ_withGlobe: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("q"), .char("w"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l"), .char("o͘")],
            [.shift, .char("z"), .char("x"), .char("c"), .char("v"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .globe, .emoji, .char(","), .space, .char("-"), .translate, .return]
        ]

        // MARK: QWERTY Layout (English mode)
        // Apple standard English keyboard: 4 rows, no number row

        /// QWERTY English - iPhone (no globe key)
        static let qwerty_English_iPhone: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("q"), .char("w"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l")],
            [.shift, .char("z"), .char("x"), .char("c"), .char("v"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .emoji, .space, .return]
        ]

        /// QWERTY English - iPhone SE / iPad (with globe key)
        static let qwerty_English_withGlobe: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("q"), .char("w"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l")],
            [.shift, .char("z"), .char("x"), .char("c"), .char("v"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .globe, .emoji, .space, .return]
        ]

        // MARK: TPS Layout (Taiwanese Phonetic Symbols / 方音符號)
        //
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

            // Row 4: ㄇ(ㆬ), ㄋ, ㄫ(ㆭ,ㄙ), ㄘ(ㄑ), ㄜ, ㆦ, ㄢ, ㆰ(ㆱ), ，(。), backspace
            [.char("ㄇ"), .char("ㄋ"), .char("ㄫ"), .char("ㄘ"), .char("ㄜ"), .char("ㆦ"), .char("ㄢ"), .char("ㆰ"), .char("，"), .backspace],

            // Row 5: ?123, ㄌ, ㆡ(ㆢ), ㄙ(ㄒ), space, ㆨ, ㆤ(ㄝ), enter
            [.numeric, .char("ㄌ"), .char("ㆡ"), .char("ㄙ"), .space, .char("ㆨ"), .char("ㆤ"), .return]
        ]

        /// TPS - iPhone SE / iPad (with globe key)
        static let tps_withGlobe: [[KeyDef]] = [
            // Row 1: ㆠ, ˋ, ˪, ㆣ, ˊ, ˇ, ˫, ˙, ㆪ, ㆩ
            [.char("ㆠ"), .char("ˋ"), .char("˪"), .char("ㆣ"), .char("ˊ"), .char("ˇ"), .char("˫"), .char("˙"), .char("ㆪ"), .char("ㆩ")],

            // Row 2: ㄅ(ㆴ), ㄉ(ㆵ), ㄍ(ㆻ), ㄏ(ㆷ), ㄧ, ㄚ, ㄞ, ㄤ, ㆫ, ㆧ
            [.char("ㄅ"), .char("ㄉ"), .char("ㄍ"), .char("ㄏ"), .char("ㄧ"), .char("ㄚ"), .char("ㄞ"), .char("ㄤ"), .char("ㆫ"), .char("ㆧ")],

            // Row 3: ㄆ, ㄊ, ㄎ, ㄗ(ㄐ), ㄨ, ㄛ, ㄠ, ㆲ, ㆥ, ㆮ
            [.char("ㄆ"), .char("ㄊ"), .char("ㄎ"), .char("ㄗ"), .char("ㄨ"), .char("ㄛ"), .char("ㄠ"), .char("ㆲ"), .char("ㆥ"), .char("ㆮ")],

            // Row 4: ㄇ(ㆬ), ㄋ, ㄫ(ㆭ,ㄙ), ㄘ(ㄑ), ㄜ, ㆦ, ㄢ, ㆰ(ㆱ), ，(。), backspace
            [.char("ㄇ"), .char("ㄋ"), .char("ㄫ"), .char("ㄘ"), .char("ㄜ"), .char("ㆦ"), .char("ㄢ"), .char("ㆰ"), .char("，"), .backspace],

            // Row 5: ?123, globe, ㄌ, ㆡ(ㆢ), ㄙ(ㄒ), space, ㆨ, ㆤ(ㄝ), enter
            [.numeric, .globe, .char("ㄌ"), .char("ㆡ"), .char("ㄙ"), .space, .char("ㆨ"), .char("ㆤ"), .return]
        ]

        // MARK: MOE Layout 1 (教育部輸入法佈局1) - TL version

        /// MOE1 TL - iPhone (no globe key)
        static let moe1_TL_iPhone: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("*"), .char("?", fullWidth: "？"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l"), .char("-")],
            [.shift, .char(",", fullWidth: "，"), .char(".", fullWidth: "。"), .char("("), .char(")"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .emoji, .space, .translate, .return]
        ]

        /// MOE1 TL - iPhone SE / iPad (with globe key)
        static let moe1_TL_withGlobe: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("*"), .char("?", fullWidth: "？"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l"), .char("-")],
            [.shift, .char(",", fullWidth: "，"), .char(".", fullWidth: "。"), .char("("), .char(")"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .globe, .emoji, .space, .translate, .return]
        ]

        // MARK: MOE Layout 1 (教育部輸入法佈局1) - POJ version

        /// MOE1 POJ - iPhone (no globe key)
        static let moe1_POJ_iPhone: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("*"), .char("?", fullWidth: "？"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l"), .char("-")],
            [.shift, .char(",", fullWidth: "，"), .char(".", fullWidth: "。"), .char("c"), .char("v"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .emoji, .space, .translate, .return]
        ]

        /// MOE1 POJ - iPhone SE / iPad (with globe key)
        static let moe1_POJ_withGlobe: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("*"), .char("?", fullWidth: "？"), .char("e"), .char("r"), .char("t"), .char("y"), .char("u"), .char("i"), .char("o"), .char("p")],
            [.char("a"), .char("s"), .char("d"), .char("f"), .char("g"), .char("h"), .char("j"), .char("k"), .char("l"), .char("-")],
            [.shift, .char(",", fullWidth: "，"), .char(".", fullWidth: "。"), .char("c"), .char("v"), .char("b"), .char("n"), .char("m"), .backspace],
            [.numeric, .globe, .emoji, .space, .translate, .return]
        ]

        // MARK: MOE Layout 2 (教育部輸入法佈局2) - TL version

        /// MOE2 TL - iPhone (no globe key)
        static let moe2_TL_iPhone: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("p"), .char("ph"), .char("m"), .char("b"), .char("ts"), .char("tsh"), .char("a"), .char("i"), .char("o"), .char("oo")],
            [.char("t"), .char("th"), .char("n"), .char("l"), .char("s"), .char("j"), .char("u"), .char("e"), .char("r"), .char("-")],
            [.shift, .char("k"), .char("kh"), .char("ng"), .char("g"), .char("h"), .char("nn"), .char("?", fullWidth: "？"), .backspace],
            [.numeric, .emoji, .char(",", fullWidth: "，"), .space, .char(".", fullWidth: "。"), .translate, .return]
        ]

        /// MOE2 TL - iPhone SE / iPad (with globe key)
        static let moe2_TL_withGlobe: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("p"), .char("ph"), .char("m"), .char("b"), .char("ts"), .char("tsh"), .char("a"), .char("i"), .char("o"), .char("oo")],
            [.char("t"), .char("th"), .char("n"), .char("l"), .char("s"), .char("j"), .char("u"), .char("e"), .char("r"), .char("-")],
            [.shift, .char("k"), .char("kh"), .char("ng"), .char("g"), .char("h"), .char("nn"), .char("?", fullWidth: "？"), .backspace],
            [.numeric, .globe, .emoji, .char(",", fullWidth: "，"), .space, .char(".", fullWidth: "。"), .translate, .return]
        ]

        // MARK: MOE Layout 2 (教育部輸入法佈局2) - POJ version

        /// MOE2 POJ - iPhone (no globe key)
        static let moe2_POJ_iPhone: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("p"), .char("ph"), .char("m"), .char("b"), .char("ch"), .char("chh"), .char("a"), .char("i"), .char("o"), .char("o\u{0358}")],
            [.char("t"), .char("th"), .char("n"), .char("l"), .char("s"), .char("j"), .char("u"), .char("e"), .char("r"), .char("-")],
            [.shift, .char("k"), .char("kh"), .char("ng"), .char("g"), .char("h"), .char("nn"), .char("?", fullWidth: "？"), .backspace],
            [.numeric, .emoji, .char(",", fullWidth: "，"), .space, .char(".", fullWidth: "。"), .translate, .return]
        ]

        /// MOE2 POJ - iPhone SE / iPad (with globe key)
        static let moe2_POJ_withGlobe: [[KeyDef]] = [
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"), .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],
            [.char("p"), .char("ph"), .char("m"), .char("b"), .char("ch"), .char("chh"), .char("a"), .char("i"), .char("o"), .char("o\u{0358}")],
            [.char("t"), .char("th"), .char("n"), .char("l"), .char("s"), .char("j"), .char("u"), .char("e"), .char("r"), .char("-")],
            [.shift, .char("k"), .char("kh"), .char("ng"), .char("g"), .char("h"), .char("nn"), .char("?", fullWidth: "？"), .backspace],
            [.numeric, .globe, .emoji, .char(",", fullWidth: "，"), .space, .char(".", fullWidth: "。"), .translate, .return]
        ]
    }

    // ========================================
    // MARK: - Numeric Keyboard (Common Symbols)
    // ========================================
    // 5-row design: Numbers + Basic punctuation + Common brackets + Function row + Bottom row

    enum Numeric {

        /// Numeric - iPhone (no globe key)
        static let iPhone: [[KeyDef]] = [
            // Row 1: Numbers
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"),
             .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],

            // Row 2: Quotation marks and CJK brackets (half-width uses curly quotes)
            [.char("\u{201C}", fullWidth: "「"), .char("\u{201D}", fullWidth: "」"),
             .char("\u{2018}", fullWidth: "『"), .char("\u{2019}", fullWidth: "』"),
             .char("（"), .char("）"),
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
            [.alphabetic, .emoji, .space, .translate, .return]
        ]

        /// Numeric - iPhone SE / iPad (with globe key)
        static let withGlobe: [[KeyDef]] = [
            // Row 1: Numbers
            [.char("1"), .char("2"), .char("3"), .char("4"), .char("5"),
             .char("6"), .char("7"), .char("8"), .char("9"), .char("0")],

            // Row 2: Quotation marks and CJK brackets (half-width uses curly quotes)
            [.char("\u{201C}", fullWidth: "「"), .char("\u{201D}", fullWidth: "」"),
             .char("\u{2018}", fullWidth: "『"), .char("\u{2019}", fullWidth: "』"),
             .char("（"), .char("）"),
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
            [.alphabetic, .globe, .emoji, .space, .translate, .return]
        ]
    }

    // ========================================
    // MARK: - Symbolic Keyboard (Advanced Symbols)
    // ========================================
    // 5-row design: Programming brackets + Currency + Book title marks + Function row + Bottom row
    // Note: Does not overlap with Numeric keyboard

    enum Symbolic {

        /// Symbolic - iPhone (no globe key)
        static let iPhone: [[KeyDef]] = [
            // Row 1: Programming brackets (half-width, ｛｝ have full-width versions)
            [.char("〔"), .char("〕"),
             .char("{", fullWidth: "｛"), .char("}", fullWidth: "｝"),
             .char("«"), .char("»"),
             .char("<"), .char(">"),
             .char("^"), .char("※")],

            // Row 2: Book title marks (always full-width)
            [.char("〈"), .char("〉"),
             .char("《"), .char("》"),
             .char("|"), .char("~"),
             .char("\\"), .char("/"),
             .char("_"), .char("=")],

            // Row 3: Currency and special symbols
            [.char("€"), .char("£"), .char("¥"), .char("¢"),
             .char("•", fullWidth: "·"), .char("°"),
             .char("©"), .char("®"), .char("™"), .char("℃")],

            // Row 4: Math symbols (natural finger position)
            [.numeric,
             .char("±"), .char("×"), .char("÷"),
             .char("≠"), .char("≈"), .char("∞"), .char("√"), .backspace],

            // Row 5: Bottom row
            [.alphabetic, .emoji, .space, .translate, .return]
        ]

        /// Symbolic - iPhone SE / iPad (with globe key)
        static let withGlobe: [[KeyDef]] = [
            // Row 1: Programming brackets (half-width, ｛｝ have full-width versions)
            [.char("〔"), .char("〕"),
             .char("{", fullWidth: "｛"), .char("}", fullWidth: "｝"),
             .char("«"), .char("»"),
             .char("<"), .char(">"),
             .char("^"), .char("※")],

            // Row 2: Book title marks (always full-width)
            [.char("〈"), .char("〉"),
             .char("《"), .char("》"),
             .char("|"), .char("~"),
             .char("\\"), .char("/"),
             .char("_"), .char("=")],

            // Row 3: Currency and special symbols
            [.char("€"), .char("£"), .char("¥"), .char("¢"),
             .char("•", fullWidth: "·"), .char("°"),
             .char("©"), .char("®"), .char("™"), .char("℃")],

            // Row 4: Math symbols (natural finger position)
            [.numeric,
             .char("±"), .char("×"), .char("÷"),
             .char("≠"), .char("≈"), .char("∞"), .char("√"), .backspace],

            // Row 5: Bottom row
            [.alphabetic, .globe, .emoji, .space, .translate, .return]
        ]
    }
}
