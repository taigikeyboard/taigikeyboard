// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
// English autocomplete + spell-correction over a frequency-ranked wordlist:
// prefix completion (binary search) + bounded Damerau-Levenshtein (OSA) correction.

package com.siansiansu.taigikeyboard.ime.text.composing

import kotlin.math.abs

/**
 * Frequency-ranked English suggestion matcher.
 *
 * Two candidate sources, merged and ranked:
 * 1. Prefix completion — words that begin with the typed token (binary search
 *    over a word-sorted index). Includes the exact word when present.
 * 2. Spell correction — words within Optimal-String-Alignment distance <= [MAX_EDIT_DISTANCE]
 *    of the token, pruned by length. Disabled for tokens shorter than
 *    [MIN_CORRECTION_LENGTH] (correction on 1-2 letters is noise).
 *
 * Ranking: prefix matches outrank corrections; within prefix by frequency; within
 * correction by edit distance then frequency; alphabetical tie-break (deterministic).
 * Output casing mirrors the input token (lowercase / Title / ALL-CAPS).
 *
 * Pure Kotlin stdlib — no Android, no I/O, no clock. The platform wrapper
 * ([EnglishAutocompleteService]) owns asset loading and lifecycle.
 */
class EnglishWordMatcher(entries: List<WordEntry>) {

    data class WordEntry(val word: String, val frequency: Long)

    // Word-sorted index for prefix binary search; frequency kept parallel for ranking.
    private val sortedWords: List<String>
    private val frequencies: LongArray

    init {
        val sorted = entries.sortedBy { it.word }
        sortedWords = sorted.map { it.word }
        frequencies = LongArray(sorted.size) { sorted[it].frequency }
    }

    /**
     * Returns up to [maxResults] suggestions for [token], re-cased to match it.
     * Empty when the token is blank or nothing matches.
     */
    fun suggest(token: String, maxResults: Int): List<String> {
        if (token.isEmpty() || maxResults <= 0) return emptyList()
        val lower = token.lowercase()

        val prefixHits = collectPrefixHits(lower)
        // Correction runs only for alphabetic tokens of sufficient length: short
        // tokens are noise, and tokens carrying an apostrophe/hyphen (don't,
        // well-known) would otherwise match unrelated words (don't→done). The
        // prefix-dedup set is built only when correction will consult it.
        val correctionHits = if (lower.length >= MIN_CORRECTION_LENGTH && lower.all { it in 'a'..'z' }) {
            collectCorrectionHits(lower, prefixHits.mapTo(HashSet()) { it.word })
        } else {
            emptyList()
        }

        return (prefixHits + correctionHits)
            .sortedWith(CANDIDATE_ORDER)
            .take(maxResults)
            .map { applyCasing(it.word, token) }
    }

    // Words whose first chars equal the token; collected via the sorted index.
    private fun collectPrefixHits(lowerToken: String): List<Candidate> {
        val start = lowerBound(lowerToken)
        val hits = ArrayList<Candidate>()
        var i = start
        while (i < sortedWords.size && sortedWords[i].startsWith(lowerToken)) {
            // The exact typed word, when it is itself a dictionary word, must not be
            // outranked by a higher-frequency completion (service before services).
            val candidateClass =
                if (sortedWords[i] == lowerToken) CandidateClass.EXACT else CandidateClass.PREFIX
            hits.add(Candidate(sortedWords[i], candidateClass, 0, frequencies[i]))
            i++
        }
        return hits
    }

    // Words within OSA distance <= MAX_EDIT_DISTANCE, length-pruned, excluding
    // anything already surfaced as a prefix hit. Reuses three DP rows across all
    // candidates to avoid per-candidate allocation.
    private fun collectCorrectionHits(
        lowerToken: String,
        prefixWords: Set<String>,
    ): List<Candidate> {
        val width = lowerToken.length + 1
        val rowA = IntArray(width)
        val rowB = IntArray(width)
        val rowC = IntArray(width)
        val hits = ArrayList<Candidate>()
        for (i in sortedWords.indices) {
            val word = sortedWords[i]
            if (abs(word.length - lowerToken.length) > MAX_EDIT_DISTANCE) continue
            if (word in prefixWords) continue
            val distance = osaDistanceWithin(word, lowerToken, MAX_EDIT_DISTANCE, rowA, rowB, rowC)
            if (distance <= MAX_EDIT_DISTANCE) {
                hits.add(Candidate(word, CandidateClass.CORRECTION, distance, frequencies[i]))
            }
        }
        return hits
    }

    // First index in sortedWords >= key (binary search lower bound).
    private fun lowerBound(key: String): Int {
        var lo = 0
        var hi = sortedWords.size
        while (lo < hi) {
            val mid = (lo + hi) ushr 1
            if (sortedWords[mid] < key) lo = mid + 1 else hi = mid
        }
        return lo
    }

    private fun applyCasing(candidate: String, token: String): String {
        if (token.isEmpty()) return candidate
        if (token.length > 1 && token.all { it.isUpperCase() }) return candidate.uppercase()
        if (token[0].isUpperCase()) return candidate.replaceFirstChar { it.uppercaseChar() }
        return candidate
    }

    // Declaration order is the ranking order: the exact typed word first, then
    // higher-frequency completions, then spell corrections.
    private enum class CandidateClass { EXACT, PREFIX, CORRECTION }

    private data class Candidate(
        val word: String,
        val candidateClass: CandidateClass,
        val distance: Int,
        val frequency: Long,
    )

    companion object {
        const val MAX_EDIT_DISTANCE = 2
        const val MIN_CORRECTION_LENGTH = 3

        // Prefix before correction; within a class, lower distance then higher
        // frequency; alphabetical tie-break keeps output deterministic.
        private val CANDIDATE_ORDER = compareBy<Candidate> { it.candidateClass }
            .thenBy { it.distance }
            .thenByDescending { it.frequency }
            .thenBy { it.word }

        /**
         * Parses `word<TAB>frequency` lines (the bundled asset format). Malformed
         * lines are skipped. Pure — testable on the JVM without the asset.
         */
        fun parseEntries(lines: Sequence<String>): List<WordEntry> {
            val out = ArrayList<WordEntry>()
            for (line in lines) {
                val tab = line.indexOf('\t')
                if (tab <= 0) continue
                val frequency = line.substring(tab + 1).toLongOrNull() ?: continue
                out.add(WordEntry(line.substring(0, tab), frequency))
            }
            return out
        }

        // Optimal String Alignment distance with early exit once the running row
        // minimum exceeds [max]. Covers adjacent transpositions (teh→the). The three
        // row buffers are caller-supplied (sized token.length+1) and reused per call.
        private fun osaDistanceWithin(
            candidate: String,
            token: String,
            max: Int,
            bufA: IntArray,
            bufB: IntArray,
            bufC: IntArray,
        ): Int {
            val n = candidate.length
            val m = token.length
            if (abs(n - m) > max) return max + 1

            var prev2 = bufA // row i-2
            var prev = bufB // row i-1
            var cur = bufC // row i
            for (j in 0..m) prev[j] = j // row 0: distance from empty candidate prefix

            for (i in 1..n) {
                cur[0] = i
                var rowMin = i
                val ci = candidate[i - 1]
                for (j in 1..m) {
                    val cost = if (ci == token[j - 1]) 0 else 1
                    var value = minOf(
                        prev[j] + 1, // deletion
                        cur[j - 1] + 1, // insertion
                        prev[j - 1] + cost, // substitution
                    )
                    if (i > 1 && j > 1 && ci == token[j - 2] && candidate[i - 2] == token[j - 1]) {
                        value = minOf(value, prev2[j - 2] + 1) // transposition
                    }
                    cur[j] = value
                    if (value < rowMin) rowMin = value
                }
                if (rowMin > max) return max + 1
                val recycled = prev2
                prev2 = prev
                prev = cur
                cur = recycled
            }
            return prev[m] // last completed row is in prev after the final rotation
        }
    }
}
