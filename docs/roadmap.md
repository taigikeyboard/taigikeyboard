# Taigi Keyboard — Roadmap

> **Type**: Planning (forward-looking)
> **Keywords**: `roadmap`, `planning`, `released versions`, `deferred items`
> **Status**: Active
> **Last updated**: 2026-06-01 (pruned to one open TODO — keyboard theme picker; shipped detail moved to released-versions index + memory)

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

kautian subcollections (腔調 + 姓名附錄 toggles + 語音差異 詞級擴展) — 5 phases MERGED, shipped **v3.6.0** (#354-#358). Detail: memory `project_kautian_subcollections.md`.

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

### Keyboard theme picker — swipe-select gallery + custom theme + save

**Status**: deferred, unscheduled (user-gated). **Goal** (USER 2026-06-01): swipe left/right through predefined themes, create a custom theme, save the selection — modeled on KeyboardKit Pro's theme shelf.

**Reference layout** (KeyboardKit Pro `KeyboardTheme.Shelf`, captured from `references/keyboardkit9.9.0` 2026-06-01):

- **Shelf**: vertical list of named category rows (STANDARD / SWIFTY / MINIMAL / COLORFUL …), each row a horizontally-scrolling strip of `A`-key preview swatches (`KeyboardTheme.ShelfItem`). 左右滑動 = horizontal scroll per category; vertical across categories. Screens: `.../KeyboardKit.docc/Resources/Views/themeshelf.jpg` + `app-themescreen.jpg` ("Pick a theme" modal, selected item = green ✓).
- **Picker screen**: `KeyboardApp.ThemeScreen` wraps the shelf as a modal (Demo: `.../Demo/KeyboardPro/DemoKeyboardView.swift:141-143`).
- **Persist**: `KeyboardThemeContext` + auto-persisted `KeyboardThemeSettings` (`.../_Pro/ProPlaceholders.swift:663,670`).
- **Custom theme**: KeyboardKit custom themes = developer-defined Swift structs only (`extension KeyboardTheme { static var greenPrimary … }` or base off `.minimal` + tweak `buttonStyles[.primary]`); **no end-user theme editor** — Taigi builds the editor + `Codable` persistence from scratch.

**OSS clone gives only the pattern** — DocC `Features/Themes-Article.md` + 2 screenshots + public API signatures. `Shelf` / `ShelfItem` rendering + gestures + geometry are closed in the KeyboardKitPro binary (clone = `{}` / `proPlaceholder` stubs). Taigi implements the shelf views itself.

**Scope notes**:

- Build on existing user-color theming (`KeyboardColorSettings` Android / `KeyboardPreviewPanel` iOS) — not a parallel system; audit overlap first.
- Cross-platform parity: Android = Compose M3 `LazyRow`-per-category × `LazyColumn`, lockstep per `.claude/rules/cross-platform-alignment.md`.
- Best-practices entry: `docs/references/mainstream-ime-comparison.md` (peer IME theme galleries).

---

## Stub redirects for legacy section anchors

The following section anchors previously lived in this file; v3.5.8 archive [`docs/releases/v3.5.8/plan.md`](releases/v3.5.8/plan.md) now hosts them. Inbound code comments citing `§ Phase N` / `§ 整句 lattice + walker S<N>` resolve via these stubs.

### Phase 0 / Phase 1 / Phase 1b / Phase 2 / Phase 3 / Phase 4 / Phase 5 / Phase 6 / Phase 7 / Phase 8 / Phase 9

See [`docs/releases/v3.5.8/plan.md` § 實作 Phases](releases/v3.5.8/plan.md#實作-phases).

### Phase 9 R2 Q3.a / Phase 9 sort_key formula / Phase 9 跨平台常數表 / Phase 9 回歸守護矩陣 / Phase 9 R3 / Phase 9 R1

See [`docs/releases/v3.5.8/plan.md` § Phase 9 — Continuous-input ranking 修復 + 主流 IME 對齊 (FINALIZED 2026-05-11)](releases/v3.5.8/plan.md#phase-9--continuous-input-ranking-修復--主流-ime-對齊-finalized-2026-05-11).

### 整句 lattice + walker (S1-S9)

See [`docs/releases/v3.5.8/plan.md` § 整句 lattice + walker](releases/v3.5.8/plan.md#整句-lattice--walker-v358-must-solve實作中).

### 連續輸入 compound-hyphen / 連續輸入 Option A

See [`docs/releases/v3.5.8/plan.md` § 連續輸入 compound-hyphen](releases/v3.5.8/plan.md#連續輸入-compound-hyphenv358-§102-option-adogfood-修復).

### 刻意不採用 / 最佳實踐對齊

See [`docs/releases/v3.5.8/plan.md` § 最佳實踐對齊](releases/v3.5.8/plan.md#最佳實踐對齊-rules--ime-主流) — the v3.5.8 plan's explicit non-goals (librime full pipeline / RIME SchemaYAML / neural LM / MOE Nail UX / etc.) remain non-goals for future rounds unless a user explicitly re-scopes them.

### Active item v3.5.8 / Phase 5 / Phase 6 / Phase 9

See [`docs/releases/v3.5.8/plan.md`](releases/v3.5.8/plan.md) header + the corresponding `§ Phase N` sections.

### 回歸守護矩陣 / 既有 dogfood 矩陣 / Verification (release-level)

See [`docs/releases/v3.5.8/plan.md` § Verification](releases/v3.5.8/plan.md#verification-release-level) + [§ 回歸守護矩陣](releases/v3.5.8/plan.md#回歸守護矩陣每-pr-跑91-為-acceptance-主).

### Phase 排序 + 狀態追蹤

See [`docs/releases/v3.5.8/plan.md` § Phase 排序 + 狀態追蹤](releases/v3.5.8/plan.md#phase-排序--狀態追蹤-cross-pr-handoffauthoritative).

---

## Per-round gates (process invariants, project-wide)

Apply to every coding round regardless of release. Authoritative source: `~/.claude/rules/round-workflow.md`.

- Each round = own branch + PR + Codex sandwich (pre + post) + `Skill(simplify)`.
- Touched-target test scope by default (e.g. `cargo test -p <crate>`, `./gradlew :module:test`).
- Cross-platform parity-correction rounds merge both platforms in lockstep.
- Doc-only rounds = admin tier — direct-to-main allowed.
- iOS `pbxproj` is user-only (`.claude/rules/ios-guidelines.md`); Android Gradle is editable.
- Engine slices require S0 golden-diff EMPTY acceptance.

---

## Closed phases / shipped audits

Historical archive entries live in Claude auto-memory `project_phase_archive.md`.

- **Keyboard theme picker** — reference layout captured; see deferred item above (only open TODO).
- **Android UI modernization** (Compose M3 chrome/overlay) — DONE 2026-05-30 (#362 / #364 / #365). 3 leaf overlays (Symbol/Layout/Candidate) View→Compose M3 over `KeyboardChromeColors`; keys stay custom-draw; `InputView`/window kept View (IME-dismiss bug zone). Memory `project_android_compose_modernization.md`.
- **v3.5.9 D = TPS 三索引** — SHIPPED, tagged `3c8bec16` 2026-05-29. `tps:` FST family parallel to `tl:` / `poj:`; mode-axis (Input + Key + FST) now three-layer symmetric. Retired `is_tps` short-circuit (`dispatch.rs`/`continuous.rs`), `tps_or_mapped_to_er` runtime branch (`search.rs`), `tps_to_tl` canonicalize chain (`classification.rs`). 6 PR (#334-#340, C-0/C-1/C-3a/C-3b/C-4/C-5). Design memo: memory `project_v359_d_tps_triindex_plan.md`.
- Roadmap Item 1 (Project Structure & File Naming Cleanup) — CLOSED 2026-05-06 (#212-#215).
- Roadmap Item 4 (Android UI Compose migration) — CLOSED 2026-05-08 (#227-#231).
- v3.5.8 連續輸入 — SHIPPED 2026-05-20 (`61df3028`). See [`docs/releases/v3.5.8/plan.md`](releases/v3.5.8/plan.md) for full plan + Phase status + design rationale + dogfood matrix.

<!-- New active items go in Active / In-flight items. New deferred items go in Out of scope / deferred. Shipped versions get a row in Released versions index + an entry in Closed phases. -->
