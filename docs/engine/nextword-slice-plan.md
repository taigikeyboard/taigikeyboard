# NextWord Slice — Plan (v3.5.5)

**Status**: pre-impl plan, authored 2026-05-01 alongside `nextword-slice-audit.md`. **Awaits Codex pre-impl review APPROVED before branch open** per `feedback_codex_review_sandwich.md`.

**Scope reference**: see `nextword-slice-audit.md` §9 for slice scope decisions and §1 for domain definition.

**Architecture target**: D mode for NextWord engine logic (platform 0 NextWord state machine / scoring math); SQLite + assoc.bin stay platform-side until v3.5.6.

---

## 1. Rust workspace delta

### 1.1 New crate `engine/nextword`

```
engine/nextword/
├── Cargo.toml
└── src/
    ├── lib.rs         # public API: pub mod api / dispatch / handle; pub use api::{Engine, NextWordError}; pub use handle::EngineHandle
    ├── api.rs         # pub struct Engine (state) + pub fn apply(intent, config) -> Outcome + pub fn reset() + pub fn snapshot() + Intent / Phase / Effect / Outcome / NextWordError
    ├── handle.rs      # pub struct EngineHandle (Mutex<Engine> + Mutex<u64> last_generation) + ::instance() + ::handle(&NextWordRequest, &AppConfig, generation) — same shape as engine/composing/src/handle.rs
    ├── dispatch.rs    # pub fn handle(&NextWordRequest, &mut Engine, &AppConfig) -> Result<NextWordResponse, NextWordError> — decode_intent + apply
    ├── decide.rs      # pure decide logic (6 intents incl. UpdateLastSelectedWord) — port of NextWordEngine.decide
    ├── scorer.rs      # pure scoring math + ranking constants
    ├── filter.rs      # filterPredictions (raw + settings + generation) -> EnginePrediction list (or stale)
    ├── booster.rs     # AutocompleteContextBooster.boost — pure first-char-set reorder
    └── types.rs       # internal Outcome / Intent / Effect / PersistedState — kept private, only proto types cross the dispatch boundary
```

**Crate dependencies** (workspace.dependencies): `prost`, `protos`, `phonetics` (path), `once_cell`, `log`, `thiserror`, **`indexmap = "2"` (NEW — required by `filter.rs` merge map for insertion-ordered iteration; matches Android `mutableMapOf` semantics; MIT/Apache dual-licensed)**. Add `nextword = { path = "./nextword" }` to workspace `Cargo.toml`.

**`#![forbid(unsafe_code)]`** at crate root (mirrors `composing` / `phonetics`).

**Send invariant** check at `lib.rs` mirroring `composing/src/lib.rs:25–28`:
```rust
const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<Engine>();
};
```

### 1.2 `engine/dispatch/src/lib.rs` route addition

Add a new arm to `match payload`:

```rust
request::Payload::Nextword(nw_req) => {
    match nextword::EngineHandle::instance().handle(&nw_req, &config, generation) {
        Ok(nw_resp) => Response { ..., payload: Some(response::Payload::Nextword(nw_resp)) },
        Err(err) => error_response(id, ErrorCode::FailInvariant, generation),
    }
}
```

Mirrors the existing composing arm exactly. New `nextword` workspace member dependency added to `engine/dispatch/Cargo.toml`.

### 1.3 Workspace `Cargo.toml` delta

Add `nextword` to `members` and `workspace.dependencies`.

---

## 2. Proto schema — `engine/protos/proto/nextword.proto`

```proto
syntax = "proto3";

package taigi.engine;

option java_package = "com.siansiansu.taigikeyboard.engine.proto";
option java_multiple_files = true;

// NextWord slice — Intent → Effect state machine.
//
// Mirrors iOS NextWord/NextWordOutcome.swift + Android
// ime/core/nextword/NextWordOutcome.kt 1:1 pre-migration. The Rust crate
// engine/nextword owns NextWordPersistedState (lastSelectedWord/Roman,
// lastSelectionTimeMs, isShowing, currentGeneration) inside a singleton
// Mutex<Engine> held by EngineHandle. Persistence (SQLite + association.bin
// mmap) stays on the platform side until v3.5.6 (Lexicon slice).
//
// Naming: oneof method matches Phonetics/Composing dispatch pattern.
//
// Lifecycle: every request carries Request.generation (envelope-level —
// IME-session ID; engine resets state on mismatch BEFORE applying the
// request, no effects from the drop). The engine ALSO maintains an
// internal current_generation that bumps on every state-mutating intent;
// it travels on QueryPredictions/ClearPredictionsUI effects so the
// platform executor can drop async SQLite results that land after
// state invalidation. See nextword-engine-boundary.md §3 + §13.6.

message NextWordRequest {
  oneof method {
    // State-mutating intents — return DecideResult.
    WordSelected         word_selected           = 10;
    Backspace            backspace               = 11;
    ContextTimeoutFired  context_timeout_fired   = 12;
    ClearForNewComposing clear_for_new_composing = 13;
    ResetFull            reset_full              = 14;
    // Android-only Space-path intent (per audit §5 #5). iOS wrappers
    // never emit this; Rust engine accepts generically. Mutates
    // last_selected_*/lastSelectionTimeMs without bumping
    // current_generation, no timer effects, emits compound-only
    // RecordCompoundAssociations effect (no prev→this bigram).
    UpdateLastSelectedWord update_last_selected_word = 15;

    // Pure post-query filter+merge+sort+limit — return FilterResult
    // (handles stale-gen drop).
    FilterPredictions    filter_predictions      = 20;

    // Stateless candidate reorder helper (NextWord-derived first-char set).
    // Pure function; engine state untouched. Codex v1 P2 placement: keep
    // on NextWordRequest rather than a new stateless dispatch arm.
    BoostCandidates      boost_candidates        = 21;

    // Pure read — return StateSnapshot.
    QueryState           query_state             = 30;
  }
}

// Per-intent fields the engine needs but cannot read itself.
message DecisionInput {
  int64 now_ms = 1;
}

// User selected a candidate or committed composing text.
// require_roman_mode: Enter-commits-raw-romanization paths.
// trigger_prediction: false on Space.
message WordSelected {
  string text                  = 1;
  string roman                 = 2;
  bool   require_roman_mode    = 3;
  bool   trigger_prediction    = 4;
  DecisionInput input          = 5;
}

// Backspace after a word selection — re-predict, never record.
message Backspace {
  string last_char     = 1;
  DecisionInput input  = 2;
}

// Executor's context-timeout fired.
message ContextTimeoutFired {
  DecisionInput input  = 1;
}

// User began composing a new syllable / digit — hide suggestions but
// keep association state intact (matches iOS clearDisplay).
message ClearForNewComposing {
  DecisionInput input  = 1;
}

// Full reset (sentence-end punctuation outside WordSelected, or empty doc).
message ResetFull {
  DecisionInput input  = 1;
}

// Android-only Space-path intent (audit §5 #5). Mutates state without
// timer effects or generation bump; emits compound-only effect.
message UpdateLastSelectedWord {
  string text          = 1;
  string roman         = 2;
  DecisionInput input  = 3;
}

// Engine-side filter+merge+sort+limit step. Platform calls this after
// SQLite returns raw rows — engine groups by (hanzi, tl), scores dict
// rows via DICT_WEIGHT and user rows via decay+learning math, sums
// scores on (hanzi, tl) collision, sorts desc by score, applies limit,
// then shapes via display-rule filter. On generation mismatch returns
// predictions=[] + was_stale=true.
message FilterPredictions {
  repeated RawNextWordPrediction raw                = 1;
  uint64                          query_generation  = 2;  // matches state.current_generation when fresh
  int64                           now_ms            = 3;  // single decision-time clock per intent
  int32                           limit             = 4;  // post-merge truncation; 30 default if 0
}

// Stateless candidate-reorder helper; engine state untouched.
message BoostCandidates {
  repeated string words                  = 1;
  repeated string predicted_first_chars  = 2;
}

message QueryState {}

// Pre-merge un-scored row tagged by source. Platform NextWordService.predict
// returns these directly post-v3.5.5; Rust filter does merge + score.
message RawNextWordPrediction {
  string hanzi        = 1;
  string tl           = 2;
  int64  count        = 3;
  int64  last_used_ms = 4;
  Source source       = 5;
}

enum Source {
  SOURCE_UNSPECIFIED = 0;  // Rust treats as FailInvariant
  SOURCE_DICT        = 1;
  SOURCE_USER        = 2;
}

message NextWordResponse {
  oneof result {
    DecideResult   decide          = 1;
    FilterResult   filter          = 2;
    BoostResult    boost           = 3;
    StateSnapshot  state_snapshot  = 4;
  }
}

message BoostResult {
  repeated string words = 1;  // boosted partition first, rest preserves original order
}

message DecideResult {
  repeated NextWordEffect effects             = 1;  // ordered, executor runs sequentially
  uint64                  current_generation  = 2;
  bool                    is_showing          = 3;
  // Echo of last_selected_word so platform can implement
  // SelectionContextProvider without holding a parallel cache.
  // Empty string == nil (Swift Optional / Kotlin nullable on the bridge side).
  string                  last_selected_word  = 4;
}

message FilterResult {
  repeated EnginePrediction predictions = 1;  // empty if was_stale
  bool                       was_stale  = 2;  // generation mismatch — late result
}

message StateSnapshot {
  string  last_selected_word  = 1;  // "" == nil
  bool    is_showing          = 2;
  uint64  current_generation  = 3;
}

// UI-ready prediction value — mirrors iOS EnginePrediction.swift +
// Android EnginePrediction.kt. Android consumes `score` for
// TaigiWord.lengthScore; iOS ignores it (iOS bridge presentation
// rule documented in nextword-engine-boundary.md §13.10).
message EnginePrediction {
  string text     = 1;
  // "" == nil (Optional on iOS, nullable on Android). Per Codex v1 P2:
  // hanzi-empty case never surfaces a non-nil empty subtitle (filter
  // emits subtitle only when roman != "" → subtitle = hanzi which by
  // shape contract is non-empty). No `has_subtitle` field needed.
  string subtitle = 2;
  string hanzi    = 3;
  string tl       = 4;
  // Merged score (sum of per-source scoreDict + calculateUserScore on
  // (hanzi, tl) collision). Android maps to TaigiWord.lengthScore.
  double score    = 5;
}

message NextWordEffect {
  oneof kind {
    RescheduleContextTimeout    reschedule_context_timeout    = 1;
    CancelContextTimeout        cancel_context_timeout        = 2;
    RecordAssociation           record_association            = 3;
    RecordCompoundAssociations  record_compound_associations  = 4;
    QueryPredictions            query_predictions             = 5;
    ClearPredictionsUI          clear_predictions_ui          = 6;
  }
}

// Single representation: ms. iOS bridge converts to TimeInterval seconds
// for Timer.scheduledTimer(withTimeInterval:); Android uses delay(Long ms).
message RescheduleContextTimeout { uint64 after_ms = 1; }
message CancelContextTimeout {}

message AssociationPair {
  string prev    = 1;
  string prev_tl = 2;
  string next    = 3;
  string next_tl = 4;
}

message RecordAssociation         { AssociationPair pair = 1; }
message RecordCompoundAssociations { repeated AssociationPair pairs = 1; }

message QueryPredictions {
  string word        = 1;
  string roman       = 2;
  uint64 generation  = 3;
  // now_ms reused by the platform's predict() call for user-row decay
  // scoring — same clock the engine used in shouldRecordAssociation.
  // Per nextword-engine-boundary.md §13.3.
  int64  now_ms      = 4;
}

message ClearPredictionsUI { uint64 generation = 1; }
```

### 2.1 Envelope delta — `engine/protos/proto/envelope.proto`

```proto
import "nextword.proto";

enum CommandType {
  CMD_UNSPECIFIED = 0;
  CMD_PHONETICS   = 1;
  CMD_COMPOSING   = 2;
  CMD_LEXICON     = 3;
  CMD_NEXTWORD    = 4;       // NEW
}

message AppConfig {
  string tone_mode                       = 1;
  string input_mode                      = 2;
  bool   oo_doubletap_enabled            = 3;
  bool   nn_doubletap_enabled            = 4;
  bool   is_translate_swapped            = 5;  // NEW — gates Hanji-mode short-circuit
  bool   is_association_recording_enabled = 6; // NEW — gates association recording
  Platform platform_id                   = 7;  // NEW — branches noise/split divergences §5 #1, #2
}

enum Platform {
  PLATFORM_UNSPECIFIED = 0;
  PLATFORM_IOS         = 1;
  PLATFORM_ANDROID     = 2;
}

message Request {
  uint32 id              = 1;
  CommandType type       = 2;
  AppConfig config_snapshot = 3;
  uint64 generation      = 4;
  oneof payload {
    PhoneticsRequest phonetics = 10;
    ComposingRequest composing = 11;
    LexiconRequest   lexicon   = 12;
    NextWordRequest  nextword  = 13;   // NEW
  }
}

message Response {
  uint32 id              = 1;
  ErrorCode error        = 2;
  uint64 generation      = 3;
  oneof payload {
    PhoneticsResponse phonetics = 10;
    ComposingResponse composing = 11;
    LexiconResponse   lexicon   = 12;
    NextWordResponse  nextword  = 13;  // NEW
  }
}
```

**JUSTIFICATION** (per Rust best-practices §3a domain↔proto rule): NextWord engine reads `is_translate_swapped` and `is_association_recording_enabled` per request. Composing slice already needs `is_translate_swapped` indirectly (autocomplete-control effects); pulling the field onto `AppConfig` is the right placement (settings-scoped, multi-slice consumer). `platform_id` is a new shape — NextWord is the first slice with documented platform divergence. Keeping it on `AppConfig` (not on the per-method NextWord payload) leaves room for future slices to read it.

**`PLATFORM_UNSPECIFIED` policy** (Codex v1 P2): NextWord dispatch receiving `Platform::Unspecified` returns `ErrorCode::FailInvariant`. Both bridges (`RustEngineBridge.nextword*` on iOS / Android) populate `AppConfig.platform_id` explicitly at construction; an unset value on the wire = invariant violation, not a silent fallback. Rust workspace tests pin this with an explicit `nextword_unspecified_platform_returns_fail_invariant` fixture.

---

## 3. Rust impl — engine/nextword/src/

### 3.1 `api.rs` — public types

```rust
use protos::engine::{NextWordEffect, NextWordResponse, AppConfig, RawNextWordPrediction};

pub struct Engine {
    state: PersistedState,
}

#[derive(thiserror::Error, Debug)]
pub enum NextWordError {
    #[error("missing method on NextWordRequest")]
    MissingMethod,
    #[error("missing decision input on intent")]
    MissingDecisionInput,
    #[error("invalid Source on RawNextWordPrediction")]
    InvalidSource,
    #[error("invalid Platform on AppConfig")]
    InvalidPlatform,
}

#[derive(Default, Clone, Debug)]
struct PersistedState {
    last_selected_word: Option<String>,
    last_selected_roman: Option<String>,
    last_selection_time_ms: i64,
    is_showing: bool,
    current_generation: u64,
}

pub enum Intent {
    WordSelected { text: String, roman: String, require_roman_mode: bool, trigger_prediction: bool, now_ms: i64 },
    Backspace { last_char: String, now_ms: i64 },
    ContextTimeoutFired { now_ms: i64 },
    ClearForNewComposing { now_ms: i64 },
    ResetFull { now_ms: i64 },
    UpdateLastSelectedWord { text: String, roman: String, now_ms: i64 },  // Android-only Space-path
}

impl Engine {
    pub fn new() -> Self { Self { state: PersistedState::default() } }
    pub fn reset(&mut self) { self.state = PersistedState::default(); }
    pub(crate) fn apply(&mut self, intent: Intent, config: &AppConfig) -> Result<protos::engine::DecideResult, NextWordError> { /* … */ }
    pub(crate) fn snapshot(&self) -> protos::engine::StateSnapshot { /* … */ }
    pub(crate) fn filter(
        &self,
        raw: Vec<RawNextWordPrediction>,
        query_generation: u64,
        now_ms: i64,
        limit: i32,
        config: &AppConfig,
    ) -> Result<protos::engine::FilterResult, NextWordError> { /* … see §3.4 */ }
}
```

### 3.2 `decide.rs` — port of `NextWordEngine.decide`

Direct port of the Swift / Kotlin decision tables in `nextword-engine-boundary.md` §4 and audit doc §4. Branch on `config.platform_id` for divergences §5 #1 (compound split separator) + §5 #2 (noise punctuation set).

`shouldRecordAssociation(state, now_ms) -> bool`:
```rust
fn should_record_association(state: &PersistedState, now_ms: i64) -> bool {
    if state.last_selected_word.is_none() { return false; }
    let delta = now_ms - state.last_selection_time_ms;
    (0..ASSOCIATION_TIMEOUT_MS).contains(&delta)
}
```

Constants live in `decide.rs` as `pub(crate) const`:
- `ASSOCIATION_TIMEOUT_MS: i64 = 10_000`
- `CONTEXT_TIMEOUT_MS: u64 = 30_000`
- `SENTENCE_END_PUNCT: &[char] = &['。', '！', '？', '.', '!', '?']`
- `IOS_NOISE_PUNCT: &[char] = &[ /* iOS subset */ ]`
- `ANDROID_NOISE_PUNCT: &[char] = &[ /* Android superset incl. spaces */ ]`

### 3.3 `scorer.rs` — port of `NextWordScorer`

```rust
pub const USER_WEIGHT: f64       = 50.0;
pub const DICT_WEIGHT: f64       = 1.0;
pub const DECAY_HALF_LIFE_HOURS: f64 = 168.0;
pub const LEARNING_BONUS: f64    = 300.0;
pub const HIGH_USAGE_DECAY_FLOOR: f64 = 0.95;
pub const LOW_USAGE_DECAY_FLOOR:  f64 = 0.30;
pub const HIGH_USAGE_THRESHOLD: i32 = 3;
pub const LN_2: f64              = 0.693;  // 3-digit literal — match Android NextWordScorer.kt LN_2

pub fn score_dict(count: i64) -> f64 { count as f64 * DICT_WEIGHT }

pub fn calculate_decay(last_used_ms: i64, now_ms: i64) -> f64 {
    let age_hours = (now_ms - last_used_ms) as f64 / 3_600_000.0;
    (-age_hours / DECAY_HALF_LIFE_HOURS * LN_2).exp()
}

pub fn calculate_user_score(count: i64, last_used_ms: i64, now_ms: i64) -> f64 {
    let decay = calculate_decay(last_used_ms, now_ms);
    let raw_score = count as f64 * USER_WEIGHT;
    let floor = if count >= HIGH_USAGE_THRESHOLD as i64 { HIGH_USAGE_DECAY_FLOOR } else { LOW_USAGE_DECAY_FLOOR };
    raw_score * decay.max(floor) + LEARNING_BONUS
}
```

### 3.4 `filter.rs` — full merge + score + sort + limit + filter (Codex v1 P1 fix)

**Full pipeline** mirrors today's iOS [`NextWordService.predict`](../../ios/Sources/TaigiKeyboard/NextWord/Services/NextWordService.swift) lines 105 + 293 + final dict↔user merge sort, and Android [`NextWordService.predict`](../../android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/NextWordService.kt) lines 215 + 304:

1. **Generation check** — if `query_generation != state.current_generation`, return `was_stale=true` immediately.
2. **Per-row scoring** — for each raw row, compute score:
   - `Source::Dict` → `scorer::score_dict(count)`
   - `Source::User` → `scorer::calculate_user_score(count, last_used_ms, now_ms)`
   - `Source::Unspecified` → fail invariant
3. **Merge by `(hanzi, tl)`** — `IndexMap<(String, String), MergedRow>` (insertion-ordered; see note below). On collision: `score += new_score`. **No TL-preservation rule** — under `(hanzi, tl)` keying both rows on the same key share the same TL by definition; the iOS / Android conditional `tl = row.tl.isEmpty ? existing.tl : row.tl` was dead code. Codex v2 P1: dropped from contract; corresponding INVARIANT_nextword_filter_preserves_nonempty_tl_on_merge fixture removed.
4. **Drop empty hanzi** — Codex v1 P2 invariant: filter requires `hanzi` non-empty.
5. **Shape via display rules** — for each merged row:
   - `use_tl = config.input_mode == "tl"`
   - `roman = use_tl ? tl : phonetics::api::tl_to_poj(&tl)` — internal Rust call, no FFI
   - if `!config.is_translate_swapped && roman.is_empty()` → drop
   - `text = if roman.is_empty() { hanzi.clone() } else { roman.clone() }`
   - `subtitle = if roman.is_empty() { String::new() } else { hanzi.clone() }`  // "" == nil per proto §2
6. **Sort desc by score** — stable sort; ties preserve insertion order.
7. **Apply limit** — truncate to `limit` (default 30 if request `limit == 0`).
8. **Return `FilterResult { predictions, was_stale: false }`**.

```rust
pub fn filter(
    state: &PersistedState,
    raw: Vec<RawNextWordPrediction>,
    query_generation: u64,
    now_ms: i64,
    limit: i32,
    config: &AppConfig,
) -> Result<protos::engine::FilterResult, NextWordError> {
    if query_generation != state.current_generation {
        return Ok(protos::engine::FilterResult { predictions: vec![], was_stale: true });
    }

    let effective_limit = if limit <= 0 { DEFAULT_LIMIT } else { limit as usize };

    // 1. score each row, fail-invariant on Source::Unspecified, merge by (hanzi, tl).
    let mut merged: indexmap::IndexMap<(String, String), MergedRow> = indexmap::IndexMap::new();
    for row in raw {
        if row.hanzi.is_empty() { continue; }
        let score = match Source::try_from(row.source).map_err(|_| NextWordError::InvalidSource)? {
            Source::Dict => scorer::score_dict(row.count),
            Source::User => scorer::calculate_user_score(row.count, row.last_used_ms, now_ms),
            Source::Unspecified => return Err(NextWordError::InvalidSource),
        };
        let key = (row.hanzi.clone(), row.tl.clone());
        merged.entry(key)
            .and_modify(|existing| { existing.score += score; })
            .or_insert(MergedRow { hanzi: row.hanzi, tl: row.tl, score });
    }

    // 2. shape + filter empty-roman in non-Hanji mode
    let mut shaped: Vec<EnginePrediction> = merged.into_iter()
        .filter_map(|(_, m)| shape_prediction(m, config))
        .collect();

    // 3. sort desc by score (stable sort — ties preserve IndexMap insertion order,
    //    which matches Android `mutableMapOf` / `LinkedHashMap` iteration. iOS pre-v3.5.5
    //    had unstable HashMap order; v3.5.5 unifies on insertion order — minor parity
    //    correction toward Android, no observable user-facing effect since results are
    //    sorted by score immediately after merge and tie-breaking only affects equal scores.)
    shaped.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // 4. limit
    shaped.truncate(effective_limit);

    Ok(protos::engine::FilterResult { predictions: shaped, was_stale: false })
}

const DEFAULT_LIMIT: usize = 30;
struct MergedRow { hanzi: String, tl: String, score: f64 }

fn shape_prediction(m: MergedRow, config: &AppConfig) -> Option<EnginePrediction> {
    let use_tl = config.input_mode == "tl";
    let roman = if use_tl { m.tl.clone() } else { phonetics::api::tl_to_poj(&m.tl) };
    if !config.is_translate_swapped && roman.is_empty() { return None; }
    let text = if roman.is_empty() { m.hanzi.clone() } else { roman.clone() };
    let subtitle = if roman.is_empty() { String::new() } else { m.hanzi.clone() };
    Some(EnginePrediction { text, subtitle, hanzi: m.hanzi, tl: m.tl, score: m.score })
}
```

**Codex v1 fixes incorporated**:
- `FilterPredictions.now_ms` + `limit` added to proto (§2 above).
- Pseudocode now uses `scorer::*` instead of `p.score` field.
- Merge / sort / limit contract documented and pinned. (TL-preservation rule was vestigial / dead code in iOS+Android source — dropped per Codex v2 P1.)
- `Source::Unspecified` returns `FailInvariant`.

**Platform `NextWordService.predict` post-v3.5.5**: returns `[(hanzi, tl, count, last_used_ms, source)]` un-merged un-scored. Both platforms drop their internal scoring/merging/sorting/limiting before crossing the bridge.

**Decision: Option A locked.** Codex v1 confirmed: "Option B undercuts the stated D-mode goal by leaving scoring math on platform."

`indexmap` is added to `engine/nextword/Cargo.toml` for stable iteration order on the merge map. Workspace `Cargo.toml` adds `indexmap = "2"` to `workspace.dependencies` (per `rules/rust-best-practices.md` §5 — version-pinned, MIT/Apache dual-licensed).

### 3.5 `booster.rs` — port of `AutocompleteContextBooster.boost`

```rust
pub fn boost_words(words: Vec<String>, predicted_first_chars: Vec<String>) -> Vec<String> {
    if predicted_first_chars.is_empty() { return words; }
    let predicted: HashSet<String> = predicted_first_chars.into_iter().collect();
    let (boosted, rest): (Vec<_>, Vec<_>) = words.into_iter().partition(|w| {
        // first Unicode scalar
        w.chars().next()
            .map(|c| predicted.contains(&c.to_string()))
            .unwrap_or(false)
    });
    boosted.into_iter().chain(rest).collect()
}
```

**Codex v1 P2 placement**: `BoostCandidates` rides as a `NextWordRequest` method (proto §2 above), NOT a separate stateless dispatch arm. Rationale: a single helper for a future autocomplete crate is not enough surface to justify an entire dispatch arm; per `rules/rust-best-practices.md` §3a domain↔proto rule, keeping it on the NextWord seam avoids extra envelope vocabulary.

**Routing path** (Codex v2 P1 fix — does NOT bypass engine mutex): the boost call goes through the standard `EngineHandle::handle` → `dispatch::handle` path. The engine mutex is acquired briefly (microseconds — no observable latency cost), but `dispatch::handle` recognises `BoostCandidates` and skips state mutation entirely (no intent decode → no apply call → no mutation, no current_generation bump). The `Request.generation` IME-session-reset rule applies as normal: a session-mismatch reset will fire if appropriate before the boost runs. `AppConfig.platform_id` validation (Codex v1 P2) is also enforced uniformly — `Platform::Unspecified` → `FailInvariant`. Bridge wrappers therefore must populate both `config` and `generation` for boost calls just like any other call.

### 3.6 `handle.rs` — singleton `EngineHandle`

Mirrors `engine/composing/src/handle.rs`:

```rust
pub struct EngineHandle {
    nextword: Mutex<Engine>,
    last_generation: Mutex<u64>,
}

impl EngineHandle {
    pub fn instance() -> &'static EngineHandle {
        static HANDLE: OnceCell<EngineHandle> = OnceCell::new();
        HANDLE.get_or_init(EngineHandle::new)
    }

    /// LOCK ORDER (Codex v1 P2): always lock `nextword` first, then
    /// `last_generation`. Never call platform / FFI / log code while
    /// either lock is held — they cannot reach back into the bridge
    /// without deadlocking. Matches engine/composing/src/handle.rs:55–60
    /// ordering.
    pub fn handle(&self, req: &NextWordRequest, config: &AppConfig, generation: u64) -> Result<NextWordResponse, NextWordError> {
        let mut engine = self.nextword.lock().expect("nextword engine mutex poisoned");
        let mut last_gen = self.last_generation.lock().expect("last_generation mutex poisoned");
        if *last_gen != generation {
            engine.reset();  // session change — silent state drop, no effects emitted
            *last_gen = generation;
        }
        dispatch::handle(req, &mut engine, config)
    }
}
```

**Note**: envelope `Request.generation` (IME session ID — drops state on mismatch) is distinct from `PersistedState.current_generation` (per-intent monotonic — tags async queries). They share `u64` width but different semantics. The proto field on `FilterPredictions` is named `query_generation` (Codex v1 P2 rename) to make the distinction obvious at call sites.

**Wrap-around test** (Codex v1 P2 + new fixture): `current_generation = u64::MAX` then state-mutating intent → `current_generation` wraps to 0 (Rust `wrapping_add(1)`). Engine MUST treat 0 as the new current; not a reset. Same wrap rule applies to envelope `last_generation`. Test pinned in §7 commit 9.

### 3.7 `dispatch.rs` — proto ↔ Intent ↔ Engine method

Direct port of the composing pattern: decode `NextWordRequest.method` into `Intent` enum, route to `engine.apply / engine.snapshot / engine.filter`, encode response.

---

## 4. iOS bridge — `RustEngineBridge.nextword*`

Add these Swift bridge methods (auto-generated by swift-bridge from the proto types or hand-rolled per the composing pattern):

```swift
extension RustEngineBridge {
    // iOS calls 5 of the 6 decide intents. UpdateLastSelectedWord is
    // Android-only (audit §5 #5); iOS bridge surface intentionally omits it.
    static func nextwordWordSelected(text: String, roman: String, requireRomanMode: Bool, triggerPrediction: Bool, nowMs: Int64, config: AppConfig, generation: UInt64) -> NextWordDecideResult
    static func nextwordBackspace(lastChar: String, nowMs: Int64, config: AppConfig, generation: UInt64) -> NextWordDecideResult
    static func nextwordContextTimeoutFired(nowMs: Int64, config: AppConfig, generation: UInt64) -> NextWordDecideResult
    static func nextwordClearForNewComposing(nowMs: Int64, config: AppConfig, generation: UInt64) -> NextWordDecideResult
    static func nextwordResetFull(nowMs: Int64, config: AppConfig, generation: UInt64) -> NextWordDecideResult
    static func nextwordFilter(raw: [RawNextWordPrediction], queryGeneration: UInt64, nowMs: Int64, limit: Int32, config: AppConfig, generation: UInt64) -> NextWordFilterResult
    static func nextwordQueryState(config: AppConfig, generation: UInt64) -> NextWordStateSnapshot
    static func nextwordBoostCandidates(words: [String], predictedFirstChars: Set<String>, config: AppConfig, generation: UInt64) -> [String]
}
```

Proto types `NextWordDecideResult`, `NextWordFilterResult`, `NextWordStateSnapshot`, `NextWordEffect`, `RawNextWordPrediction`, `EnginePrediction`, `AppConfig` are SwiftProtobuf-generated.

**EngineHandle session ID**: iOS continues to derive `Request.generation` from `documentIdentifier` cache (same pattern as composing slice — see v3.5.4 progress memo).

**`NextWordController` reduction**:
- Keep public API (`process`, `rePredictAfterBackspace`, `resetAndClearUI`, `clearDisplay`, `lastSelectedWord`, `isShowing`).
- Remove `NextWordEngine` calls; replace with `RustEngineBridge.nextword*` calls.
- Remove `persistedState` field — query via `nextwordQueryState` when `lastSelectedWord` / `isShowing` accessed (cache snapshot inside method scope).
- Keep Timer / `@MainActor` plumbing, settings build-up into `AppConfig`, generation extraction from `documentIdentifier`.
- `AppConfig.platform_id = .ios` set at every bridge call (Codex v1 P2: `PLATFORM_UNSPECIFIED` returns FailInvariant).

**`NextWordService.predict`** signature:
- Returns `[RawNextWordPrediction]` carrying un-merged un-scored rows (`hanzi`, `tl`, `count`, `last_used_ms`, `source`).
- `NextWordController.handleQueryResult` calls `RustEngineBridge.nextwordFilter(...)` with `nowMs` (decision-time), `limit` (default 30 if not set), and the `queryGeneration` tagged at dispatch time → Rust scores+merges+sorts+limits → `[EnginePrediction]`.

---

## 5. Android bridge — `RustEngineBridge.nextword*`

Mirror iOS surface PLUS Android-only `UpdateLastSelectedWord` (audit §5 #5):

```kotlin
class RustEngineBridge {
    companion object {
        fun nextwordWordSelected(text: String, roman: String, requireRomanMode: Boolean, triggerPrediction: Boolean, nowMs: Long, config: AppConfig, generation: Long): NextWordDecideResult
        fun nextwordBackspace(lastChar: String, nowMs: Long, config: AppConfig, generation: Long): NextWordDecideResult
        fun nextwordContextTimeoutFired(nowMs: Long, config: AppConfig, generation: Long): NextWordDecideResult
        fun nextwordClearForNewComposing(nowMs: Long, config: AppConfig, generation: Long): NextWordDecideResult
        fun nextwordResetFull(nowMs: Long, config: AppConfig, generation: Long): NextWordDecideResult
        // Android-only — preserves "compound only, no timer, no gen bump" semantics.
        fun nextwordUpdateLastSelectedWord(text: String, roman: String, nowMs: Long, config: AppConfig, generation: Long): NextWordDecideResult
        fun nextwordFilter(raw: List<RawNextWordPrediction>, queryGeneration: Long, nowMs: Long, limit: Int, config: AppConfig, generation: Long): NextWordFilterResult
        fun nextwordQueryState(config: AppConfig, generation: Long): NextWordStateSnapshot
        fun nextwordBoostCandidates(words: List<String>, predictedFirstChars: Set<String>, config: AppConfig, generation: Long): List<String>
    }
}
```

Generated proto types (`NextWordDecideResult` etc.) in `com.siansiansu.taigikeyboard.engine.proto`.

**`NextWordHandler` reduction**:
- Keep public API (`handleNextWordPrediction`, `updateLastSelectedWord`, `handleBackspaceForNextWord`, `getLastSelectedWord`, `isShowingNextWordCandidates`, `setShowingNextWord`, `resetContext`, `clearNextWordState`, companion `isNoise` / `splitCompoundWord` / `extractCurrentWord`).
- Replace `NextWordEngine.decide` calls with `RustEngineBridge.nextword*` decide calls.
- `NextWordHandler.updateLastSelectedWord` routes through `RustEngineBridge.nextwordUpdateLastSelectedWord` (NEW Android-only Rust intent — Codex v1 P1 fix). Removes Android `state` shadow-mutation; canonical state lives in Rust engine.
- Replace `NextWordEngine.filterPredictions` with `RustEngineBridge.nextwordFilter`.
- Companion `isNoise` / `splitCompoundWord` — these helpers are only called from `SmartbarManager` / `CandidateClickHandler` for UI-side classification (NOT inside engine routing). Two options:
   - **5a**: Add bridge methods `nextwordIsNoise(text)` / `nextwordSplitCompound(text, config)` so platform-side helpers call into Rust (single source of truth).
   - **5b**: Keep small platform-side reimplementations of these helpers for UI use (acceptable since they're idempotent pure logic).
   - Plan default = **5a** (single source of truth); Codex review confirms.
- `AppConfig.platform_id = .android` set at every bridge call.

**`NextWordService.predict`** mirror change: return un-scored rows (`hanzi`, `tl`, `count`, `last_used_ms`, `source`). Drop platform-side `NextWordScorer.{scoreDict, calculateUserScore, calculateDecay}` calls inside `predict`.

**`NextWordEngine.kt` companion constants** (`ASSOCIATION_TIMEOUT_MS`, `CONTEXT_TIMEOUT_MS`, `sentenceEndPunctuation`):
- `CONTEXT_TIMEOUT_MS` is used by `NextWordHandler.handleQueryResult` to reschedule timeout after non-empty filter result. Two paths: (a) read it from a constant in `NextWordHandler.companion` (mirrored from Rust, with cross-platform invariant comment); (b) extract from a `nextwordConstants()` bridge call. **Default (a)** — single literal in platform code is fine since the canonical source of truth (Rust crate) is enforced via INVARIANT_* parity test.
- `sentenceEndPunctuation` is used at `NextWordHandler.handleNextWordPrediction:125` for sentence-end pre-check. Move to a bridge helper `nextwordIsSentenceEnd(char)` OR keep small platform-side const literal. **Default**: bridge helper (single source of truth).

---

## 6. Generation semantics — clarification

Two `u64` generations on the wire:

| Generation | Field name on the wire | Owner | Lifecycle | Drop semantics |
|---|---|---|---|---|
| envelope IME-session ID | `Request.generation` | Platform IME session | Bumped on `onStartInputView` (composing slice precedent) | Engine resets state on mismatch — `EngineHandle::handle` clears `Engine`, sets `last_generation = current` |
| NextWord-internal | `DecideResult.current_generation` (returned to platform), `QueryPredictions.generation` (effect carries the new state's gen), `FilterPredictions.query_generation` (platform sends back when calling filter) | Rust engine | Bumped on every state-mutating intent — `WordSelected` / `Backspace` / `ContextTimeoutFired` / `ClearForNewComposing` / `ResetFull`. NOT bumped by `UpdateLastSelectedWord` (audit §5 #5 / Codex v1 P1) | Carried out on `QueryPredictions`/`ClearPredictionsUI` effects; platform tags coroutine result with this gen; `FilterPredictions(query_generation=X)` returns `was_stale=true` if `X != state.current_generation` |

Same `u64` width by accident; orthogonal in semantics. `decide.rs` modifies only `current_generation`; envelope generation is owned by `handle.rs`. `query_generation` (Codex v1 P2 rename) makes the distinction obvious at platform call sites.

**Wrap behavior**: both axes use `wrapping_add(1)` — `u64::MAX + 1 = 0` and the engine treats 0 as the new current value (NOT a reset). Test fixture in §7 commit 9 covers this.

---

## 7. Commit slicing (single PR `phase4b/v3.5.5-nextword-slice`)

Per v3.5.4 precedent — review-tractable boundaries with build/test gates, NOT independently revertable. **14 commits** (Codex v1 P3 split: original commit 5 split into 4 — decide / scorer / filter / booster).

1. **Audit + plan docs** — `docs/engine/nextword-slice-{audit,plan}.md`. Codex pre-impl APPROVED before any code commit.
2. **Refactor preparation** — small parity-preserving alignment commits:
   - iOS `NextWordController` minor adjustments noted during Codex review.
   - Both platforms: `NextWordService.predict` parallel signature plumbing for un-scored rows (introduce alongside existing scored signature; old signature kept until commit 12/13).
3. **proto + generated artifacts**:
   - `engine/protos/proto/nextword.proto` (full schema incl. UpdateLastSelectedWord, BoostCandidates, FilterPredictions w/ now_ms+limit+query_generation).
   - `engine/protos/proto/envelope.proto` (CMD_NEXTWORD + AppConfig deltas: is_translate_swapped, is_association_recording_enabled, platform_id + Platform enum + payload tag 13).
   - Regenerate `engine/protos/src/engine.rs`, swift `RustTaigi.swift`, Kotlin Java protobuf.
4. **`engine/nextword` crate scaffold** — `lib.rs` / `api.rs` / `handle.rs` / `dispatch.rs` skeletons + Workspace `Cargo.toml` member + `Send` invariant + `#![forbid(unsafe_code)]` + `indexmap` dep + `engine/dispatch::lib.rs` route arm + `protos` regen wiring.
5. **`decide.rs`** — port 6 intents (incl. `UpdateLastSelectedWord`), platform-divergence branches via `AppConfig.platform_id`, `should_record_association` strict-`<` + `>= 0` window, `compound_association_pairs`, sentence-end / noise classification.
6. **`scorer.rs`** — port `score_dict` / `calculate_decay` / `calculate_user_score` + 8 constants (USER_WEIGHT / DICT_WEIGHT / DECAY_HALF_LIFE_HOURS / LEARNING_BONUS / HIGH_USAGE_DECAY_FLOOR / LOW_USAGE_DECAY_FLOOR / HIGH_USAGE_THRESHOLD / LN_2 = 0.693).
7. **`filter.rs`** — Codex v1 P1 fix: full merge by `(hanzi, tl)` (no TL-preservation — that rule was dead code per Codex v2 P1), sum scores per source, sort desc, apply limit, stale-gen drop, shape via display rules. `Source::Unspecified` → `FailInvariant`. Empty-hanzi filter.
8. **`booster.rs` + dispatch arm** — `boost_words` partition (insertion-order preserved). Dispatch entry recognises `BoostCandidates` method and runs through standard `EngineHandle::handle` route (stateless — engine state untouched, but the brief mutex acquisition + envelope generation/platform validation enforced uniformly per Codex v2 P1).
9. **Rust workspace tests + INVARIANT fixtures** — coverage list (Codex v1+v2 expanded; counts reconciled):
   - **Decide**: 11 scenario fixtures total — 6 Android intents (incl. `UpdateLastSelectedWord` with isolation: no gen bump, no timer, compound only) + 5 iOS intents (UpdateLastSelectedWord is Android-only; iOS bridge never emits it). Splitting compound (iOS `-` only / Android `-` + whitespace) and noise punct sets covered by per-platform fixtures via `AppConfig.platform_id` branch.
   - **Scorer math**: 7 fixtures — count=0 baseline, half-life decay, double-half-life, learning bonus, high-usage floor, low-usage floor, monotone-decay-property.
   - **Filter**: 12 fixtures — 9 main scenarios (POJ↔TL conversion, empty-roman in Hanji-mode, empty-roman in roman-mode (drop), stale `query_generation`, fresh `query_generation`, dict+user merge with score sum, sort desc, limit truncation, default-limit-when-zero) + 3 invariant-failure / boundary fixtures (`Source::Unspecified` → `FailInvariant`, `Platform::Unspecified` → `FailInvariant`, generation `u64::MAX` wrap-add returns 0 as new current).
   - **Booster**: 3 fixtures — empty `predicted_first_chars` short-circuits, partition order (boosted first, rest preserves original), Unicode first-char (CJK).
   - INVARIANT_nextword_late_prediction_is_discarded.
   - INVARIANT_nextword_compound_pairs_are_sequential.
   - INVARIANT_nextword_association_window_strict_lt_10s.
   - INVARIANT_nextword_backspace_does_not_record.
   - INVARIANT_nextword_sentence_end_resets_context.
   - INVARIANT_nextword_generation_bumps_on_invalidating_intents.
   - INVARIANT_nextword_update_last_selected_word_does_not_bump_generation (NEW — Android Space-path).
   - INVARIANT_nextword_filter_drops_unspecified_source (Codex v2: replaces dropped TL-preservation invariant).
10. **iOS wiring + parity tests** — bridge Swift methods (8 surface fns: 5 decide intents + filter + booster + queryState; iOS does NOT call `nextwordUpdateLastSelectedWord` — Android-only per audit §5 #5); iOS-side parity tests pin pre-/post-bridge equivalence for every fixture from `NextWordEngineTests` + `NextWordScorerTests` + `AutocompleteContextBoosterTests`. PASS in same commit. Tolerance `1e-7` cross-language (Codex v1 P2).
11. **Android wiring + parity tests** — bridge Kotlin methods mirroring iOS surface; Android-side parity tests against same fixtures. PASS in same commit.
12. **iOS swap all call sites + delete legacy** — `NextWordController` rewires through bridge; `NextWordService.predict` switches to un-scored signature (drop old signature, drop platform scoring); delete `NextWordEngine.swift / NextWordOutcome.swift / NextWordScorer.swift / RawNextWordPrediction.swift / EnginePrediction.swift / AutocompleteContextBooster.swift`. iOS user-action checklist: pbxproj add bridge proto files + remove deleted source files (per `feedback_xcode_manual.md`).
13. **Android swap all call sites + delete legacy** — `NextWordHandler` rewires; `NextWordService.predict` switches to un-scored; delete `ime/core/nextword/{NextWordEngine,NextWordOutcome,NextWordScorer,RawNextWordPrediction,EnginePrediction}.kt`; `ime/text/composing/AutocompleteContextBooster.kt` deleted. `NextWordHandler.companion.{isNoise, splitCompoundWord}` route through bridge. Android no manual action.
14. **Final audit + dogfood** — grep audit (no production reference to deleted types); `cargo test --workspace` green; iOS S1/S2/S3 dogfood (type → predict → select → next-word bar flows); Android S1/S2/S3 dogfood. Update `behavioral-invariants.md` cross-refs to point to Rust crate.

---

## 8. Audit grep (final cleanup commit)

Production code must NOT match (test target may keep parity-test imports):

```
NextWordEngine\.|NextWordScorer\.|NextWordOutcome\.|NextWordIntent\.|NextWordEffect|NextWordPersistedState|NextWordDecisionInput|RawNextWordPrediction\.|EnginePrediction\.|AutocompleteContextBooster\.boost
```

iOS / Android both, separately.

---

## 9. Risks + mitigations

| Risk | Mitigation |
|---|---|
| `NextWordService.predict` signature change (un-scored rows) breaks iOS / Android in a non-obvious way | Step 2 introduces parallel signatures (old kept). Step 9/10 swap. Codex pre-impl review on the parallel-signature commit. |
| Generation semantics conflict (envelope vs NextWord-internal) | §6 clarifies; `handle.rs` owns envelope, `decide.rs` owns internal. Tests cover both axes independently. |
| Platform divergences §5 #1, #2 collapsed by mistake | `AppConfig.platform_id` branch in `decide.rs`; INVARIANT tests pin both branches. Codex review enforces. |
| `EnginePrediction.subtitle` Optional / nullable mapping | Codex v1 P2 fix: drop `has_subtitle`. Use `string subtitle = 2; "" == nil`. Filter contract guarantees `subtitle = ""` only when roman is empty (i.e. nil case); non-nil empty subtitle cannot arise because hanzi is required non-empty. |
| Booster placement | LOCKED Codex v1 P2: `BoostCandidates` rides as `NextWordRequest` method (NOT separate dispatch arm). |
| Android `NextWordHandler.updateLastSelectedWord` divergence (§5 #5) lost in port | Codex v1 P1 fix: explicit Rust intent `UpdateLastSelectedWord` preserves "compound-only, no timer, no gen bump". Bridge `nextwordUpdateLastSelectedWord` (Android only). INVARIANT_nextword_update_last_selected_word_does_not_bump_generation pinned. |
| Booster `predicted_first_chars` Set<String> serialization across proto | Use `repeated string`; Rust collects into `HashSet<String>`. iOS bridge accepts `Set<String>`; Android bridge accepts `Set<String>`. |
| Rust `f64.exp()` precision differs from Swift / Kotlin / JVM `exp` | IEEE 754 does NOT guarantee identical libm results across languages (Codex v1 P2). Cross-language parity tests use `accuracy: 1e-7`; Rust-local tests use `accuracy: 1e-9`. |
| `LN_2 = 0.693` vs `f64::consts::LN_2 = 0.6931471805599453` precision | Use literal `0.693` in Rust (3-digit) — explicitly matches Android `NextWordScorer.kt:46` `LN_2 = 0.693` and iOS inline `0.693` literal. Drift over 1-month decay window: ~0.03% (per Android scorer doc comment). Acceptable. INVARIANT test pinned. |
| Generation `u64::MAX` wrap behavior | Rust `wrapping_add(1)` on `current_generation`; engine treats wrapped 0 as new current (NOT a reset). Same rule for envelope `last_generation`. Test fixture in §7 commit 9 covers wrap. |

---

## 10. Codex sandwich gates (per `feedback_codex_review_sandwich.md`)

1. **Pre-impl plan review** — this doc + audit doc → `/tmp/v3.5.5_codex_review_v1.txt` → iterate until APPROVED.
2. **Pre-impl `/simplify` review** — Claude Code `/simplify` skill on the plan; reuse / quality / dead-code sweep.
3. **Per-step Codex review** during implementation if scope drifts.
4. **Post-impl branch review** on the full diff before requesting user merge.
5. **Post-impl `/simplify`** — second pass on landed code.
6. **Codex inline PR review** if any in-flight comments.

`feedback_codex_only.md` — never use Gemini; `codex exec --cd /Users/alexsu/Workspace/taigikeyboard < /tmp/<round>_codex_review.txt`.

---

## 11. Per-slice cadence template (per `project_rust_migration_cadence.md`)

1. INVARIANT_* parity tests — Rust workspace + iOS + Android, BEFORE deletion. ✓
2. ~~Per-slice Settings toggle~~ — DROPPED. Direct swap. ✓
3. Production swap: every site routes through `RustEngineBridge.nextword*`; old impl deleted in same PR. ✓
4. Soak: 1 week real-world before v3.5.6 Lexicon planning starts.
5. Codex sandwich + `/simplify` pre + post. ✓

---

## 12. Cross-references

- Audit: `nextword-slice-audit.md`.
- Boundary contract: `nextword-engine-boundary.md` §§1–13.
- Composing slice precedent: `engine/composing/src/{lib,api,handle,dispatch,transition,derived}.rs`; `engine/protos/proto/composing.proto`; `docs/engine/composing-slice-{audit,plan}.md`.
- Rust best practices: `rules/rust-best-practices.md`.
- Cross-platform alignment: `rules/cross-platform-alignment.md` §§1c, 3a, 5.1.
- Behavioral invariants: `docs/architecture/behavioral-invariants.md` §§7, 8.
- Cadence release map: memory `project_rust_migration_cadence.md`.
- Feedback memories: `feedback_codex_review_sandwich.md`, `feedback_codex_only.md`, `feedback_no_slice_toggles.md`, `feedback_round_hygiene.md`, `feedback_xcode_manual.md`, `feedback_changelog_timing.md`, `feedback_path_g_delete_mirrors.md`.
