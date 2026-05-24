## v3.5.8

Headline release: **連續輸入 (Continuous Input)**. The whole feature — engine syllabifier, whole-sentence lattice + min-cost walker, span-local candidate fetch, dual-line (roman + hanji) candidate carrier, and iOS / Android UI integration — shipped as a single user-facing release together with an Apple-compliance rebrand and a batch of release-only Android fixes. Continuous-input internals span 95 commits / Phase 0–9 + lattice S1–S8 + Bug 1/3 + six follow-up dogfood fixes; the engineering detail is summarized below, the user-visible surface is small.

### Shared (iOS + Android)

#### New Features

- **連續輸入 (Continuous Input)** — type a whole romanized phrase without committing each syllable; the engine syllabifies the buffer, builds a whole-sentence segmentation lattice, walks it for the minimum-cost path, and offers per-position candidates. Tap a candidate to commit its segment and re-rank the rest; Enter commits the raw buffer. Honors POJ / TL input mode, the custom dictionary, user-frequency learning, and next-word prediction.
- Continuous candidates render **dual-line** (romanization + 漢字) instead of single-line, matching the lexicon path so the strip no longer interleaves single- and dual-line cells across keystrokes.

#### Changes

- Donation surface removed for App Store Guideline 3.1.1 compliance: the **寄付支持** tab (linked directly to ECPay) is replaced by a neutral **關於開發者** page that links only to the official website. Applied identically on iOS and Android. (#246)

### iOS

#### New Features

- Continuous-input bridge wiring + KeyboardKit UI integration (Phase 7A / 7B): proto-carried continuous candidates decoded through `RustEngineBridge`, rendered via the existing suggestion strip with per-segment commit semantics.

#### Bug Fixes

- Continuous mid-commit no longer leaks the re-marked tail into the document as literal text on hosts that re-mark on `textWillChange`/`textDidChange` (Bug 3). Mid-commit and final-commit preedit handling converged on a single engine-driven model.
- Swap-aware continuous commit: committing a candidate while a swap (`isTranslateSwapped`) is active now commits the canonical text, and backspace pops the canonical segment (Bug 1).

#### Removed

- Platform lexicon fallback retired (continuous-input capstone): `LexiconService.swift`, `AutocompleteInputClassifier.swift`, `CandidateProcessor.swift`, `LexiconServiceHanziGuardTests.swift` deleted; `AutocompleteService` collapsed to an engine-only single candidate source. Tab3 (Hanji 漢字 dictionary) is unaffected — it uses a separate `DictionarySearchService`.

### Android

#### New Features

- Continuous-input UI integration (Phase 8): Compose smartbar candidate strip drives per-segment commit + next-word handshake from the shared engine.

#### Bug Fixes

- **Feature / FAQ icons and slideshow images now render in release builds.** They are declared by name in `content/*.json` and resolved at runtime via `Resources.getIdentifier()`, so the R8 resource shrinker (release-only) had stripped them — falling back to a placeholder. Silently broken since v3.4.7 (12 releases). Fixed with a package-qualified `res/raw` keep file covering the whole drawable type. (#293)
- **Keyboard preview in Settings now renders the key shapes.** The Compose preview ran under `SettingsTheme`, which lacks the `key_*` color attributes, so every key resolved to transparent. The preview subtree now provides `KeyboardTheme` for attribute lookup; the production IME path was already correct. (#245)
- `LoggerBackend.e()` is now debug-only — release builds no longer risk emitting IME text to logcat (security-rules zero-logs compliance). (#294)

#### Removed

- Platform lexicon fallback retired (continuous-input capstone): `LexiconService.search()` / `lookupCustomDict` / `querySystemDict` / `processCandidates` removed (class kept for Tab3 `searchWithSources` / `searchByHanzi`); `AutocompleteInputClassifier.kt` + JVM test deleted; `TaigiAutocompleteService` collapsed; `CandidateUpdateCoordinator` made mode-agnostic.

### Engine (Rust shared core)

The continuous-input feature is engine-first. Behavior summarized; see `docs/engine/continuous-input-ranking.md` + `continuous-candidate-display.md` for the spec.

- **Syllabifier** — TL BFS + TPS scanner; `syllables.fst` TL syllable-inventory FST; `dict.bin` v2 carries per-record `syllable_count`.
- **Segmentation lattice + min-cost walker (S1–S8)** — behavior-neutral lattice builder → whole-sentence best-path walker with a faithful khiin-style normalized log-probability cost model (`cost = ln(1/p) / len^0.2 · n_syls^0.2`, walker minimizes Σ). User-frequency enters the cost via a librime `formula_d` wall-clock decay (cap-before-decay) plus a McBopomofo epsilon-boost; `custom_dictionary.db` entries enter the slot-0 walker via an effective-frequency proxy. Out-of-vocabulary spans take a khiin-faithful per-character penalty so long sentences no longer collapse to bare romanization. A span-local coverage tiebreak was demoted so a single-syllable first segment isn't buried by a longer competitor.
- **Continuous candidate carrier** — `CandidateMessage` gains `roman` / `hanji` (dual-line) + `CandidateMode` (HANT / TAILO / MIXED); new continuous RPCs `EnterContinuous` / `FetchAtPos` / `CommitContinuous` / `ResetContinuous` / `ContinuousResponse`; `CustomDictEntry` carrier; next-word handshake messages (`NextWordWordSelected`, `NextWordUpdateLastSelectedWord`, `NextWordClearForNewComposing`).
- **Continuous follow-up fixes** — config-aware word-boundary spacing + per-segment case-from-raw (`Hittui` → `Hit tui`, not `HitTui`) (#288); dictionary-compound internal hyphen on manual single-syllable taps (`hitetsaboo` → `hit ê tsa-bóo`) (#295); drop `tl_abbrev` acronym collisions from continuous fetch (#297); best-candidate romanization rendered in the active POJ form (`oo`/`nn` → `o͘`/`ⁿ`) (#299); POJ-mode ASCII canonicalization so `ch-`/`oa-`/`oe-` words no longer return zero candidates (#300).
- Next-word effect is now sourced solely from the engine; the platform builders stay mode-agnostic.

### Dictionary

- `dict.bin` upgraded to **v2** — each record carries `syllable_count`, consumed by the syllabifier and lattice cost model.
- New `syllables.fst` — TL syllable-inventory FST bundled to both platforms.
- `custom_dictionary.db` entries are now reachable from the continuous-input slot-0 walker (still native SQLite for persistence; read into the engine per fetch).
- Dictionary data regenerated via `make dict` at release.

### Build / Tooling

- **GitHub Actions CI removed** (`ci.yml`) — the root `Makefile` (`make fmt-check` / `make lint` / `make test`) is now the sole gate. `security.yml` + Dependabot + PR template remain. (#274)
- Root `Makefile` gains `fmt`/`lint` targets; Spotless / ktlint baseline committed.
- Android `versionCode` auto-generated from epoch minutes.
- Android JNI migrated to `jni 0.22` `EnvUnowned` + `with_env` idiom. (#247)
- `references` gitignore rule anchored to the repo-root clone dir only.
- Confirmation required before any `git stash` (agent workflow guard).
- Dependency bumps via Dependabot: `com.diffplug.spotless 7.0.2 → 8.4.0` (#259); Gradle minor-and-patch group (#292).

### Documentation

- `docs/engine/continuous-input-ranking.md` + `continuous-candidate-display.md` — continuous-input ranking + display specs (source of truth).
- `docs/releases/v3.5.8/plan.md` (archived 2026-05-24; previously `docs/roadmap.md`) — Phase 0–9 plan + 整句 lattice + walker (S1–S9) + continuous-compound-hyphen fix tracked through merge.
- v3.5.9 refactor/maintainability plan **draft** + engine Tier-A design spec (read-only, pre-freeze) — scope later folded into v3.5.8 internals; no separate version cut. (#291)
- New mandatory **doc-lookup** rule (`rules/doc-lookup.md` + CLAUDE.md) — verify current framework APIs via docs before coding.
- MOE segmentation-architecture audit report; mainstream-IME comparison index refreshed (Trime indexed as #16).

### Removed

- GitHub Actions CI workflow (`.github/workflows/ci.yml`).
- iOS: `LexiconService.swift`, `AutocompleteInputClassifier.swift`, `CandidateProcessor.swift`, `LexiconServiceHanziGuardTests.swift`.
- Android: `AutocompleteInputClassifier.kt` + its JVM unit test; `LexiconService` keyboard-path methods.

### New Files

- Engine: `syllables.fst`; continuous-input lattice / walker / cost modules; continuous + next-word + custom-dict proto messages and generated Swift / Java.
- iOS: `Services/SuggestionCaseTransformer.swift`, continuous test suites (`AutocompleteServiceContinuousTests`, `ComposingManagerContinuousTests`, `RustEngineBridgeContinuousTests`); `AboutDeveloperView.swift` (replaces `FeedbackDetailView.swift`).
- Android: `res/raw/com_siansiansu_taigikeyboard_keep.xml` (resource-shrink keep file); `ContinuousSuggestionsContractTest.kt`; `.editorconfig`.
- `rules/doc-lookup.md`.
