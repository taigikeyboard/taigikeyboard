import Foundation

/// 台語 Flick 鍵盤佈局
///
/// 基於教育部臺灣閩南語羅馬字拼音方案（TL）設計
/// 佈局結構參考 azooKey（4列×6欄）
/// - 母音按鍵：Flick 輸入聲調（1-8聲）
/// - 子音按鍵：Flick 輸入相關子音分組
/// - 特殊按鍵：鼻化標記、標點符號
enum FlickTaigiLayout {

    // MARK: - 主佈局

    /// 台語聲調 Flick 鍵盤（TL 模式）
    ///
    /// 佈局結構（4列×6欄，參考 azooKey）：
    /// ```
    ///          x=0       x=1      x=2      x=3      x=4      x=5
    /// y=0      ☆123      a        e        i        o        ⌫
    /// y=1      ABC       oo       u        p        k        空白
    /// y=2      譯        ts       n       nn，。   「」『』   改行
    /// y=3      🌐        ，。     ？！     《》〈〉  —⋯       ┘
    /// ```
    static let taigiTone = FlickLayout(
        identifier: "taigi_tone_flick",
        displayName: "台語聲調",
        rowCount: 4,
        columnCount: 6,
        keys: [
            // ========================================
            // Column 0 (x=0): Tab 切換欄
            // ========================================
            FlickKeyPosition(row: 0, column: 0): .system(.flickTabNumber),  // ☆123
            FlickKeyPosition(row: 1, column: 0): .system(.flickTabAbc),     // ABC
            FlickKeyPosition(row: 2, column: 0): .system(.translate),       // 譯
            FlickKeyPosition(row: 3, column: 0): .system(.globe),           // 🌐

            // ========================================
            // Column 1 (x=1): 字符欄 1
            // ========================================
            FlickKeyPosition(row: 0, column: 1): vowelA,                    // a
            FlickKeyPosition(row: 1, column: 1): vowelOO,                   // oo
            FlickKeyPosition(row: 2, column: 1): consonantTs,               // ts
            FlickKeyPosition(row: 3, column: 1): punctuationComma,          // ，。

            // ========================================
            // Column 2 (x=2): 字符欄 2
            // ========================================
            FlickKeyPosition(row: 0, column: 2): vowelE,                    // e
            FlickKeyPosition(row: 1, column: 2): vowelU,                    // u
            FlickKeyPosition(row: 2, column: 2): consonantN,                // n
            FlickKeyPosition(row: 3, column: 2): punctuationQuestion,       // ？！

            // ========================================
            // Column 3 (x=3): 字符欄 3
            // ========================================
            FlickKeyPosition(row: 0, column: 3): vowelI,                    // i
            FlickKeyPosition(row: 1, column: 3): consonantP,                // p
            FlickKeyPosition(row: 2, column: 3): nasalAndPunctuation,       // nn，。
            FlickKeyPosition(row: 3, column: 3): punctuationBookTitle,      // 《》〈〉

            // ========================================
            // Column 4 (x=4): 字符欄 4
            // ========================================
            FlickKeyPosition(row: 0, column: 4): vowelO,                    // o
            FlickKeyPosition(row: 1, column: 4): consonantK,                // k
            FlickKeyPosition(row: 2, column: 4): quoteBrackets,             // 「」『』
            FlickKeyPosition(row: 3, column: 4): punctuationDash,           // —⋯

            // ========================================
            // Column 5 (x=5): 系統功能欄
            // ========================================
            FlickKeyPosition(row: 0, column: 5): .system(.delete),          // ⌫
            FlickKeyPosition(row: 1, column: 5): .system(.space),           // 空白
            FlickKeyPosition(row: 2, column: 5, height: 2): .system(.enter), // 改行（跨2列）
        ]
    )

    // MARK: - 母音按鍵定義（聲調 Flick）

    /// 母音 a：中心=a, 左=á(2), 上=à(3), 右=â(5), 下=ā(7), 長按=a̍(8)
    static let vowelA: FlickKeyDef = .vowelTone(
        "a",
        tone2: "á", tone3: "à", tone5: "â", tone7: "ā", tone8: "a̍"
    )

    /// 母音 e：中心=e, 左=é(2), 上=è(3), 右=ê(5), 下=ē(7), 長按=e̍(8)
    static let vowelE: FlickKeyDef = .vowelTone(
        "e",
        tone2: "é", tone3: "è", tone5: "ê", tone7: "ē", tone8: "e̍"
    )

    /// 母音 i：中心=i, 左=í(2), 上=ì(3), 右=î(5), 下=ī(7), 長按=i̍(8)
    static let vowelI: FlickKeyDef = .vowelTone(
        "i",
        tone2: "í", tone3: "ì", tone5: "î", tone7: "ī", tone8: "i̍"
    )

    /// 母音 o：中心=o, 左=ó(2), 上=ò(3), 右=ô(5), 下=ō(7), 長按=o̍(8)
    static let vowelO: FlickKeyDef = .vowelTone(
        "o",
        tone2: "ó", tone3: "ò", tone5: "ô", tone7: "ō", tone8: "o̍"
    )

    /// 母音 oo (o͘)：中心=oo, 左=óo(2), 上=òo(3), 右=ôo(5), 下=ōo(7), 長按=o̍o(8)
    static let vowelOO: FlickKeyDef = .vowelTone(
        "oo",
        tone2: "óo", tone3: "òo", tone5: "ôo", tone7: "ōo", tone8: "o̍o"
    )

    /// 母音 u：中心=u, 左=ú(2), 上=ù(3), 右=û(5), 下=ū(7), 長按=u̍(8)
    static let vowelU: FlickKeyDef = .vowelTone(
        "u",
        tone2: "ú", tone3: "ù", tone5: "û", tone7: "ū", tone8: "u̍"
    )

    // MARK: - 子音按鍵定義（分組 Flick）

    /// 子音 p 組：中心=p, 左=ph, 上=b, 右=m
    static let consonantP: FlickKeyDef = .consonantGroup(
        "p",
        label: "p",
        left: "ph",
        top: "b",
        right: "m"
    )

    /// 子音 k 組：中心=k, 左=kh, 上=g, 右=ng, 下=h
    static let consonantK: FlickKeyDef = .consonantGroup(
        "k",
        label: "k",
        left: "kh",
        top: "g",
        right: "ng",
        bottom: "h"
    )

    /// 子音 ts 組：中心=ts, 左=tsh, 上=s, 右=j, 下=l
    static let consonantTs: FlickKeyDef = .consonantGroup(
        "ts",
        label: "ts",
        left: "tsh",
        top: "s",
        right: "j",
        bottom: "l"
    )

    /// 子音 n 組：中心=n, 左=th, 上=t
    static let consonantN: FlickKeyDef = .consonantGroup(
        "n",
        label: "n",
        left: "th",
        top: "t"
    )

    // MARK: - 特殊按鍵定義

    /// 鼻化標記 + 標點：中心=nn, 左=，, 上=。, 右=？, 下=！
    static let nasalAndPunctuation: FlickKeyDef = .symbolGroup(
        "nn",
        label: "nn",
        left: "，",
        top: "。",
        right: "？",
        bottom: "！"
    )

    /// 引號括號：中心=「, 左=」, 上=『, 右=』
    static let quoteBrackets: FlickKeyDef = .symbolGroup(
        "「",
        label: "「」",
        left: "」",
        top: "『",
        right: "』"
    )

    /// 逗號句號：中心=，, 左=。, 上=、, 右=；, 下=：
    static let punctuationComma: FlickKeyDef = .symbolGroup(
        "，",
        label: "，。",
        left: "。",
        top: "、",
        right: "；",
        bottom: "："
    )

    /// 問號驚嘆：中心=？, 左=！, 上=～, 右=‥, 下=…
    static let punctuationQuestion: FlickKeyDef = .symbolGroup(
        "？",
        label: "？！",
        left: "！",
        top: "～",
        right: "‥",
        bottom: "…"
    )

    /// 書名號：中心=《, 左=》, 上=〈, 右=〉, 下=【】
    static let punctuationBookTitle: FlickKeyDef = .symbolGroup(
        "《",
        label: "《》",
        left: "》",
        top: "〈",
        right: "〉",
        bottom: "【"
    )

    /// 破折號省略號：中心=—, 左=⋯, 上=－, 右=＿
    static let punctuationDash: FlickKeyDef = .symbolGroup(
        "—",
        label: "—⋯",
        left: "⋯",
        top: "－",
        right: "＿"
    )
}

// MARK: - POJ 模式佈局

extension FlickTaigiLayout {

    /// 台語聲調 Flick 鍵盤（POJ 模式）
    static let taigiTonePOJ = FlickLayout(
        identifier: "taigi_tone_flick_poj",
        displayName: "台語聲調 (POJ)",
        rowCount: 4,
        columnCount: 6,
        keys: [
            // Column 0: Tab 切換欄
            FlickKeyPosition(row: 0, column: 0): .system(.flickTabNumber),
            FlickKeyPosition(row: 1, column: 0): .system(.flickTabAbc),
            FlickKeyPosition(row: 2, column: 0): .system(.translate),
            FlickKeyPosition(row: 3, column: 0): .system(.globe),

            // Column 1: 字符欄 1
            FlickKeyPosition(row: 0, column: 1): vowelA,
            FlickKeyPosition(row: 1, column: 1): vowelODot,                 // POJ: o͘
            FlickKeyPosition(row: 2, column: 1): consonantCh,               // POJ: ch
            FlickKeyPosition(row: 3, column: 1): punctuationComma,

            // Column 2: 字符欄 2
            FlickKeyPosition(row: 0, column: 2): vowelE,
            FlickKeyPosition(row: 1, column: 2): vowelU,
            FlickKeyPosition(row: 2, column: 2): consonantN,
            FlickKeyPosition(row: 3, column: 2): punctuationQuestion,

            // Column 3: 字符欄 3
            FlickKeyPosition(row: 0, column: 3): vowelI,
            FlickKeyPosition(row: 1, column: 3): consonantP,
            FlickKeyPosition(row: 2, column: 3): nasalAndPunctuation,
            FlickKeyPosition(row: 3, column: 3): punctuationBookTitle,

            // Column 4: 字符欄 4
            FlickKeyPosition(row: 0, column: 4): vowelO,
            FlickKeyPosition(row: 1, column: 4): consonantK,
            FlickKeyPosition(row: 2, column: 4): quoteBrackets,
            FlickKeyPosition(row: 3, column: 4): punctuationDash,

            // Column 5: 系統功能欄
            FlickKeyPosition(row: 0, column: 5): .system(.delete),
            FlickKeyPosition(row: 1, column: 5): .system(.space),
            FlickKeyPosition(row: 2, column: 5, height: 2): .system(.enter),
        ]
    )

    /// POJ 母音 o͘：帶點的 o
    static let vowelODot: FlickKeyDef = .vowelTone(
        "o͘",
        tone2: "ó͘", tone3: "ò͘", tone5: "ô͘", tone7: "ō͘", tone8: "o̍͘"
    )

    /// POJ 子音 ch 組：中心=ch, 左=chh, 上=s, 右=j, 下=l
    static let consonantCh: FlickKeyDef = .consonantGroup(
        "ch",
        label: "ch",
        left: "chh",
        top: "s",
        right: "j",
        bottom: "l"
    )
}

// MARK: - 數字符號 Flick 佈局

extension FlickTaigiLayout {

    /// 數字符號 Flick 鍵盤
    ///
    /// 佈局結構（4列×6欄）：
    /// ```
    ///          x=0       x=1      x=2      x=3      x=4      x=5
    /// y=0      ☆123      1        2        3        4        ⌫
    /// y=1      ABC       5        6        7        8        空白
    /// y=2      台語      9        0       ()[]     .,-/      改行
    /// y=3      🌐       ¥$€      %°#     +-×÷     <=>       ┘
    /// ```
    static let flickNumber = FlickLayout(
        identifier: "flick_number",
        displayName: "數字符號",
        rowCount: 4,
        columnCount: 6,
        keys: [
            // Column 0: Tab 切換欄
            FlickKeyPosition(row: 0, column: 0): .system(.flickTabNumber),
            FlickKeyPosition(row: 1, column: 0): .system(.flickTabAbc),
            FlickKeyPosition(row: 2, column: 0): .system(.flickTabTaigi),
            FlickKeyPosition(row: 3, column: 0): .system(.globe),

            // Column 1: 數字欄 1
            FlickKeyPosition(row: 0, column: 1): number1,
            FlickKeyPosition(row: 1, column: 1): number5,
            FlickKeyPosition(row: 2, column: 1): number9,
            FlickKeyPosition(row: 3, column: 1): symbolCurrency,

            // Column 2: 數字欄 2
            FlickKeyPosition(row: 0, column: 2): number2,
            FlickKeyPosition(row: 1, column: 2): number6,
            FlickKeyPosition(row: 2, column: 2): number0,
            FlickKeyPosition(row: 3, column: 2): symbolPercent,

            // Column 3: 數字欄 3
            FlickKeyPosition(row: 0, column: 3): number3,
            FlickKeyPosition(row: 1, column: 3): number7,
            FlickKeyPosition(row: 2, column: 3): brackets,
            FlickKeyPosition(row: 3, column: 3): symbolMath,

            // Column 4: 數字欄 4
            FlickKeyPosition(row: 0, column: 4): number4,
            FlickKeyPosition(row: 1, column: 4): number8,
            FlickKeyPosition(row: 2, column: 4): punctuationSlash,
            FlickKeyPosition(row: 3, column: 4): symbolCompare,

            // Column 5: 系統功能欄
            FlickKeyPosition(row: 0, column: 5): .system(.delete),
            FlickKeyPosition(row: 1, column: 5): .system(.space),
            FlickKeyPosition(row: 2, column: 5, height: 2): .system(.enter),
        ]
    )

    // MARK: - 數字按鍵

    static let number1: FlickKeyDef = .symbolGroup(
        "1", label: "1",
        left: "①", top: "⑴", right: "⒈"
    )

    static let number2: FlickKeyDef = .symbolGroup(
        "2", label: "2",
        left: "②", top: "⑵", right: "⒉"
    )

    static let number3: FlickKeyDef = .symbolGroup(
        "3", label: "3",
        left: "③", top: "⑶", right: "⒊"
    )

    static let number4: FlickKeyDef = .symbolGroup(
        "4", label: "4",
        left: "④", top: "⑷", right: "⒋"
    )

    static let number5: FlickKeyDef = .symbolGroup(
        "5", label: "5",
        left: "⑤", top: "⑸", right: "⒌"
    )

    static let number6: FlickKeyDef = .symbolGroup(
        "6", label: "6",
        left: "⑥", top: "⑹", right: "⒍"
    )

    static let number7: FlickKeyDef = .symbolGroup(
        "7", label: "7",
        left: "⑦", top: "⑺", right: "⒎"
    )

    static let number8: FlickKeyDef = .symbolGroup(
        "8", label: "8",
        left: "⑧", top: "⑻", right: "⒏"
    )

    static let number9: FlickKeyDef = .symbolGroup(
        "9", label: "9",
        left: "⑨", top: "⑼", right: "⒐"
    )

    static let number0: FlickKeyDef = .symbolGroup(
        "0", label: "0",
        left: "⓪", top: "⑽", right: "⒑"
    )

    static let brackets: FlickKeyDef = .symbolGroup(
        "(", label: "()[]",
        left: ")", top: "[", right: "]", bottom: "{}"
    )

    static let punctuationSlash: FlickKeyDef = .symbolGroup(
        ".", label: ".,-/",
        left: ",", top: "-", right: "/"
    )

    static let symbolCurrency: FlickKeyDef = .symbolGroup(
        "¥", label: "¥$€",
        left: "$", top: "€", right: "£", bottom: "₩"
    )

    static let symbolPercent: FlickKeyDef = .symbolGroup(
        "%", label: "%°#",
        left: "°", top: "#", right: "‰", bottom: "℃"
    )

    static let symbolMath: FlickKeyDef = .symbolGroup(
        "+", label: "+-×÷",
        left: "-", top: "×", right: "÷", bottom: "±"
    )

    static let symbolCompare: FlickKeyDef = .symbolGroup(
        "<", label: "<=>",
        left: "=", top: ">", right: "≤", bottom: "≥"
    )
}

// MARK: - ABC Flick 佈局

extension FlickTaigiLayout {

    /// ABC Flick 鍵盤（英文字母）
    ///
    /// 佈局結構（4列×6欄）：
    /// ```
    ///          x=0       x=1      x=2      x=3      x=4      x=5
    /// y=0      ☆123     @#/&_    ABC      DEF      GHI       ⌫
    /// y=1      abc       JKL      MNO     PQRS      TUV      空白
    /// y=2      台語     WXYZ     '"()    .,?!      a/A       改行
    /// y=3      🌐       0-9      +-=     *&^      _|\\       ┘
    /// ```
    static let flickAbc = FlickLayout(
        identifier: "flick_abc",
        displayName: "ABC",
        rowCount: 4,
        columnCount: 6,
        keys: [
            // Column 0: Tab 切換欄
            FlickKeyPosition(row: 0, column: 0): .system(.flickTabNumber),
            FlickKeyPosition(row: 1, column: 0): .system(.flickTabAbc),
            FlickKeyPosition(row: 2, column: 0): .system(.flickTabTaigi),
            FlickKeyPosition(row: 3, column: 0): .system(.globe),

            // Column 1: 字符欄 1
            FlickKeyPosition(row: 0, column: 1): symbolAtHash,
            FlickKeyPosition(row: 1, column: 1): letterJKL,
            FlickKeyPosition(row: 2, column: 1): letterWXYZ,
            FlickKeyPosition(row: 3, column: 1): symbolNumbers,

            // Column 2: 字符欄 2
            FlickKeyPosition(row: 0, column: 2): letterABC,
            FlickKeyPosition(row: 1, column: 2): letterMNO,
            FlickKeyPosition(row: 2, column: 2): symbolQuotes,
            FlickKeyPosition(row: 3, column: 2): symbolPlusMinus,

            // Column 3: 字符欄 3
            FlickKeyPosition(row: 0, column: 3): letterDEF,
            FlickKeyPosition(row: 1, column: 3): letterPQRS,
            FlickKeyPosition(row: 2, column: 3): symbolPunctuation,
            FlickKeyPosition(row: 3, column: 3): symbolAsterisk,

            // Column 4: 字符欄 4
            FlickKeyPosition(row: 0, column: 4): letterGHI,
            FlickKeyPosition(row: 1, column: 4): letterTUV,
            FlickKeyPosition(row: 2, column: 4): .system(.flickTabShift),
            FlickKeyPosition(row: 3, column: 4): symbolUnderscore,

            // Column 5: 系統功能欄
            FlickKeyPosition(row: 0, column: 5): .system(.delete),
            FlickKeyPosition(row: 1, column: 5): .system(.space),
            FlickKeyPosition(row: 2, column: 5, height: 2): .system(.enter),
        ]
    )

    // MARK: - 英文字母按鍵（小寫輸入）

    static let symbolAtHash: FlickKeyDef = .symbolGroup(
        "@", label: "@#/&_",
        left: "#", top: "/", right: "&", bottom: "_"
    )

    static let letterABC: FlickKeyDef = .consonantGroup(
        "a", label: "ABC",
        left: "b", top: "c"
    )

    static let letterDEF: FlickKeyDef = .consonantGroup(
        "d", label: "DEF",
        left: "e", top: "f"
    )

    static let letterGHI: FlickKeyDef = .consonantGroup(
        "g", label: "GHI",
        left: "h", top: "i"
    )

    static let letterJKL: FlickKeyDef = .consonantGroup(
        "j", label: "JKL",
        left: "k", top: "l"
    )

    static let letterMNO: FlickKeyDef = .consonantGroup(
        "m", label: "MNO",
        left: "n", top: "o"
    )

    static let letterPQRS: FlickKeyDef = .consonantGroup(
        "p", label: "PQRS",
        left: "q", top: "r", right: "s"
    )

    static let letterTUV: FlickKeyDef = .consonantGroup(
        "t", label: "TUV",
        left: "u", top: "v"
    )

    static let letterWXYZ: FlickKeyDef = .consonantGroup(
        "w", label: "WXYZ",
        left: "x", top: "y", right: "z"
    )

    static let symbolQuotes: FlickKeyDef = .symbolGroup(
        "'", label: "'\"()",
        left: "\"", top: "(", right: ")"
    )

    static let symbolPunctuation: FlickKeyDef = .symbolGroup(
        ".", label: ".,?!",
        left: ",", top: "?", right: "!"
    )

    static let symbolNumbers: FlickKeyDef = .symbolGroup(
        "0", label: "0-9",
        left: "1", top: "2", right: "3", bottom: "4"
    )

    static let symbolPlusMinus: FlickKeyDef = .symbolGroup(
        "+", label: "+-=",
        left: "-", top: "=", right: "*"
    )

    static let symbolAsterisk: FlickKeyDef = .symbolGroup(
        "*", label: "*&^",
        left: "&", top: "^", right: "~"
    )

    static let symbolUnderscore: FlickKeyDef = .symbolGroup(
        "_", label: "_|\\",
        left: "|", top: "\\", right: "`"
    )

    // MARK: - ABC 大寫佈局

    /// ABC Flick 鍵盤（大寫英文字母）
    static let flickAbcUpper = FlickLayout(
        identifier: "flick_abc_upper",
        displayName: "ABC",
        rowCount: 4,
        columnCount: 6,
        keys: [
            // Column 0: Tab 切換欄
            FlickKeyPosition(row: 0, column: 0): .system(.flickTabNumber),
            FlickKeyPosition(row: 1, column: 0): .system(.flickTabAbc),
            FlickKeyPosition(row: 2, column: 0): .system(.flickTabTaigi),
            FlickKeyPosition(row: 3, column: 0): .system(.globe),

            // Column 1: 字符欄 1
            FlickKeyPosition(row: 0, column: 1): symbolAtHash,
            FlickKeyPosition(row: 1, column: 1): letterJKLUpper,
            FlickKeyPosition(row: 2, column: 1): letterWXYZUpper,
            FlickKeyPosition(row: 3, column: 1): symbolNumbers,

            // Column 2: 字符欄 2
            FlickKeyPosition(row: 0, column: 2): letterABCUpper,
            FlickKeyPosition(row: 1, column: 2): letterMNOUpper,
            FlickKeyPosition(row: 2, column: 2): symbolQuotes,
            FlickKeyPosition(row: 3, column: 2): symbolPlusMinus,

            // Column 3: 字符欄 3
            FlickKeyPosition(row: 0, column: 3): letterDEFUpper,
            FlickKeyPosition(row: 1, column: 3): letterPQRSUpper,
            FlickKeyPosition(row: 2, column: 3): symbolPunctuation,
            FlickKeyPosition(row: 3, column: 3): symbolAsterisk,

            // Column 4: 字符欄 4
            FlickKeyPosition(row: 0, column: 4): letterGHIUpper,
            FlickKeyPosition(row: 1, column: 4): letterTUVUpper,
            FlickKeyPosition(row: 2, column: 4): .system(.flickTabShift),
            FlickKeyPosition(row: 3, column: 4): symbolUnderscore,

            // Column 5: 系統功能欄
            FlickKeyPosition(row: 0, column: 5): .system(.delete),
            FlickKeyPosition(row: 1, column: 5): .system(.space),
            FlickKeyPosition(row: 2, column: 5, height: 2): .system(.enter),
        ]
    )

    // MARK: - 英文字母按鍵（大寫輸入）

    static let letterABCUpper: FlickKeyDef = .consonantGroup(
        "A", label: "ABC",
        left: "B", top: "C"
    )

    static let letterDEFUpper: FlickKeyDef = .consonantGroup(
        "D", label: "DEF",
        left: "E", top: "F"
    )

    static let letterGHIUpper: FlickKeyDef = .consonantGroup(
        "G", label: "GHI",
        left: "H", top: "I"
    )

    static let letterJKLUpper: FlickKeyDef = .consonantGroup(
        "J", label: "JKL",
        left: "K", top: "L"
    )

    static let letterMNOUpper: FlickKeyDef = .consonantGroup(
        "M", label: "MNO",
        left: "N", top: "O"
    )

    static let letterPQRSUpper: FlickKeyDef = .consonantGroup(
        "P", label: "PQRS",
        left: "Q", top: "R", right: "S"
    )

    static let letterTUVUpper: FlickKeyDef = .consonantGroup(
        "T", label: "TUV",
        left: "U", top: "V"
    )

    static let letterWXYZUpper: FlickKeyDef = .consonantGroup(
        "W", label: "WXYZ",
        left: "X", top: "Y", right: "Z"
    )
}
