import Foundation

// MARK: - Flick 鍵盤 Tab 類型

/// Flick 鍵盤內部 Tab 類型
enum FlickTab {
    case taigi   // 台語主鍵盤
    case number  // 數字符號
    case abc     // ABC 英文
}

// MARK: - Flick 方向

/// Flick 滑動方向
enum FlickDirection: CaseIterable {
    case left   // ← 第2聲
    case top    // ↑ 第3聲
    case right  // → 第5聲
    case bottom // ↓ 第7聲
}

// MARK: - Flick 標籤顯示樣式

/// Flick 按鍵標籤顯示樣式（參考 azooKey KeyLabelType）
enum FlickLabelStyle {
    /// 四向提示模式：主字在中央，四向提示在邊緣
    /// 用於母音（聲調）、子音（分組）按鍵
    case directional

    /// 垂直副字模式：主字在上，副字（joined）在下
    /// 用於數字、符號按鍵
    case verticalSub
}

// MARK: - Flick 按鍵類型

/// Flick 按鍵定義
///
/// 支援三種類型：
/// - 母音按鍵：中心 + 四向聲調變化 + 長按第8聲
/// - 子音按鍵：中心 + 四向子音分組
/// - 系統按鍵：功能鍵（刪除、空白、Enter 等）
enum FlickKeyDef {
    /// 母音按鍵（聲調 Flick）
    /// - Parameters:
    ///   - base: 基本母音（如 "a"）
    ///   - tones: 四向聲調對應 [左:2聲, 上:3聲, 右:5聲, 下:7聲]
    ///   - tone8: 長按第8聲（如 "a̍"）
    case vowel(base: String, tones: [FlickDirection: String], tone8: String)

    /// 子音按鍵（子音分組 Flick）
    /// - Parameters:
    ///   - center: 中心子音
    ///   - label: 按鍵標籤（顯示用）
    ///   - variations: 四向子音變化
    case consonant(center: String, label: String, variations: [FlickDirection: String])

    /// 符號按鍵
    /// - Parameters:
    ///   - center: 中心符號
    ///   - label: 按鍵標籤
    ///   - variations: 四向符號變化
    case symbol(center: String, label: String, variations: [FlickDirection: String])

    /// 系統功能鍵
    case system(FlickSystemKey)
}

// MARK: - 系統功能鍵

/// Flick 鍵盤系統功能鍵
enum FlickSystemKey: CustomStringConvertible {
    case delete              // 刪除
    case space               // 空白
    case enter               // Enter
    case globe               // 切換輸入法
    case switchToQwerty      // 切換到 QWERTY
    case translate           // 翻譯（漢字/羅馬字切換）
    // Flick 內部 Tab 切換
    case flickTabTaigi       // 切換到台語 Flick（主鍵盤）
    case flickTabNumber      // 切換到數字符號 Flick
    case flickTabAbc         // 切換到 ABC Flick
    case flickTabShift       // 大小寫切換

    var description: String {
        switch self {
        case .delete: return "delete"
        case .space: return "space"
        case .enter: return "enter"
        case .globe: return "globe"
        case .switchToQwerty: return "switchToQwerty"
        case .translate: return "translate"
        case .flickTabTaigi: return "flickTabTaigi"
        case .flickTabNumber: return "flickTabNumber"
        case .flickTabAbc: return "flickTabAbc"
        case .flickTabShift: return "flickTabShift"
        }
    }
}

// MARK: - Flick 按鍵位置

/// Flick 按鍵在網格中的位置
struct FlickKeyPosition: Hashable {
    let row: Int    // 列（0-3）
    let column: Int // 欄（0-5）

    /// 按鍵寬度（預設 1）
    var width: Int = 1
    /// 按鍵高度（預設 1）
    var height: Int = 1

    init(row: Int, column: Int, width: Int = 1, height: Int = 1) {
        self.row = row
        self.column = column
        self.width = width
        self.height = height
    }
}

// MARK: - Flick 佈局

/// Flick 鍵盤佈局定義
struct FlickLayout {
    /// 佈局識別碼
    let identifier: String

    /// 顯示名稱
    let displayName: String

    /// 列數
    let rowCount: Int

    /// 行數
    let columnCount: Int

    /// 按鍵定義（位置 -> 按鍵）
    let keys: [FlickKeyPosition: FlickKeyDef]

    init(
        identifier: String,
        displayName: String,
        rowCount: Int = 4,
        columnCount: Int = 6,
        keys: [FlickKeyPosition: FlickKeyDef]
    ) {
        self.identifier = identifier
        self.displayName = displayName
        self.rowCount = rowCount
        self.columnCount = columnCount
        self.keys = keys
    }
}

// MARK: - 便捷建構方法

extension FlickKeyDef {
    /// 建立母音聲調按鍵
    /// - Parameters:
    ///   - base: 基本母音
    ///   - tone2: 第2聲（左滑）
    ///   - tone3: 第3聲（上滑）
    ///   - tone5: 第5聲（右滑）
    ///   - tone7: 第7聲（下滑）
    ///   - tone8: 第8聲（長按）
    static func vowelTone(
        _ base: String,
        tone2: String,
        tone3: String,
        tone5: String,
        tone7: String,
        tone8: String
    ) -> FlickKeyDef {
        .vowel(
            base: base,
            tones: [
                .left: tone2,
                .top: tone3,
                .right: tone5,
                .bottom: tone7
            ],
            tone8: tone8
        )
    }

    /// 建立子音分組按鍵
    /// - Parameters:
    ///   - center: 中心子音（點擊輸入）
    ///   - label: 按鍵標籤
    ///   - left: 左滑子音
    ///   - top: 上滑子音
    ///   - right: 右滑子音
    ///   - bottom: 下滑子音
    static func consonantGroup(
        _ center: String,
        label: String? = nil,
        left: String? = nil,
        top: String? = nil,
        right: String? = nil,
        bottom: String? = nil
    ) -> FlickKeyDef {
        var variations: [FlickDirection: String] = [:]
        if let left = left { variations[.left] = left }
        if let top = top { variations[.top] = top }
        if let right = right { variations[.right] = right }
        if let bottom = bottom { variations[.bottom] = bottom }

        return .consonant(
            center: center,
            label: label ?? center,
            variations: variations
        )
    }

    /// 建立符號分組按鍵
    static func symbolGroup(
        _ center: String,
        label: String? = nil,
        left: String? = nil,
        top: String? = nil,
        right: String? = nil,
        bottom: String? = nil
    ) -> FlickKeyDef {
        var variations: [FlickDirection: String] = [:]
        if let left = left { variations[.left] = left }
        if let top = top { variations[.top] = top }
        if let right = right { variations[.right] = right }
        if let bottom = bottom { variations[.bottom] = bottom }

        return .symbol(
            center: center,
            label: label ?? center,
            variations: variations
        )
    }
}

// MARK: - FlickKeyDef 輔助屬性

extension FlickKeyDef {
    /// 取得按鍵的顯示標籤
    var displayLabel: String {
        switch self {
        case .vowel(let base, _, _):
            return base
        case .consonant(_, let label, _):
            return label
        case .symbol(_, let label, _):
            return label
        case .system(let key):
            return key.displayLabel
        }
    }

    /// 取得中心字符（點擊輸入的字符）
    var centerCharacter: String? {
        switch self {
        case .vowel(let base, _, _):
            return base
        case .consonant(let center, _, _):
            return center
        case .symbol(let center, _, _):
            return center
        case .system:
            return nil
        }
    }

    /// 取得指定方向的變化字符
    func variation(for direction: FlickDirection) -> String? {
        switch self {
        case .vowel(_, let tones, _):
            return tones[direction]
        case .consonant(_, _, let variations):
            return variations[direction]
        case .symbol(_, _, let variations):
            return variations[direction]
        case .system:
            return nil
        }
    }

    /// 取得長按字符（僅母音有第8聲）
    var longPressCharacter: String? {
        switch self {
        case .vowel(_, _, let tone8):
            return tone8
        default:
            return nil
        }
    }

    /// 是否為系統按鍵
    var isSystemKey: Bool {
        if case .system = self { return true }
        return false
    }

    /// 標籤顯示樣式
    ///
    /// - 母音、子音：四向提示模式（提示在邊緣）
    /// - 符號：垂直副字模式（副字在主字下方）
    var labelStyle: FlickLabelStyle {
        switch self {
        case .vowel, .consonant:
            return .directional
        case .symbol:
            return .verticalSub
        case .system:
            return .directional
        }
    }

    /// 取得所有變化字符（用於垂直副字模式顯示）
    /// 順序：左、上、右、下
    var allVariations: [String] {
        let order: [FlickDirection] = [.left, .top, .right, .bottom]
        return order.compactMap { variation(for: $0) }
    }
}

// MARK: - FlickSystemKey 輔助屬性

extension FlickSystemKey {
    /// 系統按鍵的顯示標籤
    var displayLabel: String {
        switch self {
        case .delete:
            return "⌫"
        case .space:
            return "空白"
        case .enter:
            return "確定"
        case .globe:
            return "🌐"
        case .switchToQwerty:
            return "ABC"
        case .translate:
            return "譯"
        case .flickTabTaigi:
            return "台語"
        case .flickTabNumber:
            return "☆123"
        case .flickTabAbc:
            return "ABC"
        case .flickTabShift:
            return "a/A"
        }
    }

    /// 系統按鍵的圖示名稱（SF Symbol）
    var systemImageName: String? {
        switch self {
        case .delete:
            return "delete.left"
        case .space:
            return nil
        case .enter:
            return nil
        case .globe:
            return "globe"
        case .translate:
            return "arrow.left.arrow.right"
        case .switchToQwerty, .flickTabTaigi, .flickTabNumber, .flickTabAbc:
            return nil
        case .flickTabShift:
            return "textformat"
        }
    }
}
