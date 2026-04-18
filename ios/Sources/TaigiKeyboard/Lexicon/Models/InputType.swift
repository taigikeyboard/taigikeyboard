// MARK: - Shared-Core Candidate
// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// 輸入類型
enum InputType {
    case romanWithoutTone // "goa" → 搜尋 poj_no_tone/tl_no_tone
    case romanWithTone // "góa" → 搜尋 poj/tl
    case hanzi // "我" → 搜尋 hanzi
}
