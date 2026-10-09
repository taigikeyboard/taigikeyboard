# Maintainability Audit Follow-up — Roadmap

> **Type**: Planning (PR table over the 2026-09-30 audit)
> **Keywords**: `refactor`, `dead code`, `parity`, `desktop-core`, `userdata`, `commit resolution`, `test selection`, `naming`, `docs drift`
> **Status**: complete 2026-10-01 — R1–R12 and A merged (#274–#331)
> **Source**: `docs/reports/2026-09-30-audit-all.md` (frozen snapshot on `30a79c16`; §2 findings, §4 draft rounds, Appendices A–F)
> **Session memory**: project memory `project_maintainability_audit_2026_09_30.md` (Claude auto-memory)

---

## Mandate

- USER 2026-09-30: "ok, go, follow your recommendations; after creating a PR, merge and continue until done" — the report's §5 recommendations are adopted as decisions, and each PR merges once its gates are green.
- USER 2026-09-30: "finish all rounds; affecting user settings is fine, just don't lose data". A round may delete, reset or normalise a stored **setting** without a migration (named as a behaviour change in its PR). User **data** — the engine `userdata` stores (custom dictionary, frequency, association, learned phrases), user-created themes, copied-in fonts, `.taigi` backups — must survive every round; a round that touches one carries a migration or a proof it is untouched, plus a test.
- Not covered by the mandate: release / version / tag decisions; `.pbxproj` edits (USER only); rounds that rest on an unconfirmed bug.

## PR table

| Round | Content | PRs | Status |
|---|---|---|---|
| R1 | Dead surface: dead wire ops, platform dead code, retired settings (desktop), Android legacy compat | #274 #275 #276 #277 | Merged |
| R1 PR4 | Legacy appearance keys on iOS + Android (`keyHeightScale`, `keyFontSizeScale`, `candidateTextSizeScale`, `keyCornerRadius`, `keyBorderWidth`, `colorSettings`) — a customized look is carried into a user theme (selected when it was showing), then the keys go; the default theme reads stock values. User themes untouched | #293 | Merged |
| R2 | Parity bugfixes: (a) Android Dictionary tab toggle + TPS search, (b) Android sentence-end pre-check, (c) `en` placeholders | #278 #280 #279 | Merged |
| R2d | iOS `CandidateCellHelper.tpsFallback` on hanji-less TPS cells | R5 PR-b | Resolved in R5 PR-b: the cell shows `tlDisplayToTPS(roman)`, what the pick writes |
| R3(a) | Settings windows reach user data through engine ops; retire `DeriveCustomQueryKey` | #281 #282 | Merged |
| R3(b) | Settings-page model: PR-A presentation + launch parser, PR-B `SettingsWriter`, PR-C custom-dictionary listing state, PR-D remaining twin label / roster helpers | #284 #285 #286 #287 | Merged |
| R3(c) | Win/Linux key-intent executor + `Runtime` → `desktop-core`: PR-1 Linux characterisation tests, PR-2 core executor (+Linux), PR-3 Windows on it, PR-3b shared `DesktopRuntime`, PR-4 parity: switch re-presents the open list the same way | #288 #289 #290 #291 #292 | Merged |
| R4 | Engine user-data façade: `dispatch/src/user_data.rs` page logic → `userdata`; single-impl store traits; `cfg(not(user-data))` arms; one `CustomSearchKey` | #294 #295 | Merged |
| R5 | Engine-owned commit resolution (`CommitContinuous` returns document text + auto-space verdict, records usage) | P1 #296 · PR-a #297 · PR-c #305 · PR-b #309 #310 | Merged |
| R6 | Config normalisation in the engine (flat settings snapshot, real `input_mode = "tps"`, engine-resolved sources) | #298 #302 #303 #311 | Merged |
| R7 | Test redundancy (engine layers, platform restatements, test-local copies, shared `engine/test-support`) | R7-1 #312 · R7-2a #314 (engine seam tests) · R7-2b #315 (platform restatements) · R7-3 #313 | merged |
| R8 | Test selection: one integration binary per crate, `tools/test_select.py`, CI path filters, macOS `swift test` job | #299 #300 #301 #307 | Merged |
| R9 | Naming batch A (identifiers, files, non-iOS folders) | R9-1 #308 · R9-2 #322 (Android packages: `ime/theme`, `ime/settings`, `ime/text/{candidates,overlays,nextword}`) · R9-3a #323 (`isTranslateSwapped` → `isHanjiFirst`; persisted keys, shortcut id, i18n key frozen) · R9-3b #324 (R10-a wrapper names: `nextwordResetAll`, `nextwordSetPredictionsVisible`, `transformCandidateCase`, `searchByHanji` / `isHanji`) · R9-3c #325 (platform `hanzi` fields / locals → `hanji`; generated i18n names, iOS `additionalInfo["hanzi"]`, DB columns, backup JSON, FST prefix frozen) · R9-4a #326 (nextword `predictions_visible`, `Association{previous,previous_tl}`, `lookup_associations`, `AssociationFilter`, `SearchRow`) · R9-4b+c #327 (engine `hanzi` → `hanji` incl. test fixtures + `INVARIANT_LEX_*HANJI*` labels; SQL, `.taigi` JSON keys, `"hanzi:"` FST prefix, CSV column, `hanzi_len` frozen) · R9-4d #328 (domain `dispatch.rs` → `requests.rs`, composing tests `continuous_keys_*`) · R9-4e glossary (`docs/references/keywords.md` § Code Naming Conventions, direct to main) | **R9 complete** |
| R10 | Naming batch B (proto names; field numbers unchanged) | R10-a1 #320 (`hanji` fields, `DictionarySourceToggles`, assoc lookup out of the wire) · R10-a2 #321 (nextword `ResetAll` / `SetPredictionsVisible`, case `TransformCandidateCase`) · R10-b #330 (Effect `ClearCandidates` / `RefreshCandidates` / `ResetCandidateContext`, Req `SelectCandidate`; with R12 PR-2) · R10-b2 #331 (`CandidateMode` → `CandidateScriptKind`, field `mode` → `script_kind`; Android UI strip `CandidateMode` kept) | **R10 complete** |
| R11 | Lexicon / composing boundary (`SortKey` → `ranking`, one key-family module, visibility) | R11-1 #316 (`ranking::CandidateSortKey`, lexicon visibility) · R11-2a #317 (`phonetics::KeyFamily`) · R11-2b #318 (`KeyFamily::toneless_face`) | merged |
| R12 | `Phase::Composing` removal — ≤20-line spike first | spike GO 2026-09-30 · PR-1 #329 (engine: Continuous from the first keystroke; `EnterContinuous` / `CommitDerived` no-op; wire `ResetContinuous` → `Reset`; differential 268k lines identical) · PR-2 #330 (shells drop the second call; Req 15 / 30 / 33 and Effect 4 reserved; R10-b renames) | **R12 complete** |
| A | Docs drift (Appendix D, 57 rows) | direct to main 97da342b | Done |

Every PR runs its type's pre-gate: refactor = behaviour-freeze list (`docs/contributing/cross-platform-alignment.md` §1) + `refactor-reviewer`; bugfix = confirmed root cause; feature = reviewed plan.

## Not scheduled (USER decisions)

- **macOS over `desktop-core`** (report §5.2): taken up on 2026-10-02 as its own plan — `macos-desktop-core-roadmap.md`, with the measured inventory that replaces the "~4,000 lines" estimate.
- **Naming batch C** — persisted names (setting keys, DB columns, backup JSON, FST prefix, JNI symbol, package names): frozen per §5.6.
- **iOS top-level folder renames** (§5.8): done 2026-10-04, branch `refactor/ios-folder-renames` — the maintainer moved the synced groups in Xcode.

## Decisions recorded while running

- R3(b) PR-B: the three atomic writers (`SettingsFileStore::save`, Windows / Linux CSV export) stay separate — the Linux export creates its temp file exclusively so it follows no pre-planted symlink, and no unification keeps that.
- R3(b) PR-C: only state and rules move; which component owns the job slot differs per shell (Windows per page, Linux window-wide so an outcome survives a page rebuild) and stays there.
- R3(c) PR-4: re-presenting an open list after a switch is one core rule (`represent_list`). Windows took the Linux behaviour: a refetch with Show Candidate Window switched off takes the list down, and a switch with no list open does nothing.
- R1 PR4: the six keys were user-written until #407 removed the default-theme editor (2026-06-07), so a customized look is user content, not a stale setting. It becomes a user theme named like the editor's unnamed theme, past the five-theme cap, selected when the keyboard was showing it; a factory look is only removed.
- R5 pre-plan (2026-09-30): not a pure refactor — iOS and Android already commit differently for a hanji-less candidate. P1 (iOS, Hanji-first, non-TPS): a hanji-less cell was read as its own hanji, so it earned no auto space and Annotate in Brackets wrote `taigi (taigi)`; reproduced by `ActionHandlerUnmarkedCommitTests`, fixed toward Android and §23 / §34. P2 (TPS layout, hanji-less candidate: iOS writes a broken TPS rendering unspaced, Android writes TL spaced) is a USER decision and blocks R5 PR-b only. Order: P1 → PR-a (engine resolver + `CommitResolution`) → PR-c (macOS + desktop) → P2 → PR-b (mobile).
- USER 2026-09-30 delegated two parity choices to the recommendation ("decide for me, follow your recommendation; you may choose which platform to align with"): R5 P2 — on the TPS layout a hanji-less pick writes what its cell shows, `tlDisplayToTps(roman)`, with no auto space, on both mobiles (Android changes; this also retires the iOS R2d path). R6 P0 — with every dictionary switched off the keyboard offers no dictionary candidates, on every platform (iOS and Android align to macOS / Windows / Linux).
- R5 PR-b: only Continuous candidate taps move to the engine. A NextWord prediction tap is not a `CommitContinuous` (there is no composition to nail into), so iOS `formatOutputText` / `markedCellCommit` / `parseRomanAndHanzi` and Android `resolveUnmarkedCommit` / `resolveMarkedCellCommit` stay as the prediction resolvers, with their `UsageRecorder` (now without `hanji`: only a Continuous pick touches a learned phrase). The iOS hanji-less TPS cell moved from `tlNumericToTPS` to `tlDisplayToTPS` in the P2 commit, so the cell shows what the pick writes; the engine `TlNumericToTps` op is left with no platform caller (removed 2026-10-09, round A1b).
- R5 PR-b2: with every platform naming a script, the platform-written commit is gone — `CommitContinuous.display_text` (tag 1) is reserved, an empty `canonical_text` no longer falls back to it, and `COMMIT_SCRIPT_UNSPECIFIED` is ignored (a no-op answered `IGNORED`, as `CARET_DIRECTION_UNSPECIFIED` is a no-op) rather than read as LEAD: every sender is in this repository and names one, so an unnamed script is a bug to surface, not a pick to guess.
- R10 split: proto renames land in the proto and in the engine code that mirrors the proto name (façade fns, domain enums); platform wrapper names wait for R9-3 and internal Rust names for R9-4, so each batch touches one layer. `is_hanji_first` moves with the `isTranslateSwapped` identifiers in R9-3 (one concept, one PR). R10-a1: `AssocLookupRequest` / `Response` / `LexiconAssocEntry` were never a wire method and left the proto; `DictionaryFiltersResponse.assoc_lookup_bitmask` (tag 2, no platform reader) is reserved.
