import Foundation

/// Continuous input syllable segmenter
///
/// Segments continuous Taigi romanization input into individual syllables
/// using a syllable trie and DAG-based dynamic programming.
///
/// Algorithm:
/// 1. Build a trie of all valid TL/POJ syllables (initial x final)
/// 2. For each position in input, find all valid syllables (DAG edges)
/// 3. Use DP to find optimal segmentation (prefers longer matches)
/// 4. Tone digits (1-9) after valid syllables are consumed as part of the syllable
///
/// Example: "gua2si7soobin5hian5" -> ["gua2", "si7", "soo", "bin5", "hian5"]
enum SyllableSegmenter {

    /// Closure that checks whether any dictionary entry has the given prefix.
    /// Used for tie-breaking in DP when two segmentation paths score equally.
    typealias WordPrefixChecker = (String) -> Bool

    // MARK: - Syllable Trie

    private final class TrieNode {
        var children: [Character: TrieNode] = [:]
        var isValidSyllable: Bool = false
    }

    /// TL syllable trie root (TL initials × TL finals)
    private static let tlTrieRoot: TrieNode = buildTrie(
        initials: TaigiPhonetics.tlInitials,
        finals: TaigiPhonetics.tlFinals,
        multiCharOnsets: TaigiPhonetics.tlInitials.filter { $0.count > 1 }
    )

    /// POJ syllable trie root (POJ initials × POJ finals)
    private static let pojTrieRoot: TrieNode = buildTrie(
        initials: TaigiPhonetics.pojInitials,
        finals: TaigiPhonetics.pojFinals,
        multiCharOnsets: TaigiPhonetics.pojInitials.filter { $0.count > 1 }
    )

    /// Combined trie root (both TL and POJ syllables, for mode-agnostic segmentation)
    private static let combinedTrieRoot: TrieNode = {
        let root = TrieNode()

        // Insert all TL syllables
        for initial in TaigiPhonetics.tlInitials {
            for tlFinal in TaigiPhonetics.tlFinals {
                let syllable = initial + tlFinal
                guard !syllable.isEmpty else { continue }
                insertIntoTrie(root, syllable)
            }
        }

        // Insert all POJ syllables
        for initial in TaigiPhonetics.pojInitials {
            for pojFinal in TaigiPhonetics.pojFinals {
                let syllable = initial + pojFinal
                guard !syllable.isEmpty else { continue }
                insertIntoTrie(root, syllable)
            }
        }

        // Multi-char onsets from both systems
        for onset in TaigiPhonetics.tlInitials.filter({ $0.count > 1 }) {
            insertIntoTrie(root, onset)
        }
        for onset in TaigiPhonetics.pojInitials.filter({ $0.count > 1 }) {
            insertIntoTrie(root, onset)
        }

        insertIntoTrie(root, "nn")

        return root
    }()

    /// Build a syllable trie from initials × finals + multi-char onsets + "nn"
    private static func buildTrie(
        initials: Set<String>,
        finals: Set<String>,
        multiCharOnsets: Set<String>
    ) -> TrieNode {
        let root = TrieNode()

        for initial in initials {
            for final_ in finals {
                let syllable = initial + final_
                guard !syllable.isEmpty else { continue }
                insertIntoTrie(root, syllable)
            }
        }

        for onset in multiCharOnsets {
            insertIntoTrie(root, onset)
        }

        insertIntoTrie(root, "nn")

        return root
    }

    private static func insertIntoTrie(_ root: TrieNode, _ syllable: String) {
        var node = root
        for char in syllable {
            if node.children[char] == nil {
                node.children[char] = TrieNode()
            }
            node = node.children[char]!
        }
        node.isValidSyllable = true
    }

    // MARK: - Public API

    /// Check if the given string is a valid syllable prefix in the specified mode's trie.
    ///
    /// Walks the mode-appropriate trie character by character. Returns true if every
    /// character is walkable (i.e., the input is a prefix of at least one valid syllable).
    ///
    /// - Parameters:
    ///   - input: The string to validate (lowercase, no tone digits)
    ///   - mode: Input mode determining which trie to check
    /// - Returns: true if input is a valid prefix in the mode's syllable trie
    static func isValidPrefix(_ input: String, mode: InputMode) -> Bool {
        guard !input.isEmpty else { return true }

        let root: TrieNode
        switch mode {
        case .poj: root = pojTrieRoot
        case .tl: root = tlTrieRoot
        case .english: return true
        }

        var node = root
        for char in input.lowercased() {
            guard let next = node.children[char] else { return false }
            node = next
        }
        return true
    }

    /// Segment continuous input into syllables
    ///
    /// Supports:
    /// - Continuous input: "gua2si7soo" -> ["gua2", "si7", "soo"]
    /// - Hyphen-separated (backward compatible): "gua2-si7" -> ["gua2-", "si7"]
    /// - Mixed: "gua2si7-soo" -> ["gua2", "si7-", "soo"]
    ///
    /// Hyphens are preserved by attaching them to the preceding segment,
    /// so display logic can distinguish explicit hyphens from auto-segmented spaces.
    ///
    /// - Parameters:
    ///   - input: Raw input string
    ///   - wordPrefixChecker: Optional closure to resolve DP score ties using dictionary lookup.
    ///     When nil, ties are resolved by first-arrival (existing behavior).
    ///   - mode: Input mode determining which trie to use. Defaults to nil (combined trie).
    /// - Returns: Array of syllable strings
    static func segment(_ input: String, wordPrefixChecker: WordPrefixChecker? = nil, mode: InputMode? = nil) -> [String] {
        guard !input.isEmpty else { return [] }

        let trieRoot = trieRootForMode(mode)

        if input.contains("-") {
            return segmentWithHyphens(input, trieRoot: trieRoot, wordPrefixChecker: wordPrefixChecker)
        }

        return segmentContinuous(input, trieRoot: trieRoot, wordPrefixChecker: wordPrefixChecker)
    }

    /// Returns the appropriate trie root for the given mode
    private static func trieRootForMode(_ mode: InputMode?) -> TrieNode {
        guard let mode = mode else { return combinedTrieRoot }
        switch mode {
        case .poj: return pojTrieRoot
        case .tl: return tlTrieRoot
        case .english: return combinedTrieRoot
        }
    }

    // MARK: - Hyphen Handling

    /// Split by hyphens, segment each part, preserve hyphens on preceding segment.
    /// Leading hyphens (e.g. "-gua2", "--a") are prepended to the first real segment.
    private static func segmentWithHyphens(_ input: String, trieRoot: TrieNode, wordPrefixChecker: WordPrefixChecker? = nil) -> [String] {
        let parts = input.split(separator: "-", omittingEmptySubsequences: false)
        var result: [String] = []
        var pendingPrefix = ""

        for (index, part) in parts.enumerated() {
            let partStr = String(part)
            if !partStr.isEmpty {
                let segmented = segmentContinuous(partStr, trieRoot: trieRoot, wordPrefixChecker: wordPrefixChecker)
                if !pendingPrefix.isEmpty, !segmented.isEmpty {
                    result.append(pendingPrefix + segmented[0])
                    result.append(contentsOf: segmented.dropFirst())
                    pendingPrefix = ""
                } else {
                    result.append(contentsOf: segmented)
                }
            }
            // Track hyphen between parts (except after the final part)
            if index < parts.count - 1 {
                if !result.isEmpty {
                    result[result.count - 1] += "-"
                } else {
                    pendingPrefix += "-"
                }
            }
        }

        // Input was only hyphens (e.g. "-", "--")
        if result.isEmpty && !pendingPrefix.isEmpty {
            return [pendingPrefix]
        }

        return result
    }

    // MARK: - Core Segmentation

    /// DAG + DP segmentation for continuous (no-hyphen) input
    ///
    /// 1. At each position, walk the syllable trie to find valid syllables (DAG edges)
    /// 2. Tone digits (1-9) following a valid syllable are consumed with it
    /// 3. DP finds the path maximizing sum of squared syllable lengths
    /// 4. On score ties, uses wordPrefixChecker (if provided) to prefer dictionary-backed paths
    /// 5. Fallback: unrecognized characters are emitted as single-char segments
    private static func segmentContinuous(_ input: String, trieRoot: TrieNode, wordPrefixChecker: WordPrefixChecker? = nil) -> [String] {
        guard !input.isEmpty else { return [] }

        let lowered = input.lowercased()
        let chars = Array(lowered)
        let n = chars.count

        // Build DAG: edges[i] = list of (endPosition, syllableLength)
        var edges: [[(end: Int, len: Int)]] = Array(repeating: [], count: n)

        for i in 0..<n {
            guard !chars[i].isNumber else { continue }

            var node = trieRoot
            var j = i

            while j < n, !chars[j].isNumber, let next = node.children[chars[j]] {
                node = next
                j += 1

                if node.isValidSyllable {
                    if j < n && chars[j].isNumber {
                        // Consume tone digit: syllable + digit
                        edges[i].append((end: j + 1, len: j + 1 - i))
                    } else {
                        // No tone digit (or end of input): syllable only
                        edges[i].append((end: j, len: j - i))
                    }
                }
            }
        }

        // DP: maximize sum of squared syllable lengths
        let unreachable = Int.min / 2
        var score = Array(repeating: unreachable, count: n + 1)
        var prev = Array(repeating: -1, count: n + 1)
        score[0] = 0

        let hasTones = chars.contains { $0.isNumber }

        for i in 0..<n {
            guard score[i] > unreachable else { continue }

            // Syllable edges (only for non-digit positions)
            if !chars[i].isNumber {
                for edge in edges[i] {
                    let newScore = score[i] + edge.len * edge.len
                    if newScore > score[edge.end] {
                        score[edge.end] = newScore
                        prev[edge.end] = i
                    } else if newScore == score[edge.end], let checker = wordPrefixChecker {
                        // Tie: use dictionary prefix lookup to disambiguate
                        if resolveTie(chars: chars, prev: prev, currentEnd: edge.end,
                                      newStart: i, hasTones: hasTones, checker: checker) {
                            score[edge.end] = newScore
                            prev[edge.end] = i
                        }
                    }
                }
            }

            // Single-char fallback (for incomplete input, stranded digits, etc.)
            let fallbackScore = score[i] + 1
            if fallbackScore > score[i + 1] {
                score[i + 1] = fallbackScore
                prev[i + 1] = i
            }
        }

        // Reconstruct path from original (case-preserved) input
        var segments: [String] = []
        var pos = n

        while pos > 0 {
            let start = prev[pos]
            if start < 0 { break }

            let startIdx = input.index(input.startIndex, offsetBy: start)
            let endIdx = input.index(input.startIndex, offsetBy: pos)
            segments.append(String(input[startIdx..<endIdx]))
            pos = start
        }

        segments.reverse()
        return segments
    }

    // MARK: - Tie-Breaking

    /// Resolve a DP score tie by checking dictionary prefix matches.
    ///
    /// Reconstructs segment paths for both the current winner and the new candidate,
    /// builds continuous search keys (with default tones for toneless segments),
    /// and checks which path has dictionary matches.
    ///
    /// - Returns: `true` if the new path should replace the current one
    private static func resolveTie(
        chars: [Character],
        prev: [Int],
        currentEnd: Int,
        newStart: Int,
        hasTones: Bool,
        checker: WordPrefixChecker
    ) -> Bool {
        // Reconstruct current path segments (walking prev[] backwards)
        let currentSegments = reconstructSegments(chars: chars, prev: prev, end: currentEnd)

        // Reconstruct new path segments: walk prev[] to newStart, then append [newStart, currentEnd)
        var newSegments = reconstructSegments(chars: chars, prev: prev, end: newStart)
        let newSeg = String(chars[newStart..<currentEnd])
        newSegments.append(newSeg)

        // Need at least 2 segments in each path for meaningful comparison
        guard currentSegments.count >= 2, newSegments.count >= 2 else { return false }

        // Build search keys and check dictionary
        let currentKey = buildTieBreakKey(segments: currentSegments, hasTones: hasTones)
        let newKey = buildTieBreakKey(segments: newSegments, hasTones: hasTones)

        let currentHasMatch = checker(currentKey)
        let newHasMatch = checker(newKey)

        // Prefer new path only if it has matches and current doesn't
        if newHasMatch && !currentHasMatch {
            return true
        }

        return false
    }

    /// Walk the prev[] array backwards from `end` to reconstruct segments
    private static func reconstructSegments(chars: [Character], prev: [Int], end: Int) -> [String] {
        var segments: [String] = []
        var pos = end
        while pos > 0 {
            let start = prev[pos]
            if start < 0 { break }
            segments.append(String(chars[start..<pos]))
            pos = start
        }
        segments.reverse()
        return segments
    }

    /// Build a continuous search key from segments, adding default tones
    /// to toneless non-final segments when the input contains tone digits.
    ///
    /// Default tone: open syllable -> 1, stop final (p/t/k/h) -> 4.
    /// This matches the logic in AutocompleteService.buildSearchKey.
    /// Keys are joined without separator to match MARISA trie key format (e.g. "kin1a2jit8").
    private static func buildTieBreakKey(segments: [String], hasTones: Bool) -> String {
        let processed = segments.enumerated().map { (index, seg) -> String in
            let lowered = seg.lowercased()
            guard !lowered.isEmpty else { return "" }

            let isLast = index == segments.count - 1

            if hasTones, !isLast, let lastChar = lowered.last, !lastChar.isNumber {
                return lowered + (TaigiPhonetics.isStopTone(lowered) ? "4" : "1")
            }

            return lowered
        }

        return processed.joined()
    }

    // MARK: - Word Grouping

    /// Group syllables into words using greedy longest-match against the dictionary.
    ///
    /// Scans left-to-right, trying the longest possible sequence of syllables first.
    /// For each candidate length, builds a continuous search key (with default tones)
    /// and checks the dictionary via `wordPrefixChecker`. The longest matching group wins.
    ///
    /// When `wordPrefixChecker` is nil, each syllable becomes its own group (backward compatible).
    ///
    /// - Parameters:
    ///   - syllables: Segmented syllable array (output of `segment()`)
    ///   - wordPrefixChecker: Optional closure to check dictionary prefix matches
    /// - Returns: Array of syllable groups, where each group is a word
    static func groupIntoWords(_ syllables: [String], wordPrefixChecker: WordPrefixChecker?) -> [[String]] {
        guard let checker = wordPrefixChecker else {
            return syllables.map { [$0] }
        }

        let hasTones = syllables.contains { $0.last?.isNumber == true }

        var groups: [[String]] = []
        var i = 0

        while i < syllables.count {
            let remaining = syllables.count - i
            var matched = false

            // Try longest group first, down to length 2
            for len in stride(from: remaining, through: 2, by: -1) {
                let slice = Array(syllables[i..<(i + len)])
                let key = buildTieBreakKey(segments: slice, hasTones: hasTones)
                if checker(key) {
                    groups.append(slice)
                    i += len
                    matched = true
                    break
                }
            }

            if !matched {
                groups.append([syllables[i]])
                i += 1
            }
        }

        return groups
    }
}
