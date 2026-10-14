# Rust Core Protobuf Contract

> **Type**: Reference
> **Keywords**: `protobuf`, `Request`, `Response`, `request-id`, `generation`, `AppConfig`, `Phonetics`, `Composing`
> **Related**: [`../contributing/rust-ffi-safety.md`](../contributing/rust-ffi-safety.md), [`../architecture/behavioral-invariants.md`](../architecture/behavioral-invariants.md), [`../architecture/composing-state-boundary.md`](../architecture/composing-state-boundary.md), [`../architecture/nextword-engine-boundary.md`](../architecture/nextword-engine-boundary.md)
> **Audience**: anyone extending the engine proto contract.
> **Authoritative source**: the `.proto` files in [`engine/protos/proto/`](../../engine/protos/proto/). They are canonical; this document gives the shape and the rules, not the field list.

---

## 1. Scope

The request envelope, the per-request settings and generation rules, and the Phonetics, Composing and Case slices. Lexicon, NextWord and user-data ops are listed in §4 and documented with their own crates (`../architecture/nextword-engine-boundary.md`, `../architecture/user-data-engine-roadmap.md`).

---

## 2. Why protobuf

- One bytes-in / bytes-out entry point per platform, as khiin-rs does on iOS, macOS, Android and Windows (`references/khiin-rs/README.md`).
- Bindings: `prost` (Rust), `SwiftProtobuf` (iOS / macOS), `protobuf-javalite` (Android, `android/app/build.gradle.kts`).
- The schema evolves by adding tags and reserving removed ones; no handle table to break.
- UniFFI was considered and not adopted.

---

## 3. Single FFI entry point

The engine adapters expose one handle-less entry: `process_request_bytes(bytes: &[u8]) -> Vec<u8>` (`engine/swift-ffi/src/lib.rs`, swift-bridge) and `RustEngineBridge.processRequestBytes(ByteArray): ByteArray` (`engine/android-jni/src/lib.rs`, JNI). Both decode a `taigi.engine.Request`, call `dispatch::process_request`, and always return an encoded `Response`. Engine state lives in process singletons (`EngineHandle::instance()`); no handle crosses the boundary.

Panic catching, the request-size cap, locking, the error channel and the other entry points (logger, macOS desktop seam): [`rust-ffi-safety.md` §1](../contributing/rust-ffi-safety.md#1-ffi-boundary-discipline-s).

---

## 4. Message envelope

[`envelope.proto`](../../engine/protos/proto/envelope.proto) `Request` / `Response`:

| Field | Tag | Notes |
|---|---|---|
| `Request.id` / `Response.id` | 1 | Client correlation; the response echoes it |
| `Request.config_snapshot` | 3 | `AppConfig` for this call only — §6 |
| `Request.generation` | 4 | Platform-owned — §5; never echoed (`Response` tag 3 reserved) |
| `Response.error` | 2 | `ErrorCode` (`OK`, `FAIL_PARSE`, `FAIL_INTERNAL`, `FAIL_IO`, `FAIL_INVARIANT`) |
| `Request` tag 2 | — | Reserved (`type`); the `payload` oneof is the only routing key |

`oneof payload` — one request / response message per slice, same tag on both sides:

| Tag | Slice | `.proto` | Request message | Ops |
|---|---|---|---|---|
| 10 | Phonetics | `phonetics.proto` | `PhoneticsRequest` | 7 |
| 11 | Composing | `composing.proto` | `ComposingRequest` | 15 |
| 12 | Lexicon | `lexicon.proto` | `LexiconRequest` | 5 |
| 13 | NextWord | `nextword.proto` | `NextWordRequest` | 9 |
| 14 | Case | `case.proto` | `CaseRequest` (field `case_transform` — `case` is a Swift keyword) | 4 |
| 15 | User data | `user_data.proto` | `UserDataRequest` | 15 |

Each slice dispatches on its own `oneof method` and answers through `oneof result` (Composing: a `ComposingResponse`). Removed ops keep their tags and names `reserved`.

---

## 5. Generations — two counters, two owners

- **Envelope generation** (`Request.generation`) — platform-owned, one value per input context; the platform bumps it when the editor session changes (`ComposingManager.bumpGeneration`; next word: iOS `NextWordController.bumpEnvelopeGeneration`, Android `NextWordController.resetContext`). Rust never echoes it (the e2e trace logs the request's value, `docs/architecture/e2e-trace-schema.md`). A mutating composing / next-word request whose generation differs from the last one silently resets that engine's state before applying (no effects from the reset, `engine/composing/src/handle.rs`); a read-only query (`FetchAtPos`) at a stale generation answers the idle snapshot. Phonetics is stateless and ignores it.
- **NextWord query generation** (`current_generation`) — engine-owned (`engine/nextword`); every invalidating intent bumps it, it travels on `QueryPredictions` / `ClearPredictionsUI`, and `FilterPredictions` answers `was_stale` when the query's generation no longer matches (`docs/architecture/nextword-engine-boundary.md` §3).

---

## 6. Settings push — per-request only, never cached

- Every stateful `Request` carries `AppConfig config_snapshot`; Rust reads it for that call only and never stores settings across calls (behavioral-invariants §11 live-read).
- There is no `SetConfig` command. If a warm-up command is ever added, it must not replace the per-request snapshot.

---

## 7. Phonetics slice

[`phonetics.proto`](../../engine/protos/proto/phonetics.proto) — 7 ops:

| Family | Ops | Result |
|---|---|---|
| Core | `TlToPoj`, `GetToneVariations` | `StringResult`, `ToneVariationsResult` |
| TPS | `TlDisplayToTps`, `IsTpsToneMark`, `TpsInputAdjust` | `StringResult`, `BoolResult`, `TpsAdjustResult` |
| Platform text helpers | `IsAttachingPunctuation`, `ExternalLookupDigitForm` | `BoolResult`, `StringResult` |

- Each op has its own payload message (e.g. `TpsInputAdjust` takes `incoming` + `raw_input`).
- Pure and stateless: `phonetics::requests::handle(req)` takes no `AppConfig`. Display tone normalization is not a wire op — `composing::derived` calls `phonetics::api::normalize_tone` in-process.
- The Rust phonetics crate is the only implementation; platforms keep no mirror.

---

## 8. Composing slice

[`composing.proto`](../../engine/protos/proto/composing.proto) — `ComposingRequest.oneof method`, 15 ops:

| Tags | Group | Ops |
|---|---|---|
| 10–19 | Text-input mutators | `Start`, `Append`, `AppendHyphen`, `ReplaceLast`, `DeleteBackward`, `CommitRaw`, `CommitPreeditThenInsertExternal`, `Reset` |
| 31–32, 34 | Continuous input | `FetchAtPos` (read-only), `CommitContinuous`, `CommitPrediction` (a NextWord prediction tap) |
| 40–44 | Desktop keys | `TelexKey`, `MoveCaret`, `TpsKey`, `CommitAsShown`, `CommitAsTyped` |

`ComposingResponse`: `preedit` (1: `raw_input`, `display_text`, `caret_utf16`), ordered `effect` list (2), `is_composing` (4), `continuous` (5, `FetchAtPos` only), `commit` (6, `CommitResolution` for a `CommitContinuous` or `CommitPrediction`: outcome, document text, `wrote_romanization`, `earns_auto_space`).

`Effect.oneof kind`:

| Tag | Effect | Routed to |
|---|---|---|
| 1 | `UpdatePreedit` (`display`, `caret_utf16`) | Document (marked / composing region) |
| 2 | `ClearPreeditWithoutCommit` | Document |
| 3 | `CommitTextReplacingPreedit` | Document |
| 5 | `ClearCandidates` | Candidate strip |
| 6 | `RefreshCandidates` | Candidate strip |
| 7 | `ResetCandidateContext` | Candidate strip |
| 8 | `NextWordUpdateLastSelectedWord` | NextWord (the platform shim sends a `NextWordRequest`) |
| 9 | `NextWordWordSelected` | NextWord |
| 10 | `NextWordClearForNewComposing` | NextWord |

Tags 4 and 11 are reserved. Host API mapping per platform: [continuous-commit-and-display.md §10.6](continuous-commit-and-display.md#106-effects--host-api).

- Effects are platform-neutral — no `InputConnection`, `UITextDocumentProxy`, KeyboardKit or Compose types.
- The candidate-control effects must stay on the wire: they are how composing tells the platform strip to clear, refetch or reset its context.
- Why some intents stay distinct: `Start` vs `Append` (caret reset), `AppendHyphen` (no `"-"` string synthesized at call sites), `ReplaceLast` (TPS auto-correct), `CommitPreeditThenInsertExternal` (emoji / paste in one atomic write — splitting it reintroduces the silent finish-composing race).

---

## 8.5. Case-transform slice

[`case.proto`](../../engine/protos/proto/case.proto) — `UppercaseToneChar`, `FullUppercaseToneString`, `LowercaseToneChar`, `TransformInputCase`; one result, `CaseStringResult`.

- Mode comes from `AppConfig.input_mode`; messages do not repeat it.
- `CaseStringResult` is local to `case.proto` (no coupling to `phonetics.proto`).
- `LetterCase` has `LETTER_CASE_UNSPECIFIED = 0`; the engine treats it as lowercased.
- Candidates are not cased over this slice — the engine cases each continuous candidate from the user's raw input (`composing::continuous::recase_roman`); these ops serve keystrokes, key labels and Caps Lock.

---

## 9. Non-goals

- **No platform UI semantics** in any message: no layout or styling types, no platform text-region types (`NSRange`, `ExtractedText`). Candidate navigation and highlight stay platform-side (khiin-rs `protos/src/command.proto`: "App should decide how to show and navigate candidates").
- **No candidate ids.** A pick is `CommitContinuous` naming `consumed_bytes` and a `script`; the engine resolves the text.
- **No raw SQL or file paths** beyond `OpenUserData`: user-data SQLite is engine-owned (`engine/userdata`, `docs/contributing/rust-migration-policy.md` §6) behind typed `UserDataRequest` ops.
- **No UniFFI.**

---

## 10. Open questions

- Bytes vs string vs repeated for candidate lists (perf measurement needed).
- Streaming responses for incremental candidate updates (vs full snapshot).

---

## 11. References

- `docs/contributing/rust-best-practices.md` — §3 crate choices, §8 non-goals
- `docs/contributing/rust-ffi-safety.md` — §1 seam as built, §6 test contract
- `references/khiin-rs/khiin/src/engine.rs` — `send_command_bytes` shape
- `references/khiin-rs/README.md` — protobuf rationale + request-id correlation
- `docs/architecture/nextword-engine-boundary.md` §3 — query generation
- `docs/architecture/composing-state-boundary.md` §11.10 — Android host-cleared composing region
