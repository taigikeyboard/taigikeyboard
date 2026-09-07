# Taigi Keyboard — Roadmap

> **Type**: Planning (forward-looking)
> **Keywords**: `roadmap`, `planning`, `released versions`, `deferred items`
> **Status**: Active
> **Last updated**: 2026-09-08 (added the desktop custom-font phases)

---

## Summary

- **Forward-looking work items only.** Shipped detail lives in `docs/releases/<version>/plan.md` + `changelog/<version>.md` + Claude auto-memory.
- **Active**: none — kautian subcollections shipped v3.6.0.
- **Deferred TODO (1)**: keyboard theme picker. All other prior candidates closed 2026-06-01 (USER).
- **Release scope / timing / tag is user-gated** per [`~/.claude/rules/diagnosis-discipline.md` § No unilateral release scope].

---

## Active / In-flight items

### v3.6.1 — 使用者資料跨輸入模式 key 一致性 (USER-scoped 2026-06-03)

**Status**: Analysis done, NO code. USER 將嚴格 review 多次後決定修復方向。**Full audit**: [`docs/reports/2026-06-03-user-data-cross-mode-audit.md`](reports/2026-06-03-user-data-cross-mode-audit.md).

USER 回報詞關聯紀錄跨版本失效 (連續輸入同漢字詞之前關聯詞跑不出來)。稽核四項使用者資料功能的跨 mode (tl/poj/tps) 共享 + 單→三索引相容性:

- 🔴 **詞關聯** `user_association.db` — 真 bug。`prev_tl` 當 hard filter 但羅馬字形式因路徑/版本而異 (一般 commit=canonical TL `tâi-gí` / 連續輸入=raw slice `taigi`),exact-match miss。修向:prev_tl 降 ranking 訊號 (Codex 排 C>A>B)。**非三索引引起** (prev_tl filter 自 v3.4.7)。
- 🔴 **自訂詞庫** `custom_dictionary.db` — 真 cross-mode 缺口。單一 raw roman + mode-blind SQLite query,TL/POJ↔TPS 硬 miss。三索引未碰自訂詞路徑,但造成系統字典 tri-index / 自訂詞 single-index 不對稱。
- ✅ **詞頻** `user_frequency.db` — 已跨 mode 共享 (key=漢字優先),三索引未碰。僅 hanji-absent 自訂 TPS 詞一 sliver。
- 🟢 **備份復原** `.taigi` JSON — 相容,restore re-derive 自動 re-key。不需修 (依賴上面修好)。

**三索引價值評估**: 四 bug 無一由三索引造成,回退修不好任何一個。真橫切問題 = 使用者資料層 key 含 mode-dependent surface form (違反 `2026-05-20-triple-index-eval.md` §硬約束 #2「user-history key = candidate identity」)。**v3.6.1 主題 = 補齊該契約到關聯,不動三索引。** 三索引 (系統字典三軸搜尋 + 架構去耦) 價值獨立成立。

**DB 生命週期 + 最佳實踐 (報告 §7,Codex 驗證)**:
- **升級安全**: 現役 v4 用戶升 v3.6.1 安全 (前提推薦修法純查詢、不 bump schema)。⚠ pre-v3 休眠用戶仍會被既有 `user_association` DROP 清空關聯 (歷史遺留)。**新 cleanup migration 必須 UPDATE/DELETE,絕不可 DROP。**
- **膨脹**: 詞頻~22k / 關聯~55k bounded ✅;**Android 自訂詞無上限** (iOS 30k throw) = parity gap,user-authored 不可 LRU 驅逐 → 對齊用 hard cap + grandfather + 擋新增。
- **冗餘清理**: 關聯 dead-row (prev_tl 非 key 卻當 read filter + 被覆寫) → **推薦修法直接消滅整個 dead-row class,無需 data migration,絕不可 DELETE dead rows (relaxed query 下已非 dead)**。orphan 清理 + VACUUM (需 2x 暫存,非啟動路徑) = maintenance 後輪。
- **最佳實踐**: ✅ parameterized SQL / index / batch txn;缺 VACUUM/integrity/optimize、orphan 清理;record path 吞錯;多處 iOS/Android divergence。Codex 補: 詞頻 key=displayText 多音字合併污染 (違 #7,非 hotfix)、備份隱私政策、多進程長 txn、遷移執行緒。

**USER 拍板 2026-06-03**: v3.6.1 **一併處理全部**;well-planned PRs,context 間清除可獨立 debug;大部分 auto mode (依 Claude+Codex 建議);commit to main。**7 個實作 round (R1-R7)** + R0 admin (docs)。詳見報告 §8。

| Round | Scope | 風險 | auto? |
|---|---|---|---|
| R1 🔴 | 詞關聯 recall + dedup:查詢 `WHERE prev_word=?` + prev_tl→ORDER BY ranking (rank-before-truncate, overfetch) + `filter.rs` **toneless-collapse** (toneless→toned 同漢字;2 guardrail:分隔符+聲調不敏感 復用 `roman_reading_eq` / 不做歧義一對多)。無 migration | 低 | ✅+Codex |
| R2 | 連續輸入 commit 帶 canonical TL (寫層根治 next_tl fragmentation;**proto triple-touch** `CommitContinuous +association_tl` + 候選帶 TL + emit 僅 NextWord 不進 lattice);R5 foundational | 中 | ⚠ Codex fork 先 |
| R3 | 自訂詞 cross-mode (設計 fork→Codex;**偏好寫入多家族鍵**,查詢端 canonicalize 風險破壞 lattice byte identity) | 中-高 | ⚠ Codex fork 先 |
| R4 | Android 自訂詞 cap parity (hard cap+grandfather+擋新增,不自動驅逐) | 低 | ✅ |
| R5 ⚠ | 多音字 freq (hanji,tl) pair-key (修 #7)。**全範圍**:schema ALTER+tl + `FrequencyEntry` proto +tl + ranking key + candidate key extraction + **backup migration** + tolerant;**依賴 R2** | **高** | ⚠ Codex pre/post |
| R6 | SQLite hygiene (VACUUM on-demand 非啟動路徑/optimize/integrity/journal parity/冗餘 index/record 錯誤上拋) | 低 | ✅ |
| R7 | 備份/隱私 (learned data 是否 exclude cloud backup;與 R5 backup migration 協調) | 低 | ⛔ USER 產品決策後 |

依賴:R5→R2 (canonical TL)。R1 讀層自足。順序 R1→R2→R3→R4→R5→R6→R7。刻意不做 (YAGNI/Codex): orphan 清理。

**Root cause (2 co-bug,最終 review 確認)**: (1) `prev_tl` hard-filter → recall miss;(2) `next_tl` raw/canonical fragmentation + filter `(hanzi,tl)` merge → 重複顯示。R1 讀層 (查詢放寬+toneless-collapse) + R2 寫層 (canonical TL) 雙修。

**雙簽核**: Codex 對抗 review 7 objection → 全 resolved → 確認 pass「no remaining objection」(R1 2 guardrail 為條件)。Claude 亦無異議。

**三索引繼續實作價值 (USER #5)**: **已 100% 完成,無 pending。** 三軸全 first-class — TL 本來是 / POJ B-1#308+B-2#309 / TPS D#334-340。價值已交付 (三軸對稱搜尋 + 架構去耦),應保留;回退=失 TPS/POJ first-class 搜尋 + 重引架構債。與 v3.6.1 (使用者資料層) 正交,**v3.6.1 不動三索引**。

---

kautian subcollections (腔調 + 姓名附錄 toggles + 語音差異 詞級擴展) — 5 phases MERGED, shipped **v3.6.0** (#354-#358).

---

### Repository size — stop committing build artifacts (2026-09-07, USER-approved)

**Status**: Phase 0 done (this section + memory). PR 1 next.

The repository is **753 MB**. Source code is 61.5 MB of it; the rest is build
output committed on every rebuild. A trial `git filter-repo` measured what is
recoverable:

| | Size |
| --- | --- |
| Today | 753 MB |
| Without build artifacts | **217 MB** |
| Also without fonts | 144 MB |

Two separable problems. Gitignoring artifacts **stops growth but shrinks
nothing** — every past revision stays in the pack. Only a history rewrite
reclaims the 536 MB.

**What is generated, and by what** (verified against the pipeline, not assumed):

| Kind | Tracked | Generator |
| --- | --- | --- |
| Engine binaries — `.a` / `.so` / xcframework | 46.7 MB | `make build` |
| `dictionary/output/*` | 58.6 MB | `make dict` |
| Platform dictionary copies (×4) | 69.7 MB | `make dict` → `dictionary/build/deploy.sh` |
| Per-source `sources/*/data/<src>.csv` | ~30 MB | `dictionary/pipeline/context.py:104-106` |
| Font copies for Android / macOS / Windows | **118 MB** | since removed — see below |

`sources/*/data/raw/*` (~35 MB) is a real input and stays committed. So are the
typefaces, but no longer four times over: the four copies this table measured
became one shared `fonts/font/`, packaged by all four platforms, so 118 MB of
duplicate left the tree and `scripts/sync-fonts.sh` went with it.

**Cost of the change is close to zero for the maintainer.** Ignored files
survive `git checkout`, so `make dict` / `make build` run once per machine, and
after that only when their inputs change — which is what
`CLAUDE.md`'s stale-binary gate already requires. The recurring cost falls on
the GitHub-hosted Windows build, which checks out fresh. In the end phase 3 was
narrowed and that workflow gained only a `make fonts` step — itself removed when
the font trees were replaced by the single shared directory.

**Phases**

| # | Change | Outcome |
| --- | --- | --- |
| 1 | `version_snapshot.py` diffs releases against a committed 3.1 MB `dictionary/word-keys.tsv` instead of the 35 MB `dictionary/output/dictionary.csv` | **Merged** (#1) |
| 2 | macOS and Windows font trees become `make fonts` output and are ignored | **Merged** (#2) |
| 3 | iOS and macOS xcframeworks and the Android `.so` become `make build` output and are ignored; `CLAUDE.md` grows a bootstrap section | **Merged** (#3) — narrowed to the engine binaries |
| 4 | `git filter-repo` purges the artifacts from history | **Not doing** (USER 2026-09-07) |

**Phase 3 was narrowed on a diagnosis that turned out to be wrong.**
Bootstrapping a fresh clone showed `make dict` dying in `cleanup.py:201` with a
`TypeError` comparing a str to an int, which was read as the dictionary
artifacts being unreproducible. The real cause was that the test clone had no
`--recurse-submodules`: `taigi-converter/` was empty, node failed its import,
and every conversion returned an error string that the pipeline ingested as
data. With the submodule checked out, `make dict` completes on a fresh clone and
deploys to all four platforms. Both entry points now check for it.

The dictionary artifacts nonetheless stay tracked, on the USER's instruction
rather than on that reasoning: 「dictionary/ folder 都不要碰,只要 build 出來的
產物在各大平台上有清理就好了」. `dictionary/output/`, the per-platform copies and
the per-source pipeline CSVs are all still committed, and nothing under
`dictionary/` was changed.

That left phase 4 purging only the platform build artifacts — measured at
**753 MB → 609 MB**, a 19% reduction bought with another full commit-SHA
rewrite and 17 dropped commits. The 392 MB that made the rewrite worth doing is
in the dictionary artifacts, which are staying. USER dropped phase 4 rather than
pay the full cost for the remainder.

**Where this leaves the repository**: still 753 MB, but the growth is stopped —
the engine binaries and font copies that accounted for most of it are no longer
committed on every rebuild. Reopening phase 4 only makes sense after the
dictionary pipeline can rebuild from a clean checkout.

**Measured runtimes** (2026-09-07, warm machine, fresh clone with submodules):

| | Wall | Where it goes |
| --- | --- | --- |
| `make dict` | **~2 min** | `run.sh` 67 s across the nine per-source pipelines (`kautian` 26 s, `taihoa` 10 s; by stage, `extract` 15 s and `merge` 8 s dominate), then `build.sh` 53 s (`merge_csv` 16 s, `create_association_bin`+verify 13 s, `create_syllables_fst` 11 s, `create_fst` 7 s, `create_dictionary_bin`+verify 5 s, everything else under a second) |
| `make build` | **5.1 s warm** | all six steps; every cargo invocation reports `Finished` in 0.03–0.08 s against a warm target dir. A cold build compiles the engine for five targets and takes minutes — not measured here. |

208,595 raw input rows, every reading converted through the Node bridge, three
FST/mmap artifacts built and verified. Nothing pathological.

Two earlier figures in this document were wrong: `~30 s` was a guess, and the
`10 minutes` that replaced it was measured against a run that spent most of its
time on the broken-submodule path, failing one conversion per row.

A dead-weight note for whoever reopens this: history still carries ~190 MB of
pipeline layouts that no longer exist at `HEAD` (`dictionary/csv/`,
`dictionary/6_台日大辭典/`, `dictionary/5_台華線頂對照典/`). They would go in the
same pass.

**Invariant being overturned**: `.gitignore:88-92` states that the Android `.so`
and iOS xcframework are *both* committed "so a release tag carries a complete,
buildable engine on both platforms" (D9.2). That trade is being reversed: a tag
plus a reproducible `make build` replaces a tag that carries binaries.

### Desktop custom fonts — let the user add their own typeface (3.6.8, USER-scoped 2026-09-08)

**Status**: Phase 0 done (this section + memory `project_desktop_custom_fonts.md`). PR 1 next.
**Scope**: macOS + Windows only. iOS and Android are deliberately untouched — the USER scoped this
to the desktop train.

Today the candidate-window typeface is a closed roster of five: the system face plus the four
files in the repo-root `fonts/font/`. `CandidateFontChoice` spells that roster three times over —
as a Swift enum with PostScript names (`macos/.../Candidates/CandidateFontChoice.swift:22-48`), as
a Rust enum with file names (`windows/.../settings/choices.rs:188-245`), and as DirectWrite family
names (`windows/.../ui/render.rs:50-58`). A user who wants any other face has no way in.

**The feature**: an "add a typeface" flow in the Appearance pane. The user picks a font file; the
app copies it into its own directory, reads the face name back out of it, and the file joins the
picker's roster. Added faces can be removed again.

#### Design

**A font library, copied — not a path remembered.** The chosen file is copied into
`~/Library/Application Support/TaigiKeyboard/Fonts/` (macOS) and `%APPDATA%\TaigiKeyboard\Fonts\`
(Windows), beside a small `fonts.json` index holding, per entry, the stored file name, the display
name, and the face name the font declares. The original may live on a removable volume, be deleted,
or sit somewhere a host process cannot read; the app-owned copy is the only one that is always
there. On Windows the same directory already holds `settings.json` and the user-data databases that
the TIP reads from inside every host process, so the path is proven reachable.

**The enum does not open up.** `CandidateFontChoice` gains one unit case, `custom` (raw value
`"custom"`), and a new settings key `customFontFile` names which stored file is live. The user may
keep several installed; only one is ever selected, so one key carries it. This keeps Windows'
`FontSpec`/`FormatKey` `Copy + Hash` (`windows/.../candidates/metrics.rs:23-26`) and keeps
`SettingChoice::raw` returning `&'static str`.

**Import validates by loading.** Extension in `.ttf` / `.otf` / `.ttc`, a size ceiling (the bundled
GenYoMin is already >20 MB — 64 MB), and then the real gate: the file must load and yield a face
name — `CTFontManagerCreateFontDescriptorsFromURL` on macOS, `IDWriteFontSetBuilder1::AddFontFile`
plus `GetPropertyValues(FAMILY_NAME)` on Windows. A file that fails is refused with a message and
nothing is left behind. A `.ttc` contributes its first face, matching the single-weight model the
roster already has. Name collisions get a suffix.

**Taking effect without a restart** is the real Windows work. macOS registers the copy with
`CTFontManagerRegisterFontsForURL(.process)` in the same process that draws the candidate window,
so the next window picks it up. On Windows the TIP lives in each host process and
`load_private_fonts` runs exactly once, at `RenderFactory::new` (`render.rs:294-322`). The custom
face needs its own collection, keyed by file name + mtime + the `settings.json` revision the TIP
already watches, and both `formats` and `ellipsis` caches must be dropped when that key moves —
otherwise replacing a file under the same name keeps drawing the old outlines.

**Deliberately not adopted**: weight/variable-axis selection, a rendered preview of each face,
syncing the library between machines, and any mobile counterpart. Fonts are copied for local use
only and never redistributed, so `THIRD_PARTY_LICENSES.md` is unaffected.

#### Rounds

| PR | Scope | Est. |
|---|---|---|
| P0 | This section + memory (admin tier, direct to main) | — |
| P1 | macOS whole: font library (copy / validate / index / delete), the `custom` case, launch-time registration, Appearance-pane UI, i18n keys | ~450 |
| P2 | Windows core: settings key + `Custom` variant + library and validation in `taigi-windows-storage` (pure crates, covered by `make windows-check`) | ~300 |
| P3 | Windows render: custom private collection + revision-driven invalidation + format-cache drop | ~250 |
| P4 | Windows settings window: list, `rfd` picker filter, delete, errors | ~300 |

P1 is one large macOS PR by the USER's standing preference for fewer, larger macOS PRs.

#### Dogfood (new items, to be added to `docs/architecture/dogfood-checklist.md` in P1/P4)

- **S30 macOS** — add a typeface → select it → the candidate window redraws in it → delete the live
  one → falls back to the system face, all without restarting the input method.
- **S31 Windows** — add a typeface in the settings window; an **already-running** host (Notepad plus
  a WinUI app) shows it on the next candidate window. Replacing a file under the same name does not
  keep drawing the old one.

---

## Released versions index

Newest first. Links: release notes (`changelog/`) + detailed plan archive (`docs/releases/`) where one exists. Authoritative ship-date list: memory `project_released_versions.md`.

| Version | Ship date | Release notes | Detailed plan archive |
|---|---|---|---|
| v3.6.0 | 2026-05-31 (`b782205c`) | [`changelog/v3.6.0.md`](../changelog/v3.6.0.md) | — (kautian subcoll + dev supplement + source-toggle filtering + explicit-tone fix) |
| v3.5.9 | 2026-05-29 (`3c8bec16`) | [`changelog/v3.5.9.md`](../changelog/v3.5.9.md) | — (TPS 三索引 + Tier-A/B refactor; design memo `project_v359_d_tps_triindex_plan.md`) |
| v3.5.8 | 2026-05-20 (`61df3028`) | [`changelog/v3.5.8.md`](../changelog/v3.5.8.md) | [`docs/releases/v3.5.8/plan.md`](releases/v3.5.8/plan.md) — Phase 0-9 + 整句 lattice + walker S1-S9 + continuous-compound-hyphen fix |
| v3.5.7 | 2026-05-08 | [`changelog/v3.5.7.md`](../changelog/v3.5.7.md) | — |
| v3.5.6 | 2026-04-27 | [`changelog/v3.5.6.md`](../changelog/v3.5.6.md) | — |
| v3.5.5 | 2026-04-12 | [`changelog/v3.5.5.md`](../changelog/v3.5.5.md) | — |
| v3.5.3 | 2026-03-22 | [`changelog/v3.5.3.md`](../changelog/v3.5.3.md) | — |
| v3.5.2 | 2026-03-08 | [`changelog/v3.5.2.md`](../changelog/v3.5.2.md) | — |
| v3.5.1 | 2026-02-25 | [`changelog/v3.5.1.md`](../changelog/v3.5.1.md) | — |
| v3.5.0 | 2026-02-12 | [`changelog/v3.5.0.md`](../changelog/v3.5.0.md) | — |
| v3.4.x | 2025-2026 | [`changelog/v3.4.*.md`](../changelog/) | — |
| v3.3.x | 2025 | [`changelog/v3.3.*.md`](../changelog/) | — |

Detailed plan archives are added retroactively only when source material exists; older versions remain release-notes-only.

---

## Out of scope / deferred (truly forward-looking)

Forward-looking candidates only, NOT items already shipped. (v3.5.8-era items that read like candidates but shipped — `整句 lattice + walker`, `continuous compound-hyphen`, `Phase 9 user-freq plumb` — live in [`docs/releases/v3.5.8/plan.md`](releases/v3.5.8/plan.md).)

### 變換後羅馬字 commit — segment + numeric-tone→diacritic on Enter (v3.6.5)

**Status**: Design LOCKED, NOT implemented — pre-arranged 2026-06-29 at USER request to minimize impl-time effort (USER 「預先安排好v3.6.5的項目，減少之後實作的effort」; originally tagged v3.6.4 2026-06-20, moved because i18n took v3.6.4). Round still gated on USER UX confirm + Codex pre-impl. **Full design + code seams + Codex prompt**: memory `project_roman_convert_on_commit.md`.

In TL/POJ, Enter should commit the **converted** romanization — multi-syllable segmentation + numeric tone → tone-diacritic (`suann2ting3` → `suán-tìng`), matching PhahTaigi. Today single (`suann2`→`suán`) and hyphenated (`tai5-gi2`→`tâi-gí`) convert; un-hyphenated multi-syllable stays verbatim per §10.2. Requester = Kisaragi Hiu (same person who drove the S22/§34 literal-roman candidate). **Locked design = Option A**: preedit stays verbatim while typing (§10.2 WYSIWYG), Enter commits converted, raw output stays free via tapping the existing verbatim strip-#0 literal candidate (no new UX element). Engine work = deterministic tone-digit pre-segmentation in `phonetics::canonical_tl_form` (digit ends a syllable → no FST inventory needed; fallback verbatim on ambiguous toneless/invalid input). Platform work = Enter commits the composition's `canonical_tl` instead of raw preedit (locate each platform's return-key composition handler at impl). Touches invariants §10.2 / §17 / §34 (S22) / Core Principle #7 — feature round updates them + adds a cross-platform `INVARIANT_*` test in the same PR. Engine-only fix covers iOS+Android; `make build` (no `make dict`).

**Also scoped to v3.6.5 (USER 2026-06-29)**: a dogfood-confirmation pass over the 8 open Gmail user-report bugs (`/bug-triage` queue; full list + ids in memory `reference_gmail_bug_triage`). This is a verification checklist, not a commitment to fix all 8 — already-fixed-on-current-build reports get labeled `已修復`, still-reproducing ones become user-gated per-incident fix rounds (Core Principle #4).

### App UI i18n — multi-language (台語 TL / POJ / 漢字 + 日語 + 英語)

**Status**: Shipped — all five display languages (漢字 / English / 日本語 / Tâi-lô / Pe̍h-ōe-jī + Automatic) are in the production picker. Open: TL/POJ prose proofreading by the USER (data-only edit, not a code gate). **Full plan**: [`docs/architecture/i18n-multilang-plan.md`](architecture/i18n-multilang-plan.md).

Localize app UI (host **+ keyboard-extension overlays + FAQ content** — not host-only) into 5 display languages + an Automatic (`system`) state. Single source of truth = new in-repo `i18n/` dir (NOT separate repo — flip trigger = external translator workflow) → **codegen NATIVE resources** (`.xcstrings` / `values-*/strings.xml`), hybrid: en/ja/漢字 use OS-locale resolution + `setApplicationLocales()`, TL/POJ via an app-level `DisplayLanguage` enum selecting an explicit resource set (BCP-47 is metadata only, never the persisted key). Scope-aware key/completeness checks kill today's manual iOS↔Android mirror; native plural via codegen typed fns (no ICU); POJ hand-authored alongside TL (lockstep-gated) + diff review. Phases: P0 reconcile+inventory → P1 schema+native codegen+**live-switch reactive prototype** → P2 English vertical slice → P3a ja / P3b TL / P3c POJ (picker shows only gated languages). **#1 risk = live-switch UI invalidation** (static getter won't refresh SwiftUI/Compose; extension is a separate process). USER constraints: i18n best practices + normalize divergent wording. USER 2026-06-19: in-repo confirmed, changelog EXCLUDED from multi-language, tentatively v3.6.4 (「可能」/possibly — not firm). Remaining extension/FAQ scope + timing user-gated.

---

---

## Per-round gates (process invariants, project-wide)

Apply to every coding round regardless of release. Authoritative source: `~/.claude/rules/round-workflow.md`.

Project-specific additions only (branching, sandwich, test scope, admin tier live in that rule):

- Cross-platform parity-correction rounds merge both platforms in lockstep.
- iOS `pbxproj` is user-only (`.claude/rules/ios-guidelines.md`); Android Gradle is editable.
- Engine slices require S0 golden-diff EMPTY acceptance.

---

## Closed phases / shipped audits

- **Keyboard theme picker** (swipe gallery + custom theme) — SHIPPED v3.6.2: iOS #400-411, Android port #412-#418. Current-state reference: [`docs/ui/theme.md`](ui/theme.md).
- **Android UI modernization** (Compose M3 chrome/overlay) — DONE 2026-05-30 (#362 / #364 / #365). 3 leaf overlays (Symbol/Layout/Candidate) View→Compose M3 over `KeyboardChromeColors`; keys stay custom-draw; `InputView`/window kept View (IME-dismiss bug zone). Memory `project_android_compose_modernization.md`.
- **v3.5.9 D = TPS 三索引** — SHIPPED, tagged `3c8bec16` 2026-05-29. `tps:` FST family parallel to `tl:` / `poj:`; mode-axis (Input + Key + FST) now three-layer symmetric. Retired `is_tps` short-circuit (`dispatch.rs`/`continuous.rs`), `tps_or_mapped_to_er` runtime branch (`search.rs`), `tps_to_tl` canonicalize chain (`classification.rs`). 6 PR (#334-#340, C-0/C-1/C-3a/C-3b/C-4/C-5).
- Roadmap Item 1 (Project Structure & File Naming Cleanup) — CLOSED 2026-05-06 (#212-#215).
- Roadmap Item 4 (Android UI Compose migration) — CLOSED 2026-05-08 (#227-#231).
- v3.5.8 連續輸入 — SHIPPED 2026-05-20 (`61df3028`). See [`docs/releases/v3.5.8/plan.md`](releases/v3.5.8/plan.md) for full plan + Phase status + design rationale + dogfood matrix.

<!-- New active items go in Active / In-flight items. New deferred items go in Out of scope / deferred. Shipped versions get a row in Released versions index + an entry in Closed phases. -->
