# Maintainability Audit Follow-up (2026-10) — Roadmap

> **Type**: Planning (PR table over the 2026-10-09 audit)
> **Keywords**: `refactor`, `dead code`, `parity`, `engine-owned commit`, `dictionary search op`, `desktop-core`, `layering`, `docs drift`
> **Status**: proposed 2026-10-09 — nothing scheduled, nothing started; the maintainer decides order and start
> **Source**: `docs/reports/2026-10-09-audit-all.md` (frozen snapshot on `68f2481b`; §2 findings, §4 draft rounds, §5 open decisions)
> **Predecessor**: `maintainability-roadmap.md` (2026-09-30 audit, R1–R12 complete 2026-10-01)
> **Session memory**: project memory `project_maintainability_audit_2026_10_09.md`

---

## Mandate

- Maintainer 2026-10-09: less duplicated implementation, cross-platform consistency, readability and maintainability, high module cohesion, low coupling, dead code removed, clean architecture, YAGNI. No release scope implied.
- Out of bounds until PR #471 (layout-page restyle) merges: iOS `App/Tabs/Layout`, `App/Tabs/Theme`, `GalleryCard`, `Settings/SettingsModels.swift`, `Settings/SharedSettings.swift`; Android `ui/tabs/{layout,theme}`, `GalleryCard.kt`, `PrefHelper.kt`, `TpsCascade.kt`, `KeyboardLayoutOptions.kt`; `i18n/layout.json`. `.claude/**` is the prompt-audit session's.
- Carried over from the 2026-09-30 mandate: a round may delete or normalise a stored **setting** (named in its PR); user **data** (custom dictionary, frequency, association, learned phrases, themes, fonts, `.taigi` backups) survives every round with a migration or a proof it is untouched, plus a test.
- Not covered: release / version / tag decisions; `.pbxproj` edits (maintainer only); rounds resting on an unconfirmed bug.

## Phase table

Each PR 200–500 LOC. Pre-gates per `~/.claude/rules/round-workflow.md`; Codex sandwich on every round with design judgment; `Skill(simplify)` on the diff; post-PR `tools/test_select.py --run --platform <p>` per touched platform.

| # | Round | Type · pre-gate | PRs | Status |
|---|---|---|---|---|
| H | Docs drift — `linux-release.md` High row, then Appendix D Med + Low | admin lane, direct to main | — | Pending |
| A1a | Dead wire, composing / lexicon: `CandidateMessage` 1/5/6/7 + `script_kind` chain, pre-R6 TPS wire + `tps_wire_equivalence.rs`, `SelectCandidate(text)` (after a ≤20-line producer trace), stale proto comments; tags `reserved`, `make protos` | refactor · freeze list · `refactor-reviewer` | 1 | Pending |
| A1b | Dead wire, phonetics / userdata / nextword / envelope: `TlNumericToTps`, `LearningRecordKind.ASSOCIATION` + `previous_*`, `platform_id` reader + desktop `platform` threading, `length_score`, `EnginePrediction.score`, `UserDataReset.*_removed`, `created_at` / `updated_at`, `RecordUsage.hanji`, `Response.generation`, dead phonetics helpers, duplicate TPS tone-mark predicate, backup `origin` skip | refactor · freeze list · `refactor-reviewer` | 1 | Pending |
| A2 | Platform dead code: `displayText` fallback ×2, iOS `isRawInput`, iOS `TaigiWord.swift`, `diagnostics()` rings ×3 (log line stays), dead members, test-only `pub`, unused parameters, macOS never-shipped cleanup keys, R12 KDocs, `test_select_test.py` in CI, 19 i18n scope edits | refactor · sandwich skipped (delete-only) · i18n gate all platforms | 1 | Pending |
| B1 | `parity(mobile)`: Custom Dictionary list over `ListCustomEntries` filter / limit / offset + paging; client filter and 100-row cap deleted | parity · intended behaviour = engine + desktop · lockstep iOS + Android | 1 | Pending |
| B2 | `parity(mobile)`: Add / Edit requires romanization only; engine refusals shown; dead `tooManyEntries` map deleted | parity · maintainer confirms decision 2 · lockstep | 1 | Pending |
| C1 | Engine `DictionarySearch { query, mode, toggles, custom_enabled, limit }` op; delete `IsHanji` / `DictionaryFilters` / `SearchWithSources` / `SearchByHanji` + per-row `TlToPoj` + `u32::MAX` sentinel; golden tests | feature · Codex design pre-review | 1 | Pending |
| C2 | iOS + Android dictionary search over C1 (orchestration, badge filter, custom-first merge deleted) | refactor · `refactor-reviewer` | 1 | Pending |
| C3 | desktop-core + macOS / Windows / Linux over C1; `lookup_tl` from the engine (lookup-button drift closed) | refactor · `refactor-reviewer` · Windows box check | 1 | Pending |
| D1 | `PredictNext` returns `document_text`, `wrote_romanization`, `earns_auto_space` per prediction; golden incl. empty-bracket case | feature · Codex design pre-review · `phonetics-specialist` | 1 | Pending |
| D2 | iOS + Android prediction taps over D1: `formatOutputText` / `resolveTaigiCommit` (prediction arms), `shouldAppendAutoSpace` ×2, `effectiveSwapped` deleted; usage recorded by the engine | refactor · `refactor-reviewer` · dogfood S1 / S2 | 1 | Pending |
| D3 | Mobile TPS keys over `TpsKey` (one op per key, engine-owned Space-separator rule, one gate) | refactor · `phonetics-specialist` · dogfood S2 | 1 | Pending |
| D4 | Engine emits TPS cell text + §42 single-script split in `CandidateMessage`; shells drop per-render `TlDisplayToTps` and split / dedupe ×3 | feature / refactor · `refactor-reviewer` | 1–2 | Pending |
| E1 | `composing::requests::fetch_at_pos(req, rows: &dyn UserRowsSource)` owns two-pass ranking; dispatch passes `Option<&UserDataStores>`; 5 reach-throughs + 4 mirror-row conversions gone; `dispatch/case.rs` → `phonetics::case_transform` | refactor · `refactor-reviewer` | 1 | Pending |
| E2 | One FFI hop per commit: dispatch applies nextword intents, decide result on `ComposingResponse`; 5 shells drop the relay; `nextword-engine-boundary.md` §40 sites updated | refactor · Codex pre-review + `/code-review` · maintainer confirms decision 3 | 2 | Pending |
| E3 | File splits (pure moves): `continuous.rs` → `continuous/{walk,assemble,recase}.rs`; `api.rs` nailed-join → `nailed_join.rs`; `shadow.rs` → `{lattice,keys}`; `phonetics/api.rs` bodies out; `lexicon/continuous/mod.rs` → `tone_pin.rs` + fetch families | refactor · `refactor-reviewer` · sandwich skipped (moves) | 2–3 | Pending |
| E4 | Visibility + features: lexicon `pub(crate)`, private `mod handle`, desktop-core `pub(crate)` pass, desktop-core `user-data` feature, unconditional `dispatch/user-data` in FFI crates, 1–2-symbol edges, one error-code mapping, `ToneCase { input, kind }` | refactor · `refactor-reviewer` | 1 | Pending |
| F1 | desktop-core `perform_global(action) → GlobalOutcome { represent, refetch, announce }`; Linux / Windows / macOS keep OS calls (macOS static-icon flash = documented exception) | refactor · `refactor-reviewer` · dogfood S113 / S114 | 1 | Pending |
| F2 | Symbol-picker lifecycle in desktop-core | refactor · `refactor-reviewer` | 1 | Pending |
| F3 | Windows `session.rs` → `surface.rs`, lang-bar impls → `lang_bar.rs`, `draw_all` by layout; macOS `TaigiInputController` → `+Menu` / `+AutoSpace` | refactor · sandwich skipped (moves) | 1–2 | Pending |
| F4 | macOS defaults from `Configure`; `FullWidthPunctuation` in `SessionReply`; `TELEX_KEYS` once; session owner from core reply; atomic export write in `desktop-storage` | refactor · `refactor-reviewer` | 1 | Pending |
| G1 | Android: option lists → `ime/settings`, icons neutral; screens through view models; logger injected | refactor · after #471 | 1 | Pending |
| G2 | Android `RustEngineBridge.kt` DTOs → per-area `*Bridge.kt`; `SmartbarManager` forwarders removed | refactor · sandwich skipped (moves) | 1 | Pending |
| G3 | iOS: Platform-layer `SharedSettings.shared` consumers injected; `ActionHandler` deps injected; `UsageRecorder` concrete in `Lexicon/Services`; Engine-layer purity | refactor · `refactor-reviewer` · after #471 | 1 | Pending |
| G4 | Mobile Learning Records view models (one listing call); English current-word split, last-used date, count rule aligned | refactor + parity · maintainer confirms decisions 7–8 | 1–2 | Pending |

Active PR pointer: none.

## Best practices alignment

| Round | Constraining rule |
|---|---|
| H | `round-workflow.md` § Admin lane (docs only, direct to main) |
| A | `code-review-rules.md` §5 regression (callers in every spelling, `known-pitfalls.md` § Tests); `rust-migration-policy.md` §2 (no dead paths); proto tags `reserved` |
| B | `cross-platform-alignment.md` §1b (parity-correction tier, lockstep merge); `diagnosis-discipline.md` (intended behaviour cited from engine `requests.rs:189-215`, no bug round) |
| C, D | `AGENTS.md` § Design principles (shared logic lives in the engine; no redundant fallback); `rust-ffi-safety.md` §2 (domain façade owns the op); `planning.md` § No redundant fallback |
| E | `rust-ffi-safety.md` §2 (dispatch = routing + cross-domain feed); `rust-best-practices.md` §1 / §1a (one crate per concern, one-way edges); `rust-migration-policy.md` §1.3–1.4 (file name = role, one concern per file) |
| F | `rust-best-practices.md` §1 (desktop-core owns shared desktop behaviour); `windows-guidelines.md` / `linux-guidelines.md` (shells = OS calls) |
| G | `ios-architecture.md` §1–2 (layer imports, constructor injection); `ios-exemplar.md` §1 / §5.1 / §8 / §9.5 (IME scope ≠ app scope, per-slice bridge files, view → view model) |

| Mainstream practice | Source | This plan |
|---|---|---|
| The engine resolves what a pick writes (commit text, re-segmentation), the shell only inserts | `docs/references/mainstream-ime-comparison.md` § Segmentation / lattice; proven cite `references/khiin-rs/khiin/src/buffer/buffer_mgr.rs` (commit-and-resegment) | D1–D3 (prediction taps and TPS keys resolved by `commit_text.rs` / `TpsKey`) |
| One query op per lookup kind; the engine decides the path | `mainstream-ime-comparison.md` § Native-engine embedding / FFI threading (mobile) | C1 (`DictionarySearch` picks Hanji vs roman), E2 (one hop per commit) |
| Ranking inputs are gathered once by the engine, not re-fetched by the shell | `mainstream-ime-comparison.md` § Candidate ranking (user_freq, recency) | E1 (`fetch_at_pos` over `UserRowsSource`) |
| Segment / state status is one state machine, shells render | proven cite `references/librime/src/rime/...` (segment status) | F1–F2 (global action and symbol picker become core state, shells draw) |

**Deliberately not adopted / YAGNI**

- Shared Swift package for iOS + macOS — closed by the maintainer 2026-10-04; the ~430 identical Swift lines stay as data.
- macOS candidate-window geometry over desktop-core (~910 lines) — scoped out 2026-10-02; macOS update flow over `taigi-desktop-update` — three recorded blockers (`2026-10-02-macos-desktop-core-inventory.md` §7).
- One generated FFI header for the three desktops — consumers differ (Swift vs C vs in-process); rejected in `macos-desktop-core-roadmap.md:240`.
- Removing the ranker `(display, "")` bucket — it is the identity of a reading-less OOV pick, not a compat fallback; comment fix only (H).
- Re-deleting the ~82 platform tests that restate engine behaviour — R7-2b (#315) kept them on purpose.
- Deleting the Android `CandidateClickHandler` English branch — R1 PR2 pre-review found a real path.
- Moving the continuous fetch between `composing` and `lexicon` — E3 splits files only (decision 6).
- Naming batch C (persisted names), e2e drivers for macOS / Windows / mobile, bigram P6 / P7 — on the roadmap's not-to-re-propose list.
- No new crate for anything above: every destination is an existing crate or module (R3(b) lesson).

## Decisions recorded while running

(none yet)
