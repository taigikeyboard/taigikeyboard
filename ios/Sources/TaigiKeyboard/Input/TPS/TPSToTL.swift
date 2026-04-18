import Foundation

/// TPS → TL parser.
///
/// Used by Trie search: the dictionary stores romanization with numeric tones,
/// so TPS input must be normalized to TL (e.g. "ㄉㄧㄠˊ" → "tiau5") before lookup.
enum TPSToTL {
    /// Convert a TPS string to TL.
    ///
    /// Parse pipeline:
    /// 1. Entering-tone finals (affect both coda and tone number)
    /// 2. Non-entering tone marks
    /// 3. Initials (compound first, then single)
    /// 4. Finals (compound first, then single)
    /// 5. Unmatched characters passed through (whitespace, punctuation)
    static func convert(_ tps: String) -> String {
        guard !tps.isEmpty else { return "" }

        var remaining = tps
        var result = ""
        // In TPS, consonant codas are encoded in entering-tone symbols (ㆴㆵㆻㆷ),
        // and syllabic nasals use vowel-table characters (ㆬ, ㆭ).
        // Separate consonant/vowel flags let us detect: consonant mid-syllable = new
        // syllable boundary; tone after consonant-only = invalid.
        var hasConsonant = false
        var hasVowel = false
        var lastConsonantTPS = ""
        // Tone / entering-tone ends a syllable; the next consonant or vowel needs a space.
        var needsSpace = false

        while !remaining.isEmpty {
            var matched = false

            // 1. Entering-tone finals (priority — affects both coda and tone)
            for (tpsPattern, tlEnding, tone) in TPSTables.tones {
                if remaining.hasPrefix(tpsPattern) {
                    result += tlEnding + tone
                    remaining.removeFirst(tpsPattern.count)
                    hasConsonant = false
                    hasVowel = false
                    lastConsonantTPS = ""
                    needsSpace = true
                    matched = true
                    break
                }
            }
            if matched { continue }

            // 2. Non-entering tone marks.
            // Consonant-only (no vowel) + tone mark is an invalid syllable —
            // insert a space boundary. e.g. ㄫˊ → "ng 5"; ㆭˊ → "ng5" (ㆭ is a vowel).
            for (tpsPattern, tone) in TPSTables.toneMarks {
                if remaining.hasPrefix(tpsPattern) {
                    if hasConsonant, !hasVowel { result += " " }
                    result += tone
                    remaining.removeFirst(tpsPattern.count)
                    hasConsonant = false
                    hasVowel = false
                    lastConsonantTPS = ""
                    needsSpace = true
                    matched = true
                    break
                }
            }
            if matched { continue }

            // 3. Initials (compound first per table sort).
            // A consonant mid-syllable starts a new syllable — insert space.
            for (tpsPattern, tl) in TPSTables.consonants {
                if remaining.hasPrefix(tpsPattern) {
                    if needsSpace || hasConsonant || hasVowel {
                        result += " "
                        needsSpace = false
                    }
                    result += tl
                    remaining.removeFirst(tpsPattern.count)
                    hasConsonant = true
                    // Compound initials ending in ㄧ (ㄑㄧ→tshi etc.) already include a vowel component.
                    hasVowel = tpsPattern.hasSuffix("ㄧ")
                    lastConsonantTPS = tpsPattern
                    matched = true
                    break
                }
            }
            if matched { continue }

            // 4. Finals (compound first per table sort).
            for (tpsPattern, tl) in TPSTables.vowels {
                if remaining.hasPrefix(tpsPattern) {
                    if needsSpace {
                        result += " "
                        needsSpace = false
                    }
                    // Non-palatalized affricates (ㄗ/ㄘ/ㄙ/ㆡ) + ㄧ is invalid TPS.
                    // Must use compound initials ㄐㄧ/ㄑㄧ/ㄒㄧ/ㆢㄧ instead.
                    if tpsPattern == "ㄧ",
                       !hasVowel,
                       TPSTables.nonPalatalizedAffricates.contains(lastConsonantTPS)
                    {
                        result += " "
                        hasConsonant = false
                        lastConsonantTPS = ""
                    }
                    result += tl
                    remaining.removeFirst(tpsPattern.count)
                    hasVowel = true
                    matched = true
                    break
                }
            }
            if matched { continue }

            // 5. Unmatched — pass through (whitespace, punctuation, etc.)
            result.append(remaining.removeFirst())
            hasConsonant = false
            hasVowel = false
            needsSpace = false
            lastConsonantTPS = ""
        }

        // Post-process: oo before stop tone → o (e.g. "ook4" → "ok4", "oot8" → "ot8").
        // Reference: taigi-converter fromZhuyin rule.
        return result.replacingOccurrences(
            of: #"oo([ptk][48])"#,
            with: "o$1",
            options: .regularExpression,
        )
    }

    /// Convert a space-separated multi-syllable TPS string to TL.
    /// e.g. "ㄉㄧㄠˊ ㄙㄨˊ" → "tiau5 su5".
    static func convertMultiSyllable(_ tps: String) -> String {
        tps.split(separator: " ")
            .map { convert(String($0)) }
            .joined(separator: " ")
    }
}
