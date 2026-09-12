# Rust Core Protobuf Contract — First-Slice Reference

> **Type**: Reference (Phonetics slice = AS-IMPLEMENTED post PR #186/#187; Composing slice = AS-IMPLEMENTED in v3.5.4)
> **Keywords**: `protobuf`, `Command`, `Request`, `Response`, `request-id`, `generation`, `AppConfig`, `Phonetics`, `Composing`
> **Related**: `ffi-safety.md`, `../architecture/behavioral-invariants.md`, `../architecture/composing-state-boundary.md`, `../architecture/nextword-engine-boundary.md`
> **Audience**: anyone extending the engine proto contract.
> **Authoritative source**: `engine/protos/proto/envelope.proto` + `engine/protos/proto/phonetics.proto` (the .proto files are canonical when they diverge from this doc).

---

## 1. Scope of THIS document

- **Phonetics slice (D9.4 — MERGED)** + **Composing slice (D9.3 — MERGED in v3.5.4).**
- Lexicon, NextWord (including prediction queries / results), SQLite, custom-dictionary, candidate-scoring are outside this document — see their own crates and `../architecture/nextword-engine-boundary.md`.
- §7 reflects the merged Phonetics wire (PR #186 D9.4-Phonetics + PR #187 D9.4-cleanup). §8 reflects the merged Composing wire (v3.5.4); naming was changed from `oneof intent` to `oneof method` per the Phonetics convention adopted in PR #186.
- **Authoritative companion**: `.claude/rules/rust-best-practices.md` §3 (crate choices — `prost` for protobuf), §8 (non-goals); `.claude/rules/rust-ffi-safety.md` §4 (opaque handle pattern).

---

## 2. Why protobuf (default, not final)

- khiin-rs validates protobuf on iOS + macOS + Android + Windows TSF (single entry point shape — see `references/khiin-rs/README.md:140-152`).
- Cross-platform binding via `prost` (Rust) + `SwiftProtobuf` (iOS) + `protobuf-kotlin` (Android).
- ABI-stable: schema evolves without breaking the existing handle table.
- **UniFFI was considered and not adopted**; protobuf is the contract.

---

## 3. Single FFI entry point — logical shape

The Rust engine's logical shape, mirroring `references/khiin-rs/khiin/src/engine.rs:57`:

```rust
fn send_command_bytes(handle: EngineHandle, bytes: &[u8]) -> Vec<u8>
```

Concrete extern signatures differ per platform (per `.claude/rules/rust-ffi-safety.md` §4):

- **JNI** (`android-jni`): `JByteArray` in / `JByteArray` out, plus `EngineHandle` as `jlong` wrapped in a `@JvmInline value class` on the Kotlin side.
- **swift-bridge** (`swift-ffi`): `&[u8]` in / `Vec<u8>` out, with an `EngineBridge` struct holding the handle.
- Both wrap the body in `catch_unwind` per `ffi-safety.md` §2 and lock the `Mutex<Engine>` per `ffi-safety.md` §3.

---

## 4. Message envelope

The merged shape (`engine/protos/proto/envelope.proto`):

```protobuf
syntax = "proto3";

message Command {
  Request request = 1;
  Response response = 2;
}

message Request {
  uint32 id = 1;                       // client correlation; echoed in Response.id
  CommandType type = 2;
  AppConfig config_snapshot = 3;       // per-request live snapshot — see §6
  uint64 generation = 4;               // platform-supplied — see §5
  oneof payload {
    PhoneticsRequest phonetics = 10;
    ComposingRequest composing = 11;
    LexiconRequest lexicon = 12;
    NextWordRequest nextword = 13;
    CaseRequest case_transform = 14;   // case-transform slice — own .proto file
  }
}

message Response {
  uint32 id = 1;                       // echoes Request.id
  ErrorCode error = 2;
  uint64 generation = 3;               // echoes Request.generation
  oneof payload {
    PhoneticsResponse phonetics = 10;
    ComposingResponse composing = 11;
    LexiconResponse lexicon = 12;
    NextWordResponse nextword = 13;
    CaseResponse case_transform = 14;
  }
}

enum CommandType {
  CMD_UNSPECIFIED = 0;
  CMD_PHONETICS = 1;
  CMD_COMPOSING = 2;
  CMD_LEXICON = 3;
  CMD_NEXTWORD = 4;
  CMD_CASE = 5;
}
```

> **Field name `case_transform` (not `case`)**: `case` is a Swift keyword;
> SwiftProtobuf would backtick-escape it (`\`case\``) which is awkward at the
> bridge sites. Rust prost generates PascalCase variant `Payload::CaseTransform`
> either way. Cross-language safe.

- **`id`** prevents ordering races when the platform fires intent N+1 before N's response arrives. khiin-rs uses the same pattern (`references/khiin-rs/README.md:149-152` — "Clients should tag each Request with an id").
- **`generation`** — see §5.
- **`oneof payload`** — each slice gets its own message; new slices extend without breaking change.

---

## 5. Generation counter — platform-owned, FFI carries it through

Authoritative ownership of `currentGeneration` lives in the **platform engine executor**, not in Rust. The contract is specified in `docs/architecture/nextword-engine-boundary.md` §2 (executor owns the counter, supplies it per intent) and §3 (only state transitions that invalidate pending prediction queries bump it).

For the first-slice proto, `generation` is purely an **FFI correlation / stale-response field**:

- Platform supplies the current generation in `Request.generation`.
- Rust echoes it back in `Response.generation` without mutation.
- **Phonetics slice** does not consult or mutate `generation` — it is stateless.
- **Composing slice** carries it through transitions but does not bump it. Bumping is a NextWord concern (platform side).

This lifts the existing platform-side mechanism (iOS G5-impl + Android A5-impl, see `docs/architecture/nextword-engine-boundary.md` §2.4, §3) onto the wire so platform and Rust agree on the protocol shape. Whether NextWord generation ownership migrates into Rust is a separate decision.

---

## 6. Settings push — per-request only, never cached

- Every stateful `Request` carries `AppConfig config_snapshot` (current settings as of that call).
- Rust reads `request.config_snapshot` for **that call only**. Engine state never stores a settings struct across calls.
- Enforces `docs/architecture/behavioral-invariants.md:295-299` (§11 settings live-read invariant) at the FFI level.
- Mirrors `EngineSettings` live-read on iOS (`docs/architecture/ios-exemplar.md` §3) and Android (`docs/architecture/ios-exemplar.md` §9.2).

`CMD_SET_CONFIG` (the khiin-rs equivalent at `references/khiin-rs/khiin/src/engine.rs:296`) is OPTIONAL platform warmup / compat command. It MUST NOT replace the per-request snapshot; the per-request snapshot is the authoritative source for every operation.

---

## 7. Phonetics slice — AS-IMPLEMENTED (PR #186 + PR #187)

The merged D9.4 shape uses an `oneof method` dispatch with 15 ops grouped into 3 families. Canonical source: `engine/protos/proto/phonetics.proto`. Sketch:

```protobuf
message PhoneticsRequest {
  oneof method {
    // Phonetics core (8 ops): NormalizeTone, StripTone, PojToTl, TlToPoj,
    // NormalizeToTl, NormalizeInput, RestoreTone, GetToneVariations.
    NormalizeTone normalize_tone = 10;
    // ... (see phonetics.proto for full list)

    // Derivation (2 ops): DeriveNotone, DeriveAbbrev.
    DeriveNotone derive_notone = 20;
    DeriveAbbrev derive_abbrev = 21;

    // TPS (5 ops): ContainsTps, TlNumericToTps, TlDisplayToTps,
    // IsTpsToneMark, TpsInputAdjust.
    TpsInputAdjust tps_input_adjust = 35;
    // ...
  }
}

message PhoneticsResponse {
  oneof result {
    StringResult string_result = 10;
    StripToneResult strip_tone_result = 11;
    OptionalStringResult optional_string_result = 12;
    BoolResult bool_result = 13;
    ToneVariationsResult tone_variations_result = 14;
    TpsAdjustResult tps_adjust_result = 15;
  }
}
```

- **Per-op payload type** rather than a flat `string input` — lets each op carry its natural shape (e.g. `TpsInputAdjust` takes `incoming` + `raw_input`; `TlNumericToTps` takes `text` + `or_maps_to_er`).
- **`oneof result`** with 6 result shapes covers all 15 ops: most ops return `StringResult`; `StripTone` returns the `(bare, tone)` pair; nullable-string ops use `OptionalStringResult`; `IsTpsToneMark` / `ContainsTps` use `BoolResult`; `GetToneVariations` uses `ToneVariationsResult` (callout init-bulk-pull); `TpsInputAdjust` uses `TpsAdjustResult` carrying the adjusted char + optional `replace_last` instruction.
- Pure, stateless. Settings consulted via `AppConfig.input_mode` / `oo_doubletap_enabled` / `nn_doubletap_enabled` from §6 (no engine-side caching).
- Replaces both platforms' `PhoneticsConverter.swift` / `TaigiPhonetics.kt` + `InputNormalizer` + `ToneRestoration` + `TPSConverter` + `TPSAdjustmentBundle` entry points.
- **Two ops were removed mid-flight** (`AdjustNasalMarkerCase`, `NfdPreprocess`): originally callers reverted to platform-side helpers (`ToneUtilities.adjustNasalMarkerCase` / `TaigiUnicode.nfdPreprocessed`) for Android JVM unit-test compatibility. **(Obsolete after v3.5.3 follow-up — see `feedback_path_g_delete_mirrors.md`.)** Path G deleted the platform mirrors + their JVM unit tests; `Method::NormalizeTone` applies `adjust_nasal_marker_case` in-band as part of the normalize pipeline; `Method::NfdPreprocessForLookup` exposes the Rust helper directly. The Rust phonetics crate is now the sole owner of both algorithms.
- Thread-safe by construction (no mutable state). The unified `Mutex<Engine>` wrap from `ffi-safety.md` §3 keeps the FFI contract uniform across slices.

---

## 8. Composing slice — AS-IMPLEMENTED (v3.5.4)

> **Status**: AS-IMPLEMENTED post v3.5.4. The proto landed in `engine/protos/proto/composing.proto`; envelope tag 11 + `CMD_COMPOSING = 2` were unreserved. Field naming uses `oneof method` (Phonetics convention). Two methods were added beyond the original §8 design draft: `SetSelectedCandidateIndex` (UI-driven candidate-bar tap mutator) and `QueryState` (pure read replacing the platform `manager.isComposingText` getter). Plus `is_composing` boolean on `ComposingResponse` so the platform stops shadowing engine state.

```protobuf
message ComposingRequest {
  oneof intent {
    Start start = 1;                                           // begin new buffer; caret reset
    Append append = 2;                                         // if idle, acts as Start
    AppendHyphen append_hyphen = 3;                            // alias for Append("-")
    ReplaceLast replace_last = 4;                              // TPS auto-correct; preserves selectedCandidateIndex
    DeleteBackward delete_backward = 5;
    CommitDerived commit_derived = 6;                          // commit tone-marked form
    CommitRaw commit_raw = 7;                                  // commit literal raw input (e.g. English passthrough)
    SelectSuggestion select_suggestion = 8;                    // commit platform-resolved suggestion text
    CommitPreeditThenInsertExternal commit_preedit_then_insert_external = 9;  // atomic emoji/paste insertion
    Reset reset = 10;                                          // teardown / mode switch
  }
}

message Start { string text = 1; }
message Append { string char = 1; }
message AppendHyphen {}
message ReplaceLast { string replacement = 1; }
message DeleteBackward {}
message CommitDerived {}
message CommitRaw {}
message SelectSuggestion { string text = 1; }
message CommitPreeditThenInsertExternal { string text = 1; }
message Reset {}

message ComposingResponse {
  message Preedit {
    string raw_input = 1;              // numeric tone, ASCII (search key)
    string display_text = 2;           // diacritics (UI)
  }
  Preedit preedit = 1;
  repeated Effect effect = 2;          // ordered side effects for platform to interpret
  int32 selected_candidate_index = 3;  // -1 idle; 0 fresh composition; preserved on ReplaceLast
}

message Effect {
  oneof kind {
    CommitTextReplacingPreedit commit_text_replacing_preedit = 1;
    UpdatePreedit update_preedit = 2;
    ClearPreeditWithoutCommit clear_preedit_without_commit = 3;
    DeleteBackwardFromDocument delete_backward_from_document = 4;
    ResetAutocomplete reset_autocomplete = 5;                // clear suggestion list
    PerformAutocomplete perform_autocomplete = 6;            // run fresh query against current buffer
    ResetAutocompleteContext reset_autocomplete_context = 7; // reset selection / bigram history
  }
}

message CommitTextReplacingPreedit { string text = 1; }   // atomic replace
message UpdatePreedit { string display = 1; }
message ClearPreeditWithoutCommit {}
message DeleteBackwardFromDocument {}
message ResetAutocomplete {}
message PerformAutocomplete {}
message ResetAutocompleteContext {}
```

- **Intent set mirrors the platform `ComposingState.Intent`** sealed type exactly:
  - **iOS**: `ComposingState.swift:18-60` (`Intent` enum, 10 cases).
  - **Android**: `ime/text/composing/ComposingState.kt:47-106` (`Intent` sealed class, 10 cases).
- Why each intent is on the wire (not collapsed into fewer):
  - `Start` vs `Append` — `Append` becomes `Start` when idle but the explicit `Start` is what platform code emits at composition begin (caret reset semantics differ — see iOS `case start` at `ComposingState.swift:25` and Android `data class Start` at `ComposingState.kt:49-51`).
  - `AppendHyphen` — semantic alias kept distinct so platform call-sites don't synthesize `"-"` strings on the wire.
  - `ReplaceLast` — TPS auto-correct (`ActionHandler+KeyActions.swift:37-39` iOS, `TextInputManager.kt:850,855` Android). Intentionally preserves `selectedCandidateIndex`; emulating it via `DeleteBackward + Append` would reset the index and regress candidate-bar behavior (documented at `ComposingState.kt:62-65`).
  - `CommitDerived` vs `CommitRaw` — distinct semantics: derived = tone-marked form (e.g. "guá"), raw = literal numeric form (e.g. "gua2") used for English passthrough on Enter-at-index-0 (`ComposingState.swift:41-45`, `ComposingState.kt:73-81`). A single `commit_composition` would lose this distinction.
  - `CommitPreeditThenInsertExternal` — emoji palette / clipboard paste atomic write (`MediaInputManager.kt:155` Android; iOS emoji delegate). Splitting into commit + insert reintroduces the silent-finish-composing race this intent was added to prevent.
- Mirrors the `ComposingTransition` / `Effect` shape already in iOS+Android Phase II:
  - **iOS**: `ComposingTransition.swift:18-44` — full `Effect` enum.
  - **Android**: `ime/text/composing/ComposingTransition.kt:32-66` — parallel sealed class shape.
- `Effect` is **neutral** — no `InputConnection` / `UITextDocumentProxy` / `KeyboardKit` / `Compose` references.
- The platform interpreter maps document-mutation effects (`CommitTextReplacingPreedit` / `UpdatePreedit` / `ClearPreeditWithoutCommit` / `DeleteBackwardFromDocument`) to `setComposingText` / `commitText` / `deleteSurroundingText` / equivalent, and routes autocomplete-control effects (`ResetAutocomplete` / `PerformAutocomplete` / `ResetAutocompleteContext`) to the platform autocomplete subsystem.
- `DeleteBackwardFromDocument` is required to preserve the delete-to-empty path: when the user backspaces a 1-char raw buffer, the composing state emits `clearPreeditWithoutCommit` + `resetAutocomplete` + `deleteBackwardFromDocument` (`ComposingState.swift:144-149`, Android `ComposingState.kt:211-215`) so the host editor's last grapheme is removed atomically with the preedit clear.
- The 3 autocomplete-control effects MUST be on the wire. Composing emits them as part of normal transitions (typing, commit, reset — see `ComposingTransition.swift:33-43`, `ComposingTransition.kt:55-64`); without them on the wire, a Rust composing slice cannot tell the platform autocomplete subsystem when to clear suggestions, run a fresh query, or reset the bigram history. Candidate queries / context resets would drift even when text effects are correct. The autocomplete subsystem itself stays platform-side; only the cross-subsystem signals cross the FFI.
- `selected_candidate_index` on `ComposingResponse` mirrors `ComposingTransition.newSelectedIndex` (`ComposingTransition.swift:48`, `ComposingTransition.kt:28`). Semantics: `-1` in idle, `0` on fresh composition, **preserved on `ReplaceLast`** (`ComposingState.swift:126-132`). Platform commit paths (e.g. iOS `ComposingManager.confirmSelectedCandidate` → `availableTexts[selectedCandidateIndex]`) depend on this field — without it, append/delete/reset/replaceLast cannot synchronize the index and a stale index could commit the wrong suggestion.
- **Prediction-related effects** (`QueryPredictions`, candidate-list updates) are NOT in this slice — they belong to NextWord.

---

## 8.5. Case-transform slice — AS-IMPLEMENTED (case-transform-slice)

Canonical source: `engine/protos/proto/case.proto`. Own file (NOT a `lexicon.proto` extension) — case logic is conceptually phonetics-aware string transformation independent of lexicon search. Sketch:

```protobuf
message CaseRequest {
  oneof method {
    UppercaseToneChar       uppercase_tone_char        = 10;
    FullUppercaseToneString full_uppercase_tone_string = 11;
    LowercaseToneChar       lowercase_tone_char        = 12;
    TransformInputCase      transform_input_case       = 20;
    CapitalizeCandidate     capitalize_candidate       = 21;
    TransformSuggestion     transform_suggestion       = 22;
  }
}

enum LetterCase {
  LETTER_CASE_UNSPECIFIED = 0;
  LETTER_CASE_LOWERCASED  = 1;
  LETTER_CASE_UPPERCASED  = 2;
  LETTER_CASE_CAPS_LOCKED = 3;
}

message CaseResponse {
  oneof result {
    CaseStringResult string_result = 10;  // locally defined — no cross-module proto coupling
  }
}
```

- **Mode comes from envelope `AppConfig.input_mode`** (per phonetics convention) — messages don't re-specify mode per call.
- **`CaseStringResult` defined locally** rather than reusing `phonetics.proto::StringResult`. Avoids cross-module proto coupling so case-transform can evolve independently.
- **`LetterCase` enum** carries `LETTER_CASE_UNSPECIFIED = 0` per proto3 best practice. Engine maps Unspecified to `Lowercased` as safe-fallback (matches the safe-fallback contract used by other dispatch error paths).
- **Suggestion skip rules stay platform-side** — iOS uses `additionalInfo` flag-based markers, Android uses numeric `id` markers. Each platform's bridge filters before calling `transform_suggestion(...)`.

---

## 9. Non-goals codified

- **No platform UI semantics** in any message (per `.claude/rules/cross-platform-alignment.md:115-117`):
  - Candidate navigation ownership stays platform-side. `references/khiin-rs/protos/src/command.proto:114-117` validates this pattern: "App should decide how to show and navigate candidates".
  - No layout, styling, KeyboardKit, FlorisBoard types.
  - No platform text-region types (`NSRange`, `ExtractedText`, `TextPosition`).
- **No candidate ids in the Composing slice.** `SelectSuggestion` carries text the platform already resolved.
- **No Lexicon / NextWord proto** in this document. This includes prediction queries, prediction results, and candidate-list updates.
- **No SQLite I/O proto.** User-data DB stays platform-side permanently per `.claude/rules/rust-migration-policy.md` § User-data SQLite stays platform-native and the criteria in `.claude/rules/ios-shared-core-candidates.md` §1 (no DB / App Group / FileManager / file-system access in candidates). Excluded files appear with `status=wont_migrate` in `migration-inventory.csv` (`Lexicon/Database/*Repository.swift`, `SQLiteConnectionManager.swift`, `NextWord/Repository/*`, etc.).
- **No UniFFI signature.** Protobuf-first per the roadmap revision.

---

## 10. Open questions

- Bytes vs string vs repeated for candidate lists (perf measurement needed).
- Streaming responses for incremental candidate updates (vs full snapshot).
- Whether NextWord generation ownership migrates from platform to Rust.
- UI-driven candidate selection (e.g. candidate-bar tap, arrow-key navigation) currently mutates `selectedCandidateIndex` directly via the platform-side `setSelectedCandidateIndex` mutator (`ComposingState.swift:69-77`), bypassing `apply(Intent, ...)`. A wire intent (`SetSelectedCandidateIndex { index }`) lands when navigation-bar interaction is integrated into the Rust composing slice — decided when navigation-bar interaction moves into the Rust composing slice.

---

## 11. References

- `.claude/rules/rust-best-practices.md` — mandatory companion (§3 crate choices, §8 non-goals)
- `.claude/rules/rust-ffi-safety.md` — mandatory companion (§4 opaque handle pattern)
- `references/khiin-rs/protos/src/command.proto:114-117` — candidate display is the client app's job
- `references/khiin-rs/khiin/src/engine.rs:57` — `send_command_bytes` shape
- `references/khiin-rs/README.md:140-152` — protobuf rationale + request-id correlation
- `docs/architecture/behavioral-invariants.md:295-309` (§11 settings live-read)
- `docs/architecture/nextword-engine-boundary.md` §2, §2.4, §3 — generation counter ownership
- `docs/architecture/composing-state-boundary.md` §11.10 — Android `Effect` divergences
- `ios/Sources/TaigiKeyboard/Input/Composing/ComposingState.swift:47-49` — `selectSuggestion(String)` shape
