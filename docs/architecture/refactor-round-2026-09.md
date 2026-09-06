# Refactor round 2026-09 — five-platform readability / coupling / duplication pass

> **Type**: Plan (multi-PR)
> **Status**: APPROVED by USER 2026-09-06 (「依照你的建議 go」) — D1 = (a) extension functions
> **Workflow type**: Refactor (`~/.claude/rules/round-workflow.md` § Workflow types) — every PR is behavior-frozen per `.claude/rules/cross-platform-alignment.md §1`. No user-visible change, no persisted-data change, no ranking change.
> **Memory**: `memory/project_refactor_round_2026_09.md` (phase status + active pointer)

USER 2026-09-06: 「review engine, ios, android, macos, windows, 不影響功能的情況下, plan to 增加可讀性, 可維護性, 減少程式碼耦合, 減少重複實作, 不 over-design, 此為重構 round」.

Seeds: the refactor-merge findings the 2026-09-05 cleanup round parked (USER 「留給之後有時間再處理」) — all re-verified present on `main` 2026-09-06 — plus fresh per-module audits. Every row below is grounded in code (`file:line` read this session).

---

## 1. Goal / non-goals

- **Goal**: fewer copies of the same logic, smaller public surfaces, functions that do one thing, comments that say *why*, and **fewer tokens per future read** (USER 2026-09-06: 「減低每次查詢使用的 token」 — every line deleted here is paid back on every later `Read`). Measured by LOC removed and by call-site count collapsed — not by new abstractions added.
- **Non-goals**: new crates / packages / DI layers; any change that alters an observable property (`docs/architecture/behavioral-invariants.md`); release scope (USER-gated).
- **Over-design guard**: a helper is introduced only when it replaces ≥2 verbatim copies *or* removes a positional-argument seam that already has a context struct. "Three similar lines beat a premature abstraction" (`~/.claude/rules/code-review-rules.md §4`).

## 2. PR plan (one PR per phase; engine first because of the stale-binary gate)

| Phase | PR | Scope | Est. LOC | Risk | Gate |
|---|---|---|---|---|---|
| E1a | engine composing test helpers | `composing/tests/common/mod.rs` — shared serializer / lock / temp / `config` / `req` helpers only; per-test fixture rows and key sets stay in each file | −700 test | L | `cargo test -p composing` |
| E1b | lexicon test-only pub surface | `fetch_candidates_for_endings` + 3-arg wrapper + `PARTIAL_PREFIX_HYDRATE_CAP` → `lexicon/tests/common/` (40 call sites keep the name) | −90 src / +60 test | L | `cargo test -p lexicon` |
| E2 | engine small dedupes + docs | phonetics `is_combining_tone_mark`; composing `transition.rs` merges; visibility; misattached / stale docs | −200 | L | `cargo test -p phonetics -p lexicon -p composing` |
| E3 | lexicon `continuous.rs` internals | visitor / tail dedupe, ctx threading, face guards, clone chains | −150 | M | E2 gate + `candidate_dump` + `golden_fetch_at_pos` byte-identical + `cross_mode_parity` |
| E4 | composing `shadow.rs` / `continuous.rs` dedupes | `fused_shadow`, `span_key`, edge-weight helper, seam folds | −80 | M | same as E3 |
| I1 | iOS | bridge send helper ×5→1, SQLite helper reuse, NextWord table-creation memo | −120 | L/M | `xcodebuild … TaigiKeyboardTests` |
| A1a | Android engine bridge | `dispatch` ×6→1 (exception boundary preserved), facade-shim decision D1; diagnostics-count change as its own labelled commit | −250 | M | `:app:testDebugUnitTest` + `assembleDebug` |
| A1b | Android app-side | CSV codec symmetry, Activity scaffold, `COUNT(*)` helper | −100 | L | same |
| M1a | macOS candidate panels | base-class hoists, chevron/arrow base | −70 | L | `make -C macos test` |
| M1b | release scripts | `publish-release.sh` shared lib (macOS ↔ Windows) | −80 | L | both scripts `--dry-run` / shellcheck |
| W1 | Windows tsf (box-gated) | session / text_service helpers, `draw_text` helper, magic numbers, dead param, stale PR-era comments | −120 | M | `make windows-check` on host, then box build + dogfood (`docs/architecture/windows-release.md`) |

E1a → E1b → E2 → E3 → E4 sequential (overlapping files). I1 / A1a→A1b / M1a / M1b / W1 are independent of each other and of the E-phases, except that any platform test run after an E-merge must follow `make build` (project `CLAUDE.md` § stale-binary gate).

PR sizing follows `~/.claude/rules/planning.md` (200–500 LOC target). E1a exceeds it in *deleted* test lines only.

Codex plan review 2026-09-06 (ANALYSIS-ONLY, verdict REVISE → applied): dropped single-implementation extractions (walker `choose_edge` lift, Step-4 block moves, tsf inline/stacked math move to core), split A1 / M1, fixed the E1b caller claim, widened E3 / E4 / I1.3 freeze lists, added the sixth Android `dispatch`, and flagged decision D1.

---

## 3. Per-phase candidate tables (grounded in code)

### E1a / E1b — engine test fixtures

| # | file:line | smell | change |
|---|---|---|---|
| E1a | `engine/composing/tests/{golden_fetch_at_pos,tps_space_pinned_tone,roman_only_display_dedup,tps_display_dedup,continuous_slot0_dict_roman,continuous_explicit_tone}.rs` preambles 250–860 lines each | `engine_install_lock`, `write_temp`, `build_tkdb_v3`, `build_dictionary_fst`, `build_syllables_fst`, `empty_association_bin`, `install_fixture`, `config`, `req` re-declared per file (`fn req` ×10, `config` ×9, `unique_temp_path` ×5, `engine_install_lock` ×6). **Not** byte-identical fixtures: `golden_fetch_at_pos.rs:178` installs a POJ family, `continuous_explicit_tone.rs:93` TL-only | create `engine/composing/tests/common/mod.rs` (lexicon precedent: `engine/lexicon/tests/common/mod.rs:32-122`) holding only the serializer / lock / temp-path / `config` / `req` helpers; each test keeps its own `fixture_rows` + key set; `_tps` variants become parameters |
| E1b | `engine/lexicon/src/continuous.rs:433-520` `fetch_candidates_for_endings` (`#[doc(hidden)]`; callers `tests/span_local_fetch.rs` ×26, `tests/user_freq_plumb.rs` ×14, doc mention `ranking/src/score.rs`); `:639-645` 3-arg `fetch_candidates_for_keys` wrapper; `PARTIAL_PREFIX_HYDRATE_CAP` pub | test-only pub surface + its own mode→prefix match (`:449-454`) | move the endings wrapper (same name, same signature) into `lexicon/tests/common/`; 40 call sites unchanged; 3-arg wrapper callers become `_with_barriers(keys, &[], &[], …)`; cap → `pub(crate)` with a `tests/common` re-read via the public fetch |

### E2 — engine small dedupes + docs

| # | file:line | smell | change |
|---|---|---|---|
| E2.1 | `lexicon/src/continuous.rs:1434-1446 is_combining_tone_mark` ≡ `composing/src/shadow.rs:1158 is_tone_combining_mark` ≡ keys of `phonetics/src/tables.rs:61 COMBINING_TO_TONE_NUM` | same predicate three ways, lock-step test `shadow.rs:2091` exists only to pin the copies | `pub fn is_combining_tone_mark(char)` in `phonetics` (tables-backed); delete both copies + lock-step test |
| E2.2 | `composing/src/transition.rs:125-137` vs `:622-641` | `step_response` / `continuous_step_response` byte-identical | delete the second; 3 in-file callers |
| E2.3 | `transition.rs` abort trio ×6 (`:228,305,325,561,689,881`) + finalize trio ×8 (`:392,428,497,516,537,727,757,836`) | ordered effect vectors hand-written; order is load-bearing per module header | `abort_continuous_effects()` / `finalize_effects(text)` return the trio only; every caller composes its **full** vector explicitly. Freeze: per-branch complete effect order — `NextWord*` is not always last (`:692` appends composing effects after it); `intent_coverage.rs` + `invariants.rs` pin each branch |
| E2.4 | `composing/src/continuous.rs:357-366 fetch_via_lexicon_inner` | pass-through, 1 caller | inline at `:1073` |
| E2.5 | `shadow.rs:902 strip_ascii_tone_digits`, `:1187 apply_normalize_with_offsets` `pub(crate)` | zero external callers | private |
| E2.6 | docs: `lexicon/src/continuous.rs:522-575` doc block attached to `for_each_exact_reading` instead of `fetch_candidates_for_keys` (`:639` undocumented); `:1484-1510` dispatcher doc glued to `is_tps_acronym_face_hit`; stale line cites `:1049`, `shadow.rs:70`, `composing/src/continuous.rs:1224`, `dispatch.rs:480`; `continuous.rs:343` claims `remove_display_duplicates` dead (called from `ranking/src/process.rs:71,95`); module headers narrating Phase 5/6 + PR numbers (`lexicon/src/continuous.rs:1-90`, `:98-104`, `:302-315`) | re-attach, delete stale numbers, trim history to contract + why (`~/.claude/rules/ai-friendly-code.md` § Comments) |

### E3 — lexicon `continuous.rs` internals

| # | file:line | smell | change | freeze |
|---|---|---|---|---|
| E3.1 | `:668-745` vs `:1176-1220` | per-rowid visitor body duplicated (`record` → `passes_filter` → toneless match → space-pin → source mask → `record_to_candidate`); only push-vs-max differs (Codex-verified: no other filter delta) | `fn exact_candidates_for_key(…, sink: impl FnMut(RawCandidate))` | freeze: matched key, source mask, rowid traversal order, max uses strict `>` so ties keep the **first** candidate, NaN score handling unchanged (`:1223`); `span_local_fetch.rs`, `user_freq_plumb.rs` |
| E3.2 | `:762-800` vs `:1093-1112` | custom-entry merge + `dedupe_by_roman_hanji_span` + `SortKey` sort tail verbatim ×2 — also on the partial-prefix path (`:1056-1100`), not only exact / walker | `fn merge_custom_dedupe_sort(out, ctx, raw_len, coverage_kind)` | freeze: `coverage_kind` per path, custom-entry TPS space-pin gate, dedupe winner (first by stable index), stable enumerate index before sort; `item12_custom_dedupe_tests`, `sort_key_tests`, `partial_prefix_syllable_reach.rs` |
| E3.3 | `:1132-1175` + `composing/src/continuous.rs:461-475` | `best_candidate_for_key` 7-arg wrapper (zero production callers) + 9-arg `_with_barriers` (`#[allow(clippy::too_many_arguments)]`) re-thread `ContinuousFetchCtx` fields positionally; composing walker seam `:476` threads `enabled_sources_bitmask` positionally and `assemble_candidates` destructures `ContinuousKeys` into a 4-tuple (`:1008-1027`, `|(_, _, _, inv)|` at `:1058`) | take `&ContinuousFetchCtx` / `&ContinuousKeys`; delete 7-arg wrapper; both `allow` attributes removable | `span_local_fetch.rs:1765-1908`, `golden_fetch_at_pos.rs` |
| E3.4 | `:1312-1330 / 1380-1391 / 1475-1490` vs `:1568-1580 / 1609-1618 / 1620-1630` | 6 guards = 3 families × {eq, starts_with}; tone-bearing predicate repeated at 9 sites | per family one `fn reconstruct_faces(record_tl) -> (primary, Option<alias>)`; keep the `NASAL_OO_ALIAS_SPELLING` gate (`:1605`) — real behavior | `abbrev_collision_guard_tests`, `poj_abbrev_*`, `dispatcher_tests`, `nasal_oo_alias_face_tests`, `tests/{tps,tlpoj}_partial_prefix_abbrev.rs` |
| E3.5 | `:1842-1900 record_to_candidate`, `:2038-2070 dedupe_by_roman_hanji_span` | `tl.clone()` ×2 + `hanzi.clone()`; dedupe clones every key into a `HashMap` then usually early-returns | compute `display_text` last; dedupe on borrowed keys + `retain` (winner = lowest index, same as today) | `record_to_candidate_carrier_tests`; partial-prefix callers (`:1094`) covered by `tlpoj_partial_prefix_abbrev.rs` |

Not in E3 (parity-gated, own decision): `:1405-1431 derive_poj_notone_for_match` vs `phonetics/src/syllable.rs:34 poj_num_syllable_ends_from_tl` — replace **only** if `tests/poj_notone_parity.rs` proves byte-equality over all 168k rows; otherwise leave.

### E4 — composing `continuous.rs` / `shadow.rs` structure

| # | file:line | smell | change | freeze |
|---|---|---|---|---|
| E4.1 | `shadow.rs:945-954` vs `:1296-1304` (partial `:582-583`) | `lowercase → canonicalize_poj_shadow → build_hyphen_shadow → build_separator_shadow` retyped; S6 byte-identity relies on lock-step | `fn fused_shadow(raw, mode)`; callers apply own tone rule | `custom_toneless_key` unit tests, `build_keys_tl_poj_diacritic.rs`, `tps_space_pinned_tone.rs::space_pin_also_gates_custom_dictionary_entries` |
| E4.2 | `shadow.rs:505-540, 605, 637` vs `continuous.rs:534-585 (557, 578)` | per-span key triple (`fst_body_for_span` + `key_final_only_offsets` + `span_end_pins_unmarked_tone` + `format!`) derived twice. ⚠ Two coordinate conventions coexist: final-only offsets take barriers with `start < b ≤ end` then subtract `start`; tone-pin compares the **global** `end` against global barriers | `shadow::span_key(shadow, start, end, mode, barriers) -> SpanKey` that documents both conventions in one place; left-anchored path passes `start = 0`. New unit tests: non-zero `start`, barrier exactly at `start`, cross-barrier span | `build_keys_tps.rs`, `tps_space_pinned_tone.rs`, `continuous_explicit_tone.rs`, `golden_fetch_at_pos.rs` |
| E4.3 | `continuous.rs:630-632` and `:700-705` | freq lookup + `decayed_user_weight_delta` sequence duplicated inside the walker edge closure | `fn edge_user_weight_delta(freq_map, now_ms, display_text, canonical_tl) -> f64` only — the closure itself and the Step 4 / 4b blocks (`:1119-1370`) are **not** lifted (single implementations; dropped at Codex review) | `golden_fetch_at_pos.rs`; freeze also pins the FULL-exclude-**before**-truncate order at `:1328-1367` |
| E4.4 | `shadow.rs:214-225 build_shadow_lattice` + `:426-437 left_anchored_keys_from_lattice` | each has one caller (`dispatch::build_keys_tl_with_inventory` test seam); wrapper doc admits "keeps historical test seams byte-compatible" | fold into the seam; keep both `dispatch.rs:394-419` seams (they pin TPS divergence) | `build_keys_tps.rs` (42 seam call sites unchanged) |

### I1 — iOS

| # | file:line | smell | change |
|---|---|---|---|
| I1.1 | `ios/Sources/TaigiKeyboard/Engine/RustEngineBridge+{CaseTransform:145-161, Composing:641-657, Lexicon:668-684, NextWord:459-475, Phonetics:236-252}.swift` | identical 17-line encode → `process_request_bytes` → decode → `recordFailure` block ×5 | `static func send(_ request: Taigi_Engine_Request, op: String) -> Taigi_Engine_Response?` in `RustEngineBridge.swift` next to `sendRawBytes` (`:92`) |
| I1.2 | `Lexicon/Database/UserFrequencyPruner.swift:41-52 rowCount` (sentinel −1), `CustomDictionaryCapacityPolicy.swift:34-45 currentEntryCount` (sentinel 0) | hand-rolled prepare/step/finalize for `SELECT COUNT(*)` while `SQLiteBindingHelpers.swift:46 sqliteQueryScalarInt` exists | `(try? sqliteQueryScalarInt(db: db, …)) ?? <same sentinel>` — sentinel preserved per call site |
| I1.3 | `NextWord/Repository/NextWordSchema.swift:161-175` private `tableExists` / `columnExists` (throwing) vs `SQLiteBindingHelpers.swift:76 sqliteTableExists` / `:88 sqliteColumnExists` (non-throwing) | same query, two error contracts | one throwing implementation pair in `SQLiteBindingHelpers`; the existing non-throwing names stay as thin `(try? …) ?? false` wrappers because `UserFrequencySchema.swift:57` (`tableExists`) and `UserFrequencySchema.swift:57` / `CustomDictionarySchema.swift:47` (`columnExists`) must keep returning `false` on error. Freeze: NextWord migration (`NextWordSchema.swift:117, 148`) keeps propagating **both** prepare and step errors |
| I1.4 | `NextWord/Services/NextWordService.swift:70-74, 238-240, 334` `_tableCreationTask` + generation memo | cleanup round noted the same memo shape ×3 across services — **verify first** (only NextWord confirmed this session); dedupe only if the other two are verbatim |
| I1.5 | `App/Tabs/Dictionary/Views/FrequencyDataView.swift` (175) / `AssociationDataView.swift` (186) share 10 members (`filterText`, `displayLimit`, `showClearAlert`, `handleImport`, `importExport`, …); view models share 14 | two near-copies | **judgment-gated**: extract only the toolbar + filter + clear-alert scaffold if the diff after I1.1–I1.3 shows it stays a single generic container; otherwise leave (two views ≠ over-design trigger) |

iOS↔macOS: 17 same-named Swift files, 5,439 diff lines, 0 identical (bridge files on macOS are 40–60 % the size of iOS). A shared Swift package is **not** proposed.

### A1a — Android engine bridge

| # | file:line | smell | change |
|---|---|---|---|
| A1.1 | `engine/{CaseTransformBridge:182, LexiconBridge:494, LexiconBridge:643 rankingDispatch, NextWordBridge:318, PhoneticsBridge:243}.kt` + `ComposingBridge.kt:342-352` | **six** request→`dispatchRaw`/`sendRawBytes`→parse helpers in two styles: Case/Lexicon (`:193`, `:500`) catch the JNI `Throwable`, log via `backend.w`, and bypass `recordFailure`; NextWord/Phonetics/Composing/ranking go through `sendRawBytes` (`RustEngineBridge.kt:1174`, which does **not** catch JNI throws) + `recordFailure` | one `internal fun dispatch(op, build: Request.Builder.() -> Unit): Response?` on `RustEngineBridge` that keeps the `try/catch(Throwable)` boundary (so Case/Lexicon callers never start throwing) and calls `recordFailure`. Diagnostics counter (`RustEngineBridge.kt:1147`) has no production reader — only `androidTest/.../RustEngineBridgeTest.kt:151` — but Case/Lexicon failures start counting: land that as its **own commit** in the PR, titled `parity:` (`cross-platform-alignment.md §1b`, iOS records everywhere) |
| A1.2 | `engine/RustEngineBridge.kt` (1,404 lines): 46 one-line delegations `= XBridge.f(…)` (`:111` onward); 295 facade call sites in 18 files **and** 27 direct sub-bridge call sites in 8 files (`LexiconService.kt` ×9, `TaigiKeyboardApplication.kt` ×3, `DictionarySearchViewModel.kt` ×3, `SuggestionCaseTransformer.kt` ×3, `KeyLabelCaseCache.kt` ×3, `NextWordService.kt` ×2, `ComposingManager.kt` ×2, `TextInputKeyHandler.kt` ×2); test `KautianSubcollWireEncodeTest.kt:3,74` references `LexiconBridge` directly | two entry points for one engine; shims are pure duplication | **Decision D1 (USER)** — both delete the 46 shims and leave one entry point; JNI `external fun`s, `dispatchLog`, logger/diagnostics state stay in the `object` either way: **(a)** move sub-bridge bodies into `fun RustEngineBridge.xxx(…)` extension functions in the same files (call sites unchanged, 27 direct sites rewritten to the facade, sub-bridge private state → file-private; mirrors iOS `RustEngineBridge+*.swift`, `docs/architecture/ios-exemplar.md`); **(b)** (Codex's pick) delete the facade shims and rewrite the 295 facade call sites to the sub-bridge objects (domain explicit at each call site; larger but grep-mechanical diff). Claude leans (a) — smaller call-site churn, keeps the iOS-shaped API `cross-platform-alignment.md §2` asks Android to mirror; Codex: "no basis beyond Swift look-alike" |

### A1b — Android app-side

| # | file:line | smell | change |
|---|---|---|---|
| A1.3 | `ui/tabs/dictionary/AssociationDataViewModel.kt:83-89` inline `escape` encode + `:114-124 parseCSV` | association CSV codec lives in the ViewModel while frequency codec lives in `ime/dictionary/DictionaryCsvCodec.kt:50-60` | `encodeAssociationCSV` / `decodeAssociationCSV` beside the frequency pair. Guard exception, stated: this is a responsibility move into an **existing** home, not a new helper |
| A1.4 | `settings/*Activity.kt` ×8 `onCreate` | `PrefHelper` → `TypefaceLoader` → `FontFamily` → `setupEdgeToEdge()` → `setContent { ProvideDisplayLanguage { TaigiKeyboardTheme { … } } }` repeated | `ComponentActivity.setTaigiContent(content: @Composable (FontFamily) -> Unit)` in `settings/`; 8 call sites |
| A1.5 | `CustomDictionaryService.kt:193,428`, `UserFrequencyService.kt:150,358,447`, `NextWordService.kt:716` | `SELECT COUNT(*)` + cursor/moveToFirst ×6 | `SQLiteDatabase.rowCount(table)` extension (returns −1 on empty cursor, as today) |

Verified **not** worth touching: `ime/core/PrefHelper.kt` already uses `by preference(...)` delegates.

### M1a — macOS candidate panels · M1b — release scripts

| # | file:line | smell | change |
|---|---|---|---|
| M1.1 | `Candidates/{Horizontal:169, Vertical:403, Expandable:805}CandidatePanel.swift applyHighlightColor` | identical loop over item views ×3 (`CandidateBasePanel.swift:381` is an empty hook) | base owns the loop over an `allItemViews` accessor; subclasses supply the array |
| M1.2 | `HorizontalCandidatePanel.swift:175-183` vs `ExpandableCandidatePanel.swift:811-819 updateCorners` | same override: pill corners when paged/overflowing else `super` | base `updateCorners()` consults `var wantsPillCorners: Bool { false }`; two subclasses override the bool |
| M1.3 | `Candidates/CandidateChevronView.swift` (87) vs `CandidatePageArrowView.swift` (119) | share 10 members (`baseImageWidth`, `basePadding`, `baseSpacing`, `baseSymbolPointSize`, `configuration`, `imageWidth`, `padding`, `spacing`, `intrinsicContentSize`, `mouseUp`) | `CandidateEdgeControlView` base (cleanup-round seed) |
| M1.4 (= M1b) | `macos/scripts/publish-release.sh` (306) ↔ `windows/scripts/publish-release.sh` (244): `anonymous_curl`, `anonymous_status`, `commit_site_file` | duplicated shell helpers | `scripts/lib/release-site.sh` sourced by both (both trains keep their own flow) |

Leave: `Controller/TaigiInputController.swift:492-650 handle()` — 158 lines but a flat `switch intent` (allowed shape per `ai-friendly-code.md` § Function design); `CandidateBackdrop.swift` three backdrop impls (protocol by design).

### W1 — Windows tsf (box-gated)

| # | file:line | smell | change |
|---|---|---|---|
| W1.1 | `session.rs:258-263, 461-471, 581-591` + closures `:473-486` vs `:594-601` | editor prologue ×3, commit closure ×2 | `fn editor_inputs(&self)`, `fn commit_under_session(..)`; `run_key` keeps its own |
| W1.2 | `session.rs:139,171,385,550,643,693,812`; `text_service.rs:653` | `coordinator_if_built().and_then(try_lock)` ×8 | `Runtime::try_coordinator()` beside `runtime.rs:111 coordinator_if_built` |
| W1.3 | `session.rs:236,499,546,679,731,807`; `text_service.rs:419` | presenter clone-outside-borrow + hide ×7 | `fn presenter()`, `fn hide_candidates(token)` — the "no COM under the borrow" rule documented once |
| W1.4 | `candidate_window.rs:880-1045 draw_cell`; 5× `cached_layout` + `SetColor` + `DrawTextLayout(…ENABLE_COLOR_FONT)` (`:911, 942, 965, 1002, 1023`) + 1 in `mode_flash.rs:152` | same 3-call draw sequence ×6 | `draw_text` helper only; the inline/stacked split and the text-box / easing / thumb math move into `taigi-windows-core` were dropped at Codex review (single implementations, no second copy) |
| W1.5 | `candidate_window.rs:212-222` vs `:1101-1105, 1145-1149`; magic `4.0, -3.0, 5.5, 1.5, 3.0` | width and centre derived separately | named constants + `symbol_centre_x` |
| W1.6 | `candidate_window.rs:234, 1302`; `mode_flash.rs:63` | `monitor_at(caret origin)` ×3 | `monitor_for_caret()` |
| W1.7 | `session.rs:164-183 composing_flags` unused `identity` (`let _ = identity;`) | dead parameter, 2 callers | drop |
| W1.8 | stale PR5b/PR6/PR10 comments: `session.rs:11-12`, `contexts.rs:28` (truncated sentence), `text_service.rs:78-79, 365`, `registration.rs:27-30`, `lang_bar.rs:195`; `core/composing/coordinator.rs:66 allocate_token` has zero tsf callers (only `tests/composing_manager.rs:773`) | history-as-doc; orphan API | rewrite present-tense; delete `allocate_token` or route `token_for` through it — state the choice in the PR |

Leave: `session.rs:222-349 run_key` (ordering-critical, documented hazards); `edit_session.rs:44-88 run_sync` + `com_out_buffer.rs` (deliberate `unsafe`, #677 UB fix); `text_service.rs:273-361 deactivate` 9-tuple; `ui/window.rs` wndproc; `taigi-windows-storage` `is_ready/open/open_blocking` ×3 (12-line delegations — a trait would be over-design); `taigi-windows-settings/src/winui/pages/custom_dictionary.rs:ensure_loaded` (227 lines, WinUI, not host-testable — not audited to `file:line` depth this round).

---

## 4. Explicitly NOT in this round (behavior-touching or out of a refactor's reach)

| Item | Why not a refactor |
|---|---|
| `engine/ranking/src/score.rs:539 is_nonspacing_mark` (strict Unicode `Mn`) vs `engine/phonetics/src/derivation.rs:82` (hard-coded ranges) | the two predicates differ on exotic input; unifying is a parity decision + dogfood, not a move |
| `engine/phonetics/src/tps.rs:257 is_tps_tone_mark` (8 fixed scalars incl. U+0307) vs `tps_adjust.rs:29-55 TONE_MARK_CHARS` (table-derived, **excludes** U+0300–036F) | different sets; `tps_adjust` version backs the `Method::IsTpsToneMark` op — unification changes a public op |
| `derive_poj_notone_for_match` → `poj_num_syllable_ends_from_tl` | only with a 168k-row parity proof (§3 E3 note) |
| 6 byte-identical `*.pb.swift` macOS ↔ iOS (11,014 lines) | build-graph change in `.xcodeproj` / `Package.swift` — user-only config |
| Settings-key table ×2 (`SettingsStore.swift` ↔ `keys.rs`) | needs a generator = new tooling |
| Comment-narration prune in `shadow.rs` / `composing/continuous.rs` (58–62 % comment lines, 59 + 74 phase/PR lines) | doc-tier pass; E2.6 trims module headers only, per-line narration is its own admin-tier commit |
| Splitting the 485-line walker (`composing/src/continuous.rs:461-770`) and the 470-line `assemble_candidates` (`:1000-1370`); moving tsf cell / easing / thumb math into `taigi-windows-core` | single implementations — the §1 guard requires ≥2 copies; readability-only moves on M-risk golden paths were dropped at Codex review |
| engine `WalkerSlot0`↔`RawCandidate` mirror, `bounded` fetch trio, `prefix_index.rs` four lookups, `SyllableReach`/`KeyFace` | each doc explains a deliberate divergence (sign bridge D3, PR #351 cap ordering, `INVARIANT_LEX_LOOKUP_ROWIDS_ORDER`) |

---

## 5. 最佳實踐對齊 / Best-practices alignment

| Rule / reference | Where it binds |
|---|---|
| `~/.claude/rules/round-workflow.md` § Workflow types — Refactor pre-gate = behavior-freeze boundary | §3 "freeze" columns list the observable properties + covering tests per row; Codex pre-impl reviews the regression surface per PR |
| `.claude/rules/cross-platform-alignment.md §1` / `§1b` | every PR title `refactor(...)`; A1.1's diagnostics-count change is the one parity-correction hunk, labelled and tested on iOS + Android |
| `~/.claude/rules/code-review-rules.md §2, §4` | helpers only where ≥2 verbatim copies exist (counts cited per row); no traits / protocols for 1 use |
| `~/.claude/rules/ai-friendly-code.md` § Comments, § Function design | E2.6 / W1.8 doc fixes; flat `switch` bodies (macOS `handle()`, Windows `run_key`) deliberately kept |
| `docs/architecture/ios-exemplar.md` + `cross-platform-alignment.md §2` (Android mirrors iOS) | A1.2 extension-function split mirrors iOS `RustEngineBridge+*.swift` |
| `.claude/rules/rust-best-practices.md` (dependency direction, one-way crate graph) | E2.1 moves the shared predicate *down* to `phonetics` (leaf) — no new edge |
| project `CLAUDE.md` Core Principle #6 (direction-first over fix-scope) | A1.2 chooses the larger extension-fn rewrite over patching 8 direct call sites |
| `docs/references/mainstream-ime-comparison.md` | not consulted — no algorithm or ranking change in this round; nothing is borrowed |

Deliberately not adopted: shared Swift package for iOS/macOS (diverged, §3 I1 note); a `Store` trait over the three Windows stores; a generic `LearningDataView<Row>` on iOS unless I1.5's judgment gate passes; a Kotlin base class for `Frequency/AssociationDataViewModel` (Android guidelines prefer composition; 14 shared names but different row types).

---

## 6. Gates (per PR)

1. Codex pre-impl ANALYSIS-ONLY on the regression surface (callers + freeze list) → implement → Codex post-impl + `/simplify` → commit → push → `gh pr create` → post-PR parallel verification for the touched platform(s).
2. Engine PRs (E1a–E4): `cargo test -p <crate> --no-fail-fast` — the known-red lib test in `composing` (`lattice::cost::tests::corpus_total_freq_matches_dictionary_csv`) otherwise stops the run before the integration-test binaries; E3/E4 additionally `candidate_dump` + `golden_fetch_at_pos` + `cross_mode_parity` unchanged; `make build` after merge (stale-binary gate) before any platform test run. Known-red `corpus_total_freq_matches_dictionary_csv` stays red (USER-gated).
3. `swiftformat` / `cargo fmt --all` / `spotlessApply` are **not** run repo-wide (pre-existing drift, memory ⚠).
4. W1 is verified only on the Windows box (`ssh win`, `make windows-check` on host first); dogfood before merge.
