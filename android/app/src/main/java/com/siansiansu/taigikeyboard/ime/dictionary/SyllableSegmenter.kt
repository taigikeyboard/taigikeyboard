package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode

/**
 * Continuous input syllable segmenter
 *
 * Segments continuous Taigi romanization input into individual syllables
 * using a syllable trie and DAG-based dynamic programming.
 *
 * Algorithm:
 * 1. Build a trie of all valid TL/POJ syllables (initial x final)
 * 2. For each position in input, find all valid syllables (DAG edges)
 * 3. Use DP to find optimal segmentation (prefers longer matches)
 * 4. Tone digits (1-9) after valid syllables are consumed as part of the syllable
 *
 * Example: "gua2si7soobin5hian5" -> ["gua2", "si7", "soo", "bin5", "hian5"]
 *
 * Ported from iOS SyllableSegmenter.swift
 */
/** Closure that checks whether any dictionary entry has the given prefix. */
typealias WordPrefixChecker = (String) -> Boolean

object SyllableSegmenter {

    // MARK: - Syllable Trie

    private class TrieNode {
        val children = mutableMapOf<Char, TrieNode>()
        var isValidSyllable = false
    }

    // Phonetics data referenced from TaigiPhonetics (single source of truth)
    private val tlInitials get() = TaigiPhonetics.tlInitials
    private val tlFinals get() = TaigiPhonetics.tlFinals
    private val pojInitialFromTL get() = TaigiPhonetics.pojInitialFromTL

    /** TL syllable trie root (TL initials x TL finals) */
    private val tlTrieRoot: TrieNode by lazy {
        buildTrie(
            initials = tlInitials,
            finals = tlFinals,
            multiCharOnsets = tlInitials.filter { it.length > 1 }.toSet()
        )
    }

    /** POJ syllable trie root (POJ initials x POJ finals) */
    private val pojTrieRoot: TrieNode by lazy {
        val pojInitials = tlInitials.map { pojInitialFromTL[it] ?: it }.toSet()
        val pojFinals = tlFinals.map { pojInputFinal(it) }.toSet()
        buildTrie(
            initials = pojInitials,
            finals = pojFinals,
            multiCharOnsets = pojInitials.filter { it.length > 1 }.toSet()
        )
    }

    /** Combined trie root (both TL and POJ syllables, for mode-agnostic segmentation) */
    private val combinedTrieRoot: TrieNode by lazy {
        val root = TrieNode()

        // Insert all TL syllables
        for (initial in tlInitials) {
            for (tlFinal in tlFinals) {
                val syllable = initial + tlFinal
                if (syllable.isEmpty()) continue
                insertIntoTrie(root, syllable)
            }
        }

        // Insert all POJ syllables
        val pojInitials = tlInitials.map { pojInitialFromTL[it] ?: it }.toSet()
        val pojFinals = tlFinals.map { pojInputFinal(it) }.toSet()
        for (initial in pojInitials) {
            for (pojFinal in pojFinals) {
                val syllable = initial + pojFinal
                if (syllable.isEmpty()) continue
                insertIntoTrie(root, syllable)
            }
        }

        // Multi-char onsets from both systems
        for (onset in tlInitials.filter { it.length > 1 }) {
            insertIntoTrie(root, onset)
        }
        for (onset in pojInitials.filter { it.length > 1 }) {
            insertIntoTrie(root, onset)
        }

        insertIntoTrie(root, "nn")

        root
    }

    /**
     * Build a syllable trie from initials x finals + multi-char onsets + "nn"
     */
    private fun buildTrie(
        initials: Set<String>,
        finals: Set<String>,
        multiCharOnsets: Set<String>
    ): TrieNode {
        val root = TrieNode()

        for (initial in initials) {
            for (final_ in finals) {
                val syllable = initial + final_
                if (syllable.isEmpty()) continue
                insertIntoTrie(root, syllable)
            }
        }

        for (onset in multiCharOnsets) {
            insertIntoTrie(root, onset)
        }

        insertIntoTrie(root, "nn")

        return root
    }

    /**
     * Convert TL final to POJ keyboard-input form
     *
     * Only handles differences that affect raw keystroke input:
     * - ua -> oa, ue -> oe (prefix substitution)
     * - ing -> eng, ik -> ek (exact match)
     */
    private fun pojInputFinal(tlFinal: String): String {
        var result = tlFinal

        when {
            result.startsWith("ua") -> result = "oa" + result.drop(2)
            result.startsWith("ue") -> result = "oe" + result.drop(2)
        }

        if (result == "ing") result = "eng"
        if (result == "ik") result = "ek"

        return result
    }

    private fun insertIntoTrie(root: TrieNode, syllable: String) {
        var node = root
        for (char in syllable) {
            node = node.children.getOrPut(char) { TrieNode() }
        }
        node.isValidSyllable = true
    }

    // MARK: - Public API

    /**
     * Check if the given string is a valid syllable prefix in the specified mode's trie.
     *
     * Walks the mode-appropriate trie character by character. Returns true if every
     * character is walkable (i.e., the input is a prefix of at least one valid syllable).
     *
     * @param input The string to validate (lowercase, no tone digits)
     * @param mode Input mode determining which trie to check
     * @return true if input is a valid prefix in the mode's syllable trie
     */
    fun isValidPrefix(input: String, mode: InputMode): Boolean {
        if (input.isEmpty()) return true

        val root = when (mode) {
            InputMode.POJ -> pojTrieRoot
            InputMode.TL -> tlTrieRoot
            InputMode.ENGLISH -> return true
        }

        var node = root
        for (char in input.lowercase()) {
            val next = node.children[char] ?: return false
            node = next
        }
        return true
    }

    /**
     * Segment continuous input into syllables
     *
     * Supports:
     * - Continuous input: "gua2si7soo" -> ["gua2", "si7", "soo"]
     * - Hyphen-separated (backward compatible): "gua2-si7" -> ["gua2-", "si7"]
     * - Mixed: "gua2si7-soo" -> ["gua2", "si7-", "soo"]
     *
     * Hyphens are preserved by attaching them to the preceding segment,
     * so display logic can distinguish explicit hyphens from auto-segmented spaces.
     *
     * @param input Raw input string
     * @param wordPrefixChecker Optional closure to resolve DP score ties using dictionary lookup.
     *   When null, ties are resolved by first-arrival (existing behavior).
     * @param mode Input mode determining which trie to use. Defaults to null (combined trie).
     * @return List of syllable strings
     */
    fun segment(
        input: String,
        wordPrefixChecker: WordPrefixChecker? = null,
        mode: InputMode? = null
    ): List<String> {
        if (input.isEmpty()) return emptyList()

        val trieRoot = trieRootForMode(mode)

        return if (input.contains("-")) {
            segmentWithHyphens(input, trieRoot, wordPrefixChecker)
        } else {
            segmentContinuous(input, trieRoot, wordPrefixChecker)
        }
    }

    /** Returns the appropriate trie root for the given mode */
    private fun trieRootForMode(mode: InputMode?): TrieNode {
        if (mode == null) return combinedTrieRoot
        return when (mode) {
            InputMode.POJ -> pojTrieRoot
            InputMode.TL -> tlTrieRoot
            InputMode.ENGLISH -> combinedTrieRoot
        }
    }

    // MARK: - Hyphen Handling

    /**
     * Split by hyphens, segment each part, preserve hyphens on preceding segment.
     * Leading hyphens (e.g. "-gua2", "--a") are prepended to the first real segment.
     */
    private fun segmentWithHyphens(
        input: String,
        trieRoot: TrieNode,
        wordPrefixChecker: WordPrefixChecker? = null
    ): List<String> {
        val parts = input.split("-")
        val result = mutableListOf<String>()
        var pendingPrefix = ""

        for ((index, part) in parts.withIndex()) {
            if (part.isNotEmpty()) {
                val segmented = segmentContinuous(part, trieRoot, wordPrefixChecker)
                if (pendingPrefix.isNotEmpty() && segmented.isNotEmpty()) {
                    result.add(pendingPrefix + segmented[0])
                    result.addAll(segmented.drop(1))
                    pendingPrefix = ""
                } else {
                    result.addAll(segmented)
                }
            }
            // Track hyphen between parts (except after the final part)
            if (index < parts.size - 1) {
                if (result.isNotEmpty()) {
                    result[result.size - 1] = result[result.size - 1] + "-"
                } else {
                    pendingPrefix += "-"
                }
            }
        }

        // Input was only hyphens (e.g. "-", "--")
        if (result.isEmpty() && pendingPrefix.isNotEmpty()) {
            return listOf(pendingPrefix)
        }

        return result
    }

    // MARK: - Core Segmentation

    /**
     * DAG + DP segmentation for continuous (no-hyphen) input
     *
     * 1. At each position, walk the syllable trie to find valid syllables (DAG edges)
     * 2. Tone digits (1-9) following a valid syllable are consumed with it
     * 3. DP finds the path maximizing sum of squared syllable lengths
     * 4. On score ties, uses wordPrefixChecker (if provided) to prefer dictionary-backed paths
     * 5. Fallback: unrecognized characters are emitted as single-char segments
     */
    private fun segmentContinuous(
        input: String,
        trieRoot: TrieNode,
        wordPrefixChecker: WordPrefixChecker? = null
    ): List<String> {
        if (input.isEmpty()) return emptyList()

        val lowered = input.lowercase()
        val chars = lowered.toCharArray()
        val n = chars.size

        // Build DAG: edges[i] = list of (endPosition, syllableLength)
        data class Edge(val end: Int, val len: Int)
        val edges = Array(n) { mutableListOf<Edge>() }

        for (i in 0 until n) {
            if (chars[i].isDigit()) continue

            var node = trieRoot
            var j = i

            while (j < n && !chars[j].isDigit()) {
                val next = node.children[chars[j]] ?: break
                node = next
                j++

                if (node.isValidSyllable) {
                    if (j < n && chars[j].isDigit()) {
                        // Consume tone digit: syllable + digit
                        edges[i].add(Edge(end = j + 1, len = j + 1 - i))
                    } else {
                        // No tone digit (or end of input): syllable only
                        edges[i].add(Edge(end = j, len = j - i))
                    }
                }
            }
        }

        // DP: maximize sum of squared syllable lengths
        val unreachable = Int.MIN_VALUE / 2
        val score = IntArray(n + 1) { unreachable }
        val prev = IntArray(n + 1) { -1 }
        score[0] = 0

        val hasTones = chars.any { it.isDigit() }

        for (i in 0 until n) {
            if (score[i] <= unreachable) continue

            // Syllable edges (only for non-digit positions)
            if (!chars[i].isDigit()) {
                for (edge in edges[i]) {
                    val newScore = score[i] + edge.len * edge.len
                    if (newScore > score[edge.end]) {
                        score[edge.end] = newScore
                        prev[edge.end] = i
                    } else if (newScore == score[edge.end] && wordPrefixChecker != null) {
                        // Tie: use dictionary prefix lookup to disambiguate
                        if (resolveTie(chars, prev, edge.end, i, hasTones, wordPrefixChecker)) {
                            score[edge.end] = newScore
                            prev[edge.end] = i
                        }
                    }
                }
            }

            // Single-char fallback (for incomplete input, stranded digits, etc.)
            val fallbackScore = score[i] + 1
            if (fallbackScore > score[i + 1]) {
                score[i + 1] = fallbackScore
                prev[i + 1] = i
            }
        }

        // Reconstruct path from original (case-preserved) input
        val segments = mutableListOf<String>()
        var pos = n

        while (pos > 0) {
            val start = prev[pos]
            if (start < 0) break
            segments.add(input.substring(start, pos))
            pos = start
        }

        segments.reverse()
        return segments
    }

    // MARK: - Tie-Breaking

    /**
     * Resolve a DP score tie by checking dictionary prefix matches.
     *
     * Reconstructs segment paths for both the current winner and the new candidate,
     * builds continuous search keys (with default tones for toneless segments),
     * and checks which path has dictionary matches.
     *
     * @return true if the new path should replace the current one
     */
    private fun resolveTie(
        chars: CharArray,
        prev: IntArray,
        currentEnd: Int,
        newStart: Int,
        hasTones: Boolean,
        checker: WordPrefixChecker
    ): Boolean {
        // Reconstruct current path segments (walking prev[] backwards)
        val currentSegments = reconstructSegments(chars, prev, currentEnd)

        // Reconstruct new path segments: walk prev[] to newStart, then append [newStart, currentEnd)
        val newSegments = reconstructSegments(chars, prev, newStart).toMutableList()
        val newSeg = String(chars, newStart, currentEnd - newStart)
        newSegments.add(newSeg)

        // Need at least 2 segments in each path for meaningful comparison
        if (currentSegments.size < 2 || newSegments.size < 2) return false

        // Build search keys and check dictionary
        val currentKey = buildTieBreakKey(currentSegments, hasTones)
        val newKey = buildTieBreakKey(newSegments, hasTones)

        val currentHasMatch = checker(currentKey)
        val newHasMatch = checker(newKey)

        // Prefer new path only if it has matches and current doesn't
        return newHasMatch && !currentHasMatch
    }

    /** Walk the prev[] array backwards from `end` to reconstruct segments */
    private fun reconstructSegments(chars: CharArray, prev: IntArray, end: Int): List<String> {
        val segments = mutableListOf<String>()
        var pos = end
        while (pos > 0) {
            val start = prev[pos]
            if (start < 0) break
            segments.add(String(chars, start, pos - start))
            pos = start
        }
        segments.reverse()
        return segments
    }

    /**
     * Build a continuous search key from segments, adding default tones
     * to toneless non-final segments when the input contains tone digits.
     *
     * Default tone: open syllable -> 1, stop final (p/t/k/h) -> 4.
     * Keys are joined without separator to match MARISA trie key format (e.g. "kin1a2jit8").
     */
    private fun buildTieBreakKey(segments: List<String>, hasTones: Boolean): String {
        return segments.mapIndexed { index, seg ->
            val lowered = seg.lowercase()
            if (lowered.isEmpty()) return@mapIndexed ""

            val isLast = index == segments.size - 1

            if (hasTones && !isLast && lowered.last().let { !it.isDigit() }) {
                lowered + if (TaigiPhonetics.isStopTone(lowered)) "4" else "1"
            } else {
                lowered
            }
        }.joinToString("")
    }

    // MARK: - Word Grouping

    /**
     * Group syllables into words using greedy longest-match against the dictionary.
     *
     * Scans left-to-right, trying the longest possible sequence of syllables first.
     * For each candidate length, builds a continuous search key (with default tones)
     * and checks the dictionary via wordPrefixChecker. The longest matching group wins.
     *
     * When wordPrefixChecker is null, each syllable becomes its own group (backward compatible).
     *
     * @param syllables Segmented syllable array (output of segment())
     * @param wordPrefixChecker Optional closure to check dictionary prefix matches
     * @return List of syllable groups, where each group is a word
     */
    fun groupIntoWords(syllables: List<String>, wordPrefixChecker: WordPrefixChecker?): List<List<String>> {
        if (wordPrefixChecker == null) {
            return syllables.map { listOf(it) }
        }

        val hasTones = syllables.any { it.lastOrNull()?.isDigit() == true }

        val groups = mutableListOf<List<String>>()
        var i = 0

        while (i < syllables.size) {
            val remaining = syllables.size - i
            var matched = false

            // Try longest group first, down to length 2
            for (len in remaining downTo 2) {
                val slice = syllables.subList(i, i + len)
                val key = buildTieBreakKey(slice, hasTones)
                if (wordPrefixChecker(key)) {
                    groups.add(slice.toList())
                    i += len
                    matched = true
                    break
                }
            }

            if (!matched) {
                groups.add(listOf(syllables[i]))
                i++
            }
        }

        return groups
    }
}
