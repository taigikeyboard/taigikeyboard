import Foundation

/// Dictionary source enum matching DB column names
enum DictionarySource: String, CaseIterable {
    case kautian    // 教育部臺灣台語常用詞辭典
    case taigitv    // 台語新詞辭庫
    case itaigi     // iTaigi 華台對照典
    case sitbut     // 台灣植物名彙
    case taihoa     // 台華線頂對照典
    case taijit     // 台日大辭典
    case kungge     // 台語工藝詞庫
    case stti       // 學科術語辭典
    case khpoo      // 腔口補充資料
    case khiin      // 在來字
    case lkk        // LKK漢羅合用建議用字
    case dev        // 開發補充資料
    case custom     // 自訂詞庫

    /// Short display name for badge
    var displayName: String {
        switch self {
        case .kautian: return "教典"
        case .taigitv: return "台語新詞"
        case .itaigi:  return "iTaigi"
        case .sitbut:  return "植物名彙"
        case .taihoa:  return "台華對照"
        case .taijit:  return "臺日"
        case .kungge:  return "工藝辭典"
        case .stti:    return "學科術語"
        case .khpoo:   return "補充資料"
        case .khiin:   return "補充資料"
        case .lkk:     return "漢羅合用"
        case .dev:     return "補充資料"
        case .custom:  return "補充資料"
        }
    }
}

/// Search result with source information for dictionary exploration
struct DictionarySearchResult {
    let id: Int
    let roman: String       // Display form (POJ or TL based on user setting)
    let tl: String          // Raw TL from database (for Chhoe Taigi URL)
    let hanzi: String?
    let frequency: Int
    let sources: [DictionarySource]

    /// Build Chhoe Taigi lookup URL using TL digit form
    var chhoeURL: URL? {
        let tlDigit = Self.toTLDigit(tl)
        guard !tlDigit.isEmpty else { return nil }
        guard let encoded = tlDigit.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) else {
            return nil
        }
        return URL(string: "https://chhoe.taigi.info/s?s=su&f=e&lmjf=ki&lmj=\(encoded)")
    }

    /// Build MOE Dictionary lookup URL using TL digit form
    var moeURL: URL? {
        let tlDigit = Self.toTLDigit(tl)
        guard !tlDigit.isEmpty else { return nil }
        guard let encoded = tlDigit.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) else {
            return nil
        }
        return URL(string: "https://sutian.moe.edu.tw/zh-hant/tshiau/?lui=tai_su&tsha=\(encoded)")
    }

    /// Convert TL display form (with diacritics) to TL digit form for URL
    /// e.g. "tāi-tsì" → "tai7-tsi3"
    static func toTLDigit(_ tl: String) -> String {
        let syllables = tl.lowercased().split(separator: "-", omittingEmptySubsequences: false)
        let converted = syllables.map { syllable -> String in
            normalizeSyllableToDigit(String(syllable))
        }
        return converted.joined(separator: "-")
    }

    /// Normalize a single syllable from diacritics to digit tone
    private static func normalizeSyllableToDigit(_ syllable: String) -> String {
        guard !syllable.isEmpty else { return "" }

        let withNasalConverted = syllable
            .replacingOccurrences(of: "\u{207F}", with: "nn")
            .replacingOccurrences(of: "\u{1D3A}", with: "nn")

        // Already has digit tone — strip tone 1 and 4 for external dictionary URLs
        if let lastChar = withNasalConverted.last, lastChar.isNumber {
            let tone = String(lastChar)
            if tone == "1" || tone == "4" {
                return String(withNasalConverted.dropLast())
            }
            return withNasalConverted
        }

        let nfd = withNasalConverted.decomposedStringWithCanonicalMapping
        let withOoConverted = nfd.replacingOccurrences(of: "\u{0358}", with: "o")

        var toneNumber = ""
        var withoutTone = ""

        for scalar in withOoConverted.unicodeScalars {
            if let tone = TaigiPhonetics.combiningToToneNum[scalar] {
                toneNumber = tone
            } else {
                withoutTone.append(String(scalar))
            }
        }

        // Tone 1 (open) and 4 (checked) are omitted in external dictionary URLs
        if toneNumber.isEmpty || toneNumber == "1" || toneNumber == "4" {
            return withoutTone
        }

        return withoutTone + toneNumber
    }
}
