// Classifies a dictionary-query input, which decides which trie / index column it routes through.

import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

enum InputType {
    case romanWithoutTone // "goa" → searches poj_no_tone/tl_no_tone
    case romanWithTone // "góa" → searches poj/tl
    case hanzi // "我" → searches hanzi
}
