# TaigiKeyboard — Roadmap

> **Type**: Planning (forward-looking)
> **Keywords**: `roadmap`, `planning`, `released versions`, `release trains`
> **Status**: Active
> **Last updated**: 2026-10-02 (macOS over desktop-core planned). 2026-10-01: maintainability audit follow-up complete. 2026-09-30: mobile v3.6.10 / v3.6.11 rows; bigram LM closed. 2026-09-26: Active items all merged — collapsed into Closed phases; design bodies frozen in `docs/reports/2026-09-26-shipped-roadmap-design-notes.md`

---

## Summary

- **Forward-looking work items only.** Shipped detail lives in `docs/releases/<version>/plan.md` + `changelog/mobile-<version>.md`.
- **Active**: E1 P5 (P5a edge pick → P5b list sort), § Active / In-flight items. Pending dogfood is tracked in `docs/architecture/dogfood-checklist.md`.
- **Open candidates**: every unfinished, parked or brainstorm item across the roadmaps and reports is listed once in § Open candidates (unscheduled), with a link to its design.
- **Release scope / timing / tag is the maintainer's call.**

---

## Active / In-flight items

Nothing in flight: everything scoped through 2026-10-08 has merged (see Closed phases). Pending dogfood: `docs/architecture/dogfood-checklist.md`.

---

## Released versions index

Newest first. Two trains since 2026-09: mobile `mobile-x.y.z` (iOS + Android) and desktop `desktop-x.y.z` (macOS + Windows + Linux; earlier desktop tags `macos-v*` / `windows-v*` live in the website repo). Links: release notes (`changelog/`) + detailed plan archive where one exists. Authoritative ship-date list: memory `project_released_versions.md`.

| Version | Ship date | Release notes | Detailed plan archive |
|---|---|---|---|
| mobile v3.6.11 | 2026-09-29 (`mobile-3.6.11` @ `cb410a17`) | [`changelog/mobile-v3.6.11.md`](../changelog/mobile-v3.6.11.md) | — (Android 3.6.10 upgraders get the custom dictionary + frequency back; `association.bin` v2 word pairs; collapsible theme preview) |
| mobile v3.6.10 | 2026-09-28 (`mobile-3.6.10` @ `1da2ebaa`, re-cut; first cut `ec5fe549` 2026-09-27) | [`changelog/mobile-v3.6.10.md`](../changelog/mobile-v3.6.10.md) | — (one-handed mode, photo theme background, learned phrases, typed separators, No Hyphens, user data in the engine; mobile skips v3.6.9) |
| desktop v3.6.10 | 2026-09-24 (`desktop-3.6.10` @ `f6f00b47`, Linux-only patch; macOS / Windows stay on 3.6.9) | [`changelog/desktop-v3.6.10.md`](../changelog/desktop-v3.6.10.md) | — (Fcitx5 selection keys, vertical-window arrow keys, Linux Fonts pane removed) |
| desktop v3.6.9 | 2026-09-23 (`desktop-3.6.9` @ `6523379e`, re-cut; first Linux packages uploaded 2026-09-24) | [`changelog/desktop-v3.6.9.md`](../changelog/desktop-v3.6.9.md) | [`docs/reports/2026-09-26-shipped-roadmap-design-notes.md`](reports/2026-09-26-shipped-roadmap-design-notes.md) — learned phrases, Telex tone 1 / 4, input-source menu |
| mobile v3.6.8 | 2026-09-12, re-cut 2026-09-14 (`mobile-3.6.8` @ `4022956b`, first cut `3a399505`) | [`changelog/mobile-v3.6.8.md`](../changelog/mobile-v3.6.8.md) | — (Candidate Display picker, Show Typed Text default on, POJ tone placement + `o͘ⁿ`, auto-space follows commit, Android strip lag; mobile skips v3.6.6/v3.6.7) |
| desktop v3.6.8 | 2026-09-11 (`desktop-3.6.8` @ `440171d2`, same-version overwrite ×2) | [`changelog/desktop-v3.6.8.md`](../changelog/desktop-v3.6.8.md) | [`docs/reports/desktop-3.6.x-design-notes.md`](reports/desktop-3.6.x-design-notes.md) — Telex, candidate-window toggle, symbol picker, composing caret, ⇧+slot, custom + installed fonts, Shortcuts blocks |
| desktop v3.6.7 | 2026-09-04 (macOS first 2026-09-03; assets overwritten several times through 2026-09-05; tags live in the website repo) | [`changelog/desktop-v3.6.7.md`](../changelog/desktop-v3.6.7.md) | — (first Windows release; POJ `au` tone placement; Candidate Display picker on desktop) |
| v3.6.6 | 2026-08-28 (macOS-only package; no tag in this repo) | [`changelog/desktop-v3.6.6.md`](../changelog/desktop-v3.6.6.md) | — (letter-key candidate selection, Caps Lock ABC, in-app update download; iOS KeyboardKit 10.9.0) |
| v3.6.5 | 2026-08-28 (`v3.6.5` @ `84304d82`) | [`changelog/mobile-v3.6.5.md`](../changelog/mobile-v3.6.5.md) | — (macOS first release; TPS + candidate-accuracy round; §40 NextWord contract; Android 11 floor) |
| v3.6.4 | 2026-08-07 (`v3.6.4` @ `ed38499f`) | [`changelog/mobile-v3.6.4.md`](../changelog/mobile-v3.6.4.md) | — (App UI i18n — five display languages) |
| v3.6.3 | 2026-06-20 (`v3.6.3` @ `acea9a8f`) | [`changelog/mobile-v3.6.3.md`](../changelog/mobile-v3.6.3.md) | — (TPS fixes: explicit tone, tone 9, `ir`; single-initial input; Android autocorrect / vibration / rich-editor backspace) |
| v3.6.2 | 2026-06-12 (`v3.6.2` @ `c550e100`) | [`changelog/mobile-v3.6.2.md`](../changelog/mobile-v3.6.2.md) | — (keyboard theme picker + custom theme editor; Show Romanization toggle) |
| v3.6.1 | 2026-06-06 (`v3.6.1` @ `d1259966`) | [`changelog/mobile-v3.6.1.md`](../changelog/mobile-v3.6.1.md) | — (cross-mode user-data consistency R1–R7, Hanji-with-romanization literal candidate, backup exclusion; contracts in `architecture/behavioral-invariants.md`) |
| v3.6.0 | 2026-05-31 (`b782205c`) | [`changelog/mobile-v3.6.0.md`](../changelog/mobile-v3.6.0.md) | — (kautian subcoll + dev supplement + source-toggle filtering + explicit-tone fix) |
| v3.5.9 | 2026-05-29 (`3c8bec16`) | [`changelog/mobile-v3.5.9.md`](../changelog/mobile-v3.5.9.md) | — (TPS tri-index + Tier-A/B refactor; design memo `project_v359_d_tps_triindex_plan.md`) |
| v3.5.8 | 2026-05-20 (`61df3028`) | [`changelog/mobile-v3.5.8.md`](../changelog/mobile-v3.5.8.md) | [`docs/releases/v3.5.8/plan.md`](releases/v3.5.8/plan.md) — Phase 0-9 + whole-sentence lattice + walker S1-S9 + continuous-compound-hyphen fix |
| v3.5.7 | 2026-05-08 | [`changelog/mobile-v3.5.7.md`](../changelog/mobile-v3.5.7.md) | — |
| v3.5.6 | 2026-04-27 | [`changelog/mobile-v3.5.6.md`](../changelog/mobile-v3.5.6.md) | — |
| v3.5.5 | 2026-04-12 | [`changelog/mobile-v3.5.5.md`](../changelog/mobile-v3.5.5.md) | — |
| v3.5.3 | 2026-03-22 | [`changelog/mobile-v3.5.3.md`](../changelog/mobile-v3.5.3.md) | — |
| v3.5.2 | 2026-03-08 | [`changelog/mobile-v3.5.2.md`](../changelog/mobile-v3.5.2.md) | — |
| v3.5.1 | 2026-02-25 | [`changelog/mobile-v3.5.1.md`](../changelog/mobile-v3.5.1.md) | — |
| v3.5.0 | 2026-02-12 | [`changelog/mobile-v3.5.0.md`](../changelog/mobile-v3.5.0.md) | — |
| v3.4.x | 2025-2026 | [`changelog/mobile-v3.4.*.md`](../changelog/) | — |
| v3.3.x | 2025 | [`changelog/mobile-v3.3.*.md`](../changelog/) | — |

Detailed plan archives are added retroactively only when source material exists; older versions remain release-notes-only.

---

## Open candidates (unscheduled)

Forward-looking candidates only, NOT items already shipped. None is assigned to a release; scope and timing are the maintainer's call. (v3.5.8-era items that read like candidates but shipped — `whole-sentence lattice + walker`, `continuous compound-hyphen`, `Phase 9 user-freq plumb` — live in [`docs/releases/v3.5.8/plan.md`](releases/v3.5.8/plan.md).)

### Other open items

One line each; the linked section holds the design, the measurements and the open questions. Device dogfood lives in [`architecture/dogfood-checklist.md`](architecture/dogfood-checklist.md) (every pending item marked PASS 2026-10-02).

**Project and legal (USER)**

| Item | Status | Where |
|---|---|---|
| SignPath Foundation code-signing application | rejected 2026-10-02; USER will reapply later — keep `CODE_SIGNING_POLICY.md` + `windows-build.yml` provenance build | [`go-public-checklist.md`](go-public-checklist.md), [`CODE_SIGNING_POLICY.md`](CODE_SIGNING_POLICY.md) |

**Engine and dictionary**

| Item | Status | Where |
|---|---|---|
| Retire the old dictionary `frequency` after E1 | Recorded, not started — maintainer 2026-10-08: "record it in the roadmap, handle it when there is time". Still read: the equal-cost tie-break (`CandidateSortKey` −score / −freq; ~69 % of rows are corpus-unseen and share one cost), the search-page tie-break, `association.bin` dictionary-derived counts (`associations.py`), CSV / FST row order + duplicate max-aggregation (`merge_csv.py`), the `dict.bin` u32 field, `CandidateMessage.score`. Steps: pick a replacement tie-break for unseen words, a count source for the association rows, then the format bump. Related: proto `TaigiWord.length_score` has no reader since P5b; a custom twin of a dictionary word carries no corpus cost (sorts below its priced homophone in the list) | [`reports/2026-10-08-e1-p5b-list-sort.md`](reports/2026-10-08-e1-p5b-list-sort.md) §5 |

**Not to re-propose** (USER-closed): `zh-TW` README, xcconfig signing, Android Gradle proto plugin, dictionary-source licensing follow-up, romanization spelling correction, corpus expansion P1b incl. the TAT application, a full TL / POJ / en / ja proofreading pass of the app UI (fix reported typos per incident instead), `$` sentence-start opener (its `association.bin` data is removed in its own PR), smart-suggestion techniques outside the bigram model (3, 5, 6, 9, 10, 11 incl. the Android `IME_FLAG_NO_PERSONALIZED_LEARNING` gap), Windows candidate-window paint latency and UIA exposure (USER 2026-10-02: "remove"); converted-romanization commit on Enter (`suann2ting3` → `suán-tìng`), TL mode (臺羅模式) tone key commits without a candidate window, and mobile predictions after Space (USER 2026-10-02: "remove"); bigram P6 walker term / P7 hanji-only sources / D7 two-word context; e2e drivers for macOS, Windows, Android, iOS; invariant-label PR2; naming batch C (persisted names stay frozen); shared Swift package for macOS + iOS (open-source round 10, USER 2026-10-04: close — see § Closed phases).

## Per-round gates (process invariants, project-wide)

Apply to every change regardless of release:

- Cross-platform parity-correction rounds merge both platforms in lockstep.
- iOS `pbxproj` is user-only (`docs/contributing/ios-guidelines.md`); Android Gradle is editable.
- Engine slices require S0 golden-diff EMPTY acceptance.

---

## Closed phases / shipped audits

- **E1 unified word frequency** — COMPLETE 2026-10-08: P0–P4 (#458, #460, #461, #462, #463, #465) put the walker's segmentation on one corpus probability (`dictionary.bin` v4 `walker_cost`, α 10); P5a (#466) picks each edge's word by it; P5b (#468) sorts the candidate list and the dictionary search page by it (engine-owned search order; platform re-sorts deleted). Dogfood S116–S119; `frequency` retirement → § Open candidates. Design: [`architecture/unified-word-frequency-roadmap.md`](architecture/unified-word-frequency-roadmap.md).
- **Desktop TPS — Hanji conversion in the preedit** (arm B of U8) — COMPLETE 2026-10-05: H-P1–H-P5 MERGED (#398, #401, #402, #403, #404). Under TPS the preedit shows the predicted Hanji and ↓ opens the candidates of the word at the caret; TL and POJ unchanged. Invariant §59, dogfood S94 / S95. Design: [`architecture/desktop-tps-hanji-conversion-roadmap.md`](architecture/desktop-tps-hanji-conversion-roadmap.md).
- **iOS top-level folder renames** — branch `refactor/ios-folder-renames` (2026-10-04): `Composition/` → `ServiceGraph/`, `Autocomplete/` → `Candidates/`, theme files `Settings/` → `Theme/`, `DebugLogger.swift` → `Logging/`; the maintainer re-pointed the synced groups in Xcode. Source: [`reports/2026-09-30-audit-all.md`](reports/2026-09-30-audit-all.md) Appendix F.
- **Fedora × IBus e2e** — branch `fix/fedora-ibus-e2e`: the nightly matrix runs all eight cells again. Two stacked causes: Fedora's rpm writes a system registry cache that ibus-daemon's default `--cache auto` takes before reading `IBUS_COMPONENT_PATH` (the driver now passes `--cache none`), and the engine's socket-file lookup skipped an empty `/etc/machine-id` where libibus keeps it (`taigikeyboard-ibus` `bus.rs` now mirrors `ibus_get_local_machine_id`). Detail: [`architecture/e2e-testing-roadmap.md`](architecture/e2e-testing-roadmap.md) § PR table PR4.
- **Residual platform twins** — branch `refactor/residual-platform-twins` (audit 2026-09-30 Appendix B, callers re-verified 2026-10-04): the engine now owns the auto-space attaching set (`IsAttachingPunctuation`; the iOS / Android / macOS `AutoSpacePunctuation` and desktop-core `ATTACHING` copies are gone, macOS asks the engine directly), the external-lookup digit-tone fold (`ExternalLookupDigitForm`; URL assembly stays per platform; `StripTone` / `NfdPreprocessForLookup` retired) and each search row's sources (`TaigiWord.sources`; `source_bitmask` reserved, `LexiconBitmask` ×2 + desktop `SOURCE_BITS` gone). `SuggestionCaseTransformer` ×2 and the `TransformCandidateCase` op were deleted. Parity-corrections: mobile custom rows `ho͘2` → `hoo2` (+ NFC, ASCII tone digits) in lookup URLs; Android's second NextWord prediction is no longer ALL CAPS under Caps Lock. Found, not changed: mobile `shouldAppendAutoSpace`, macOS `FullWidthPunctuation` vs desktop-core `full_width.rs`, Android lookup URLs encode a space as `+` (iOS / desktop `%20`).
- **Shared Swift package for macOS + iOS (open-source round 10)** — CLOSED 2026-10-04 by the maintainer, not built; the premise did not survive macOS over desktop-core. Inventory of the 19 same-named Swift files under `ios/Sources` and `macos/Sources`: the 7 `*.pb.swift` are byte-identical generated code that `checks.yml` already diffs; `DisplayLanguage` shares about 80 lines (iOS adds `resolution` for its `.lproj` bundles); every other twin genuinely diverged — `RustEngineBridge` (iOS diagnostics ring + DEBUG assert vs macOS response-id check, release log off), `RustEngineBridge+UserData` and `UserDataClient` (iOS async + typed responses vs macOS sync + paging), `EngineSettings` (iOS protocol vs macOS struct + mode enums), `DebugLogger` (subsystem, trace prefix), `StringResolver` / `DisplayLanguageStore` (`.lproj` + App Group vs generated maps + `SettingsStore`), `StringKey` / `StringResolverFormats` (per-platform i18n key scope). `RustVec+UInt8` and the bridge are also tied to each platform's own Rust archive and swift-bridge glue. macOS references only `envelope` + `user_data` proto types; the other five compile because `envelope.proto` imports them. `AutoSpacePunctuation` stays with "Residual platform twins" above.
- **Stale `.proto` comments** — #390: `lexicon.proto` documents `DEV` as toggleable; `composing.proto` drops the nonexistent `continuous-input-ranking.md` §10.11 cite and describes the iOS commit through `HostTextWriter`. Swift (iOS + macOS) and Java bindings regenerated, comments only.
- **macOS Custom Dictionary confirmation** — #386 MERGED: Delete All Custom Words and Delete Learning Records ask first, as on Windows and Linux (inventory S11). The other three S11 divergences (filter trim, reload after a failed write, off-page selection) are listed in the PR, not changed.
- **macOS over desktop-core** — COMPLETE 2026-10-03: P1–P15 MERGED (#342–#365); the macOS key path is `taigi-desktop-core` over `taigi-macos-ffi` (candidate-window geometry, the settings backend, global shortcuts and the update flow stay Swift). Design + PR table: [`architecture/macos-desktop-core-roadmap.md`](architecture/macos-desktop-core-roadmap.md); inventory [`reports/2026-10-02-macos-desktop-core-inventory.md`](reports/2026-10-02-macos-desktop-core-inventory.md).
- **Desktop TPS mode** (方音符號 on macOS, Windows and Linux) — COMPLETE 2026-10-03: P1–P6 MERGED (#367, #368, #372, #373, #374, #376, #378, #381; Dachen layout + Shift layer, Switch Phonetic Symbols, candidate window on demand, on-screen key panel with clicks). Dogfood S89–S93. Design: [`architecture/desktop-tps-roadmap.md`](architecture/desktop-tps-roadmap.md).
- **Learning Records page** — COMPLETE 2026-10-03: P0 `5bab5b9b`; P1 #366, P2 #369, P3+P4 #370, P5+P6 #371 MERGED (view, correct the count of, and delete one learned row on all five platforms). Design: [`architecture/learning-records-page-roadmap.md`](architecture/learning-records-page-roadmap.md).
- **iOS #352 Flutter host commit** — #375 `6c9c2465` MERGED 2026-10-03 (one host write per event; draft #357 closed); device dogfood PASS 2026-10-04. Contract: `behavioral-invariants.md` `INVARIANT_composing_host_commit_one_write_per_event`.

- **Maintainability audit follow-up** — COMPLETE 2026-10-01: R1–R12 + docs drift MERGED (#274–#331). PR table: [`architecture/maintainability-roadmap.md`](architecture/maintainability-roadmap.md); source audit: [`reports/2026-09-30-audit-all.md`](reports/2026-09-30-audit-all.md).
- **Bigram language model** — CLOSED 2026-09-30 (USER, after the Android dogfood): P0–P5 MERGED (#267, #268, #270–#272) + the punctuation-context fix #273; P6 not opened, P7 not adopted. `association.bin` v2 word keys shipped in mobile v3.6.11. Design + status: [`architecture/bigram-lm-roadmap.md`](architecture/bigram-lm-roadmap.md).
- **User data in the engine** — P0–P9d MERGED 2026-09-26 (#219–#237): the four user-data SQLite stores are engine-owned (`engine/userdata`). Design + PR table: [`architecture/user-data-engine-roadmap.md`](architecture/user-data-engine-roadmap.md). Device dogfood pending (iOS first look OK).
- **Mobile custom-theme color roles** — P0–P7 MERGED 2026-09-27 (#252, #255, #257, #258; P3 dropped, premise false). Tiers + rollout rules: [`ui/theme.md`](ui/theme.md) § Custom Theme Color Roles.
- **Identical desktop menus; Linux update check added then removed** — phases 1–5 MERGED 2026-09-25 (#175–#178, site #20); Linux half reversed the same day (#193, no update check on Linux). S77. Design: [`reports/2026-09-26-shipped-roadmap-design-notes.md`](reports/2026-09-26-shipped-roadmap-design-notes.md) § Linux update check.
- **Learned phrases** — #109–#113 MERGED 2026-09-20; own store PR-A–D #125–#128 MERGED 2026-09-21; store engine-owned since user-data P3c. S62. Design: [`reports/2026-09-26-shipped-roadmap-design-notes.md`](reports/2026-09-26-shipped-roadmap-design-notes.md) § Learned phrases.
- **Mobile custom theme — one background surface, gradient direction, photo background** — A–D #90–#93 MERGED 2026-09-20 (+ follow-up E). S54 / S55. Design: [`reports/2026-09-26-shipped-roadmap-design-notes.md`](reports/2026-09-26-shipped-roadmap-design-notes.md) § Mobile custom theme; current state [`ui/theme.md`](ui/theme.md).
- **Desktop Telex keys for tone 1 / 4** — #98 `17850f17` MERGED 2026-09-19. S57. Design: [`reports/2026-09-26-shipped-roadmap-design-notes.md`](reports/2026-09-26-shipped-roadmap-design-notes.md) § Desktop Telex keys.
- **Desktop input-source menu — global shortcut rows** — #100 `f763d8bf` MERGED 2026-09-20. S59. Notes: [`reports/2026-09-26-shipped-roadmap-design-notes.md`](reports/2026-09-26-shipped-roadmap-design-notes.md) § Desktop input-source menu.
- **Desktop 3.6.8 items** — custom fonts #16, installed typefaces #45 (S42 / S43 PASS), Telex keys + candidate-window toggle #17–#22, symbol picker #26–#28, composing caret #29–#31, ⇧ + slot key #35, Shortcuts pane #36; shipped in desktop v3.6.8. Dogfood S30–S38. Design: [`reports/desktop-3.6.x-design-notes.md`](reports/desktop-3.6.x-design-notes.md).
- **kautian subcollections** (accent + Surname Appendix toggles + pronunciation-difference word-level extension) — 5 phases MERGED, shipped **v3.6.0** (#354-#358).
- **v3.6.1 user-data key consistency across input modes** — CLOSED / shipped: rounds R1–R7 MERGED 2026-06-03/04 (#382–#388; association recall + canonical-TL commit + custom words cross-mode + Android cap parity + `(hanji, tl)` frequency key + SQLite hygiene + backup exclusion). Dogfood items S11–S16. Triple index kept. The user-data stores later moved into the engine (see User data in the engine above); the canonical-TL key contract lives in `architecture/behavioral-invariants.md`.
- **App UI i18n — multi-language** (Hanji / English / Japanese / Tâi-lô / Pe̍h-ōe-jī + Automatic) — SHIPPED; all five display languages in the production picker. No standing proofreading pass (USER 2026-10-02): reported typos are fixed per incident. Outcome: `docs/contributing/i18n.md`, `behavioral-invariants.md` §37–39, `system-overview.md` §3 (`make i18n`).
- **Keyboard theme picker** (swipe gallery + custom theme) — SHIPPED v3.6.2: iOS #400-411, Android port #412-#418. Current-state reference: [`docs/ui/theme.md`](ui/theme.md).
- **Android UI modernization** (Compose M3 chrome/overlay) — DONE 2026-05-30 (#362 / #364 / #365). 3 leaf overlays (Symbol/Layout/Candidate) View→Compose M3 over `KeyboardChromeColors`; keys stay custom-draw; `InputView`/window kept View (IME-dismiss bug zone). Memory `project_android_compose_modernization.md`.
- **v3.5.9 D = TPS tri-index** — SHIPPED, tagged `3c8bec16` 2026-05-29. `tps:` FST family parallel to `tl:` / `poj:`; mode-axis (Input + Key + FST) now three-layer symmetric. Retired `is_tps` short-circuit (`dispatch.rs`/`continuous.rs`), `tps_or_mapped_to_er` runtime branch (`search.rs`), `tps_to_tl` canonicalize chain (`classification.rs`). 6 PR (#334-#340, C-0/C-1/C-3a/C-3b/C-4/C-5).
- Roadmap Item 1 (Project Structure & File Naming Cleanup) — CLOSED 2026-05-06 (#212-#215).
- Roadmap Item 4 (Android UI Compose migration) — CLOSED 2026-05-08 (#227-#231).
- v3.5.8 continuous input — SHIPPED 2026-05-20 (`61df3028`). See [`docs/releases/v3.5.8/plan.md`](releases/v3.5.8/plan.md) for full plan + Phase status + design rationale + dogfood matrix.

<!-- New active items go in Active / In-flight items. New deferred items go in Out of scope / deferred. Shipped versions get a row in Released versions index + an entry in Closed phases. -->
