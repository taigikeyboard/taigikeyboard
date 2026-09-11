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

**Status**: implemented on `feature/macos-custom-fonts` (PR #16) — macOS and Windows both. Awaiting real-device dogfood (S30 + S31).
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
| P1 | macOS whole: font library, `CandidateFontSelection`, launch-time registration, UI, i18n keys | done |
| P2 | Windows core + storage + platform: the selection type, the two-key resolver, the file half (host-tested) and the DirectWrite half | done |
| P3 | Windows render: a collection per custom face, an id per loaded resource, mtime+length as the change detector, format caches dropped with it | done |
| P4 | Windows settings window: the 字型管理 pane, `+` / `−`, the file dialog's font filter | done |

All of it landed in one PR (#16) at the USER's instruction, macOS first and Windows after.

**The UI shape took three tries** (USER 2026-09-08). A section in 外觀 with a per-row delete
button, then a sheet, then a pane for the custom fonts alone — each was rejected, and the
reason each time was the same one: choosing a typeface and managing the list are two
selections that look alike. The answer was to put the bundled roster and the user's own
typefaces in ONE list, in a 字型管理 pane of its own, where the selected row IS the typeface
in use — which is how System Settings states a list like that (聲音's output devices,
顯示器's displays). 外觀 lost its font row; each pane's reset restores the rows it shows.

#### Dogfood (new items, to be added to `docs/architecture/dogfood-checklist.md` in P1/P4)

- **S30 macOS** — add a typeface → select it → the candidate window redraws in it → delete the live
  one → falls back to the system face, all without restarting the input method.
- **S31 Windows** — add a typeface in the settings window; an **already-running** host (Notepad plus
  a WinUI app) shows it on the next candidate window. Replacing a file under the same name does not
  keep drawing the old one, and a file another host still holds refuses to be deleted with a message
  rather than silently.

---

### Desktop installed typefaces — the fonts the OS already has, listed and selectable (USER-scoped 2026-09-11)

**Status**: P0 (this section + memory). PR1 macOS and PR2 Windows to follow.
**Scope**: macOS + Windows only (desktop train). iOS / Android untouched.

**The report** (USER 2026-09-11): a user tried to use a typeface their Mac already has and the
字型管理 pane refused it with `another typeface is already called …`. Not a defect in that check —
it is the `nameAlreadyResolves` refusal at `CustomFontLibrary.swift:322`, and it is right: the
library's model is *import a font FILE, copy it, register the copy, draw the copy*, and a name a
system face already carries resolves to that face, so a registered copy would never draw. The
model simply has no path for a typeface that is already installed. The user is left with a dead
end and an error message that reads as a bug.

USER 2026-09-11 decisions: list the installed typefaces directly in the same table (「如果是系統字
型，有辦法在UI直接列出來使用嗎？」); **do not filter them by Hanji coverage** — Taiwanese is Hanji
AND romanization, so a Latin-only face is a legitimate choice for the roman half (「因為台語有漢字跟
羅馬字,所以字體都是必須的」); add a **search field** to keep a 200–300-row list usable.

#### Design (grounded in code)

**A third kind of row, not a third library.** `CandidateFontSelection` gains one case,
`installed(family)` — a family name the OS reports right now. Nothing is copied, nothing is
registered, nothing is owned: the OS is the authority on what exists, the same way the directory
is the authority for the user's own files. The table reads **bundled five → imported → installed**,
the selected row IS the typeface in use, as today (`FontManagementPage.rows`).

**Stored as the third value of the existing pair.** `fontType` holds a new raw value
`"installed"` (`"system"` is taken — it is `CandidateFontChoice.system`'s raw value) and a new key
`installedFontFamily` names the family. Same shape as `"custom"` + `customFontFile`
(`SettingsStore.swift:280-297`, `font_selection.rs:31-63`): an older build or another platform
reads `"installed"` as unknown and falls to the system face; the two keys are written together by
the one writer. Family name rather than PostScript name: it is what the listing API hands back on
both platforms, and it survives the OS swapping a face's file under the same family.

**Listing.** macOS `CTFontManagerCopyAvailableFontFamilyNames()`, dropping names that start with
`.` (hidden UI faces); measured 2026-09-11 on this Mac: 261 families, 33 ms cold, 13–16 ms warm,
sort included — below anything the user can feel. Windows
`IDWriteFactory::GetSystemFontCollection(check_for_updates = TRUE)`, each family's name through
`IDWriteLocalizedStrings` (vtable call, not the `&mut [T]` wrapper — `windows/clippy.toml`). Read
**every time the pane appears**, never cached across a launch, so a font installed or removed in
Font Book / Settings shows up on the next visit. macOS additionally observes
`kCTFontManagerRegisteredFontsChangedNotification` while the pane is open and re-reads;
Windows has no cheap equivalent and re-reads on entry only (a `WM_FONTCHANGE` hook is a
follow-up if dogfood wants it). The search field filters the in-memory array; it never re-queries.

**Rendering.**
- macOS: through `RegisteredFace`, the one resolution path (USER 2026-09-10 — do not split it).
  It grows a family-matching entry point: a descriptor with `kCTFontFamilyNameAttribute`
  mandatory, matched fresh, the family checked on the result, memoized under the same lock. The
  same `kCTFontManagerRegisteredFontsChangedNotification` clears the memo (today only
  `CustomFontLibrary.withdraw` does, `RegisteredFace.swift:66-77`), so a face removed while the
  input method runs stops resolving instead of drawing out of a stale object. Not found →
  `.builtIn(.system)`, preference kept — the same rule as a missing custom file
  (`SettingsStore.candidateFontSelection`).
- Windows: `CreateTextFormat` with the **system collection** (`None`), no `CustomFontId`, no
  private collection to keep alive. `FontSpec` / `FormatKey` are `Copy` and cannot hold a
  `String`, so the render factory interns family names into a small `InstalledFontId(u32)` the
  way `CustomFontId` names a loaded resource; the format cache is keyed on it. Family not in the
  system collection → default face, preference kept.

**The pane.** One search field above the table (`TextField` with a magnifying-glass label on
macOS; `TextBox` on Windows, as 自訂詞庫's), filtering all three groups by case- and
diacritic-insensitive contains. The selected row stays selected while filtered out; clearing the
field shows it again. `+` (import a file) unchanged. `−` disabled on bundled and installed rows —
an installed row has nothing to delete. The `nameAlreadyResolves` refusal stays: it is still the
right answer for a file whose face is already there, and the row for that face is now in the table.

**Deliberately not adopted**
- Filtering to Hanji-capable faces (USER refused 2026-09-11 — the roman half needs Latin faces too).
- A sub-page or sheet for the installed list (USER refused popups 2026-09-08; the search field
  keeps one table usable).
- Reusing `customFontFile` for the family name: one key would carry two kinds of identity and
  the Windows reader treats that value as a path component (`remove_stored`).
- Storing the PostScript name of the family's regular face at pick time: brittle across OS font
  updates, and not what the listing API returns.
- Per-weight / style selection inside a family: the roster is single-weight today.

#### Rounds

| PR | Scope | Est. |
|---|---|---|
| P0 | This section + memory (admin tier, direct to main) | — |
| PR1 | macOS: `installed` case, `installedFontFamily` key, `RegisteredFace` family matching + notification-driven memo clear, pane rows + search field, i18n keys, tests | ~400 LOC |
| PR2 | Windows: `Installed(InstalledFontId)` + interner, stored pair, system-collection format path, `GetSystemFontCollection` listing, pane rows + `TextBox` search; `check-box` | ~500 LOC |

#### Dogfood (to be added to `docs/architecture/dogfood-checklist.md` in PR1 / PR2)

- **S42 macOS** — 字型管理 lists the Mac's families after the five bundled + imported rows; typing
  in the search field narrows all three groups; select an installed family → candidate window
  redraws in it without restart. Install a font in Font Book while the pane is open → it appears;
  remove the selected one → candidate window falls back to the system face, pane selection reads
  系統, no crash. Importing a file whose face is installed still refuses, and the face is in the list.
- **S43 Windows** — same in the settings window, with an already-running host (Notepad + a WinUI
  app) drawing the chosen family on the next candidate window; pane open time not perceptibly
  slower than 3.6.8.

---

### Desktop Telex tone keys + candidate-window toggle (USER-scoped 2026-09-08)

**Status**: all rounds MERGED 2026-09-09 — P1 #17 `6888be67`, P2 #18 `cbee26d1`, P3 #19 `447154ea`, P4 #20 `938994fa`, guide follow-up P5 #21 `544d77a2`, P6 #22. Awaiting real-device dogfood (S32 + S33 + S34).
**Scope**: macOS + Windows only (desktop train). iOS / Android untouched apart from regenerated
engine bindings — the new composing intent is additive.

USER 2026-09-08: 「參考 Telex 方案 … 使用 Telex 的方式選取聲調,改用 1~9 數字選取候選詞 … 預設是
標準,1~9 打聲調,qwdfz 選候選詞,使用者可以選擇 Telex,英文字母打聲調,1~9 選候選詞,兩種反過來」;
「在一般設定加上一個 toggle,可以取消候選窗,預設開啟」; 「移除『選字齒』的 shift/control/option
三個選項,並且也移除『選字齒』的快捷鍵設定」; 「台語有一些 tsh 三個字母的字,也想辦法幫使用者方便打字」.

#### Design (grounded in code, Codex pre-impl reviewed 2026-09-08)

**Key table.** The letters no TL or POJ syllable spells are `c d f q v w x y z` (`r` is NOT free:
the dialect finals `ir` / `er` are in the dictionary). Six of them carry tones, two carry functions,
`c` stays a plain letter because POJ spells `ch` / `chh` with it:

| Key | Meaning | TL | POJ |
|---|---|---|---|
| `v` `y` `d` `w` `x` `q` | tone 2 3 5 7 8 9 | `tev` → té | `pay` → pà |
| `z` | affricate initial | `z` → `ts`, `zh` → `tsh` | `z` → `ch`, `zh` → `chh` |
| `f` | hyphen | `taidfgiv` → tâi-gí | same |
| `1`–`9` | candidate slot | | |

This is the kahiok scheme (madmaxieee/taigi-telex) minus its `c` → `tsh` key. Uppercase tone
keys carry the same tone; `Z` → `Ts` / `Ch`. Tone 6 is not offered (no free letter; USER 2026-09-08
accepted). `nn` / `oo` are native spellings the engine already handles. khiin-rs (`s f l j w`) and
the Cathaylab Keyman keyboards (`s f w x v`) were rejected because `s` / `l` / `j` are TL initials
and need escape rules.

**Standard vs Telex are the same eight letters, swapped.** Standard = digits type tones,
`q w d f z x v y ;` pick candidates (today's default). Telex = letters type tones, digits pick.
The slot key set is therefore DERIVED from the scheme, not a setting: `CandidateSlotKeySet`
shrinks to `{bareKeys, digits}`; the ⇧ / ⌃ / ⌥ sets, the 選字齒 picker row and its i18n key
`bindingSlotModifier` go. The bare-letter reservation (a letter the slots use cannot be recorded as
a shortcut, `ComposingKeyChord.swift:124`, `windows/.../keys/chord.rs:55`) stays and covers both
schemes with one set; only the modifier-chord collision branches
(`ShortcutActions.swift:435-472`, `shortcut_actions.rs:262-271`) are deleted.

**Semantics live in the engine.** One new composing intent `TelexKey { key }` (tag 40, a new
family in `composing.proto`). The engine resolves `z` by `config.input_mode`, and edits the pending
`raw` tail: no trailing tone digit → append the digit; a different trailing digit → replace it;
the same digit → no-op (no double-tap cancel, so a held key cannot flip-flop; Backspace removes
the digit). Tail empty or ending in `-` → no-op. Nailed segments are never touched; selection
resets as `append_continuous` does (`transition.rs:782`). Buffer stays numeric-tone (`tai5`), so
the syllabifier, literal-roman candidate and auto-space contracts are untouched (Codex Q1).
Platforms only gate on the scheme setting and classify `v y d w x q z f` → `.telexKey`; idle tone
keys / `f` pass through to the host like idle digits do today (`ComposingKeyIntent.swift:291`),
idle `z` starts a composition.

**Candidate window toggle** `isCandidateWindowEnabled` (default on). Off = no fetch, no window;
flipping it clears the source and hides any open window; composition still promotes to continuous.
Space and Enter fall back to `CommitRaw` (romanization with tone marks, `tai5` → `tâi`); a digit
mid-composition without a window commits-then-inserts like punctuation (auto-space may yield
`tâi 3` — pinned).

**Deliberately not adopted**: repurposing the dead `AppConfig.tone_mode`; literal-letter escape
(`vv` → `v`, never a Taigi syllable); mobile Telex; dictionary segmentation for "last syllable".

#### Rounds

| PR | Scope | Est. |
|---|---|---|
| P0 | This section + memory (admin tier, direct to main) | done |
| P1 | engine: `TelexKey` intent, proto, dispatch arm, transition, tests; `make build` regenerates bindings | done #17 |
| P2 | macOS: `toneInputScheme` setting + General-pane picker with mode-aware legend, classifier, derived slot set, delete ⇧/⌃/⌥ + 選字齒 row + modifier collision code, tombstone `candidateSlotModifier` | done #18 (+886/−927) |
| P3 | Windows: mirror of P2; drops `bindingSlotModifier` from i18n | done #19 |
| P4 | Candidate-window toggle, both platforms | done #20 |

**Learned in review** (Codex post-impl, all applied): the bare-letter shortcut reservation had to
stay scheme-independent AND the launch pass must clear global rows recorded on a typing key before
the change (a bare `z` recorded pre-2026-09-08 would fire through Carbon / the TSF hotkey before the
classifier saw the Telex key); a shifted number-row key is refused by key code on both recorder
paths so the event and the registry bridge agree; the candidate-window toggle hides the bar
*before* the next key is classified, since the setting can flip faster than its observer runs.

#### Follow-up: Telex guide as a global shortcut (USER 2026-09-09)

USER: 「telex 的說明文字不要放在說明文字下面,而是要在『快速齒』頁面加一個 Telex 說明的快捷鍵,使用
快捷鍵就可以快速叫出一個鍵盤 mapping 的選單可以看,然後可以按 esc 退出,或是其他按鈕退出,繼續打字」.

**Design (Codex pre-impl 2026-09-09, 9 points applied).** The legend under the 聲調拍法 picker goes
(both panes; `settings.toneSchemeTelexLegendTl/Poj` deleted once both consumers are gone). A fifth
global action `showTelexGuide` (last row of 快速齒, default `⌃⌘/` on macOS, `Ctrl+Alt+/` on
Windows — `Ctrl+Alt+T` was rejected: JetBrains Surround With) TOGGLES a floating guide panel: the
same non-activating HUD chrome as the mode flash (`ModeFlashPanel.swift`, `ui/mode_flash.rs`), no
timer, centred on the working screen (the hotkey path has no client, so no caret anchor), rows
key | meaning | example spelled for the romanization in use (`z` = ts/ch, tone 9 `tsa̋ng` vs
`chăng`). Dismissal happens in the per-key entry before classification: the guide hides, Escape
(no host chord) is swallowed even mid-composition, every other key falls through; on Windows the
Test phase answers TRUE for every non-modifier key while the guide shows, so Deliver is guaranteed
to arrive. The other global actions hide the guide before they run. The panel is owned by the
session that raised it (macOS token; Windows context) and goes with document / context / thread
focus loss. Always available, not gated on the Telex scheme.

| PR | Scope | Est. |
|---|---|---|
| P5 | macOS: action + `TelexGuidePanel` + dismissal + legend removal + tests | done #21 |
| P6 | Windows: action + GUID + preserved key + `ui/telex_guide.rs` + dismissal + legend removal; deletes the legend i18n keys | done #22 |

**Learned in review** (Codex post-impl P5/P6, applied): every doorway that never reaches the session
(settings, check-for-updates, the lang-bar menu) and every "hide all IME UI" request must take the
guide down itself; the guide must close in read-only contexts too, so its dismissal sits above the
read-only bail; a held preserved-key chord re-fires `OnPreservedKey`, so the toggle is once-per-press.

#### Dogfood (added to `docs/architecture/dogfood-checklist.md` as S32 / S33; S34 for the guide)

- **S32 Telex** — TL: `tev` → té, `tsangq` → tsa̋ng, `zhi` → tshi, `taidfgiv` → tâi-gí, `tev`+`y` → tè,
  `tev`+`v` unchanged, digit picks the slot, `q w d f` no longer pick. POJ: `zit` → chit,
  `zhiunnw` → chhiūⁿ, `chit` stays `chit`. Standard: unchanged from today.
- **S33 Candidate window off** — no window ever appears; Space / Enter write `tâi`; toggling
  mid-composition hides the window without losing text.

---

### Desktop symbol picker (USER-scoped 2026-09-09)

**Status**: all rounds MERGED 2026-09-09 — P1 macOS #26 `843e3453`, P2 Windows #27 `d5a8940b`, P3 flat list #28 `7245a5b2`. Awaiting real-device dogfood (S36, both platforms).
**Scope**: macOS + Windows only (desktop train). Engine untouched — no `make build`.

USER 2026-09-09: 「增加快捷鍵叫出特殊符號選單(包含標點符號、括號、特殊符號),風格為候選詞選單,
快捷鍵不能設定太難按,或是太複雜的組合 … 選取後合起來(注意括號的部分因為是成對,避免 user 要打開
選單兩次的情形),或者是按某一個按鍵退出」. Fork answers (USER 2026-09-09): 「Fork A 不要更改 `
快捷鍵,這是台語輸入法的共識, Fork B 依照你的建議處理」 — bare `` ` `` stays 漢羅對調; the picker
gets `⌃⌘,` / `Ctrl+Alt+,`; two-level menu with Escape closing from either level.

#### Design (grounded in code, Codex pre-impl reviewed 2026-09-09, 10 points CONFIRMED)

**Trigger routes through the key path, not Carbon / a preserved key.** A sixth global action
`showSymbolPicker` (快速齒 row, recorder, conflict resolver, i18n label all reused) that
`ShortcutHotkeys.registerHandlers` skips on macOS; `TaigiInputController.handle` matches the
recorded chord by key code + normalised modifiers right after the Telex-guide block
(`TaigiInputController.swift:567-572`). Windows already matches non-preserved chords in
`session.rs::global_action_for` (:1371); the picker adds a context-aware branch there and is NOT
in `preserved_keys::PRESERVED` (roster test exemption widened). Why: the pick and the bracket
insert need a client / edit session and a caret rect, which only the key path has (the hotkey path
has no client, `TaigiInputController.swift:505-508`; Chromium deadlock rule :161-166); read-only
contexts are already bailed above.

**Panel = a second instance of the candidate window.** `CandidatePanel.show` /
`CandidateWindow::show` take only `[CandidateCellContent]` (plain strings) + a caret rect — no
engine coupling. macOS opens `CandidatePanel.init` to internal and keeps a `symbolPicker`
instance with its own owner; Windows constructs a second `CandidatePresenter` without the UI-less
element wiring (host-drawn lists are not offered for the picker; documented limitation). Layout
follows the user's candidate-layout setting. Every teardown that hides the composing bar hides the
picker too: `hideForHandover`, `hidePalettes`, deactivation, Windows pending hides, and the
font-cache release at `FontManagementPage.swift:204`.

**Keys while open** (intercepted before the classifier, like the guide, but consuming): the
scheme-derived slot keys pick; arrows / PageUp / PageDown / the recorded paging chords / Tab
navigate through the panel's own `navigate`; Return picks the highlighted cell; **Escape closes
from either level**; any other key closes the picker and falls through to normal handling.
Windows Test phase answers TRUE for every non-modifier key while the picker shows; Deliver runs
the original event through the pipeline exactly once. Trigger auto-repeat neither reopens nor
selects. Preserved actions bypass the interceptor, so their handlers dismiss the picker.

**One list** (USER 2026-09-09, after dogfood: 「開啟的時候還有選擇標點符號、特殊符號,選擇後才能選,
會造成使用者的體驗中斷,我認為可以放在一起試試看,讓使用者開啟後直接選擇使用」). The whole table in
file order — punctuation, then bracket pairs, then special symbols — so the first key after the
chord is already a pick. The JSON keeps its three groups as documentation and the validation's
unit; the picker flattens them. The controller keeps one boolean (open / closed); selection lives
in the panel, never duplicated. Shipped two-level in #26 / #27 and flattened the same day.

**Bracket pairs are one cell** — `「」` `『』` `（）` `《》` `〈〉` `【】` `﹁﹂` `﹃﹄` `〔〕` `［］`
`｛｝` `“”` `‘’` `()` `[]` `{}` `<>` `«»` `⟨⟩` `⌈⌉` `⌊⌋` — inserted as one string with the
caret AFTER the closing half on both platforms. IMK has no selection setter; TSF could shift the
selection but parity + determinism win. Insert bypasses full-width remapping (`()` stays `()`)
and takes the attaching-punctuation auto-space swap (`guá ` + `，` → `guá，`). The composing
manager is told about the external text like any pass-through.

**Mid-composition = commit first, then open** (vChewing). A visible highlighted candidate →
the same path as `commitHighlightedCandidate` (selected cell's script, no flip); otherwise the
same path as `.commit`. The picker opens only after the composition has actually ended, and the
anchor is asked for afterwards; Windows waits for the edit session to succeed and revalidates the
context owner.

**One desktop data source.** `symbols/desktop-symbols.json` — three categories, ordered
insertion strings — read as a bundle resource on macOS and `include_str!` on Windows. No
generator. Mobile `SymbolData` (`ios/.../Overlays/SymbolData.swift`, Android `SymbolData.kt`)
stays as it is: its cells are single halves, a different contract.

**i18n**: `desktop.shortcutShowSymbolPicker` only. The three category labels shipped with the
two-level menu went with it.

**Deliberately not adopted**: bare `` ` `` for the picker (mainstream 新注音 / McBopomofo /
vChewing convention, but USER keeps it for 漢羅對調 — 「台語輸入法的共識」); caret-between-halves
via marked text or synthetic ← events; vChewing's nested categories (shipped, then flattened at
USER request — one more keystroke before the first symbol read as an interruption); a symbol-table
generator shared with mobile; engine involvement (`Effect` has no caret kind).

#### Rounds

| PR | Scope | Est. |
|---|---|---|
| P0 | This section + memory (admin tier, direct to main) | done |
| P1 | macOS: action + key-path match + second `CandidatePanel` instance + JSON table + i18n + S36 + tests | done #26 |
| P2 | Windows mirror: `symbols.rs` (`include_str!`), `keys/symbol_picker.rs`, `ShowSymbolPicker` key-sink chord, second `CandidatePresenter` (popup only), `KeyWork::InsertSymbol` | done #27 |
| P3 | Both: one flat list, level state and category labels removed (USER dogfood 2026-09-09) | done #28 |

Codex-named regression risks (all in S36): shortcut theft, stale focus ownership, partial commits,
duplicate insertion after an edit-session failure, orphaned panels.

**Learned in review** (Codex post-impl P1/P2 + `/simplify`, all applied): the chord and every
navigation key touch nothing, so the auto-space arm goes back at the branch head and only the pick
that writes spends it; a window that refused to show (no display, `CreateWindowExW` failed) must
leave no level behind or an invisible picker swallows the slot keys; the settings doorway never
reaches the key path, so it dismisses the picker itself; on Windows the window is the ONE owner —
the level carries no context token, and a posted focus hide needs no reconciliation; the caret is
read under a `TF_ES_READ` session with the focus generation revalidated across it, and the popup is
shown outside it; the auto-space swap is one helper for typed and picked text.

---

### Desktop composing caret — move inside the typed romanization (USER-scoped 2026-09-09)

**Status**: all rounds MERGED 2026-09-09 — P1 engine #29 `64e6b0b7`, P2 macOS #30 `0f3a7933`, P3 Windows #31 `3a273bff`. Awaiting real-device dogfood (S37, both platforms).
**Scope**: macOS + Windows only (desktop train). Engine changes — `make build` after P1.

USER 2026-09-09: 「allow user 可以使用方向鍵編輯正在輸入的字 例如使用者輸入ka2，可以使用方向鍵移動到k跟a中間,
輸入h 此時候選詞會刷新成kha2,讓使用者如果不小心輸入錯字，可以再修正」. Fork answers (USER 2026-09-09):
「候選窗上下左右移動是很重要的功能,評估使用其他按鍵的可能性」 — the bare arrows stay candidate
navigation; 「這個功能可以顯示在快速齒,但我不打算讓使用者更改」 — fixed chord, shown read-only;
「到頭/尾和刪最後一字不需要,只需要方向鍵左右移動即可」 — no Home/End, no forward delete.

#### Design (grounded in code, Codex pre-impl reviewed 2026-09-09)

**Keys.** While composing, `⌥←` / `⌥→` (macOS) and `Ctrl+←` / `Ctrl+→` (Windows) move the caret one
character. Every other key keeps its meaning: the bare arrows, Tab, `[` `]`, PageUp/Down and the
slot keys still drive the candidate window (horizontal layout: ←→ step, ↑↓ page —
`HorizontalPageLayout.swift:143-153`), Shift+arrow still commits and hands the host its selection
(`ComposingKeyIntent.swift:211`), Home/End and forward Delete stay commit-then-pass-through. Idle:
the chord reaches the host untouched. Why these chords: they are each platform's own "jump a word"
chord; `⌃←/→` never reaches an IME on macOS (Mission Control), `Alt+←/→` is back/forward in
Explorer and browsers and rides `WM_SYSKEYDOWN`. The classifier gets one fixed tier — composing +
exactly that modifier + ←/→ → `.moveCaret` — ahead of the candidate-navigation tier
(`ComposingKeyIntent.swift:211`, `keys/intent.rs:103`) and the host-chord tier (`:270`, `:153`).
Not in `ComposingAction` / `ComposingKeyBindings`; `ComposingKeyChord` keeps refusing arrows.

**快速齒 shows it read-only.** One row after the composing rows, before 恢復預設: label + the chord
as text (macOS `LabeledContent` + `Text`, Windows `cards::row` + `TextBlock`), no recorder, one
i18n key (`desktop.shortcutMoveComposingCaret`) in all five languages.

**Caret lives in the engine** (D1, Codex CONFIRM). `caret: usize` = UTF-8 byte boundary into the
PENDING `raw`, stored beside it in `Phase::Composing { raw, caret }` / `Phase::Continuous { raw,
caret, nailed }` (`api.rs:23-32`); `Idle` has none. Default `raw.len()`. Both desktops own the
buffer through the engine and mirror each other, so a platform-side caret would mean a new
`SetBuffer` op and the insert logic written twice. librime keeps the same model
(`references/librime/src/rime/context.h:102`).

**One intent** `MoveCaret { direction: Left | Right }` (wire `CaretDirection`; unspecified or unknown
= no step) — steps one `char`, clamped to the pending
tail (never enters a nailed segment; at the edge = no-op, no effect). Returns `[UpdatePreedit]`
only, NO `PerformAutocomplete`: the buffer is unchanged, candidates / highlight / page /
generation stay. McBopomofo's `setCursor` is likewise a bare assignment
(`reading_grid.cpp:43-46`).

**Caret-aware mutations** (D3, Codex CONFIRM + three amendments): `Append` inserts at the caret
(`raw.insert_str`, caret += len); `DeleteBackward` removes the char before the caret (caret 0 with
a non-empty pending = no-op; an empty pending keeps today's unnail under Continuous, and deleting
the last char of a Composing buffer keeps today's `DeleteBackwardFromDocument` — `transition.rs:278`,
a mobile/desktop difference the shared helper must not erase; after an unnail the caret sits at the
end of the restored text);
`ReplaceLast` swaps the char before the caret and keeps `selected_candidate_index`
(`transition.rs:237`); `TelexKey` = `apply_telex_key(&raw[..caret], key)` + the untouched tail,
caret = the converted prefix's byte length; `Start` and `CommitContinuous` (nail) put the caret at
the new end; **`EnterContinuous` keeps the caret** — it is a phase promotion with no
`UpdatePreedit` (`transition.rs:599`), moving the caret there would desync the screen. With the
caret at the end every op is byte-identical to today, so iOS / Android (which never send
`MoveCaret`) keep their behaviour; the proptest generator (`proptest_sequences.rs:16`) gains
`MoveCaret`, Unicode, tone digits and POJ toggles plus a caret-boundary invariant.

**Candidates** (D6, Codex CONFIRM): unchanged — the whole pending buffer, left-anchored at byte 0
(`dispatch.rs:140`); `consumed_bytes` stays a pending-prefix length, never caret-relative;
picking a candidate nails as today and resets the caret to the new tail's end. Deliberately not
McBopomofo's "candidates for the node at the caret" (`KeyHandler.mm:2492`): USER's example expects
`kha2` candidates for the whole buffer, and our lattice is left-anchored longest-match.

**Display caret** (D4, prefix-derivation REFUTED by Codex): `Preedit.caret_utf16` +
`Effect::UpdatePreedit.caret_utf16` = the caret's UTF-16 offset in `display_text`, produced by
`derived_display_with_boundaries(raw) -> (display, raw byte boundary → display UTF-16 offset)`:
the raw and display strings are walked in lockstep after the chain has run (`derived.rs`), relying
only on what the chain can do to a character: insert marks, drop a character, change case, fold a
POJ double tap — the fold recognised by its display signature (dot / `ⁿ` / `ᴺ`) plus the next raw
letter, since matching alone cannot tell the second tap from a later identical letter (`hooon`).
The premise "the chain never adds a base letter" is pinned by a phonetics test next to the code
that could break it. Threading offsets through the chain itself was weighed and dropped: six
phonetics functions would need offset-aware twins, and the `NormalizeTone` proto that all four
platforms call would change shape for a desktop-only need. Deriving the prefix alone and
taking its length is wrong, not cosmetic: `ng|5` displays `n̂g` and lands the caret before `g`
while the insert happens after it (`phonetics/src/tl.rs:64`); `ka2|i` shows the caret before `2`;
nailed `珠` + `|a` drops the roman-spacing space. Syllable edges map exactly; inside a syllable the
raw boundary after the k-th base letter maps after the k-th base letter of the display, then past
any following combining mark (the caret never splits a grapheme — McBopomofo clamps the same way,
`KeyHandler.mm:2429`). Many-to-one positions (`ho|o` → `ho͘`, `tin|n` → `tiⁿ`, the tone digit, a TPS
separator marker) get no visible step; accepted. `shadow.rs` offset helpers are NOT reused: they map
consumed ranges and forbid expanding replacements (`shadow.rs:1209`, `:1225`).

**Executors** (D7): macOS `setMarkedText(..., selectionRange: NSRange(location: caret_utf16,
length: 0))` (`ClientEffectExecutor.swift:29-38`; IMK needs `length == 0` and a uniform underline
for the caret to show — vChewing note). Windows: a new `select_caret(range, caret_utf16)` = clone →
`Collapse(TF_ANCHOR_START)` → `ShiftEnd(cch)` (check the reported shift; a region boundary can
shorten it) → `Collapse(TF_ANCHOR_END)` → `SetSelection`, used ONLY by the preedit update; commit
and external insert keep `select_end_of` (`composition.rs:210`, `:221`). Pattern from khiin-rs
`windows/ime/src/tip/composition_mgr.rs:168-184` and KeyKey41 `StateEditSession.cpp:286-305`;
`ITfRange::ShiftEnd` doc-lookup cited in the P3 PR. The desktop effect decoders must carry the new
field (`RustEngineBridge+Composing.swift:343` reads only `payload.display` today). A caret move
does not refetch candidates on either platform.

**Deliberately not adopted**: librime's "arrows navigate candidates only while the caret is at the
end, else move it" (`selector.cc:195-220` — an implicit mode switch); McBopomofo / vChewing's
state split (our bar shows while typing, there is no closed state); syllable-unit caret
(USER wants `k|a`); Shift+arrow marking inside the composition (host selection); Home/End and
forward Delete (USER 2026-09-09 「不需要」); user-recordable chord (USER 「不打算讓使用者更改」);
azooKey-Desktop's no-caret model.

#### Rounds

| PR | Scope | Est. |
|---|---|---|
| P0 | This section + memory + S37 (admin tier, direct to main) | done `f3475194` |
| P1 | engine, one PR of two commits (P1a had no caller of its own, and a `pub(crate)` fn behind a private module cannot ship unused): (a) `derived::display_caret_utf16` — lockstep raw↔display alignment, POJ folds recognised by signature (`hooon` counterexample), premise pinned in phonetics; (b) caret in `Phase`, `MoveCaret` proto + dispatch, caret-aware Append / DeleteBackward / ReplaceLast / TelexKey through `step_composing` / `step_continuous`, `Preedit.caret_utf16` via `combined_display_with_tail`, tests + proptest (TL + POJ double-tap) | done #29 |
| P2 | macOS: `.moveCaret` tier, `ComposingManager.moveCaret`, decoder + executor selection, no refetch on move, read-only 快速齒 row + i18n, tests, S37 | done #30 |
| P3 | Windows mirror: `intent.rs`, `manager.rs`, `composition.rs::select_caret`, read-only row, tests; plus the continuous-config alignment on both desktops | done #31 |

**Found in P3 (2026-09-09), fixed on both desktops in P3, still open on mobile**: after a candidate is
nailed under hanji-first, the nail rendered the prefix with no separator (`台gi`, `CommitContinuous`
carries the continuous config) but every mutator after it — `Append`, `DeleteBackward`, `TelexKey` —
sent the base config, whose spacing flags are unset, so the next keystroke re-rendered `台 gi`.
Roman-first was unaffected (both configs agree). P3 makes every op that renders the composition —
`EnterContinuous` included, whose already-Continuous answer is a snapshot the manager mirrors — send
the continuous config on macOS and Windows (`RustEngineBridge+Composing.swift`,
`windows/.../engine/composing.rs`), `MoveCaret` too; only `Reset` carries none. Pinned by
`move_caret_and_typing_after_a_nail_keep_the_hanji_first_rendering` (Windows) and
`testCommitCandidate_nailed_underHanjiFirst_theNextKeystrokeKeepsThePrefixUnspaced` (macOS).
iOS (`RustEngineBridge+Composing.swift:210-268`) and Android (`ComposingBridge.kt:30-117`) still
send the base config for `Append` / `DeleteBackward` — the same drift, left for the mobile round
(USER 2026-09-04: mobile is the next session).

Codex-named regression risks (all in S37): a caret offset past a region boundary on TSF, a
decoder dropping the new field (caret snaps to the end), candidate refetch skipped after an insert,
mobile byte-parity of `Append` / `DeleteBackward` at the end.

**Gate note (P3)**: the Windows box's App Control policy refused to execute the freshly built
`TaigiKeyboardSettings` test binary twice (`os error 4551`, 應用程式控制原則已封鎖此檔案); the host-side
gate (native tests, gnu clippy, msvc check, DLL exports) and the box's platform crate passed. Merged
on that basis; the box policy is an environment item for the USER.

### Desktop ⇧ + slot key — the 漢羅 commit aimed at a slot (USER-scoped 2026-09-10)

**Status**: MERGED 2026-09-10 — #35 `4db4aa92` (macOS + Windows in one PR). Awaiting real-device dogfood (S38, both platforms).
**Scope**: macOS + Windows only (desktop train). No engine change — no `make build`.

USER 2026-09-10: 「評估使用 shift + qwzx...選字齒 可以反向輸出(臨時拍羅馬字/漢字)某個候選詞的可能性。目前反向輸出只能使用方向齒 +
space 打出來。我希望使用者可以使用shift + 選字齒,跳轉到某個候選詞,反向輸出的可能性」. Fork answers (USER 2026-09-10「1,2,3依照你的建議」):
① one rule under both tone schemes (Standard `⇧Q…⇧;`, Telex `⇧1…⇧9`), accepting that with the window up `⇧;` no longer types `:`
and Telex `⇧1…⇧9` no longer type `!@#$%^&*(` — the punctuation still lands after Enter / Space; ② `;` included, nine slots
alike; ③ no hint drawn in the candidate window; plus 「快速齒加一個快速齒提示,類似「修正...」那個快速齒一樣,使用者不能修改」 —
a read-only 快速齒 row like the composing-caret one.

#### Design (grounded in code, Codex pre-impl reviewed 2026-09-10)

**Keys.** While the candidate window is up, exactly ⇧ on a slot key commits that slot in the script the cell does NOT
stand for — Space (`commitAlternateScript`) aimed at a slot instead of at the highlight, without walking the
highlight there. Every other key keeps its meaning: bare slot keys, Space, Return / ⇧Return, Tab / ⇧Tab, the arrows
(`⇧←` stays the host's selection), the paging keys, Esc, Backspace. Only exactly ⇧: any host modifier beside it is the
host's chord as before. With no window up (or the window switched off) the keys fall through unchanged — `⇧Q` is the
capital the composition takes, `⇧;` the `:` it types.

**Classifier** (`ComposingKeyIntent.swift` slot tier / `intent.rs` Tier 3): `selectCandidateSlot(_:flip:)` /
`SelectCandidateSlot { slot, flip }` (Codex: one case with a flag, not a second case — the controller branch is shared).
Bare → `flip: false`; `CandidateSlotKeySet.shiftedSlot(for:)` / `shifted_slot_for_event` → `flip: true`. Resolved off the
**key code** for the digits (`kVK_ANSI_1…9` / `VK_1…9`) and `;` (`kVK_ANSI_Semicolon` / `VK_OEM_1`), because both
platforms' "unmodified" characters keep Shift (`⇧3` reads `#`, `⇧;` reads `:`); the letters read as their capital and the
existing case fold handles them. US/ANSI positions — a layout whose `;` or number row sits elsewhere resolves the key at
the US position (Codex RISK, accepted: the recorder's ⇧number-row refusal already makes this trade; the bare path stays
character-matched and layout-agnostic).

**Controller**: `commitPresented(at: candidateIndex(forKeySlot:), flip:)` / `commit_candidate(index, flip, …)` — the
same slot→index mapping as the bare keys (§34 unkeyed literal shifts the keys by one), the same "nothing to write" answer
as Space for a cell with one script (羅馬字 mode, the literal), the same auto-space verdict from the commit itself, the
same nailed-partial refresh. 漢羅濫: each cell's own other script, like Space.

**Recorder**: `⇧;` is refused as a chord by key code on both platforms, beside the number row (`ComposingKeyChord.make(_:)`
/ `make_from_press`), so no new binding can be shadowed by the slot tier. A `⇧:` chord recorded before this round is NOT
swept and would be shadowed while the window is up (Codex Q1 RISK, accepted: no default ships on it; the user can re-record).

**快速齒 row**: read-only, after the composing commit rows (`shortcutCommitAlternateScriptInSlot`), drawn from the live
slot set — `⇧Q … ⇧;` / `⇧1 … ⇧9` (Windows `Shift+Q … Shift+;` / `Shift+1 … Shift+9`) — by the recorder rows' renderer.

**Deliberately not adopted**: a second intent case (`commitAlternateScriptInSlot`) — Codex Q5; drawing `⇧` beside the
cells — USER ③; a per-layout "base character with Shift removed" probe (`UCKeyTranslate` / `ToUnicodeEx` with Shift
cleared) — correct but plumbing through both platforms' snapshots for a US-position trade the recorder already makes;
Standard-only enablement — USER ①.

**Codex-named regression risks (all in S38)**: the shifted digit on a keypad (`⇧` + keypad `3` carries no number-row
code → falls through as before); Windows UI-less hosts where `is_showing()` is owner + non-empty content rather than a
visible popup — the same state the bare slot keys already act on.

### Desktop 快速齒 pane — three title-less blocks (USER-scoped 2026-09-10)

**Status**: MERGED 2026-09-10 — #36 `db54a8d5` (macOS + Windows in one PR). Awaiting real-device dogfood (both platforms).
**Scope**: macOS + Windows only (desktop train). Layout + comments; no engine change, no `make build`.

USER asked first for an AUDIT — whether the 快速齒 roster is internally consistent, and whether its
modifier prefixes jump between Option / Shift / Control or land on chords a hand cannot reach. The
audit found the roster consistent: bare key = the main action, `⇧` = that action's reverse
(`⇧Tab`, `⇧Return`, `⇧`+slot), `⌃⌘` / `Ctrl+Alt` = the global switches, with `` ` `` the one
USER-fixed exception (2026-09-09) and `⌥←→` / `Ctrl+←→` the host's own word-jump borrowed once.

**Deliberately NOT changed** (evaluated, rejected — do not re-propose):

- Slot-key order `q w d f z x v y ;` (`slot_key_set.rs:36`) walks a zigzag (slot 7 → 8 jumps from
  the left hand's bottom row to the right hand's top row), but the order IS the keyboard's own
  left-to-right, top-to-bottom scan, so a user can rebuild it by looking down; every cell draws its
  key name. Reordering would spend the muscle memory of the shipped 3.6.7.
- Two spellings of "flip": bare Space flips the highlighted cell, `⇧`+slot flips a named one. The
  only consistent alternative is `⇧Space`, which is harder to press than bare Space.
- Two dismiss rules: the Telex card closes on any key, the symbol menu on Escape. Different object
  kinds — a legend versus a list being chosen from. Both answer Escape.
- Listing the fixed tier (bare arrows, Escape, Backspace, Caps Lock↔ABC, Shift tap) on the pane.

**What shipped**: the ~15 rows, previously one group, are now three title-less blocks — through the
candidates, out of the composition, the switches. Boundaries are the rosters that already exist
(`ComposingAction.groups[0]`, `[1]`, `ShortcutAction.allCases`), so a row cannot move on the pane
without moving in its roster. macOS splits one `Section` into three; Windows inserts two
`cards::section_gap()`, the break four other panes already use. Row order and behaviour identical.

**No headers** (USER 2026-09-10, 「不寫多餘的說明文字」). A titled variant was costed and declined:
the third block holds three switches AND three keys that raise a window, so 「切換」 would be wrong
for half of it — titles would force a fourth block, four new i18n keys in five languages, and a
longer pane. Recorded in memory rather than here.

**Comment corrections carried in the same PR** (all pre-existing, Codex-found): the pane doc argued
AGAINST splitting the list and claimed an `@AppStorage` the view does not use; `ShortcutKeyRecorder`
said the list's seam "is not supposed to show"; `shortcuts.rs` said "five global chords" (six) and
cited `ShortcutSettingsView.swift:578-603` in a 213-line file; `ComposingAction.groups` claimed the
input-source menu draws the groups — nothing but the pane and its roster test consumes them.

**Gate note**: macOS 853 tests / 0 failures; Windows host side green (native tests, gnu clippy, msvc
check, TSF release build + exports, `cargo fmt --check`). `check-box` refused the freshly built
`TaigiKeyboardSettings` test binary again (`os error 4551`, 應用程式控制原則已封鎖此檔案) — the same
gate answered `check-box OK` against `main` earlier in the session, so it is the new binary's hash,
not the change. Same environment item as #35's gate note.

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
