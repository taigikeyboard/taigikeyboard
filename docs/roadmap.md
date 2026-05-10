# Taigi Keyboard — Roadmap

> **Type**: Planning
> **Keywords**: `roadmap`, `planning`, `v3.5.8`, `continuous-input`, `連續輸入`
> **Status**: Active
> **Last updated**: 2026-05-10 (Phase 1 merged; Phase 1b marked N/A after pre-impl audit; Phase 2 implementation in flight)

---

## Summary

- Single source of truth for forward-looking work items
- Active item is v3.5.8 連續輸入 (continuous input) — sole focus this release cycle
- Closed phases / shipped audits live in auto-memory `project_phase_archive.md`; do not relist here
- Deferred items live at the bottom under "Out of scope" — short summaries only, no version scheduling

---

## Active item — v3.5.8 連續輸入 (Continuous Input)

**Status**: Plan approved 2026-05-10
**Plan source-of-truth**: this document. The phase status table below is the authoritative cross-PR handoff record — every PR updates it.
**Maintainer-only auto-memory mirror**: a private Claude Code agent-memory file may exist on the maintainer's machine; it is **not** required for contributors and **not** the source of truth — its contents must be reproducible from this roadmap + git log.

### Why this release

使用者輸入一長串多音節羅馬字 (`taigikhipuann`) 或 TPS 注音符號後,需要逐一從候選詞挑選,挑一個就消耗對應的輸入緩衝區、剩餘部份重新分段查詢——直到緩衝區清空。今天的引擎只支援整段輸入對應單一候選詞,輸入超過一個音節就只會回傳完整字串的少數匹配。連續輸入是和主流 IME (Pinyin / Zhuyin / Khiin / 教育部台語鍵盤) 相比最明顯的功能落差。

### Why all-Rust core

Phase IV-B 已關閉,跨平台演算法皆位於 `engine/`。連續輸入的所有判斷邏輯 (syllabifier、composition state machine、span-local candidate ranking) 都應在 Rust 完成,iOS/Android 僅做 UI 渲染與 keystroke forwarding。

### Tone 1 / Tone 4 影響

- TL/POJ:tone 1 無調符;tone 4 無調符但**必有 -p / -t / -k / -h 入聲韻尾**。toneless lookup 之下 `tai`(可能是調 1/5/7)、`bak`(只能是調 4)在語言上沒有歧義——因為 `bak` 的 `-k` coda 已綁定 tone 4
- 真正的歧義是**邊界切割**:輸入 `taibak` 必須切成 `tai+bak` (T1+T4),不能切成 `tai+ba+k` (`k` 不能單獨成節)——這由 syllable inventory 的 longest-match 處理
- TPS:tone marks `ˊˇˋ˙˫` 是天然的 O(n) terminator;tone 4 標於 entering coda 的小寫 ㄅㄉㄍㄏ
- **結論**:tone 1/4 不需要特殊 UI,但 syllable inventory 必須完整收錄所有合法音節的 toneless 形式 (含 -ptkh 結尾的 tone-4 音節)

---

### 走查範例 (acceptance criteria)

#### Case A — `tsua` (同 toneless key 下不同 syllable_count)

*前提*:fused toneless key 已存在 — 由上游 `notone` CSV stage 提供 (Phase 1b N/A,詳見下方 §Phase 1b)。

```
input = "tsua"
syllabifier::valid_span_endings("tsua", pos=0)  → {3, 4}
lexicon 查兩次:
  span [0,3) toneless="tsu"  → 珠(syll=1), 子(syll=1), ...
  span [0,4) toneless="tsua" → 紙(syll=1), 珠仔(syll=2), ...
                              ↑↑↑ 珠仔 命中 `tl_notone=tsua` (上游 notone stage 已 fused)
候選列表 (按 score 排序):
  [紙(0,4,syll=1), 珠仔(0,4,syll=2), 珠(0,3,syll=1), 子(0,3,syll=1), ...]

點 紙       → consumed=(0,4) → buffer=空 → commit "紙",結束
點 珠仔     → consumed=(0,4) → buffer=空 → commit "珠仔",結束
點 珠       → consumed=(0,3) → pending="a" → 重切 "a" → endings={1}
            → 候選 [仔, 阿, 矮, ...] → 點 仔 → 最終 "珠仔"
```

#### Case B — `taigikhipuann` (連續 4 音節)

```
input = "taigikhipuann"
syllabifier 從 pos=0 出發,endings 含 {3, 5, 8, 13}
候選列表合併:
  [台語齒盤(0,13,syll=4), 台語(0,5,syll=2), 台(0,3,syll=1), ...]
任意路徑都能達到 "台語齒盤":
  路徑 1:點 台語齒盤 → 一步完成
  路徑 2:點 台語 → pending="khipuann" → 點 齒盤 → 完成
  路徑 3:點 台 → 語 → 齒 → 盤 (逐字)
```

#### Case C — `taigi` + nextword 邊界

```
input = "taigi"
syllabifier endings={3, 5} → 候選 [台語(0,5,syll=2), 台(0,3,syll=1), ...]

選「台語」(final): composing 呼 nextword.WordSelected("台語", "tai-gi", trigger_prediction=true)
                  → nextword 接手 post-commit prediction (台語 → 老師/課本/...)

選「台」(中段):    composing 呼 nextword.UpdateLastSelectedWord("台", "tai")
                  → nextword state 更新但不發 effect、不 bump generation
                  → 候選列由 composing → lexicon span [0,2)=`gi` 主導
                  → 候選 [語, 義, 倪, ...]
                  選「語」 → final commit "台語" (整段) → nextword 接手
```

#### Case D — TPS `ㄉㄞˊㄨㄢˊ...`

tone-mark `ˊ` 是 unambiguous terminator → endings 直接由 mark 位置決定,流程同 A/B 但 syllabifier 走 `tps.rs` 路徑。

---

### 既有模組職責 (grounded in actual code, 2026-05-10)

| 模組 | 真實職責 | 不做的事 |
|---|---|---|
| `composing` (`api.rs:17`) | Preedit 狀態機 (`Phase = Idle | Composing { raw }`) + 12 個 preedit intents | **不**做候選查詢,**不**依賴 lexicon |
| `nextword` (`api.rs:42-84`) | 7 個 state-mutating intents + `FilterPredictions` 過濾平台餵進來的 raw predictions + `BoostCandidates` | **不**自己抓 prediction (平台從外部來源餵),**不**碰 lexicon,**沒有** buffer 狀態 |
| `lexicon` | FST prefix lookup + dictionary.bin record reader | — |

**今日 `taigi` 流程**:平台送整段 lexicon → 拿到 toneless key 起頭的 entry (可能拿到「台語」但**不會**拿到「台」單字,因為今天是整段對齊) → 使用者選「台語」→ `composing.SelectSuggestion` + `nextword.WordSelected` → 平台從外部源抓 next-word predictions → 餵 nextword.FilterPredictions 排序 → 更新候選列。

### 架構選擇:continuous-input 留在 composing crate

| | 整合進 nextword (直覺選項) | 留在 composing (採用) |
|---|---|---|
| nextword 今日職責 | filter+score (無 buffer 狀態) | 同左 |
| 加 buffer state | nextword.PersistedState 必須大膨脹 | composing.Phase enum 自然延伸 |
| 加 lexicon coupling | nextword 今天**從不**依賴 lexicon → 跨界 | composing 的 transition.rs 已能 route |
| 程式變動量 | **大** (nextword 重構) | **中** (composing 加 Phase + nextword 邊界協調) |
| 概念清晰度 | nextword 兼任兩種異質職責 | 各司其職,符合 SRP |

**結論**:留在 composing,但用 nextword 既有的 intents (無需修改 nextword) 做 state 同步。

### Continuous-input ↔ nextword 邊界規則

- **中段 commit** (buffer 仍非空):composing 內部呼叫 `nextword.UpdateLastSelectedWord(text, roman, now_ms)` (此 intent 既有,不發 timer/effect/不 bump generation)。候選列由 composing → lexicon span-local 100% 主導,**不**呼叫 nextword.FilterPredictions
- **Final commit** (buffer 清空):composing 呼 `nextword.WordSelected(整段 hanji, 整段 roman, trigger_prediction=true)`。後續行為等同今日 single-token 流程
- **Continuous abort** (reset / mode switch / focus loss):composing 呼 `nextword.ClearForNewComposing(now_ms)`
- **nextword crate 程式完全不變** (只是被新 caller 從另一個 crate 呼叫)

---

## 實作 Phases

### Phase 0 — Roadmap rewrite + memory 建檔 (本 PR)

**Files**:
- `docs/roadmap.md` — 重寫 (本檔)
- `memory/project_v358_continuous_input.md` — 新建,phase status table + active PR 指標
- `memory/MEMORY.md` — 加索引

**規模**:~300 LOC docs,單一 admin PR

---

### Phase 1 — dict.bin v2 + syllable_count

**Files** (post-Phase-1 ground truth — corrects three plan-author file-path errors caught in pre-impl review):
- `engine/lexicon/src/dictionary_reader.rs` — bump `SUPPORTED_VERSION: u32 = 2`, add `u8 syllable_count` after `tl_len`, advance `RECORD_FIXED_PREFIX` 8→9, surface `v1→v2` rebuild guidance in the version-mismatch error (the version check lives here, not in `handle.rs` as the original plan said)
- `dictionary/build/create_dictionary_bin.py` (the canonical builder; the original plan referenced a non-existent `build_dictionary.py`) — bump `VERSION = 2`, encode + verify `syllable_count`
- `dictionary/build/dictionary_records.py` — populate `syllable_count` on `DictionaryRecord` from the existing `_syllable_count(tl)` hyphen-count helper (the original plan referenced `phonetics::syllable::split`, which is not a public Rust API)
- `docs/engine/binary-format.md` — v1→v2 layout diff
- `dictionary/output/dictionary.bin` + iOS / Android assets — regenerated in lockstep so installs do not fail the version check

**Out of scope for this phase**:**不**動 FST trailer (`derivation_type_u8` / `form_u8`)。candidate 消耗計算改用顯式 span (見 Phase 5)。

**Tests**:
- `engine/lexicon/tests/dictionary_reader_v2.rs` — 讀取手刻 v2 binary、驗證 syllable_count + truncated/min-size 邊界
- `engine/lexicon/tests/rejects_v1.rs` — 期待 `LexiconError::InvalidBinary` 含 "v1→v2",且未相關版本 (e.g. v99) 不外洩此字樣

**規模**:S (~150 LOC + 1 binary fixture)

---

### Phase 1b — (N/A) FST 多音節 fused toneless 變體 key — 已由上游 notone stage 提供

**Status**:**N/A** — 原計畫 (2026-05-10 plan-mode approved) 假設「FST 對多音節 entry 的 toneless key 仍保留 syllable separator」,Phase 1 merge 後 pre-impl audit 證實這個前提錯誤,實作不需要。為避免後續 phase 重編號,本節保留為占位並記錄 audit 結論。

**Audit 結論 (2026-05-10)**:

- `dictionary/common/notone.py::remove_tone()` 用 `re.sub(r"[\d\-]", "", text)` —— 同時脫掉 digits 與 hyphens
- `dictionary/common/stages/notone.py:14` 把這個 transform 套到 `tl_num` / `poj_num`,產出 fused `tl_notone` / `poj_notone`
- `dictionary/build/create_fst.py::collect_pairs()` 直接把 fused `tl_notone` / `poj_notone` 欄位 emit 進 FST
- 結果:**多音節 entry 在 FST 上早就有 fused toneless key** (例:珠仔 rowid=146421 的 `tl_notone=tsua` → FST `tl:tsua` 已含此 rowid)
- 輸入端 `engine/phonetics/src/normalization.rs::normalize_input("tsua")` 因 `has_tone_marks=false` 不加 default tone,直接回 `"tsua"`,與 FST key 對齊
- 因此 Phase 3/5 §走查範例 Case A (toneless input `tsua` 同時取得 紙 + 珠仔) 在現行 pipeline 下成立,**不需 builder 改動**

**範圍**:本 N/A 結論限縮於 **Roman toneless lookup 的 separator-stripping** 部分。下列獨立議題 **不**因此被涵蓋,如有需求另開 phase:

- `tl_notone` 仍可保留 `ⁿ` / `o͘`,而引擎輸入 normalize 會折成 ASCII `nn` / `oo` → 形成 key 對不上的子集 (例:存在 `tl:tsiuⁿthuan` 但無 `tl:tsiunnthuan`)
- TPS bopomofo explicit-tone 輸入 (例:`ㄗㄨㄚˋ → tsua2`) 屬 tone-disambiguation,非 separator stripping

**鎖定**:`engine/lexicon/tests/fused_toneless_key.rs` (本 PR 新增) — 用 synthetic FST + 雙 rowid 驗證 fused toneless key 同時 surface 單音節 + 多音節 entry。`dictionary/common/notone.py` + `dictionary/build/create_fst.py` 加 cross-reference docstring/comment 以防上游 regex 無聲改動。

**規模**:本 N/A PR ~50 LOC test + docstring/註解 + roadmap/memory 修正,無 builder 變動。

---

### Phase 2 — TL syllable inventory FST

**Files**:
- `dictionary/output/syllables.fst` (~50 KB)
- `dictionary/build/create_syllables_fst.py` (新檔):從 canonical TL records 抽 TL key → 用 `phonetics::syllable::split` 切音節 → 過 `phonetics::syllable::is_valid` phonotactic 驗證 → 同時產出 numeric (`tai1`、`bak4`) 與 toneless (`tai`、`bak`) 兩形式 → 寫成 fst::Map
- `engine/build-helpers/fst-builder/` — 加新呼叫點

**POJ 處理**:不單獨建 inventory。輸入 POJ → 走 `phonetics::poj::to_tl` 正規化 → 套用 TL inventory。

**Tests**:`engine/lexicon/tests/syllables_fst.rs` — 抽樣 50 合法 / 50 不合法。邊界:`bak` (T4)、`tai` (T1/T5/T7)、`khih` (T4-h)、`m`、`ng`、`tsh`、`oo`、`uainn`。

**規模**:M (~250 LOC build script + ~50 LOC Rust loader + tests)

---

### Phase 3 — 純函數 syllabifier (TL + TPS)

**重要**:syllabifier **不**回傳「single best segmentation」。必須回傳「從 pos 出發所有合法的 syllable-prefix 切點」,讓下游 lexicon 對每個切點做獨立查詢——這是支援 `tsua` 同時命中 `紙(span=4, syll=1)` + `珠仔(span=4, syll=2)` + `珠(span=3, syll=1)` 的關鍵 (fused toneless key 已由上游 notone stage 提供 — 詳見 §Phase 1b)。pure longest-match (khiin 做法) 只會給 span 4 不會給 span 3,於是漏掉「珠」的短切候選;global lattice (librime 做法) 過度設計。我們選**多 span endings + span-local 查詢**的中庸路線。

**Files**:
- `engine/composing/src/syllabifier/mod.rs` (新)
- `engine/composing/src/syllabifier/tl.rs` — 核心 API:
  ```rust
  pub fn valid_span_endings(
      input: &str,
      pos: usize,
      inv: &SyllableInventory,
      max_syllables: usize,
  ) -> Vec<usize>
  ```
  從 pos 做 BFS 走 syllable 圖,深度上限 `max_syllables` (估 6-8)。範例:`input="tsua"`, pos=0 → `{3, 4}`。
- `engine/composing/src/syllabifier/tps.rs` — O(n) 掃 tone-mark terminator (`ˊˇˋ˙˫` + entering coda 小寫 ㄅㄉㄍㄏ)
- `engine/composing/tests/syllabifier_tl.rs`、`syllabifier_tps.rs`

**Reuse**:`engine/phonetics/src/syllable.rs::split`、`is_valid`、`tps.rs` tone-mark 集合。

**Test 矩陣**:
- `tsua` → `{3, 4}` ← 關鍵 case
- `tai` → `{3}`
- `taibak` → `{3, 6}`
- `khihthau` → `{4, 8}`
- `taixyz` → `{3}` (tail 不消耗)
- `taigikhipuann` → `{3, 5, 8, 13}`
- TPS bopomofo 串 → 累進 endings

**Performance**:`max_syllables=8` 限制下 O(n × 3 × 8) worst case,可忽略。

**規模**:M (~450 LOC + tests)

---

### Phase 4 — `Phase::Continuous` 加入 composing engine + nextword 邊界協調

**Files**:
- `engine/composing/src/api.rs:17` — 擴充 `Phase` enum:
  ```rust
  Phase::Continuous { raw: String, committed: Vec<CommittedSegment> }
  CommittedSegment { display_text, raw_span: (usize, usize), raw_text, syllable_count: u8 }
  ```
- `engine/composing/src/transition.rs` — 新增 `Intent::EnterContinuous` / `CommitAtPos` / `ResetContinuous` / `BackspaceContinuous` 處理:
  - **Commit**:取得 candidate `consumed_span`,將 `raw[start..end]` 包成 `CommittedSegment` 推入 committed,`pending = raw[end..]`,重新呼叫 syllabifier 切 pending
  - **Backspace**:committed 非空 → pop 最後一個 segment,`raw = popped.raw_text + raw`,重新切;否則退出 Continuous mode
- `engine/composing/src/handle.rs` — 確保 `Phase::Continuous` 通過 Send / Sync 編譯期斷言
- `engine/composing/tests/continuous_phase.rs` (新)

**Generation / revision**:沿用既有 `request.generation` 模型,**不**新增 snapshot_id。

**Nextword 邊界協調** (本 phase 必做):
- 中段 commit:composing 構造 `NextWordRequest { method: UpdateLastSelectedWord(...) }` proto,經 `nextword::EngineHandle::instance().handle(&req, config, generation)` 公開 API 進入 (既有 intent)
- Final commit (buffer 空):構造 `NextWordRequest { method: WordSelected(...) }` (整段 hanji + 整段 roman, trigger_prediction=true)
- Continuous abort:構造 `NextWordRequest { method: ClearForNewComposing(...) }`
- **重要**:`nextword::api::Intent` enum 是 `pub(crate)` (`api.rs:42`),composing **不能**直接 reference;一律走 proto + `EngineHandle` 公開 API
- nextword crate 程式**完全不變**;composing 的 Cargo.toml 加 `nextword = { workspace = true }` dependency
- Test 必須驗證中段 commit 後 `nextword.snapshot().current_generation` 不變 (因 UpdateLastSelectedWord 不 bump generation)

**規模**:M-L (~550 LOC + tests)

---

### Phase 5 — Span-local candidate fetch (lexicon + ranking)

**演算法**:對 `valid_span_endings(input, pos)` 的每個 end,以 `input[pos..end]` 作為 toneless key 查 FST。同 toneless key 下不同 syllable_count 的 entry 都會回 (由 dict.bin syllable_count 區分),所以 `tsua` 在 span [0,4) 同時拿到 `紙(syll=1)` 和 `珠仔(syll=2)`;span [0,3) 拿到 `珠(syll=1)`。最後合併排序。

**Files**:
- `engine/lexicon/src/api.rs` — 新增:
  ```rust
  pub fn fetch_candidates_for_endings(
      input: &str,
      pos: usize,
      endings: &[usize],
      mode: KeyMode,
      filter: Filter,
  ) -> Vec<RawCandidate>
  ```
- `engine/lexicon/src/search.rs:124+` — 新增 `lookup_at_span` 變體
- `engine/composing/src/dispatch.rs` — `EnterContinuous` / `FetchAtPos` 路徑串接
- `engine/ranking/src/score.rs` — 新增 `ContinuousScore`:`score = freq × (1.0 + 0.1 × (syll-1)) × user_freq_boost`,無 bigram

**Each Candidate carries**:
```
consumed_span: (start: u32, end: u32)
syllable_count: u8
display_text: String
score: f32
form: u8  // numeric / notone / abbrev / hanzi
```

**Tests**:`engine/lexicon/tests/span_local_fetch.rs`
- `tsua` → 候選列必含 {(紙, span=(0,4), syll=1), (珠仔, span=(0,4), syll=2), (珠, span=(0,3), syll=1)}
- `taigikhipuann` → 必含 {(台, syll=1), (台語, syll=2), (台語齒盤, syll=4)} (視 dict 是否有 4-syll 詞而定)
- `taixyz` → 只回單音節候選 (因 endings={3})

**規模**:M (~400 LOC + tests)

---

### Phase 6 — Proto + dispatch RPC

**Files**:
- `engine/protos/proto/composing.proto:97` — `ComposingResponse` 加新 oneof:
  - `EnterContinuousResp` / `FetchAtPosResp` / `CommitContinuousResp` / `ResetContinuousResp`
  - 每 response 帶:`committed_display`、`pending_display`、`selected_span`、`candidates: repeated CandidateMessage` (含 `consumed_span` + `syllable_count`)
- 對應 `ComposingRequest` 加 oneof:`EnterContinuous { raw, mode }` / `FetchAtPos { position }` / `CommitContinuous { position, candidate_id }` / `ResetContinuous {}`
- `engine/scripts/gen-platform-protos.sh` — 必須一併更新 (per `feedback_proto_gen_script.md`)。.proto 改動後 iOS .pb.swift + Android .java bindings 也要 regenerate 並 commit
- `engine/protos/build.rs` — 同上更新
- `engine/composing/src/dispatch.rs` — 路由

**Cross-platform parity 設計**:UI **不**重新計算 committed/pending 顯示文字——這些字串由 engine 在 response 中直接給出,iOS/Android 只 render。

**規模**:M (~400 LOC proto + bindings + dispatch + tests)

---

### Phase 7 — iOS UI 整合

**Files**:
- `ios/Sources/Composition/CompositionRoot.swift` — 新增 Continuous mode 偵測:當 raw input 長度 ≥ 閾值 (估 ~3 chars) 且包含合法音節邊界時,呼叫 `EnterContinuous`
- `ios/Sources/Autocomplete/...` (KeyboardKit AutocompleteProvider 子類) — render candidate strip;tap → `CommitContinuous(position, candidate_id)`,刷新候選
- 新增 ComposingTextStrip 元件 (在 keyboard 上方一行) — 顯示 `committed_display + pending_display` (per MOE app UX 規範)
- Backspace handler:Continuous mode 改呼 `BackspaceContinuous`

**iOS-specific gotchas**:
- KeyboardKit setMarkedText / commitText 對應:committed segments 用 `commitText`,pending 用 `setMarkedText`
- Settings 切換 (TL/POJ/TPS) mid-composition → `ResetContinuous`

**規模**:M-L (~500 LOC Swift + tests)

---

### Phase 8 — Android UI 整合

**Files**:
- `android/.../ime/core/TaigiKeyboard.kt` — 同 iOS,偵測 + 路由到 `EnterContinuous`
- `android/.../ime/text/smartbar/SmartbarManager.kt` — render candidate strip + tap handler
- `android/.../ime/core/InputView.kt` — 加 ComposingTextStrip composable

**Android-specific gotchas**:
- `setComposingText` for pending、`commitText` for committed
- 旋轉 / focus loss → `onFinishInput` / `onStartInput` → `ResetContinuous`
- IME mode swap → 同上 reset
- `setComposingText("")` 應在 commit 後立即清空 marked text 區

**規模**:M-L (~500 LOC Kotlin + tests)

---

### Phase 9 — Corner-case dogfood + cleanup

**Test 矩陣**:
1. Tone 1/4 邊界:`taibak` / `bakkiann` / `khihthau` / `taigikhipuann`
2. TPS 全部音節:`ㄉㄞˊㄨㄢˊㄉㄞˊㆣㄧˋㄌㄛˊㄇㄚˋㆢㄧ˫` → 「臺灣台語羅馬字」
3. Mid-composition mode switch (TL ↔ POJ ↔ TPS) → reset 一致
4. Focus loss → buffer 清空,no zombie state
5. Stale candidate tap → response 帶舊 generation,平台忽略
6. Partial invalid tail:`taixyz` → 切出 `tai`,`xyz` 留在 pending
7. Uppercase / hyphen / apostrophe:`Tai-gi`、`pe̍h-ōe-jī`、`a'au`
8. Paste / emoji during composition → `ResetContinuous` 後 commit
9. Punctuation / space / Enter → commit boundary
10. Hardware keyboard arrow keys → reset (本輪不支援 mid-buffer 編輯)

**Round-A/B/C dogfood**:在 iPhone + Android 實機跑 S1 / S2 / S3 (per `feedback_perf_gate.md`)。

**Codex sandwich**:每個 phase PR 都要 pre-impl + post-impl Codex review (per `feedback_codex_review_sandwich.md`)、merge 前跑 `/codex-pr-review` (per `feedback_pr_bot_catches_codex_misses.md`)。

**規模**:S (~100 LOC + dogfood notes)

---

## 最佳實踐對齊 (rules/ + IME 主流)

### Rust (Phase 1-6) — 依 `rules/rust-best-practices.md`

- Workspace 結構:`composing/syllabifier/{tl,tps}.rs` 子模組沿用既有 crate-level pattern;**不**新建獨立 crate
- Crate-level `#![forbid(unsafe_code)]` 沿用
- `thiserror` for errors、`prost` for proto、`Mutex<Engine>` + `OnceCell` singleton 全沿用既有模式
- Generation 模型 (無 snapshot_id):對齊 `nextword/src/api.rs:26-33` wrapping_add 設計
- syllabifier 是 pure fn,可獨立 unit test

### iOS (Phase 7) — 依 `rules/ios-guidelines.md` + `rules/ios-architecture.md`

- KeyboardKit 隔離:Continuous mode 偵測在 `Composition/` 層,KK API 不滲入
- 新檔需 user 手動加 Xcode target (per `feedback_xcode_manual.md`)
- 不做資料夾級重命名 (per `feedback_pbxproj_sync_gap.md`)
- AutocompleteProvider 走既有介面,**不**自製 popup

### Android (Phase 8) — 依 `rules/android-guidelines.md` + `feedback_gradle_editable.md`

- §1 shared-core candidate:Kotlin 側只做 IME service 路由與 Compose 渲染,所有邏輯在 Rust
- §7 Compose:Item 4 已 retire 1637 LOC legacy View → Continuous UI 直接 Compose,不新建 `View` 子類
- §8 IME-specific:`setComposingText` for pending、`commitText` for committed
- 不重新打開 `project_ime_window_arch.md` 已 closed 的 inset hazard

### Cross-platform alignment — 依 `rules/cross-platform-alignment.md`

- §1b parity-correction tier:Phase 7 + 8 必須在同一 release tag 之前都合進 main,**不**單平台先發
- Engine 提供顯示字串 (`committed_display` / `pending_display` / `selected_span`),平台只 render

### IME 主流做法

| 主流做法 | 來源 | 本 plan 對應 |
|---|---|---|
| Buffer + committed segments + raw_char_count | khiin-rs `BufferMgr` (`buffer_mgr.rs:44-66, 1096-1129`) | Phase 4 `Phase::Continuous { raw, committed }` |
| 變動 syllable_count 候選同列 | librime `UnionTranslation` (`script_translator.cc:111-178`) | Phase 5 span-local fetch 合併 |
| Segment status state machine | librime `Segment::{Void, Guess, Selected, Confirmed}` (`context.h:101-105`) | Phase 4 `CommittedSegment` 簡化版 |
| Commit + truncate + re-segment | khiin-rs `focus_candidate` (`buffer_mgr.rs:1096-1129`) | Phase 4 transition.rs |
| Visible composing strip | MOE Tailo `composingTextString` UI affordance | Phase 7/8 ComposingTextStrip |
| T4 entering coda 由 final consonant 推斷 | khiin-rs `converter.rs:351-363` | Phase 2 inventory 預收 -ptkh 結尾即可 |
| Rule-based fused toneless | librime `algebra.cc:107-140` derive rules | (N/A — 本 repo 由上游 notone stage 預先 fuse `tl_notone` / `poj_notone`,FST 直接繼承) |
| Tone marks 為 unambiguous terminator | TPS 設計本身 | Phase 3 tps.rs O(n) scan |
| frequency-derived cost + length / syllable bias 排序 | khiin-rs `khiin/src/data/segmenter.rs` (cost = ln(1/p) / word_len_bias × syllable_bias 區塊) | Phase 5 `ContinuousScore` 簡化版 (`freq × (1.0 + 0.1 × (syll-1))`) |
| Generation counter for stale async | librime + nextword 既有 | 沿用 |

**刻意不採用** (YAGNI):
- librime full SyllableGraph + Translator pipeline → 過度工程
- librime Spelling Algebra full pipeline → 不需要;fused toneless 已由上游 notone stage 提供 (§Phase 1b N/A)
- librime SchemaYAML 動態載入 → 三模式硬編碼
- khiin-rs Continuous/Classic/Manual 三 InputMode → 直接做 Continuous,不暴露切換
- MOE NailCandidate 雙向 commit/decommit → forward-only

---

## Phase 排序 + 狀態追蹤 (cross-PR handoff,authoritative)

**This is the authoritative status record across PRs.** 開 PR 時把對應 row Status 改為 `In progress (PR #N)`;PR merge 前同一個 diff 內把 Status 改為終局 `Merged in PR #N` (commit SHA 可由 GitHub / `git log` 推出,不需在表格內維護)。每個 phase PR 完整自我更新它那一 row 的 Status,不依賴後續 PR 補登。

| Phase | 規模 (LOC 估) | Blocking 後續 | Visible to user | Status |
|---|---|---|---|---|
| 0 — roadmap rewrite | ~300 docs | 1, 1b, ... | (admin) | **Merged in PR #248** |
| 1 — dict.bin v2 + syllable_count | ~150 + tests | 5 | No | **Merged in PR #249** |
| **1b — FST fused toneless key** | (admin-tier ~50 LOC) | — (premise 失效,不再 block 後續) | No | **N/A — Merged in PR #250** (squash `2c826b96`); invariant locked by `engine/lexicon/tests/fused_toneless_key.rs`,upstream contract on `notone.py::remove_tone` |
| 2 — syllable inventory FST | ~300 + tests | 3 | No | In progress (PR #251) |
| 3 — syllabifier (TL + TPS) | ~450 + tests | 4, 5 | No | Pending |
| 4 — `Phase::Continuous` + nextword 邊界 | ~550 + tests | 6 | No | Pending |
| 5 — span-local candidate fetch | ~400 + tests | 6 | No | Pending |
| 6 — proto + dispatch RPCs | ~400 + bindings | 7, 8 | No | Pending |
| 7 — iOS UI 整合 | ~500 Swift + tests | — | **Yes** | Pending |
| 8 — Android UI 整合 | ~500 Kotlin + tests | — | **Yes** | Pending |
| 9 — dogfood + corner-case fixes | ~150 + dogfood | — | (polish) | Pending |

**Status legend**:Pending / In progress (PR #N) / Merged in PR #N / Blocked (reason)

**Active PR pointer**:Phase 2 (TL syllable inventory FST) in flight as PR #251。Phase 0 merged in PR #248,Phase 1 merged in PR #249,Phase 1b N/A merged in PR #250 (squash `2c826b96`)。

**總計**:11 個 PR (其中 1b 已降級為 admin-tier N/A PR),加總約 4500 LOC + tests。多數 hand-reviewed code PR 落在 200-550 LOC (Phase 4 ~550 是上限);Phase 6 的 generated bindings (proto → .pb.swift / .java) 不計入 review size。

**v3.5.8 release tag** = Phase 1-9 全部完成後 cut。**不**做中途 partial release (per `feedback_no_slice_toggles.md`,no fallback toggle;Continuous 是 direct swap)。Phases 1、2 storage prep 可在 Phase 3 開工前先合進 main——不影響使用者行為 (Phase 1b 已 N/A)。

---

## Per-round gates (每個 phase PR 都跑)

- 每 PR 啟動只靠本 `docs/roadmap.md` Active item 章節 (含 Phase 排序狀態表) + git log 即可重建脈絡 (per `feedback_round_hygiene.md`)
- Codex pre-impl + post-impl review (per `feedback_codex_review_sandwich.md`)
- 每 PR merge 前跑 `/codex-pr-review` (per `feedback_pr_bot_catches_codex_misses.md`)
- iOS / Android 實機 dogfood S1/S2/S3 (per `feedback_perf_gate.md`)
- 不引入 fallback toggle (per `feedback_no_slice_toggles.md`)
- pbxproj 仍 user-only (per `feedback_xcode_manual.md`),build.gradle 可直接編輯 (per `feedback_gradle_editable.md`)
- **每 PR 必須在自己的 diff 裡完整自我更新「Phase 排序 + 狀態追蹤」表的對應 Status** — 開 PR 時改成 `In progress (PR #N)`;PR merge 前(同一個 diff)再改成終局 `Merged in PR #N`。不依賴後續 PR 補登,不依賴外部 auto-memory

---

## Verification (release-level)

最終 v3.5.8 release verification (Phase 9 dogfood):
- 在 iPhone + Android 實機,連續輸入 10 句生活台語 (混 TL / TPS / hanji)
- 對照 MOE Taigi APK 同類流程,確認 UX 一致或更好
- 測 tone 1/4 邊界 case 全綠
- 測 mode switch / focus loss / stale tap / paste edge cases 全綠
- 無 keyboard dismiss、無 leak (per `feedback_perf_gate.md`)

---

## Out of scope (deferred)

下列項目 v3.5.8 **不做**,等 v3.5.8 dogfood 結果決定優先序後再開新 round:

### Borrow librime spelling-algebra (full pipeline)
**Source**: 2026-05-05 librime architecture comparison
**Status**: §Phase 1b 確認 N/A — fused toneless key 早就由上游 `dictionary/common/notone.py::remove_tone()` 在 CSV 階段提供。完整 librime pipeline (declarative `spelling_rules.toml` + Rust rule engine + FST trailer `derivation_type_u8` / `form_u8` 兩位元組 + ranking credibility multiplier) 留作後續候選。詳見 git 歷史 `docs/roadmap.md` 在 commit `52e15d83` 之前的 Item 2 內容。

### Continuous-input nextword bigram 整合
連續輸入中段排序加 nextword bigram boost (給 buffer-driven candidate × 1.3 if 同時在 nextword top-K)。本 phase 已預埋 state sync,可無痛上。

### Back-edit / undo / cursor 任意位置編輯
v3.5.8 只支援 forward-only commit;backspace 限「pop committed segment」。未來若 dogfood 顯示需求再加。

### FST trailer 兩位元組 (`derivation_type_u8` / `form_u8`)
原 roadmap Item 2 + Item 3 step 1 設計;v3.5.8 改用顯式 `consumed_span` 不需要 trailer。若未來引入完整 spelling-algebra pipeline 才一併 ship。

---

## Closed phases / shipped audits

歷史條目見 auto-memory `project_phase_archive.md`:
- Roadmap Item 1 (Project Structure & File Naming Cleanup) — CLOSED 2026-05-06 (PRs #212-#215)
- Roadmap Item 4 (Android UI Compose migration) — CLOSED 2026-05-08 (PRs #227-#231)

<!-- 新 active item 加在 Active item 章節之前;deferred 加在 Out of scope 章節 -->
