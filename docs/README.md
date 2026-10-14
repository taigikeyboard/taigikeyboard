# TaigiKeyboard documentation

Start here:

- Build and test: [`BUILDING.md`](BUILDING.md) · install for users: [`INSTALL.md`](INSTALL.md)
- Architecture: [`architecture/system-overview.md`](architecture/system-overview.md)
- Style guides and per-platform rules: [`contributing/README.md`](contributing/README.md)
- Open work: [`roadmap.md`](roadmap.md) · release history: [`../CHANGELOG.md`](../CHANGELOG.md)

Status: **Active** = living contract, kept in sync with the code · **Reference** = design record or research, the code wins where they differ · **Done** = finished plan kept for the design decisions code cites · **Historical** = frozen dated snapshot.

## `engine/` — engine behaviour

| File | Description | Status |
|------|-------------|--------|
| `engine/composing.md` | Composing state machine (`Phase::Idle` / `Continuous`, intents, effects) — `engine/composing` | Active |
| `engine/continuous-input-ranking.md` | Continuous candidate pipeline, sort key and user weight (§3.2), walker cost | Active |
| `engine/continuous-commit-and-display.md` | Continuous commit / display contract (Model B, §10) | Active |
| `engine/continuous-candidate-display.md` | Candidate carrier (`roman` + optional `hanji`) and the §15 rules code cites | Active |
| `engine/syllabifier.md` | Syllable parsing, `SyllableInventory` (`syllables.fst`), lattice use | Active |
| `engine/tone.md` | Tone marks, tone numbers, nasal marker — `engine/phonetics` | Active |
| `engine/tps.md` | TPS (Taiwanese Phonetic Symbols) — `engine/phonetics::tps` | Active |
| `engine/nextword.md` | Next-word association lookup and scoring — `engine/nextword` | Active |
| `engine/custom-dictionary.md` | Custom dictionary, CSV and `.taigi` backup — `engine/userdata` | Active |
| `engine/binary-format.md` | `dictionary.fst`, `dictionary.bin`, `association.bin`, `syllables.fst` formats | Active |
| `engine/rust-core-proto.md` | Proto envelope and per-slice request / response map | Active |
| `engine/diagnostics.md` | Device info collected for bug reports | Active |
| `engine/migration-inventory.csv` | Every shipped engine item and each native keep / won't-migrate row | Active |

## `architecture/` — contracts, release, plans

| File | Description | Status |
|------|-------------|--------|
| `architecture/system-overview.md` | Five-platform architecture: system context, crate graph, build pipeline, keystroke flow | Active |
| `architecture/behavioral-invariants.md` | Cross-platform behaviour contract (`§N`, `INVARIANT_*` labels) | Active |
| `architecture/dogfood-checklist.md` | Real-device acceptance items `Sn` — read before a dogfood pass | Active |
| `architecture/composing-state-boundary.md` | Composing engine ↔ platform binding (§2.2 effects, §11 Android `InputConnection`) | Active |
| `architecture/nextword-engine-boundary.md` | Next-word engine ↔ platform binding (§13 Android) | Active |
| `architecture/keyboard-body-invariants-android.md` | Android keyboard-body geometry and touch-target invariants | Active |
| `architecture/data-artifacts-portability.md` | Binary artifacts and SQLite stores: portability contract, decision register | Active |
| `architecture/build-artifacts.md` | What is committed vs generated, and why; timings | Active |
| `architecture/e2e-trace-schema.md` | Test-build-only JSONL trace contract | Active |
| `architecture/desktop-release.md` | Desktop release flow: stage, test, publish, announce | Active |
| `architecture/macos-release.md` | macOS signing, notarization, package | Active |
| `architecture/windows-release.md` | Windows installer, signing status, hosted build | Active |
| `architecture/linux-release.md` | Linux packages and how they are built and attached | Active |
| `architecture/manual-release-notes.md` | Release trains, version numbers, store notes | Active |
| `architecture/incident-log.md` | Incidents behind the rules in `contributing/known-pitfalls.md` | Reference |
| `architecture/pr-number-migration.md` | Resolving `#NNN` written before 2026-09-07 | Reference |
| `architecture/maintainability-roadmap-2026-10.md` | 2026-10-09 audit follow-up — H, A1a, A1b, A2 merged; A3 next | Active |
| `architecture/custom-layouts-roadmap.md` | Mobile custom romanization layouts — iOS (phase 1) in progress, Android next | Active |
| `architecture/e2e-testing-roadmap.md` | End-to-end test system — Linux automated, other platforms manual | Active |
| `architecture/macos-roadmap.md` | macOS IME design decisions D1–D11 | Done |
| `architecture/macos-desktop-core-roadmap.md` | macOS key path on the shared `taigi-desktop-core` — D1–D9, parity decisions | Done |
| `architecture/windows-roadmap.md` | Windows TSF IME design W1–W17 | Done |
| `architecture/linux-roadmap.md` | Linux Fcitx5 + IBus design L1–L13, named divergences | Done |
| `architecture/desktop-tps-roadmap.md` | Desktop TPS input mode — decisions U2–U9, design D0–D7 | Done |
| `architecture/desktop-tps-hanji-conversion-roadmap.md` | Desktop TPS Hanji conversion — H1–H9, guard rails | Done |
| `architecture/learning-records-page-roadmap.md` | Learning Records page on all five platforms | Done |
| `architecture/user-data-engine-roadmap.md` | User-data stores moved into `engine/userdata` — U1–U11 | Done |
| `architecture/unified-word-frequency-roadmap.md` | E1 unified word frequency for the walker | Done |
| `architecture/bigram-lm-roadmap.md` | Cross-word bigram model — D1–D7, not-adopted list | Done |

## `ui/` — app and keyboard UI

| File | Description | Status |
|------|-------------|--------|
| `ui/app-ui.md` | App tabs, Home content, Dictionary screens | Active |
| `ui/layout.md` | Keyboard layouts per platform, device size classes | Active |
| `ui/case.md` | Shift, Caps Lock, case transform | Active |
| `ui/theme.md` | Themes, fonts, custom theme editor | Active |
| `ui/emoji.md` | Emoji keyboard (iOS / Android) over `emoji/` data | Active |
| `ui/callouts.md` | Long-press callouts and tone-variation menus | Active |

## `references/` — other IMEs

| File | Description | Status |
|------|-------------|--------|
| `references/mainstream-ime-comparison.md` | IME comparison index — entry point for best-practice citations | Reference |
| `references/chiakey-reference.md` | ChiaKey | Reference |
| `references/khiin-reference.md` | khiin-rs | Reference |
| `references/moe-taigi-reference.md` | MOE Taigi IME (segmentation, nail mechanism) | Reference |
| `references/rime-reference.md` | librime | Reference |
| `references/keywords.md` | Standard names for core logic and UI concepts | Reference |

## `reports/` and `releases/` — frozen snapshots

A report stays while code cites it, it holds rationale found nowhere else, or its research is still open; otherwise it is deleted (git keeps the text).

| File | Description |
|------|-------------|
| `reports/desktop-3.6.x-design-notes.md` | Design record: installed typefaces, Telex, symbol picker, composing caret, ⇧+slot, Shortcuts pane, custom fonts |
| `reports/2026-09-28-bigram-corpus-spike.md` | Bigram LM P1 spike: corpus alignment and `association.bin` v2 sizing |
| `reports/2026-09-30-audit-all.md` | Maintainability audit (§3 = layers not to re-flag) |
| `reports/2026-10-02-macos-desktop-core-inventory.md` | macOS ↔ `desktop/` duplication inventory behind the desktop-core roadmap |
| `reports/2026-10-08-e1-walker-baseline-corrections.md` | E1 walker gold-set measurements |
| `reports/2026-10-08-e1-p2b-subsegment-tiebreak.md` | E1 P2b — out-of-vocabulary corpus word splitting |
| `reports/2026-10-08-e1-p3-walker-cost.md` | E1 P3 — walker prices edges on `walker_cost` |
| `reports/2026-10-08-e1-p4-calibration.md` | E1 P4 — walker calibration |
| `reports/2026-10-08-e1-p5a-edge-pick.md` | E1 P5a — edge word pick by `walker_cost` |
| `reports/2026-10-08-e1-p5b-list-sort.md` | E1 P5b — list sort by `walker_cost`; `frequency` retirement open |
| `reports/2026-10-09-audit-all.md` | Maintainability audit behind `architecture/maintainability-roadmap-2026-10.md` |
| `reports/2026-10-10-sim-real-text-tps-windows-android.md` | Real-text TPS typing simulation (Windows, Android) |
| `releases/v3.5.8/plan.md` | v3.5.8 continuous-input plan (code still cites its phases) |

## Other

| Path | Description |
|------|-------------|
| `contributing/` | Per-platform guides and cross-cutting rules; index in `contributing/README.md` |
| `phonetics/` | Authoritative TL / POJ / TPS reference and TPS auto-correct rules |
| `CODE_SIGNING_POLICY.md` | Code-signing policy for released binaries |
