import Foundation

/// TL → TPS parser.
///
/// Used for candidate display: dictionary entries are stored as TL romanization,
/// but must be shown as TPS (方音符號) when the user's input is TPS.
enum TLToTPS {
    /// Convert a TL string to TPS.
    ///
    /// Hyphens in TL (e.g. "gua2-gua2") become spaces in TPS ("ㄍㄨㄚˋ ㄍㄨㄚˋ").
    ///
    /// - Parameters:
    ///   - tl: TL string (e.g. "gua2-gua2")
    ///   - orMapsToER: `true` maps "or" → ㄜ (dialect variant); `false` maps "or" → ㄛ (default).
    /// - Returns: TPS string (e.g. "ㄍㄨㄚˋ ㄍㄨㄚˋ")
    static func convert(_ tl: String, orMapsToER: Bool = false) -> String {
        guard !tl.isEmpty else { return "" }

        let syllables = tl.split(separator: "-", omittingEmptySubsequences: false)
        return syllables
            .map { convertSyllable(String($0), orMapsToER: orMapsToER) }
            .joined(separator: " ")
    }

    /// Convert a display-form romanization (with diacritical tone marks) to TPS.
    ///
    /// Dictionary entries use display form with diacritics (e.g. "n̂g", "guá").
    /// This normalizes to numeric TL (e.g. "ng5", "gua2") first, then converts.
    ///
    /// - Parameters:
    ///   - displayRoman: Romanization with diacritical tone marks
    ///   - orMapsToER: see `convert(_:orMapsToER:)`
    static func convertFromDisplay(_ displayRoman: String, orMapsToER: Bool = false) -> String {
        guard !displayRoman.isEmpty else { return "" }

        let numericTL = displayRoman
            .split(separator: "-", omittingEmptySubsequences: false)
            .map { syllable -> String in
                let (bare, tone) = TaigiPhonetics.stripToneMark(String(syllable))
                let normalized = TaigiPhonetics.normalizeToTL(bare.lowercased())
                return normalized + tone
            }
            .joined(separator: "-")

        return convert(numericTL, orMapsToER: orMapsToER)
    }

    // MARK: - Private

    /// Convert a single TL syllable to TPS.
    ///
    /// Separates initial, final, and tone parts for correct post-processing:
    /// - Standalone m/ng → syllabic form (ㄇ→ㆬ, ㄫ→ㆭ)
    /// - Palatalized initial + "nn" → nasalized ㆪ (e.g. tsinn → ㄐㆪ)
    /// - "o" before stop tone → "oo" form (e.g. "ok4" → ㆦㆻ, not ㄛㆻ)
    private static func convertSyllable(_ syllable: String, orMapsToER: Bool) -> String {
        guard !syllable.isEmpty else { return "" }

        var remaining = syllable.lowercased()
        var consonant = ""
        var vowel = ""
        var tone = ""

        // 1. Initial (longest match first; table is pre-sorted).
        for (tl, tps) in TPSTables.tlToTPSConsonants {
            if remaining.hasPrefix(tl) {
                consonant = tps
                remaining.removeFirst(tl.count)
                break
            }
        }

        // 2. Checked-tone suffix (p4/t4/k4/h4/p8/t8/k8/h8).
        for (tl, tps) in TPSTables.tlToTPSCheckedTones {
            if remaining.hasSuffix(tl) {
                remaining.removeLast(tl.count)
                tone = tps
                break
            }
        }

        // 3. General tone digit if no checked tone was found.
        if tone.isEmpty, let lastChar = remaining.last, lastChar.isNumber {
            let toneDigit = String(lastChar)
            remaining.removeLast()
            for (tlTone, tpsTone) in TPSTables.tlToTPSTones where toneDigit == tlTone {
                tone = tpsTone
                break
            }
        }

        // 4. Finals (greedy, longest match first).
        while !remaining.isEmpty {
            var matched = false
            for (tl, tps) in TPSTables.tlToTPSVowels {
                if remaining.hasPrefix(tl), !TPSTables.stopConsonants.contains(tl) {
                    let effectiveTPS = (orMapsToER && tl == "or") ? "ㄜ" : tps
                    vowel += effectiveTPS
                    remaining.removeFirst(tl.count)
                    matched = true
                    break
                }
            }
            if !matched {
                vowel.append(remaining.removeFirst())
            }
        }

        // 5. Post-process: standalone m/ng → syllabic form.
        if vowel.isEmpty {
            if consonant == "ㄇ" { consonant = ""; vowel = "ㆬ" }
            else if consonant == "ㄫ" { consonant = ""; vowel = "ㆭ" }
        }

        // 5b. "ing" special case — ㄧㆭ → ㄧㄥ.
        if vowel == "ㄧㆭ" { vowel = "ㄧㄥ" }

        // 6. Palatalized initial + "nn" → nasalized ㆪ.
        if consonant.hasSuffix("ㄧ"), vowel == "ㄣㄣ" {
            consonant = String(consonant.dropLast())
            vowel = "ㆪ"
        }

        // 7. "o" → "oo" form before stop tone (e.g. "ok4" → ㆦㆻ not ㄛㆻ).
        // Reference: taigi-converter toZhuyin rule.
        if vowel.contains("ㄛ"), !tone.isEmpty,
           let first = tone.unicodeScalars.first?.value,
           stopTonePrefixes.contains(first)
        {
            vowel = vowel.replacingOccurrences(of: "ㄛ", with: "ㆦ")
        }

        return consonant + vowel + tone
    }

    /// Entering-tone finals that cause `ㄛ → ㆦ` rewriting:
    /// ㆴ (p), ㆵ (t), ㆻ (k). Excludes ㆷ (h) — `oh4` stays ㄛㆷ.
    private static let stopTonePrefixes: Set<UInt32> = [
        0x31B4, // ㆴ  p
        0x31B5, // ㆵ  t
        0x31BB, // ㆻ  k
    ]
}
