# Continuous-Input Candidate Display — Spec

> **Type**: Specification (problem + proposed fix, partially shipped)
> **Keywords**: `Continuous`, `Candidate`, `display`, `roman`, `hanji`, `subtitle`, `dual-line`, `wire-schema`, `eliminate-fallback`
> **Related**: [continuous-input-ranking.md](continuous-input-ranking.md), [composing.md](composing.md), [autocomplete.md](autocomplete.md), [binary-format.md](binary-format.md), [`rules/cross-platform-alignment.md`](../../rules/cross-platform-alignment.md)
> **Status**: §4 dual-line carrier shipped (Items 5–6); §15 fallback retire **COMPLETE** — Items 7–12 closed every engine syllabification gap and **Item 13 (v3.5.8 capstone) retired the platform lexicon fallback** so the Continuous engine is the single candidate source. All in v3.5.8 per `feedback_v358_full_scope.md` (satisfaction-gated; no `v3.5.9+` deferrals). Item 13 = v3.5.8 feature-complete.
> **Author**: Dogfood findings 2026-05-11. Source observation = user during v3.5.8 dogfood. §15 added 2026-05-11 (night) per user pivot 「engine 內部處理所有切音節邏輯,fallback 是冗餘」.
> **Adjacent spec (2026-05-13)**: [`continuous-input-ranking.md`](continuous-input-ranking.md) §10 — Commit Behavior & Display Split. Composing buffer (`rawInput`) vs candidate[0] (segmented) split + Enter / Tap-0 / Tap-N commit dispatch. Grounded in MOE `KeySectionsModel` (§10.1.1). Drafted; co-confirm pending in the same Codex pass as this doc.

---

## Summary

The v3.5.8 continuous-input candidate carrier (`CandidateMessage` in `composing.proto`) ships a **single `display_text` string** per candidate. Both platform UIs then build candidate cells with **subtitle = nil**, producing a single-line render. The legacy lexicon path emits the same dictionary records as **`title = roman` + `subtitle = hanji`** dual-line cells.

Because the two paths are mutually exclusive **within one `autocomplete()` call** but **toggle between calls** (Continuous active vs. fall-through), the user observes:

- **Temporal interleave** across keystrokes — a strip in continuous mode renders single-line; the next keystroke that fails to syllabify falls through to lexicon → dual-line.
- **Slot-0 vs slots 1..n contrast** within one strip (non-Continuous lexicon path only, per §10.5 Mode Gating) — slot-0 composing-text cell is always single-line (no hanji possible for pending preedit), while slots 1..n alternate based on which path produced them. In Continuous mode (§10.1.2), slot 0 is the engine ranker's top candidate and may itself be dual-line.

Expected behavior: **every dictionary-sourced continuous candidate renders dual-line (roman + hanji)** with the same display contract as the lexicon path. Single-source-of-truth data flow: one proto carrier, one cell shape, one set of UI rules.

---

## 1. Problem Statement

### 1.1 Symptoms (user-reported dogfood 2026-05-11)

1. **Continuous candidate cells show only the engine-decided display string** (typically hanji-when-available, else roman), never both in parallel.
2. **Same strip can mix single-line and dual-line cells** when the Continuous path returns some candidates and lexicon fall-through later contributes others on a subsequent keystroke.
3. **Slot-0 (`isComposingText`) cell is always single-line** ([historical 2026-05-11 reading: "this is correct — pending preedit has no hanji to show"]; **superseded** by [`continuous-input-ranking.md`](continuous-input-ranking.md) §10.1.2 for Continuous mode — slot 0 in Continuous mode is the engine ranker's top candidate, which **may** be dual-line; see §10.2 segmented-rendering rule), but it visually clashes with surrounding dual-line lexicon cells, amplifying the inconsistency.

User expectation:

> 預期必須只有「漢字+羅馬字並行」,並且是單向資料流。

### 1.2 Why this matters

- **Dogfood is qualitative gate** (per [`feedback_perf_gate.md`](../../knowledge/feedback)) — UX inconsistency drowns out signal for the remaining Phase 9 sub-PRs (9.4a / 9.4b / 9.5 / 9.6).
- **Phase 9.6 will compound the problem** — custom_dict integration will route custom candidates through the same `CandidateMessage` carrier; without a roman/hanji split, custom continuous candidates ship single-line too.
- **Mainstream IME parity** — MOE Tâi-gí (`tutgInputLine` candidates) and khiin-rs (`khiin/src/candidate.rs`) both expose roman + hanji as separate display fields; aligning brings Taigi Keyboard back into the mainstream pattern.

---

## 2. Current Implementation (cite-and-trace)

### 2.1 Two build paths

| Path | Trigger | Cell shape | Source code |
|---|---|---|---|
| Lexicon (legacy) | Continuous returns empty OR `continuousFetcher == nil` | dual-line | iOS `convertToSuggestions` ([`AutocompleteService.swift:309-322`](../../ios/Sources/TaigiKeyboard/Autocomplete/Services/AutocompleteService.swift)) / Android `autocomplete` lexicon branch ([`TaigiAutocompleteService.kt:88-124`](../../android/app/src/main/java/com/siansiansu/taigikeyboard/ime/text/composing/TaigiAutocompleteService.kt)) |
| Continuous (Phase 7B/8) | `Phase::Continuous` active + non-empty `ContinuousResponse.candidates` | single-line | iOS `buildContinuousSuggestions` ([`AutocompleteService.swift:273-292`](../../ios/Sources/TaigiKeyboard/Autocomplete/Services/AutocompleteService.swift)) / Android `buildContinuousSuggestionsForCandidates` ([`TaigiAutocompleteService.kt:228-253`](../../android/app/src/main/java/com/siansiansu/taigikeyboard/ime/text/composing/TaigiAutocompleteService.kt)) |

### 2.2 Where the roman/hanji split is lost

Dictionary records DO carry both fields:

```rust
// engine/lexicon/src/dictionary_reader.rs:40-51
pub struct DictionaryRecord {
    pub bitmask: u16,
    pub frequency: u32,
    pub hanzi: Option<String>,   // ← present
    pub tl: String,              // ← present
    pub syllable_count: u8,
}
```

The TL romanization is dropped at `record_to_candidate`:

```rust
// engine/lexicon/src/continuous.rs:400-404
let mode = derive_mode(hanzi.as_deref());
let display_text = hanzi.unwrap_or(tl);  // ← `tl` consumed here; roman lost forever
let freq_data = freq_map.get(&display_text).copied().unwrap_or_default();
```

`RawCandidate` keeps only `display_text`:

```rust
// engine/lexicon/src/continuous.rs:152-207 (excerpt)
pub struct RawCandidate {
    pub consumed_span: (u32, u32),
    pub syllable_count: u8,
    pub display_text: String,   // ← single string
    pub score: f32,
    pub form: u8,
    pub frequency: u32,
    pub bitmask: u16,
    pub mode: CandidateMode,
    pub recency_rank: u8,
}
```

Wire mirror is single-string too:

```proto
// engine/protos/proto/composing.proto:245-253
message CandidateMessage {
  uint32         consumed_span_start = 1;
  uint32         consumed_span_end   = 2;
  uint32         syllable_count      = 3;
  string         display_text        = 4;   // ← single string
  float          score               = 5;
  uint32         form                = 6;
  CandidateMode  mode                = 7;
}
```

Platform decode just propagates the single string:

| Platform | Bridge struct | Build path |
|---|---|---|
| iOS | `RustEngineBridge.ContinuousCandidate` ([`RustEngineBridge.swift:513-539`](../../ios/Sources/TaigiKeyboard/Engine/RustEngineBridge.swift)) — has `displayText: String` only | `buildContinuousSuggestions` emits `Suggestion(text: c.displayText, title: c.displayText, subtitle: nil, ...)` |
| Android | `RustEngineBridge.ContinuousCandidate` ([`RustEngineBridge.kt:635-643`](../../android/app/src/main/java/com/siansiansu/taigikeyboard/engine/RustEngineBridge.kt)) — has `displayText: String` only | `buildContinuousSuggestionsForCandidates` emits `TaigiWord(roman = c.displayText, hanzi = null, ...)` |

### 2.3 Cell render rules (already correct — needs both fields)

iOS `CandidateCellHelper` ([`CandidateCellHelper.swift:24-57`](../../ios/Sources/TaigiKeyboard/Autocomplete/Views/CandidateCellHelper.swift)) already handles three modes correctly **provided** `suggestion.text` carries roman and `suggestion.subtitle` carries hanji:

| Mode | `displayTitle` | `displaySubtitle` |
|---|---|---|
| Standard | `suggestion.text` (= roman) | `suggestion.subtitle` (= hanji) |
| `isTranslateSwapped` | `suggestion.subtitle` (= hanji) | `suggestion.text` (= roman) |
| `isTPSLayout` | `suggestion.subtitle` (= hanji, or TPS fallback) | `nil` (TPS never shows dual-line) |

Android `TaigiWord.displayText` ([`TaigiWord.kt:38-40`](../../android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/TaigiWord.kt)) prioritizes hanji-then-roman for commit; UI render side (in `SmartbarCandidateStrip`) consults `roman` + `hanzi` fields directly when present.

**Conclusion**: the existing UI layer is already capable of rendering dual-line — the missing piece is the data carrier (proto + RawCandidate + ContinuousCandidate) splitting the two fields.

---

## 3. Why the user sees "交錯" (interleave)

### 3.1 Within one `autocomplete()` call — mutually exclusive

iOS [`AutocompleteService.swift:129-138`](../../ios/Sources/TaigiKeyboard/Autocomplete/Services/AutocompleteService.swift):

```swift
if let fetcher = continuousFetcher {
    let candidates = fetcher.fetchContinuousCandidates()
    if !candidates.isEmpty {
        let suggestions = buildContinuousSuggestions(...)
        return Autocomplete.Result(inputText: text, suggestions: suggestions)
    }
}
// fall-through to lexicon path
```

Android [`TaigiAutocompleteService.kt:71-77`](../../android/app/src/main/java/com/siansiansu/taigikeyboard/ime/text/composing/TaigiAutocompleteService.kt) has the same shape. So within one fetch the strip is **all-single-line** or **all-dual-line** — never mixed in slots 1..n.

### 3.2 Across keystrokes — toggles

`Phase::Continuous` activates when the syllabifier successfully cuts the input (`build_keys_tl` / `build_keys_tps` returns non-empty endings). It deactivates / returns empty when:

- Syllable inventory unavailable (cold-start race)
- Input contains hyphens / POJ diacritics not in inventory (deferred to Phase 9.4b shadow buffer)
- ~~TPS tone-1 untoned syllables (deferred to Phase 9.4a)~~ **RESOLVED — Phase 9 Item 7 / 9.4a** (`tps::valid_span_endings` next-initial-seen rule + `build_keys_tps` digitless tone-1 accept)
- Single-character prefix below the first valid ending

Each toggle flips the strip's cell shape, producing the user-observed "交錯" across successive keystrokes.

### 3.3 Slot-0 vs slots 1..n

> **Superseded by [`continuous-input-ranking.md`](continuous-input-ranking.md) §10.1.2 for Continuous mode (Item 4 shipped 2026-05-14)**: the Continuous path no longer emits a composing-text cell at slot 0 — `candidate[0]` is the engine ranker top. The text below remains accurate for the **non-Continuous lexicon path only** (§10.5 Mode Gating).

Slot-0 (`isComposingText`, lexicon path only) is ALWAYS single-line:

- iOS `createComposingTextSuggestion` ([`AutocompleteService.swift`](../../ios/Sources/TaigiKeyboard/Autocomplete/Services/AutocompleteService.swift)): `subtitle: nil`
- Android `createComposingTextCell` ([`TaigiAutocompleteService.kt`](../../android/app/src/main/java/com/siansiansu/taigikeyboard/ime/text/composing/TaigiAutocompleteService.kt)): `hanzi = null`

This is **correct** — pending preedit has no hanji yet to display. But when slots 1..n switch to dual-line (lexicon path), slot-0's single-line stands out, amplifying the inconsistency.

---

## 4. Proposed Solution — Option A (wire-level dual-field carrier)

### 4.1 Goal

Continuous candidates ship **both** roman (TL) and hanji (when present) on every wire frame. Platform UI builds dual-line cells via the same fields the lexicon path uses. Single-line render reserved only for TAILO candidates (`hanzi = None`) where there is genuinely nothing to put in the subtitle — matching lexicon path behavior exactly.

### 4.2 Wire schema change

```proto
// engine/protos/proto/composing.proto
message CandidateMessage {
  uint32         consumed_span_start = 1;
  uint32         consumed_span_end   = 2;
  uint32         syllable_count      = 3;
  string         display_text        = 4;   // unchanged — committed text / freq key
  float          score               = 5;
  uint32         form                = 6;
  CandidateMode  mode                = 7;
  // v3.5.8 dogfood follow-up — separate display fields for dual-line render.
  string         roman               = 8;   // display romanization, always present (TL, or POJ-display in POJ mode — engine-rendered in handle_fetch_at_pos)
  optional string hanji              = 9;   // hanji display (None for TAILO)
}
```

**Why retain `display_text`**:

- Sidechannel used by `commitContinuous(display_text)` ([`engine/composing/src/transition.rs:484-490`](../../engine/composing/src/transition.rs)) for byte-aligned commit
- `user_frequency.db` write key on both platforms (iOS [`ActionHandler+Suggestions.swift:81-83`](../../ios/Sources/TaigiKeyboard/Actions/ActionHandler+Suggestions.swift) / Android [`CandidateClickHandler.kt:345-349`](../../android/app/src/main/java/com/siansiansu/taigikeyboard/ime/text/smartbar/CandidateClickHandler.kt))
- Continues to mean "what the engine considers canonical for commit + frequency tracking" (= `hanji.unwrap_or(roman)`)

The new fields are **display-only** sidechannels. The engine remains authoritative on commit / frequency keys via `display_text`.

**Why `optional` on `hanji` only**: roman (`tl` field) is mandatory in `DictionaryRecord`; hanji is `Option<String>`. proto3 `optional` keyword distinguishes "TAILO candidate, no hanji exists" (present = false) from "wire-frame defect" — defends future cases where empty-string could be ambiguous.

### 4.3 Rust changes

```rust
// engine/lexicon/src/continuous.rs
pub struct RawCandidate {
    // ... existing fields
    pub display_text: String,
    pub roman: String,            // ← new: = DictionaryRecord.tl at this layer; handle_fetch_at_pos renders it for the input mode (TL, or POJ-display in POJ)
    pub hanji: Option<String>,    // ← new: always = DictionaryRecord.hanzi
    // ... existing fields
}

fn record_to_candidate(record: DictionaryRecord, ...) -> RawCandidate {
    let DictionaryRecord { bitmask, frequency, syllable_count, hanzi, tl } = record;
    let mode = derive_mode(hanzi.as_deref());
    let roman = tl.clone();
    let display_text = hanzi.clone().unwrap_or(tl);
    // ... freq_data lookup unchanged
    RawCandidate {
        // ... existing fields
        display_text,
        roman,
        hanji: hanzi,
        // ... existing fields
    }
}
```

```rust
// engine/composing/src/dispatch.rs
fn raw_to_proto_candidate(c: RawCandidate) -> CandidateMessage {
    CandidateMessage {
        consumed_span_start: c.consumed_span.0,
        consumed_span_end:   c.consumed_span.1,
        syllable_count:      c.syllable_count as u32,
        display_text:        c.display_text,
        score:               c.score,
        form:                c.form as u32,
        mode:                c.mode.to_proto_i32(),
        roman:               c.roman,           // ← new
        hanji:               c.hanji,           // ← new (Option<String> maps to proto3 optional)
    }
}
```

### 4.4 iOS bridge

```swift
// ios/Sources/TaigiKeyboard/Engine/RustEngineBridge.swift
public struct ContinuousCandidate: Equatable {
    public let consumedSpanStart: UInt32
    public let consumedSpanEnd: UInt32
    public let syllableCount: UInt32
    public let displayText: String
    public let score: Float
    public let form: UInt32
    public let mode: CandidateMode
    public let roman: String        // ← new
    public let hanji: String?       // ← new (proto3 optional → Swift Optional)
}
```

```swift
// ios/Sources/TaigiKeyboard/Autocomplete/Services/AutocompleteService.swift
// Post-Item 4 baseline: signature has no `composingText:` param and no
// `createComposingTextSuggestion` insert (slot 0 == candidate[0] per
// `continuous-input-ranking.md` §10.1.2). The `← was:` markers below show
// the Item 5/6 additions (proto `roman` / `hanji` fields + dual-line subtitle).
internal func buildContinuousSuggestions(
    from candidates: [RustEngineBridge.ContinuousCandidate],
) -> [Autocomplete.Suggestion] {
    candidates.map { c in
        Autocomplete.Suggestion(
            text: c.roman,                                  // ← was: c.displayText
            title: c.roman,                                 // ← was: c.displayText
            subtitle: (c.hanji?.isEmpty == false) ? c.hanji : nil,  // ← was: nil
            additionalInfo: [
                "isContinuous": "true",
                "consumedBytes": String(c.consumedSpanEnd),
                "syllableCount": String(c.syllableCount),
                "displayText": c.displayText,               // unchanged — commit / freq sidechannel (γ)
            ],
        )
    }
}
```

### 4.5 Android bridge + UI

```kotlin
// android/.../engine/RustEngineBridge.kt
data class ContinuousCandidate(
    val consumedSpanStart: Int,
    val consumedSpanEnd: Int,
    val syllableCount: Int,
    val displayText: String,
    val score: Float,
    val form: Int,
    val mode: CandidateMode,
    val roman: String,        // ← new
    val hanji: String?,       // ← new (proto3 optional)
)
```

```kotlin
// android/.../ime/text/composing/TaigiAutocompleteService.kt
// Post-Item 4 baseline: signature has no `composingText` param and no
// `createComposingTextCell` insert (slot 0 == candidate[0] per
// `continuous-input-ranking.md` §10.1.2). The `← was:` markers below show
// the Item 5/6 additions (proto `roman` / `hanji` fields + dual-line hanzi).
internal fun buildContinuousSuggestionsForCandidates(
    candidates: List<RustEngineBridge.ContinuousCandidate>,
): List<TaigiWord> =
    candidates.mapIndexed { index, c ->
        TaigiWord(
            id = index + 1,
            roman = c.roman,                                       // ← was: c.displayText
            hanzi = c.hanji?.takeIf { it.isNotEmpty() },           // ← was: null
            lengthScore = null,
            additionalInfo = mapOf(
                TaigiWord.MetadataKeys.IS_CONTINUOUS to "true",
                TaigiWord.MetadataKeys.CONSUMED_BYTES to c.consumedSpanEnd.toString(),
                TaigiWord.MetadataKeys.SYLLABLE_COUNT to c.syllableCount.toString(),
                TaigiWord.MetadataKeys.DISPLAY_TEXT to c.displayText,  // unchanged — commit / freq sidechannel (γ)
            ),
        )
    }
```

### 4.6 Slot-0 stays single-line

> **Superseded by [`continuous-input-ranking.md`](continuous-input-ranking.md) §10.1.2 for Continuous mode** (2026-05-13).
> In Continuous mode, slot 0 is the engine ranker's top candidate rendered with segmentation, **not** a dedicated composing-text cell. The visual distinction described below no longer applies (removed in commit `224a8aa3`).
> The text below is retained as historical context and remains accurate for the **non-Continuous lexicon path only** (§10.5 Mode Gating).

`createComposingTextSuggestion` / `createComposingTextCell` unchanged — pending preedit has no hanji. **This is intentional** (matches lexicon path's slot-0 contract). Per Q4 below, accept the slot-0 vs slots 1..n contrast as inherent to "pending vs committed-candidate" visual distinction.

---

## 5. Edge Cases (per `CandidateMode`)

| Mode | `DictionaryRecord.hanzi` | `roman` (= tl) | `hanji` (= hanzi) | Cell render |
|---|---|---|---|---|
| HANT | `Some("臺灣")` | `"tâi-uân"` | `Some("臺灣")` | dual-line: `tâi-uân` / `臺灣` |
| TAILO | `None` | `"tāi"` | `None` | single-line: `tāi` (matches lexicon path TAILO) |
| MIXED | `Some("hip相")` | `"hip-siòng"` | `Some("hip相")` | dual-line: `hip-siòng` / `hip相` |

TPS-layout interaction (`isTPSLayout = true`):

- `CandidateCellHelper.displayTitle` returns `subtitle` (= hanji) when present, else TPS fallback
- For HANT/MIXED continuous candidates: title shows hanji, no subtitle (single-line, but hanji-primary — correct TPS behavior)
- For TAILO continuous candidates: `subtitle = nil`, title falls through to TPS conversion of roman — matches lexicon path

`isTranslateSwapped = true`:

- `displayTitle` returns subtitle (= hanji) as primary; `displaySubtitle` returns text (= roman)
- All three modes render correctly: HANT/MIXED swap; TAILO stays single-line roman

---

## 6. Alternatives Considered

### 6.1 Option B — Platform-side dictionary re-lookup

UI looks up roman/hanji by `display_text` on each candidate via a second FFI call.

**Rejected**:
- Violates "engine owns display strings" principle ([`project_v358_continuous_input.md` §關鍵設計決定 §8](../../knowledge/feedback))
- Violates [`rules/cross-platform-alignment.md`](../../rules/cross-platform-alignment.md) "behavior contract pinned at engine layer"
- Doubles FFI overhead per fetch (typically 5-30 candidates)
- Race window: between FetchAtPos response and platform re-lookup the dictionary state could change

### 6.2 Option C — Merge lexicon + continuous build paths

Refactor so all candidates flow through one carrier; lexicon path becomes an internal data source consumed by the continuous-path code.

**Rejected**:
- Scope = full v3.5.9+ release; out of v3.5.8 Phase 9 timebox
- Would touch the lexicon `search` API surface that 10+ platform call sites depend on
- Lexicon path has different ranking (linear `process_candidates`, no SortKey lattice) — merging requires re-design of `engine/ranking`

### 6.3 Option D — Wire `hanji_or_empty` (no proto3 optional)

Always emit `hanji` as a `string` field, use empty string to signal TAILO.

**Rejected**:
- Loses distinction between "TAILO" vs "field accidentally omitted by buggy producer"
- proto3 `optional` keyword has been stable since proto3 v3.15 (2021) — no compatibility blocker
- Existing `composing.proto` already uses `optional` for `ComposingResponse.continuous` ([`composing.proto:196`](../../engine/protos/proto/composing.proto)); precedent established

---

## 7. Wire Backward-Compat Analysis

proto3 additive change — new fields default to empty when absent.

| Producer | Consumer | Wire | Behavior |
|---|---|---|---|
| Old Rust (no roman/hanji) | Old platform | `display_text` only | unchanged — current behavior |
| Old Rust | New platform | `roman = ""`, `hanji = absent` | platform sees empty roman → would fall back to `display_text` for safety (defensive read needed) |
| New Rust | Old platform | New fields ignored | unchanged — current behavior (graceful) |
| New Rust | New platform | Full wire | dual-line render (target) |

**Defensive read (platform side)**: if `roman.isEmpty()` after decode, fall back to `displayText` for cell title. This protects against partial rollout where the librust_taigi.a was rebuilt but proto regen was skipped, or where a stale wire reaches the platform. (Phase 6 already requires lockstep regen per [`feedback_proto_gen_script.md`](../../knowledge/feedback) so this should never happen in practice.)

**Asset coupling**: librust_taigi.a is bundled inside the app on both platforms — no over-the-air engine update, no out-of-band wire skew. Release ships proto + Rust + iOS bridge + Android bridge in lockstep.

---

## 8. Test Matrix

### 8.1 Rust unit tests

| Test | Location | Asserts |
|---|---|---|
| `record_to_candidate_populates_roman_and_hanji` | `engine/lexicon/src/continuous.rs` (mod test) | HANT/TAILO/MIXED records → correct `roman` + `hanji` |
| `raw_to_proto_candidate_propagates_roman_hanji` | `engine/composing/src/dispatch.rs` (mod test) | All three modes round-trip |
| `derive_mode_consistent_with_hanji_presence` | existing (no change) | Verify mode/hanji invariant: `hanji.is_none() ⇔ mode == TAILO` |

### 8.2 Rust integration tests

| Test | Location | Asserts |
|---|---|---|
| `span_local_fetch_carries_roman_hanji_for_hant` | `engine/lexicon/tests/span_local_fetch.rs` | `RawCandidate` for hanji entry has both fields |
| `span_local_fetch_tailo_has_no_hanji` | same | `RawCandidate.hanji.is_none()` for roman-only entry |

### 8.3 iOS bridge tests

| Test | Location | Asserts |
|---|---|---|
| `decodeContinuousCandidate_carriesRoman` | `ios/Tests/TaigiKeyboardTests/Engine/RustEngineBridgeContinuousTests.swift` | proto wire with `roman = "tsua"` → `ContinuousCandidate.roman == "tsua"` |
| `decodeContinuousCandidate_carriesHanjiAsOptional` | same | proto wire with `hanji = "珠"` → `.hanji == "珠"`; absent → `.hanji == nil` |
| `buildContinuousSuggestions_emitsDualLineForHant` | `AutocompleteServiceContinuousTests.swift` (new) | HANT candidate → `Suggestion(title: roman, subtitle: hanji)` |
| `buildContinuousSuggestions_emitsSingleLineForTailo` | same | TAILO candidate → `Suggestion(subtitle: nil)` |

### 8.4 Android bridge tests

| Test | Location | Asserts |
|---|---|---|
| `decode_continuous_candidate_carries_roman` | `android/app/src/test/.../engine/RustEngineBridgeContinuousTest.kt` (new) | Mirror of iOS |
| `buildContinuousSuggestionsForCandidates_emits_TaigiWord_with_hanji` | `ContinuousSuggestionsContractTest.kt` (existing) | New assertion: HANT candidate → `TaigiWord.hanzi != null` |

### 8.5 Acceptance / regression

| Scenario | Expected |
|---|---|
| Type `tsua` (TL, continuous) | All candidate cells dual-line (roman + 珠 / 抓 / etc.). **Slot 0 = engine ranker's top candidate** (per `continuous-input-ranking.md` §10.1.2; superseded prior "= `tsua` single-line composing cell" expectation). |
| Type `taixyz` (continuous fails → lexicon fallback) | Still dual-line; identical to before |
| Type `peⁿ` (POJ diacritic, continuous defers) | lexicon path dual-line; unchanged |
| Type `ㄉㄧㄠˊ` (TPS) | Continuous candidates: title shows hanji (TPS-layout rule) |
| Toggle `isTranslateSwapped` mid-composing | Continuous candidates swap title/subtitle correctly |

---

## 9. Open Questions for Codex Pre-Impl Consult

Before any code is written, Codex consult should resolve:

### Q1 — Field names

Options:

- **A**: `roman` + `hanji` (matches Taiwanese romanization heritage spelling 漢字 → hanji)
- **B**: `tl` + `hanzi` (matches `DictionaryRecord` field names)
- **C**: `romanization` + `hanji_text` (verbose)

Recommendation: **A**. Aligns with `TaigiWord.kt` Android field name (`hanzi`), `taigi-converter` zh-TW glossary, and existing `tl_notone` / `tl` already-overloaded `tl` would conflict with the "FST key" usage.

Trade-off: `DictionaryRecord` uses `hanzi` (Hanyu Pinyin spelling). Stick with `hanzi` everywhere for consistency, OR rename `DictionaryRecord` field. Recommended: keep `hanzi` everywhere in code; UI strings + commit messages use 漢字 / hanji as preferred.

### Q2 — Should `display_text` stay on the wire?

Option **Keep** (recommended): platform commit + frequency tracking are existing contracts; `display_text` = `hanji ?? roman` derivable but already-encoded means no platform-side recomputation.

Option **Derive**: platforms compute `displayText = hanji?.takeIf { it.isNotEmpty() } ?: roman` on decode, drop wire field.

Trade-off: keeping wire field costs ~5-15 bytes per candidate × ~30 candidates = ~450 bytes per fetch wire overhead. Negligible. Removing it changes the commit/frequency contract = larger blast radius. **Keep**.

### Q3 — Proto field tags

Adding `roman = 8` + `hanji = 9` extends `CandidateMessage` past `mode = 7`. Tags 1-7 already in use; tags 8-9 free. Safe.

### Q4 — Slot-0 (`isComposingText`) cell

Stays single-line — pending preedit has no hanji yet. Confirm Codex agrees this is the correct invariant and NOT a regression to fix.

### Q5 — Sidechannel `displayText` in `additionalInfo`

iOS [`ActionHandler+Suggestions.swift:56`](../../ios/Sources/TaigiKeyboard/Actions/ActionHandler+Suggestions.swift): `let displayText = suggestion.additionalInfo["displayText"] ?? suggestion.text`

After Option A, `suggestion.text` is roman (not displayText). The sidechannel must stay because TPS layout's `CandidateCellHelper.suggestionToHandle` can rewrite `suggestion.text` via `tlNumericToTPS` — the original engine-supplied displayText would be lost without the sidechannel (per the existing comment at line 33-39 of that file).

**Decision**: keep sidechannel; no change.

### Q6 — TPS-layout primary-display selection

For TPS continuous candidates with HANT mode (`hanji = Some("台")`), `CandidateCellHelper.displayTitle` returns subtitle = hanji = "台". For TAILO mode (`hanji = None`), title falls through to `tpsFallback(suggestion)` = `RustEngineBridge.tlNumericToTPS(suggestion.text)` where `suggestion.text = roman`.

This means a TAILO continuous candidate (e.g. raw English-leaning entries) under TPS layout displays Bopomofo-converted roman — which is **identical** to lexicon-path TAILO under TPS. Confirm Codex agrees this is correct.

### Q7 — `isTranslateSwapped` × Continuous

Lexicon path's swap behavior is well-tested. After Option A, continuous candidates have the same `Suggestion(text:, subtitle:)` shape so swap should "just work". Codex consult: any non-obvious interaction with `consumedBytes` / `syllableCount` decode when the cell title becomes hanji (user-tap path)?

**Pre-analysis**: tap path goes through `additionalInfo["isContinuous"] == "true"` check first (iOS [`ActionHandler+Suggestions.swift:40`](../../ios/Sources/TaigiKeyboard/Actions/ActionHandler+Suggestions.swift) / Android [`CandidateClickHandler.kt:300`](../../android/app/src/main/java/com/siansiansu/taigikeyboard/ime/text/smartbar/CandidateClickHandler.kt)), reads `consumedBytes` + `syllableCount` from sidechannel, calls `commitContinuous(displayText: sidechannel.displayText, ...)`. Swap affects render only, not tap routing. Should be safe.

### Q8 — Phase 9 sub-PR positioning

Options:

- **A**: New sub-PR **Phase 9.4a-display** before TPS tone-1 (9.4a) — un-block dogfood baseline for subsequent phases
- **B**: New sub-PR **Phase 9.7** after 9.6 — preserve existing 9.4 → 9.6 numbering
- **C**: Renumber 9.4a → 9.4b → 9.4c → 9.5 → 9.6 → 9.7 (= TPS tone-1 / shadow buffer / display fix / 台灣台語 / custom / consult-final)

Recommendation: **A** with rename to `Phase 9.4a` (display fix) and the current 9.4a/9.4b shifted to 9.4b/9.4c. Per [§1.2](#12-why-this-matters) the display fix unblocks clean dogfood signal for the rest of Phase 9.

### Q9 — Pair with Finding 1 (dashed border removal)?

Finding 1 in [`project_v358_dogfood_findings.md`](../../knowledge/feedback) — user-decide to remove the dashed border overlay on slot-0. Should it ship together (one PR) or separately (two PRs)?

Options:

- **A**: One PR — Finding 1 is trivial (~30 LOC) and naturally bundles with the display contract change. Codex sandwich covers both.
- **B**: Two PRs — Finding 1 = pure UI / decision call; Finding 2 = wire schema. Different review focus.

Recommendation: **B**. Cleaner review boundaries; Finding 1 can land first as a quick UI PR even before Codex consults on Finding 2.

---

## 10. Out-of-Scope (YAGNI guards)

Per [`feedback_no_future_planning.md`](../../knowledge/feedback) and [`rules/rust-best-practices.md`](../../rules/rust-best-practices.md) §non-goals:

- **No** `CandidateMode`-based rendering rules — Phase 9.2 mode is metadata-only; cell shape is decided by `hanji` presence (mirrors lexicon path)
- **No** new `display_strategy` / `display_hints` proto field — single roman + hanji pair is sufficient
- **No** changes to `commitContinuous` wire (display_text/consumed_bytes/syllable_count stays exactly as is)
- **No** changes to `engine/ranking` SortKey **base policy** (display fields don't enter ranking) — see §15.5 for how partial-prefix candidates fit the 8-dim SortKey (S8 demoted `-coverage` to a weak tiebreak below score/freq)
- **No** changes to `user_frequency.db` schema (commit key remains `display_text`)
- **No** custom_dictionary integration (still scheduled for Phase 9.6 — wire fields will naturally flow once custom path emits `RawCandidate`)
- **No** keyboard-level mode toggle (HanjiMode / TailoMode like MOE) — Taigi Keyboard's `isTranslateSwapped` axis is the deliberate UX differentiator (§11 + §15.1)

**Previously out-of-scope, NOW IN SCOPE per §15** (user pivot 2026-05-11 night):

- POJ-diacritic input continuous-mode support (canonicalize per-syllable inside engine — §15.3.B)
- Partial-prefix continuous-mode support (engine prefix-match path when syllabifier returns empty — §15.3.D)
- Platform lexicon-path retire for romanization-input flows (engine becomes single source of candidates — §15.4)

---

## 11. Mainstream IME Cite-and-Trace

Per [`feedback_plan_cite_best_practices.md`](../../knowledge/feedback) — display contract precedents:

| IME | Carrier | Field shape |
|---|---|---|
| MOE Tâi-gí Android (per [`project_v358_continuous_input.md`](../../knowledge/feedback) MOE探索) | `NailCandidate { VocType, candidate_id }` + `tutgInputLine` lookup | Distinct `hanji` / `tailo` fields per `NailCandidate`; UI renders both |
| khiin-rs ([`docs/references/khiin-reference.md`](../references/khiin-reference.md)) | `Candidate { tl: String, ascii: String, hanji: String }` | Three fields; `Bigram` ranker reads `hanji`, UI reads pair |
| librime (CJK general) | `CandidateInfo { text, comment, type }` | `text` = primary (hanji); `comment` = secondary (annotation, often romanization) |
| azooKey (per [`docs/references/azookey-reference.md`](../references/azookey-reference.md)) | `Candidate { text: String, ruby: String? }` | `text` = primary, `ruby` = furigana / secondary |

All four reference IMEs ship **at least two display fields** per candidate. Taigi Keyboard's current single-`display_text` shape is the outlier; Option A brings parity.

---

## 12. Cross-Platform Parity Tier

Per [`rules/cross-platform-alignment.md`](../../rules/cross-platform-alignment.md) §1b parity-correction tier:

- Wire schema change ships in **same release tag** on both platforms
- iOS + Android `ContinuousCandidate` data class gains identical 2 fields
- `buildContinuousSuggestions` / `buildContinuousSuggestionsForCandidates` updated together
- §1c shared-core-candidate constraint: this is a wire-schema additive change with backward-compat defaults; no shared-core algorithm extracted yet — moot

---

## 13. Implementation Phasing (rough — Codex finalizes)

| Step | Files | LOC est. |
|---|---|---|
| 1. Proto schema | `engine/protos/proto/composing.proto` + regen `.pb.swift` + `.java` | ~10 + auto-regen |
| 2. Rust `RawCandidate` + `record_to_candidate` | `engine/lexicon/src/continuous.rs` | ~20 + 2 new unit tests |
| 3. Rust `raw_to_proto_candidate` | `engine/composing/src/dispatch.rs` | ~5 + 1 propagation test |
| 4. iOS `ContinuousCandidate` + decode | `ios/.../Engine/RustEngineBridge.swift` (struct + `composingFetchDispatch` decode) | ~15 + 1 bridge wire test |
| 5. iOS `buildContinuousSuggestions` | `ios/.../Autocomplete/Services/AutocompleteService.swift` | ~5 + 2 service-level tests |
| 6. Android `ContinuousCandidate` + decode | `android/.../engine/RustEngineBridge.kt` (data class + `composingFetchDispatch` decode) | ~15 + 1 bridge wire test |
| 7. Android `buildContinuousSuggestionsForCandidates` | `android/.../ime/text/composing/TaigiAutocompleteService.kt` | ~5 + extend existing `ContinuousSuggestionsContractTest.kt` |
| 8. Defensive read fallback | iOS + Android `if roman.isEmpty()` paths | ~6 |
| 9. Acceptance dogfood + manual `xcframework` rebuild + `assembleDebug` | user-run | — |
| 10. Roadmap row update | `docs/roadmap.md` § Phase 9 | ~3 lines |

**Total estimate**: ~80-120 LOC handcoded + ~50 LOC tests + auto-regen wire bindings. Comparable in size to Phase 7A bridge wiring or Phase 9.2 mode carrier.

---

## 14. Status & Next Action

- **2026-05-11 (day)**: §1-14 display fix drafted from dogfood findings; recorded in [`project_v358_dogfood_findings.md`](../../knowledge/feedback)
- **2026-05-11 (night)**: §15 added per user pivot — eliminate platform-side lexicon fallback; engine becomes single source of candidates (MOE `tutgInputLine` analog). All work scoped to v3.5.8 per [`feedback_v358_full_scope.md`](../../knowledge/feedback) (user wording: 「v3.5.8 的版本就是連續打字的版本,修復到我滿意為止」).
- **Pending**: Codex weekly quota recovery → pre-impl consult on §9 + §15.7 Open Questions
- **Then**: implement display fix (§1-14) + fallback retire (§15) as Phase 9 continuation; sub-PR count not capped per `feedback_round_hygiene.md` 200-500 LOC/PR discipline

This spec is **frozen** until Codex consult; updates after that should be tracked in commit messages, not retroactive edits.

---

## 15. Architectural Extension — Eliminate Lexicon Fallback

> Added 2026-05-11 (night). Original §1-14 still holds for the display-fix subset; this section adds the architectural simplification user requested after reviewing the §3 "interleave" root-cause analysis.

### 15.1 Motivation

User wording: 「fallback 是冗餘設計,讓邏輯複雜化,應像 MOE 那樣在 engine 內部就處理所有切音節邏輯」.

Three reinforcing reasons:

1. **Two-path complexity** is the root cause of dogfood Finding 2. Even after Option A (§4) ships dual-line cells on both paths, the platform still has `if continuous returns empty → fallback to lexicon` branching. The branching itself is what produces the temporal toggle described in §3.2. Eliminating it gives a **truly single-directional data flow**:

   ```
   user input → engine (handles all syllabification + lookup) → candidates → UI
   ```

   This is the MOE `tutgInputLine` model, captured in `references/moe_taigi_apk/decompiled/sources/moe/taigi/Tailo.java:77` — there is no second `GetCandidatesAtBackupPath` API.

2. **Engine is already authoritative on Composing state** ([`engine/composing/src/api.rs`](../../engine/composing/src/api.rs) Phase enum + Phase 4 auto-promote contract). The platform calling `lexicon::search` directly from `autocomplete()` bypasses Composing state entirely — a layering violation that worsens as Phase::Continuous coverage broadens.

3. **MOE parity audit**:
   - Engine-side carrier already has both fields ([`CandidateModel.java`](../../references/moe_taigi_apk/decompiled/sources/android/moe/taiwanese/taigi/data/local/model/CandidateModel.java) `tailos: List<String>` + `vocabulary: String`)
   - MOE's keyboard-level `HanjiMode` / `TailoMode` filter is a different UX axis from our `isTranslateSwapped` — **we deliberately keep dual-line** as the differentiator (§11 cite + user wording 「雙行是我比 MOE 做到更優秀的部分」)
   - But MOE's engine-as-single-source architecture aligns with what we're moving toward

### 15.2 Scope clarification — what stays platform-side

Per Q1 clarification 2026-05-11, **the only input modes are TL / POJ / TPS romanization** + composing-text slot-0. There is **no hanzi input mode**. A hanzi (CJK) composing buffer must never produce keyboard candidates — historically a **defensive D-8 guard** in the platform `LexiconService.search`. **DONE (Item 11 + Item 13)**: the guard now lives entirely in the engine (`handle_fetch_at_pos` `is_hanzi(raw)` → empty carrier, §15.3.E); the platform `LexiconService.search` D-8 guard + its parity tests were deleted with the lexicon-fallback retire (§15.4). See `docs/architecture/behavioral-invariants.md` §14 (re-pointed engine-ward).

### 15.3 Engine coverage gaps to close

| # | Gap | Current state | Engine work to do |
|---|---|---|---|
| **A** | TPS tone-1 untoned (`ㄉㄞ`) | ~~`tps::valid_span_endings` only sees tone-mark / 入聲韻尾 terminators~~ | **DONE — Phase 9 Item 7 / 9.4a**: `tps::valid_span_endings` next-initial-seen + trailing-tone-1 rule; `phonetics::is_tps_initial` / `is_tps_char`; `build_keys_tps` accepts digitless toneless fragment |
| **B** | POJ-diacritic input (`pe̍h`, `chóa`, `peⁿ`) | ~~`to_ascii_lowercase` in `build_keys_tl` doesn't strip diacritics~~ | **DONE — Item 9**: `dispatch.rs::canonicalize_poj_shadow` per-syllable canonicalize + offset map composed with the Item 8 hyphen shadow |
| **C** | Hyphenated TL (`tai-bak`, `pe̍h-ōe-jī`) | ~~`tl_syll::valid_span_endings` BFS, no hyphen inventory entries~~ | **DONE — Item 8**: `dispatch.rs::build_hyphen_shadow` shadow buffer + byte-offset map |
| **D** | Partial prefix (`t`, `gu`, anything shorter than first valid syllable ending) | ~~Syllabifier returns `{}` → continuous empty → fallback to lexicon prefix-match~~ | **DONE — Item 10**: `handle_fetch_at_pos` falls through to `fetch_partial_prefix_candidates` (`prefix_index.lookup_prefix`) with the `coverage_kind` SortKey dim (§15.5) ranking partial below full-syllable |
| **E** | Hanzi accidentally in composing buffer | ~~Platform D-8 guard in `LexiconService` returns `[]`~~ | **DONE — Item 11 + Item 13**: `handle_fetch_at_pos` checks `is_hanzi(raw)` at top of dispatch → empty `ContinuousResponse`; the platform `LexiconService.search` D-8 guard was deleted with the §15.4 retire |

All engine coverage gaps closed by Items 7–12; **Item 13 (capstone) then retired the platform lexicon fallback** so the engine is the single candidate source.

### 15.4 Platform simplification — DONE (Item 13)

After §15.3 landed, both platforms collapsed to a single path:

```swift
// iOS — AutocompleteService.swift autocomplete(_:)
func autocomplete(_ text: String) async throws -> Autocomplete.Result {
    guard !text.isEmpty, let composing = activeComposingContext() else {
        return Autocomplete.Result(inputText: text, suggestions: [])
    }
    let candidates = continuousFetcher?.fetchContinuousCandidates() ?? []
    let suggestions = buildContinuousSuggestions(from: candidates)
    return Autocomplete.Result(inputText: text, suggestions: suggestions)
}
```

```kotlin
// Android — TaigiAutocompleteService.kt autocomplete(...)
suspend fun autocomplete(rawInput: String, displayText: String, ...): List<TaigiWord> {
    if (rawInput.isEmpty() || displayText.isEmpty()) return emptyList()
    val candidates = continuousFetcher()
    return buildContinuousSuggestionsForCandidates(candidates)
}
```

Empty-engine state: per §15.6 acceptance criteria, the Continuous strip is empty (`buildContinuousSuggestions` returns `[]`); the inline pre-edit retains the composing buffer, and Enter still commits the raw tail via Item 3's `Phase::Continuous` `Intent::CommitRaw` arm. The pre-§10 "always insert slot-0 composing-text cell" affordance was retired in Item 4 (`docs/engine/continuous-input-ranking.md` §10.1.2 supersedes notice).

What was deleted — **asymmetric** because iOS Tab3 was already decoupled to a separate `DictionarySearchService` while Android Tab3 shares the `LexiconService` class:

| Platform | Deleted | Notes |
|---|---|---|
| iOS | **Whole** `LexiconService.swift` (~273 LOC) + `CompositionRoot.lexiconService` | iOS `LexiconService` was 100% autocomplete-owned (verified: sole consumer was `AutocompleteService`). iOS Tab3 uses `DictionarySearchService` → bridge `lexiconSearchByHanzi/WithSources`, NOT `LexiconService`. |
| iOS | `AutocompleteInputClassifier.swift` (~22 LOC); `AutocompleteService` lexicon path (`searchLexicon` / `applyContextBoost` / `remapBoostedWords` / `convertToSuggestions` / `buildSuggestions` / `createComposingTextSuggestion`) + ctor/provider slim (`lexiconService` / `nextWordService` / `settingsProvider` / `selectionContext` / `setSelectionContextProvider`) | `autocomplete(_:)` collapsed to engine-only; no `await` remains so the stale-result guard was dropped. |
| iOS | `LexiconServiceHanziGuardTests.swift` (~57 LOC) | D-8 guard now engine-owned (Item 11) — see `behavioral-invariants.md` §14. |
| Android | `LexiconService.kt` `search()` + `lookupCustomDictionary` / `querySystemDictionaries` / `processCandidates` + ctor slim (`customDict` / `userFreq`) | **Class kept** — Tab3 (`DictionarySearchViewModel`) uses `searchWithSources` / `searchByHanzi` on the same class, so only the autocomplete-only method surface was removed. |
| Android | `AutocompleteInputClassifier.kt` (~18 LOC); `TaigiAutocompleteService.kt` lexicon path (`applyContextBoost` / `remapBoostedWords` / `createComposingTextCell`) + ctor slim (mode-agnostic; `lexicon` / `nextWord` / `settings` dropped); `CandidateUpdateCoordinator` mode-cache + dead-arg cleanup | — |
| Android | `LexiconServiceHanziGuardTest.kt` (~28 LOC) | Same engine-owned guard rationale. |

iOS = whole-file deletion; Android = method-level deletion within a kept class. Same intended behavior (engine single source); different impl surface. (Earlier drafts of this section estimated ~120 LOC iOS + "LexiconService stays for Tab3" — that was Android-centric and **wrong for iOS**, corrected here from grounded code.)

Empty-engine state: per §15.6, the strip is empty; the inline pre-edit retains the composing buffer and Enter commits the raw tail via Item 3's `Phase::Continuous` `Intent::CommitRaw` arm.

`LexiconService` as a class **survives only on Android** (Tab3 `searchWithSources` / `searchByHanzi`). On iOS the class is gone entirely; Tab3 is served by the orthogonal `DictionarySearchService`. iOS file deletions require a manual Xcode project (`pbxproj`) update by the maintainer (`feedback_xcode_manual` / `feedback_pbxproj_sync_gap`).

### 15.5 Ranking adjustment for partial-prefix candidates

Partial-prefix candidates (§15.3.D, `consumed_span = (0, raw.len())` where raw covers no complete syllable) must **rank below** full-syllable candidates so a user typing `gu` still sees regular `gua` / `guá` lexicon hits but ranked under any actual phrase match if one exists.

**Proposed `SortKey` extension**: prepend a new `coverage_kind` dimension (`u8`) ahead of the existing `tier`:

| `coverage_kind` | Meaning |
|---|---|
| `0` | Full-syllable match (existing — `valid_span_endings` produced an ending) |
| `1` | Partial-prefix match (new — syllabifier returned empty, `prefix_index.lookup_prefix` produced hits) |

This pushes ALL partial candidates strictly below ALL full-syllable candidates regardless of frequency. Inside `coverage_kind == 1`, the `(tier, recency_rank, -score, -freq, -coverage_bytes, ...)` lexicographic policy applies (Tier 0 = "consumed full buffer" is rare here since partial-prefix `consumed_span = (0, raw.len())` may or may not equal full buffer — `raw_len` comparison still valid).

> **v3.5.8 整句 lattice + walker S8 — `-coverage_bytes` demoted (dim 3 → dim 6).** Pre-S8 a graded longest-coverage-first rule sat directly above `-score`/`-freq` inside a tier (the original pre-walker Gap-A surfacing of phrases). With the slot-0 whole-sentence walker now owning phrase priority (§7.3 "G1 closed by S2"), that rule only buried the short single-syllable first-segment candidate the user wants for segment-by-segment selection (dogfood: typing `guaikingkahuekhoo`, 「我」/Guá ranked behind even 2-syllable candidates). `-coverage_bytes` is now a weak tiebreak below `-score`/`-freq` — it separates two candidates only when score AND freq are equal. This matches librime's per-segment menu (`references/librime/src/rime/gear/script_translator.cc` `kNumExactMatchOnTop`): keep multi-length candidates visible, but never let a longer code-length bury a shorter strict match. Engine-only, no wire/proto/Model-B change.

**Wire impact**: `coverage_kind` stays internal to `RawCandidate` (does NOT enter `CandidateMessage`); same pattern as `recency_rank` ([`continuous.rs:195-206`](../../engine/lexicon/src/continuous.rs)).

**SortKey insertion order**:

```rust
struct SortKey {
    coverage_kind: u8,             // Item 10 — 0 = full-syllable, 1 = partial-prefix
    tier: u8,                      // 0 = consumed_span_end == raw_len, else 1
    recency_rank: u8,              // 0 = recent, 1 = stale
    neg_score: Reverse<NonNanF32>, // freq × syll_bias × boost, desc
    neg_freq: Reverse<u32>,        // raw freq, desc
    neg_coverage: Reverse<u32>,    // S8 — DEMOTED here (was dim 3); weak tiebreak
    source_rank: u8,               // custom=0 … default=5
    stable_idx: u32,               // insertion order
}
```

(8 dimensions. PR-9.1 = 7; Item 10 prepended `coverage_kind`; S8 relocated `neg_coverage` from dim 3 to dim 6.)

### 15.6 Test matrix for fallback retire — DONE

| Test class | Location | What it pins |
|---|---|---|
| `partial_prefix_engine_path` | `engine/composing/tests/dispatch_continuous.rs` | `Phase::Continuous` + `raw = "gu"` (no valid ending) → `FetchAtPos` returns `prefix_index.lookup_prefix("tl:gu")` hits with `coverage_kind = 1` |
| `poj_diacritic_canonicalize` | same | `Phase::Continuous` + `raw = "pe̍h"` → canonicalized to `pek` → `tl:pek` key → expected hits |
| `hanzi_guard_in_engine` | `engine/composing/src/dispatch.rs` (mod test) | `raw = "我好"` (CJK chars) → `FetchAtPos` returns empty `ContinuousResponse` (carrier present, candidates empty) |
| `sort_key_partial_below_full` | `engine/lexicon/src/continuous.rs` (mod sort_key_tests) | Construct two `RawCandidate` — one with `coverage_kind = 0` lowest freq, one with `coverage_kind = 1` highest freq — assert full-syllable wins |
| `platform_autocomplete_no_lexicon_branch` | iOS `AutocompleteServiceContinuousTests.swift::testAutocomplete_EmptyEngine_NoLexiconBranch_EmptyResult` / Android `TaigiAutocompleteServiceTest.kt` | Mock `continuousFetcher` returning empty → `autocomplete` result is **empty `[]`** (no slot-0 composing cell, no lexicon path — the path no longer exists). Plus a positive control: non-empty engine result passes through 1:1 with `isContinuous` set and no `isComposingText`. |

### 15.7 Open Questions for Codex Pre-Impl Consult — RESOLVED

> **Resolved across Items 7–13** (each ran its own Codex pre-impl sandwich). Q15.1–Q15.5 settled in Items 10–11 (`coverage_kind` = internal `u8`; partial-prefix pass-through `syllable_count`; `tl:`-namespaced key; partial-prefix always final-commit; `MIN_PREFIX_LEN = 1`). **Q15.6 (Tab3 untouched)**: confirmed in Item 13 from grounded code — iOS Tab3 = `DictionarySearchService` (never used `LexiconService`); Android Tab3 = `LexiconService.searchWithSources`/`searchByHanzi` (kept; only the autocomplete `search()` surface was deleted). Q15.7 sub-PR plan superseded by the Item 7–13 numbering. Original text retained below for provenance.

Adds to §9 (historical — resolved as above):

**Q15.1 — `coverage_kind` enum or u8?**

Same dilemma as Phase 9.2 `CandidateMode` — does this graduate to a typed enum, or stay a u8 ordinal? Recommendation: **u8 internal-only** (not on wire, not user-facing) — keeps the `derive(Ord)` SortKey clean without an extra enum.

**Q15.2 — Partial-prefix `syllable_count` value?**

The current `syllable_count: u8` field in `RawCandidate` is dictionary-source (`DictionaryRecord::syllable_count`). For partial-prefix hits, the dictionary record's syllable_count still applies (`紙` = 1, `珠仔` = 2) — should it be passed through unchanged, or set to `0` to signal "partial"?

Recommendation: **pass through unchanged**. `coverage_kind` already discriminates partial vs full; `syllable_count` retains its dictionary meaning for downstream sort tie-breaks.

**Q15.3 — `prefix_index.lookup_prefix` key shape**

`lookup_prefix` returns all rowids matching a prefix. For partial `gu`, the key is `tl:gu` and matches all dictionary entries starting with `tl:gu` (gua, gun, guan, ...). But what about `gu1` (with tone digit)? Per the existing `build_keys_tl` digit-strip rule, `gu1` → `tl:gu` prefix. Same hits.

Edge case: user types `1` alone (degenerate). `build_keys_tl` strips → empty key → return empty (avoid scanning entire FST). Pin in test.

**Q15.4 — `consumed_span_end` for partial-prefix candidates on commit**

When user taps a partial-prefix candidate, commit semantics: consume the full raw buffer or just the prefix? Lexicon path's current behavior = consume full input as one block (no continuous segmentation). Partial-prefix in continuous: same? Or per-syllable consumption based on the candidate's `syllable_count`?

Recommendation: **consume full raw** (`consumed_span = (0, raw.len())`). This matches the user's mental model — "I started typing and tapped one of these suggestions before I finished a syllable" → commit what I typed + the candidate's display, no leftover pending.

But this conflicts with the multi-syllable phrase commit pattern (`taiuantaigi → tap 「臺」 → leftover uantaigi`). Codex consult: should partial-prefix never appear in `coverage_kind = 1` mode AND simultaneously enable mid-commit? Or always force final-commit for partial?

Initial proposal: **partial-prefix candidates always final-commit** (consume full buffer, exit to Idle). Multi-syllable mid-commit only applies to `coverage_kind = 0` full-syllable candidates. This keeps tap semantics predictable.

**Q15.5 — Engine prefix-fetch threshold**

Should engine ALWAYS try prefix-match when syllabifier returns empty, or only when `raw.len() >= MIN_PREFIX_LEN`?

- ALWAYS: even `t` returns prefix hits. Consistent, no surprise empty strips.
- Threshold (e.g., MIN = 2): single char returns empty (avoid noise from `t` matching thousands of entries).

Codex consult: pick threshold based on `prefix_index.lookup_prefix` performance characteristics + UX preference. Default proposal: **MIN_PREFIX_LEN = 1** (no threshold) — matches v3.5.7 lexicon behavior where single char also returned candidates.

**Q15.6 — Tab3 dictionary search untouched?**

Confirm `LexiconService.search` for Tab3 hanzi search stays exactly as is — no shared retire. Tab3 doesn't go through `Phase::Continuous` and has its own UX (search bar, browsing). Per existing `docs/architecture/codex-review-2026-04-19.md` Tab3 is fully separate.

**Q15.7 — Sub-PR breakdown**

Combined display fix + fallback retire is ~700-950 LOC. Per `feedback_round_hygiene.md` 200-500 LOC/PR discipline, suggest:

| Sub-PR | Scope | LOC est. |
|---|---|---|
| **Phase 9.4a** | TPS tone-1 syllabifier (original) | ~80 |
| **Phase 9.4b** | Hyphenated TL shadow buffer (original) | ~150 |
| **Phase 9.4c** (NEW) | POJ diacritic canonicalize in `build_keys_tl` | ~80 |
| **Phase 9.4d** (NEW) | Partial-prefix engine path + `coverage_kind` SortKey extension | ~250 |
| **Phase 9.4e** (NEW) | Hanzi guard port + Option A wire-level display dual-field carrier (combined — both touch `handle_fetch_at_pos` / `CandidateMessage`) | ~200 |
| **Phase 9.6** | Custom dict integration (original) | ~150 |
| **Phase 9.7** (NEW) | Platform `autocomplete()` simplification + lexicon-path retire + Finding 1 dashed border removal | ~80 deletion + UI |

Total: 7 sub-PRs. All v3.5.8. (Original 9.5 handled by user offline, not in PR plan.)

Codex confirms ordering / dependencies. Suggest dependency:
- 9.4a, 9.4b, 9.4c, 9.4d independent of each other (each engine-internal)
- 9.4e depends on 9.4a-d for `coverage_kind` enum to land first
- 9.7 depends on 9.4a-e all merged (platform retire safe only after engine completeness)
- 9.5 / 9.6 independent of fallback work

### 15.8 Risk register

> **Status (Item 13 shipped)**: partial-prefix flood + `coverage_kind` ranking risks closed by Items 10/11 engine tests. The **dogfood regression risk is the live capstone-acceptance gate** — Codex pre-impl F8 found no unimplemented input form (Items 7–12 cover TPS tone-1 / hyphen / POJ diacritic / partial-prefix / hanzi guard / custom dict); confirm on-device with the row-3 input matrix.

| Risk | Mitigation |
|---|---|
| Partial-prefix hits flood candidate strip (e.g., `t` matches 500+ entries) | `prefix_index.lookup_prefix` already has internal capacity; cap at `MAX_CANDIDATES = 30` in engine (mirror lexicon path) |
| Ranking surprises after `coverage_kind` insertion (full-syllable should still dominate when both present) | `partial_prefix_engine_path` test + `sort_key_partial_below_full` test pin invariant |
| Dogfood regression on input that worked via fallback but breaks under engine | Test matrix §15.6 + acceptance: type every input form (`t`, `gu`, `gua`, `gua2`, `guá`, `ㄍㄨㄚˋ`, `tai-bak`, `pe̍h`) and verify candidate strip non-empty after engine retire |
| Tab3 broken by accident during retire | Tab3 is a separate consumer on both platforms (iOS `DictionarySearchService`; Android `LexiconService.searchWithSources`/`searchByHanzi`, kept). Only the autocomplete `search()` surface was deleted; pinned by `platform_autocomplete_no_lexicon_branch`. |
| Codex consult overhead (8 sub-PRs × pre+post sandwich) | per `feedback_codex_review_sandwich.md` — each sub-PR small enough that sandwich is ~30min round-trip; total ~8-10 hours Codex consult time across all 8 PRs, spread over v3.5.8 dogfood window |

### 15.9 Out-of-scope confirmation (under §15)

Still out of scope even under expanded §15:
- MOE-style keyboard-level mode toggle (HanjiMode / TailoMode) — we keep `isTranslateSwapped` UX (§10 + §11)
- Hanzi-input mode as a primary input flow — Q1 clarified there is no such mode in our keyboard
- Tab3 dictionary search refactor — orthogonal feature
- Custom dict early-bind into engine (still Phase 9.6 as platform-side query path)
- v3.5.9+ planning — `feedback_v358_full_scope.md` rule reinforced ("修復到滿意為止")

---
