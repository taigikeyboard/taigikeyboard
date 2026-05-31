## v3.6.0

Headline release: **教典 (kautian) subcollection toggles** — turn individual 腔調 (accent) readings and the 姓名 (name) appendix on or off — plus **詞庫增補檔案 (dev supplement) as a toggleable dictionary source** (~2.5k curated words), **keyboard candidates now honour the dictionary source toggles**, and a **continuous-input explicit-tone correctness fix**. Android also lands a Compose Material 3 modernization of the keyboard chrome overlays. Dictionary grows +9447 / −14 entries vs v3.5.9.

### Shared (iOS + Android)

#### New Features

- **教典 (kautian) subcollection toggles.** The MOE 教育部臺灣台語常用詞辭典 source splits into independently gateable subcollections: 10 腔調 (accent) variants + the 姓名附錄 (name appendix). Each is a nested toggle under the kautian master in the dictionary settings; all default ON (no behaviour change on upgrade). Turning a subcollection off removes only its rows — a word that also belongs to another enabled source stays visible. (#354–#357)
- **語音差異 word-level accent variants.** Multi-character kautian headwords now generate per-accent readings by substituting each syllable with that character's dialect reading (same-morpheme gated), not just single characters. Generated variants are equal-ranked with the base reading; the per-accent toggles manage how many surface. (#358)
- **詞庫增補檔案 (dev supplement) dictionary source.** A new user-toggleable source under 補充資料 (default ON), seeded with six curated reference sets — 一府五院 / 內政部菜市仔名 / 台臺 / 教典僻智識 / 數字時間日期 / 行政區 (2477 net-new words). The toggle title links to the source repository; a 建中整理、提供 credit is added to the 致謝 page. (#368 / #369)
- **Keyboard candidates honour the dictionary source toggles.** Continuous-input candidates on the keyboard now apply the same 12 source toggles + kautian subcollection toggles that the dictionary browse tab already respected. Previously the keyboard always queried all sources regardless of settings. (#359)

#### Bug Fixes

- **Continuous input honours an explicit numeric tone.** Typing a fully-toned syllable like `tai5` now surfaces only tone-5 readings instead of every tone. Toneless input (`tai`) still surfaces all tones — the no-tone affordance is preserved. Applies to TL / POJ; TPS (Bopomofo tone marks) and mixed multi-syllable input stay toneless. (#367)

#### Changes

- **Updated dictionary data** — +9447 / −14 (hanzi, tl) entries vs v3.5.9, driven by the kautian accent variants and the dev supplement.

### iOS

#### New Features

- Dictionary settings (Tab 3): nested kautian subcollection toggles (10 腔調 + 姓名附錄) under the MOE master, plus the 補充資料 → 詞庫增補檔案 source row.
- 致謝 page: 建中整理、提供 section.

### Android

#### New Features

- Dictionary settings: nested kautian subcollection toggles + the 詞庫增補檔案 source row, mirroring iOS.
- 致謝 page: 建中整理、提供 entry.

#### Bug Fixes

- **Keyboard body no longer cold-opens collapsed to the candidate bar.** On a cold open the keyboard occasionally rendered only the smartbar (~158px) with the keys missing; reopening self-healed. Layout + appearance are now published synchronously before the Compose body mounts, so the first composition reads populated state. (#363)

#### Changes

- **Toggle switches use the native Material 3 style.** The four Switch call sites drop the iOS-green emulation and restore the M3 default (checked track = primary, 48dp min touch target). Intentional cross-platform divergence — iOS keeps its native green toggle. (#360)

#### Refactoring

- **Keyboard chrome overlays migrated View → Compose Material 3.** The symbol (#362), layout (#364), and candidate (#365) selection overlays are now thin `ComposeView` hosts rendering M3 content over a shared `KeyboardChromeColors` seam (user-customizable keyboard colours, not the M3 brand palette). The candidate overlay's `RecyclerView` + adapter became a `LazyColumn` over a pure `CandidateRowLayout` row-packer. Keys stay custom-drawn; the root InputView/window stays a View. (#362 / #364 / #365 / #366)
- Three deprecated UI APIs replaced with current equivalents: `Icons.Outlined.MenuBook` → `Icons.AutoMirrored.Outlined.MenuBook`, `DisplayMetrics.scaledDensity` → `TypedValue.applyDimension`, `EmojiCategory.values()` → `.entries`. (#361)

### Engine (Rust shared core)

- **`dictionary.bin` v2 → v3.** Each record gains a `u16 kautian_subtag` (bit0 = main, bits1–10 = accent[10], bit11 = name) so the engine can filter by subcollection. `SUPPORTED_VERSION` 2 → 3; v1/v2 binaries loud-reject with a rebuild marker. (#355)
- **Subcollection filter.** `enabled_sources_bitmask` high region carries the user's subcollection enable mask (bit13 = active sentinel, bits14–25 = 12-bit enable layout mirroring the subtag); `effective_source_bitmask` is the single chokepoint, dropping only the kautian source bit when all of a row's subcollections are off (DD6 — multi-source rows survive via their other source). (#355 / #356)
- **Toggle → wire encode.** `compute_filters` emits the kautian subcollection wire bits when the master is on and the sub-message is present; message presence is the active sentinel (absent ⇒ legacy all-on). (#356)
- **`shadow::fst_body_for_span`** — continuous key-build now keeps verbatim digits for a fully-toned TL/POJ span so `lookup_exact` / `lookup_prefix` hit the toned `tl:<tl_num>` / `poj:<poj_num>` FST family and filter to the typed tone; applied at all three continuous key-build sites (span-local, Step 4b partial, walker edge). (#367)
- **`enabled_sources_bitmask` (FetchAtPos field 5)** threaded from dispatch through `assemble_candidates` into the walker; `best_candidate_for_key` no longer hardcodes `u32::MAX`, closing a leak that could re-surface a toggled-off source at slot 0. Absent (0) ⇒ `u32::MAX` sentinel for legacy callers. (#359)
- Generated proto + xcframework + jniLibs artifacts regenerated via `make build`.

### Dictionary

- **kautian subcollection provenance.** Three new `kautian.csv` columns — `kautian_accent_mask` (10-bit dialect set), `kautian_name`, `kautian_main` — built from the raw sheets at `select` (before the dialect melt collapses 腔調 identity), applied at a new `kautian_provenance` stage, UNION-merged across cross-source dedup. (#354)
- **`kautian_accent_wordgen` stage** (after `frequency`, before `poj`) generates the word-level accent variants from the 語音差異 single-char sheet; same-morpheme gate (DD9), conservative 漢字數 == 音節數 skip (DD4), accent-bitset OR (DD1), equal-rank (DD4b), in-stage collision merge. (#358)
- **POJ integrity gate.** `verify_poj_integrity.py` replaces the verbose audit pass — a fatal gate that halts the build when `poj` / derived columns diverge from `convert_tl_to_poj(tl)`, ~18× faster via per-distinct-tl Node IPC dedup (identical 0/0/19 result). (#352)
- **Version diff vs the previous release tag.** `version_snapshot.py` no longer stores per-release `.tsv` snapshots; it set-diffs (hanzi, tl) pairs against the previous release tag's `dictionary.csv` read via `git show <tag>:…` (git already retains every tagged csv). Report-only never halts the build; release mode warns loudly when no diff base resolves. (#352 / #353)
- Regenerated via `make dict` at release.

### Build / Tooling

- `dictionary/.gitignore` added; `merge_csv.py` writes `output/.build_stats.json` drop counts.

### Documentation

- `docs/architecture/behavioral-invariants.md` §17 `INVARIANT_CONTINUOUS_EXPLICIT_TONE_FILTER` — pins the explicit-tone contract the #367 bug violated.
- `docs/architecture/keyboard-body-invariants-android.md` — `INVARIANT_keyboard_body_layout_published_before_compose_mount` (#363).
- `docs/engine/binary-format.md` — dictionary.bin v3 subtag layout + dev bit always-on → user-toggleable.
- `docs/references/mainstream-ime-comparison.md` — indexed four new reference IMEs (mozc / rakukan / PIME / MacishType).
- `CLAUDE.md` Core Principle #7 — Taiwanese word identity = (漢字, 羅馬字) pair.

### Removed

- Android: `CandidateOverlayAdapter.kt` + six overlay XML layouts (`candidate_overlay{,_row}.xml`, `candidate_grid_cell.xml`, `candidate_long_cell.xml`, `layout_selection_overlay.xml`, `symbol_selection_overlay.xml`) — superseded by the Compose overlays. (#362 / #364 / #365 / #366)
- Dictionary: `build/audit.py` + `audit_report.txt` + 14 `audit/*.csv` — replaced by the focused POJ-integrity gate. (#352)
- Engine: `engine/lexicon/tests/dictionary_reader_v2.rs` — replaced by `dictionary_reader_v3.rs`. (#355)

### New Files

- Android: `ime/text/smartbar/{SymbolOverlayContent,LayoutOverlayContent,CandidateOverlayContent,CandidateRowLayout,KeyboardChromeColors}.kt`, proto `KautianSubcollToggles.java`, tests `KautianSubcollWireEncodeTest.kt` / `CandidateRowLayoutTest.kt`.
- Dictionary: `build/{verify_poj_integrity,version_snapshot}.py`, `common/{kautian_accent_wordgen,kautian_provenance}.py` (+ matching `common/stages/*`), tests `test_dictionary_bin_v3.py` / `test_kautian_accent_wordgen.py` / `test_kautian_provenance.py`, `dictionary/.gitignore`.
- Engine: `engine/composing/tests/continuous_explicit_tone.rs`, `engine/lexicon/tests/dictionary_reader_v3.rs`.
