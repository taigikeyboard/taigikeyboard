package com.siansiansu.taigikeyboard.ime.text.composing

import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.engine.proto.CandidateMessage
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord.MetadataKeys
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * v3.5.8 Phase 9 Item 4 — pins the Continuous-input candidate metadata
 * contract that
 * [com.siansiansu.taigikeyboard.ime.text.composing.TaigiAutocompleteService]
 * (producer) and
 * [com.siansiansu.taigikeyboard.ime.text.smartbar.CandidateClickHandler]
 * (consumer) share via
 * [com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord.additionalInfo].
 *
 * Mirrors iOS `AutocompleteServiceContinuousTests.swift`. If iOS / Android
 * disagree on these key strings, the platform tap path silently mis-aligns
 * `commitContinuous` consumed-byte offsets and corrupts the engine pending
 * buffer — invisible to the user until they hit a bad commit boundary.
 *
 * Per `docs/engine/continuous-input-ranking.md` §10.1.2 (supersedes legacy
 * slot-0 model) + §10.3 commit contract: Continuous mode has NO
 * composing-text cell at slot 0. `candidate[0]` is the engine ranker top;
 * Tap-0 commits `candidate[0].display_text` (clarification γ — canonical
 * dictionary string, NOT the roman-with-spaces visual form).
 *
 * Scope: pure JVM unit tests against the top-level
 * [buildContinuousSuggestionsForCandidates] helper (no Robolectric needed).
 * Full `ComposingManager` round-trip + `CandidateClickHandler` tap-decode
 * tests await Robolectric / mockk infra.
 */
class ContinuousSuggestionsContractTest {

    /**
     * v3.5.8 Phase 9 Item 5 — builds a [RustEngineBridge.ContinuousCandidate]
     * with default `roman`/`hanji` matching the test fixture's `displayText`.
     * Production engine emits `roman` = `DictionaryRecord.tl` and `hanji` =
     * `DictionaryRecord.hanzi`; the legacy tests here pre-date that split and
     * assert against `displayText` semantics only, so defaulting `roman = displayText`
     * and `hanji = null` keeps their intent intact while letting new Item 5
     * tests override either field explicitly.
     */
    // 中文: Item 5 — 測試用 helper,roman 預設等於 displayText、hanji 預設 null;新 Item 5 測試可覆寫。
    private fun cand(
        consumedSpanStart: Int = 0,
        consumedSpanEnd: Int,
        syllableCount: Int = 1,
        displayText: String,
        score: Float = 1.0f,
        form: Int = 1,
        mode: RustEngineBridge.CandidateMode = RustEngineBridge.CandidateMode.HANT,
        roman: String? = null,
        hanji: String? = null,
    ): RustEngineBridge.ContinuousCandidate = RustEngineBridge.ContinuousCandidate(
        consumedSpanStart = consumedSpanStart,
        consumedSpanEnd = consumedSpanEnd,
        syllableCount = syllableCount,
        displayText = displayText,
        score = score,
        form = form,
        mode = mode,
        roman = roman ?: displayText,
        hanji = hanji,
    )

    @Test
    fun `slot 0 is candidate top with isContinuous flag, no composing cell`() {
        // v3.5.8 Phase 9 Item 4: §10.1.2 supersedes notice — the Continuous
        // path no longer inserts a `createComposingTextCell` at index 0.
        // `candidate[0]` IS slot 0, carrying `IS_CONTINUOUS="true"` so the
        // click handler routes it through `handleContinuousCandidateClick`.
        val candidates = listOf(
            cand(
                consumedSpanStart = 0,
                consumedSpanEnd = 4,
                syllableCount = 1,
                displayText = "tsua",
                score = 1.0f,
                form = 1,
                mode = RustEngineBridge.CandidateMode.HANT,
            ),
        )
        val result = buildContinuousSuggestionsForCandidates(candidates)

        assertEquals("Continuous path emits exactly N cells (no slot-0 composing cell)", 1, result.size)
        val slot0 = result[0]
        assertEquals("Slot 0 == candidate[0] (id starts at 1, defense-in-depth)", 1, slot0.id)
        assertEquals("true", slot0.additionalInfo[MetadataKeys.IS_CONTINUOUS])
        assertNull("slot 0 must NOT carry isComposingText (legacy slot-0 model superseded)", slot0.additionalInfo[MetadataKeys.IS_COMPOSING_TEXT])
    }

    @Test
    fun `continuous candidates carry exact metadata key strings`() {
        val candidates = listOf(
            cand(
                consumedSpanStart = 0,
                consumedSpanEnd = 4,
                syllableCount = 1,
                displayText = "tsua",
                score = 1.0f,
                form = 1,
                mode = RustEngineBridge.CandidateMode.HANT,
            ),
            cand(
                consumedSpanStart = 0,
                consumedSpanEnd = 7,
                syllableCount = 2,
                displayText = "珠仔",
                score = 0.5f,
                form = 1,
                mode = RustEngineBridge.CandidateMode.HANT,
            ),
        )
        val result = buildContinuousSuggestionsForCandidates(candidates)
        val first = result[0]
        val second = result[1]

        // First candidate (slot 0 — was slot 1 before Item 4)
        assertEquals("true", first.additionalInfo[MetadataKeys.IS_CONTINUOUS])
        assertEquals("4", first.additionalInfo[MetadataKeys.CONSUMED_BYTES])
        assertEquals("1", first.additionalInfo[MetadataKeys.SYLLABLE_COUNT])
        assertEquals("tsua", first.additionalInfo[MetadataKeys.DISPLAY_TEXT])

        // Second candidate (slot 1 — was slot 2 before Item 4)
        assertEquals("true", second.additionalInfo[MetadataKeys.IS_CONTINUOUS])
        assertEquals("7", second.additionalInfo[MetadataKeys.CONSUMED_BYTES])
        assertEquals("2", second.additionalInfo[MetadataKeys.SYLLABLE_COUNT])
        assertEquals("珠仔", second.additionalInfo[MetadataKeys.DISPLAY_TEXT])
    }

    @Test
    fun `consumedBytes uses consumedSpanEnd not consumedSpanStart`() {
        // Engine spans are byte-relative-to-pending-buffer; commit consumes
        // bytes [start, end). The consumer needs `end` to know how many
        // bytes to drop from pending. Mid-commit candidate.
        val candidates = listOf(
            cand(
                consumedSpanStart = 3,
                consumedSpanEnd = 7,
                syllableCount = 1,
                displayText = "uan",
                score = 1.0f,
                form = 1,
                mode = RustEngineBridge.CandidateMode.HANT,
            ),
        )
        val result = buildContinuousSuggestionsForCandidates(candidates)
        assertEquals("7", result[0].additionalInfo[MetadataKeys.CONSUMED_BYTES])
    }

    @Test
    fun `displayText sidechannel is engine-supplied raw, not roman field rewrite`() {
        // Codex P1 fix `f01559cf` on iOS — TPS layout would view-rewrite the
        // roman field, breaking commitContinuous alignment. Sidechannel is
        // the contract.
        val candidates = listOf(
            cand(
                consumedSpanStart = 0,
                consumedSpanEnd = 6,
                syllableCount = 2,
                displayText = "tâi-uân",
                score = 1.0f,
                form = 1,
                mode = RustEngineBridge.CandidateMode.HANT,
            ),
        )
        val result = buildContinuousSuggestionsForCandidates(candidates)
        assertEquals(
            "displayText sidechannel must equal engine's raw displayText",
            "tâi-uân",
            result[0].additionalInfo[MetadataKeys.DISPLAY_TEXT],
        )
    }

    @Test
    fun `gamma clarification — displayText sidechannel decouples from roman`() {
        // v3.5.8 Phase 9 §10.3 clarification γ at the producer boundary.
        // After Item 6, the producer populates `TaigiWord.roman` from
        // `candidate.roman` (TL romanization, visual form) and
        // `DISPLAY_TEXT` from `candidate.displayText` (= hanji ?? roman,
        // canonical commit string). On a HANT candidate they DIVERGE —
        // `roman` shows the TL romanization, sidechannel carries the
        // hanji. The consumer
        // (`CandidateClickHandler.handleContinuousCandidateClick`) reads
        // `DISPLAY_TEXT` exclusively — no `?:` fallback to
        // `selectedWord.roman` — so γ holds regardless of which field
        // mutates. This test pins the sidechannel emission as the
        // authoritative commit-string carrier.
        val candidates = listOf(
            cand(
                consumedSpanStart = 0,
                consumedSpanEnd = 7,
                syllableCount = 2,
                displayText = "臺灣",
                score = 1.0f,
                form = 1,
                mode = RustEngineBridge.CandidateMode.HANT,
                roman = "tâi-uân",
                hanji = "臺灣",
            ),
        )
        val result = buildContinuousSuggestionsForCandidates(candidates)
        assertEquals(
            "DISPLAY_TEXT sidechannel is the canonical commit string (γ)",
            "臺灣",
            result[0].additionalInfo[MetadataKeys.DISPLAY_TEXT],
        )
        assertEquals(
            "Item 6: TaigiWord.roman carries roman (visual form), diverging from sidechannel (γ)",
            "tâi-uân",
            result[0].roman,
        )
        assertNotEquals(
            "Item 6 divergence — visual roman MUST NOT collapse onto canonical commit string",
            result[0].roman,
            result[0].additionalInfo[MetadataKeys.DISPLAY_TEXT],
        )
    }

    // --- v3.5.8 Phase 9 Item 6 — dual-line carrier shape ---

    /**
     * HANT candidate (`hanji = "臺灣"`) renders dual-line: `roman` carries
     * TL romanization, `hanzi` carries the hanji string. Tap-0 commits
     * via the sidechannel `DISPLAY_TEXT` = hanji.
     */
    @Test
    fun `Item 6 — HANT candidate emits dual-line carrier`() {
        val candidates = listOf(
            cand(
                consumedSpanEnd = 7,
                syllableCount = 2,
                displayText = "臺灣",
                mode = RustEngineBridge.CandidateMode.HANT,
                roman = "tâi-uân",
                hanji = "臺灣",
            ),
        )
        val result = buildContinuousSuggestionsForCandidates(candidates)
        assertEquals("HANT roman = TL romanization", "tâi-uân", result[0].roman)
        assertEquals("HANT hanzi = hanji string", "臺灣", result[0].hanzi)
        assertEquals("臺灣", result[0].additionalInfo[MetadataKeys.DISPLAY_TEXT])
    }

    /**
     * TAILO candidate (`hanji = null`) renders single-line: `hanzi`
     * stays null; tap commits the engine's `DISPLAY_TEXT` sidechannel
     * (= roman for TAILO).
     */
    @Test
    fun `Item 6 — TAILO candidate emits single-line carrier`() {
        val candidates = listOf(
            cand(
                consumedSpanEnd = 4,
                syllableCount = 1,
                displayText = "tāi",
                mode = RustEngineBridge.CandidateMode.TAILO,
                roman = "tāi",
                hanji = null,
            ),
        )
        val result = buildContinuousSuggestionsForCandidates(candidates)
        assertEquals("TAILO roman = TL romanization", "tāi", result[0].roman)
        assertNull("TAILO hanzi = null (no hanji)", result[0].hanzi)
        assertEquals("tāi", result[0].additionalInfo[MetadataKeys.DISPLAY_TEXT])
    }

    /**
     * MIXED candidate (`hanji` carries Latin letters per `derive_mode`
     * NFKD scan in `engine/lexicon/src/continuous.rs`). Renders
     * dual-line the same way HANT does.
     */
    @Test
    fun `Item 6 — MIXED candidate emits dual-line carrier`() {
        val candidates = listOf(
            cand(
                consumedSpanEnd = 9,
                syllableCount = 2,
                displayText = "hip相",
                mode = RustEngineBridge.CandidateMode.MIXED,
                roman = "hip-siòng",
                hanji = "hip相",
            ),
        )
        val result = buildContinuousSuggestionsForCandidates(candidates)
        assertEquals("MIXED roman = TL romanization", "hip-siòng", result[0].roman)
        assertEquals("MIXED hanzi = hanji string", "hip相", result[0].hanzi)
        assertEquals("hip相", result[0].additionalInfo[MetadataKeys.DISPLAY_TEXT])
    }

    /**
     * Defensive: a wire defect where `hanji = ""` (engine invariant
     * says `null` for TAILO, but a faulty producer might emit an empty
     * string) collapses to `null` so the cell renders single-line
     * rather than as a hanji line containing only whitespace. Mirrors
     * the spec §4.5 `c.hanji?.takeIf { it.isNotEmpty() }` guard.
     */
    @Test
    fun `Item 6 — hanji present-empty collapses to null hanzi`() {
        val candidates = listOf(
            cand(
                consumedSpanEnd = 4,
                syllableCount = 1,
                displayText = "tāi",
                mode = RustEngineBridge.CandidateMode.TAILO,
                roman = "tāi",
                hanji = "",
            ),
        )
        val result = buildContinuousSuggestionsForCandidates(candidates)
        assertNull(
            "present-empty hanji must collapse to null so the cell stays single-line",
            result[0].hanzi,
        )
    }

    @Test
    fun `synthetic ids stay outside English and NextWord sentinel ranges`() {
        // English: id <= -100. NextWord: id < 0 && id > -100.
        // Lexicon-path slot-0 composing-text cell (non-Continuous mode only):
        // id == 0. Continuous: id >= 1. Routing decision keys off
        // additionalInfo NOT id, but the sentinel ranges must not be
        // shadowed lest the click handler picks the wrong branch on a
        // metadata-decode failure fallthrough.
        val candidates = (0 until 5).map { i ->
            cand(
                consumedSpanStart = 0,
                consumedSpanEnd = 3,
                syllableCount = 1,
                displayText = "c$i",
                score = 1.0f,
                form = 1,
                mode = RustEngineBridge.CandidateMode.HANT,
            )
        }
        val result = buildContinuousSuggestionsForCandidates(candidates)
        result.forEach { word ->
            assertTrue("Continuous id must be >= 1, was ${word.id}", word.id >= 1)
            assertNotEquals("id == 0 reserved for lexicon-path composing cell", 0, word.id)
        }
    }

    @Test
    fun `empty candidate list emits empty list (caller handles fall-through)`() {
        // v3.5.8 Phase 9 Item 4: §10.7 edge case "Empty buffer" / partial
        // prefix — strip is empty when engine returns no candidates. Caller
        // (`TaigiAutocompleteService.autocomplete`) already guards
        // `if (continuousCandidates.isNotEmpty())` before invoking this
        // helper and falls through to the lexicon path when empty, which
        // re-inserts its own slot-0 composing-text cell per §10.5 mode
        // gating. The helper contract here is simply: zero candidates →
        // zero cells, no synthetic slot-0 fallback.
        val result = buildContinuousSuggestionsForCandidates(emptyList())
        assertTrue("Empty candidates → empty suggestions", result.isEmpty())
    }

    // --- v3.5.8 Phase 9.2 — CandidateMode wire decode ---

    @Test
    fun `CandidateMode decode maps all four wire values`() {
        // Pins UNSPECIFIED=0, HANT=1, TAILO=2, MIXED=3 from
        // `engine/protos/proto/composing.proto::CandidateMode`. Mirrors
        // iOS RustEngineBridgeContinuousTests.testCandidateModeDecode_AllWireValues.
        assertEquals(RustEngineBridge.CandidateMode.UNSPECIFIED, RustEngineBridge.CandidateMode.decode(0))
        assertEquals(RustEngineBridge.CandidateMode.HANT, RustEngineBridge.CandidateMode.decode(1))
        assertEquals(RustEngineBridge.CandidateMode.TAILO, RustEngineBridge.CandidateMode.decode(2))
        assertEquals(RustEngineBridge.CandidateMode.MIXED, RustEngineBridge.CandidateMode.decode(3))
    }

    @Test
    fun `CandidateMode decode falls back to UNSPECIFIED for unknown wire values`() {
        // Forward-compat: a wire value the platform binding doesn't recognize
        // (e.g. a newer engine added a fourth variant) must collapse to
        // UNSPECIFIED rather than crash or randomly map. Mirrors iOS Codex F8.
        assertEquals(RustEngineBridge.CandidateMode.UNSPECIFIED, RustEngineBridge.CandidateMode.decode(99))
        assertEquals(RustEngineBridge.CandidateMode.UNSPECIFIED, RustEngineBridge.CandidateMode.decode(-1))
    }

    // --- v3.5.8 Phase 9 Item 5 — `roman` / `hanji` wire schema ---

    /**
     * `string roman = 8` is non-optional; protobuf-javalite round-trips
     * it verbatim. Empty string is the default; explicit assignment of
     * a non-empty value must survive a serialize/deserialize pair so
     * the bridge decode path `roman = msg.roman` produces the same
     * String the engine emitted.
     */
    // 中文: Item 5 — roman 為非 optional;protobuf-javalite wire round-trip 必須保留原值。
    @Test
    fun `CandidateMessage roman field round-trips through wire`() {
        val msg = CandidateMessage.newBuilder()
            .setRoman("tâi-uân")
            .build()
        val bytes = msg.toByteArray()
        val decoded = CandidateMessage.parseFrom(bytes)
        assertEquals("tâi-uân", decoded.roman)
    }

    /**
     * `optional string hanji = 9` distinguishes "field absent on the
     * wire" (TAILO candidate — `hasHanji() == false`) from "field set
     * to empty string" (defective producer — `hasHanji() == true`,
     * `hanji == ""`). The bridge decode rule
     * `if (msg.hasHanji()) msg.hanji else null` relies on this
     * presence accessor; if protobuf-javalite ever stopped
     * distinguishing absence from empty, bridge consumers would
     * mis-classify TAILO candidates as `hanji = ""` and the dual-line
     * render rule from `docs/engine/continuous-candidate-display.md`
     * §5 would break.
     */
    // 中文: Item 5 — hanji 為 proto3 optional;wire absent vs Some("") 必須由 hasHanji() 區分。
    @Test
    fun `CandidateMessage hanji optional absent vs present-empty`() {
        // Default-constructed message has hanji absent.
        val absent = CandidateMessage.newBuilder().build()
        assertFalse("default-constructed must have hanji absent", absent.hasHanji())

        // Wire round-trip preserves absence.
        val decodedAbsent = CandidateMessage.parseFrom(absent.toByteArray())
        assertFalse(
            "absence survives wire round-trip — TAILO candidates must decode to hanji null",
            decodedAbsent.hasHanji(),
        )

        // Explicit empty-string set flips presence to true.
        val presentEmpty = CandidateMessage.newBuilder().setHanji("").build()
        assertTrue(
            "explicit empty-string assignment flips presence — distinguishes 'producer set field' from 'absent'",
            presentEmpty.hasHanji(),
        )
        val decodedPresent = CandidateMessage.parseFrom(presentEmpty.toByteArray())
        assertTrue(decodedPresent.hasHanji())
        assertEquals("", decodedPresent.hanji)

        // Non-empty content also wire-round-trips with presence.
        val presentHant = CandidateMessage.newBuilder().setHanji("臺灣").build()
        val decodedHant = CandidateMessage.parseFrom(presentHant.toByteArray())
        assertTrue(decodedHant.hasHanji())
        assertEquals("臺灣", decodedHant.hanji)
    }
}
