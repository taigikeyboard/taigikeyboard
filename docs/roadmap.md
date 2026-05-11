# Taigi Keyboard — Roadmap

> **Type**: Planning
> **Keywords**: `roadmap`, `planning`, `v3.5.8`, `continuous-input`, `連續輸入`
> **Status**: Active
> **Last updated**: 2026-05-10 (Phase 0/1/1b/2/3 merged; next = Phase 4 `Phase::Continuous` + nextword 邊界)

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

> **設計 vs. 原計畫(2026-05-10 pre-impl Codex co-decide,記在這以避免 review churn)**:
> 1. **Nextword 同步走 Effect-based handshake,不從 composing 直呼 `nextword::EngineHandle::instance().handle()`**——保留 `composing/handle.rs:54-58` 的 lock-order 紀律,composing 不知 nextword 存在,proto 不在 transition 內構造。`composing/Cargo.toml` 不加 nextword dep。
> 2. **Backspace 折回既有 `Intent::DeleteBackward`,不新開 `BackspaceContinuous` Intent**——避免一個使用者動作有兩條 Intent 路徑。
> 3. **Intent 命名對齊 Phase 6 proto**:`CommitContinuous`(不是 `CommitAtPos`)。
> 4. **Phase::Continuous 下既有 12 個 Intent 全部明寫行為**(完整實作 7 個、保守 snapshot no-op 5 個);不留 fall-through、不 `unimplemented!()`。Phase 7 平台整合若不接受某 no-op 行為,改名相應 test 即可。

**Proto** (`engine/protos/proto/composing.proto:117`) — `Effect.kind` oneof 加三個 nextword-sync variant (tags 8/9/10):
- `NextWordUpdateLastSelectedWord { text, roman }` — 中段 commit 後平台展開為 `NextWordRequest { method: UpdateLastSelectedWord }`
- `NextWordWordSelected { text, roman, trigger_prediction }` — final commit (pending 空) 後平台展開為 `WordSelected`
- `NextWordClearForNewComposing {}` — `ResetContinuous` / `Reset` 在 Continuous phase 觸發,平台展開為 `ClearForNewComposing`

`now_ms` clock 由平台 FFI shim 注入(沿用既有 nextword 呼叫慣例),不在 Effect payload。iOS `.pb.swift` + Android `.java` bindings 一併 regen 並 commit(per `feedback_proto_gen_script.md`)。`engine/scripts/gen-platform-protos.sh` 不需改動(只改 .proto)。

**Rust** (`engine/composing/src/api.rs:17`) — 擴充 `Phase` enum + 加新 struct:
```rust
pub enum Phase {
    Idle,
    Composing { raw: String },
    Continuous {
        raw: String,                       // un-committed pending tail
        committed: Vec<CommittedSegment>,  // ordered committed segments
    },
}

pub struct CommittedSegment {
    pub display_text: String,    // e.g., "紙"
    pub raw_text: String,        // e.g., "tsua"
    pub raw_span: (usize, usize),// byte offsets in original raw input
    pub syllable_count: u8,
}
```

**3 個新 Intent** (`engine/composing/src/api.rs` Intent enum):
- `EnterContinuous` — 從非空 `Composing { raw }` 轉到 `Continuous { raw, committed: [] }`,emit 0 effects (preedit 內容不變);Idle/Continuous/empty-Composing 下 snapshot no-op
- `CommitContinuous { display_text: String, consumed_bytes: usize, syllable_count: u8 }` — 取 `pending[..consumed_bytes]` 包成 segment 推入 committed,`pending = pending[consumed_bytes..]`:
  - **mid-commit** (pending 還有剩):留在 Continuous,emit `[CommitTextReplacingPreedit(segment), UpdatePreedit(new_pending), NextWordUpdateLastSelectedWord, PerformAutocomplete]`
  - **final-commit** (`consumed_bytes == pending.len()`,新 pending 為空):退出到 Idle,emit `[CommitTextReplacingPreedit(segment), ResetAutocomplete, ResetAutocompleteContext, NextWordWordSelected]`
  - 邊界錯誤 (out-of-range / 0 / non-char-boundary / empty display) → snapshot no-op,不 panic
- `ResetContinuous` — 退出到 Idle,emit `[ClearPreeditWithoutCommit, ResetAutocomplete, NextWordClearForNewComposing]`

**12 個既有 Intent 在 `Phase::Continuous` 下的行為**:
- **完整實作**:
  - `Reset` 等同 `ResetContinuous`
  - `Append { ch }` / `AppendHyphen`:append 到 pending 尾,emit `[UpdatePreedit, PerformAutocomplete]`;committed 不動
  - `ReplaceLast`:operate on pending 尾;若收斂為 empty pending + empty committed → exit to Idle (Codex post-impl finding #2)
  - `DeleteBackward` 折回 backspace:
    - pending 非空 → drop last char of pending;空-空 → exit Idle 並 emit document-side backspace
    - pending 空 & committed 非空 → pop 最後 segment,emit `[DeleteBackwardFromDocument × N, NextWordCorrection, UpdatePreedit, PerformAutocomplete]`(N = popped display 字元數;若還有 committed 殘留則 NextWordCorrection = `NextWordUpdateLastSelectedWord(prev_segment)`,否則 = `NextWordClearForNewComposing` — Codex post-impl finding #1)
  - `SetSelectedCandidateIndex` / `QueryState`:snapshot
- **Reset-then-apply** (Codex post-impl finding #3,避免靜默丟字):
  - `Start { text }`:drop continuous + enter_composing(text)
  - `SelectSuggestion { text }`:exit Idle + commit text 直接上屏 (text 空時降為 ResetContinuous)
  - `CommitPreeditThenInsertExternal { text }`:exit Idle + commit (pending derived ++ text) (text 空時 noop)
- **Snapshot no-op** (沒帶 text、語意確定無資料遺失):`CommitDerived` / `CommitRaw` — Continuous mode 已透過 mid-commit 自動 commit,這兩個 Intent 在 Continuous 沒有對應動作

**Test observability** (`engine/composing/src/api.rs` Engine):新增 `pub fn snapshot_state(&self) -> EngineState` 讓 `tests/continuous_phase.rs` 可 inspect `Phase::Continuous` 內的 committed/pending(`ComposingResponse` 的 committed/pending 欄位延到 Phase 6 才加)。

**Tests** (`engine/composing/tests/continuous_phase.rs`):effect 順序 pin 死位置 (不是 membership)。涵蓋 enter / mid-commit / final commit / DeleteBackward 折回 / Reset / 12 Intent matrix。

**Files (確定 touch)**:
- `engine/protos/proto/composing.proto` (3 個新 Effect variants + 3 個新 message)
- `ios/Sources/TaigiKeyboard/Engine/Generated/Composing.pb.swift` (regen)
- `android/app/src/main/java/com/siansiansu/taigikeyboard/engine/proto/*.java` (regen)
- `engine/composing/src/api.rs` (Phase / Intent / CommittedSegment / snapshot_state)
- `engine/composing/src/transition.rs` (3 新 Intent + 12 既有 Intent 的 Continuous-phase 分支 + 3 個新 Effect constructor)
- `engine/composing/tests/continuous_phase.rs` (新)

**規模**:~600 LOC handcoded(Rust + tests)+ generated bindings(per roadmap.md:449 不計 review size)

---

### Phase 5 — Span-local candidate fetch (lexicon + ranking)

**演算法**:對 `valid_span_endings(input, pos)` 的每個 end,以 `input[pos..end]` 作為 toneless key 查 FST。同 toneless key 下不同 syllable_count 的 entry 都會回 (由 dict.bin syllable_count 區分),所以 `tsua` 在 span [0,4) 同時拿到 `紙(syll=1)` 和 `珠仔(syll=2)`;span [0,3) 拿到 `珠(syll=1)`。最後合併排序。

**Files**:
- `engine/lexicon/src/continuous.rs` (新模組) — 新增 `pub struct RawCandidate { consumed_span, syllable_count, display_text, score, form }` + 公開入口:
  ```rust
  pub fn fetch_candidates_for_endings(
      input: &str,
      pos: usize,
      endings: &[usize],
      enabled_sources_bitmask: u32,
      user_freq_boost: f32,
      prefix_index: &PrefixIndex,
      dict: &DictionaryReader,
  ) -> Vec<RawCandidate>
  ```
  Codex pre-impl review (2026-05-10) confirmed:**(a) drop `mode: KeyMode` 參數**(syllabifier 已保證輸入是 canonical TL ASCII;TL/TPS 都走 `tl:` 前綴,POJ 此 phase 不需要,YAGNI per `feedback_no_future_planning.md`);**(b) `RawCandidate` 與實作放新模組 `continuous.rs`,不塞進 `api.rs`**(`api.rs` 是 proto-shaped bridge,實作層獨立模組更 cohesive);**(c) `lookup_at_span` 內聯在 `continuous.rs`**,不污染 `search.rs` 的既有 IndexSet+exact+prefix 流程。
- `engine/lexicon/src/lib.rs` — `pub mod continuous;` + `pub use continuous::{fetch_candidates_for_endings, RawCandidate, FORM_NOTONE};`
- `engine/lexicon/Cargo.toml` — 新增 `ranking = { workspace = true }` (lexicon → ranking 單向 dep,無 cycle:ranking 不 dep lexicon)
- `engine/ranking/src/score.rs` — 新增 `pub fn calculate_continuous_score(freq: u32, syllable_count: u8, user_freq_boost: f32) -> f32`,公式 = `freq × (1.0 + 0.1 × max(0, syll-1)) × user_freq_boost`,無 bigram。`ranking/src/lib.rs` 加 re-export。`calculate_score` (6-component additive `ScoreBreakdown`) 不動 — 連續輸入用獨立公式,不重用既有 ranking 公式 (Codex Fork 4 ACCEPT)。
- ~~`engine/composing/src/dispatch.rs`~~ — **不在 Phase 5**:`EnterContinuous` / `FetchAtPos` 的 proto request/response 載體在 Phase 6 才存在;Phase 5 預先加 `Intent::FetchAtPos` 跟 escape hatch 會多一層 throw-away API,Phase 6 又要拆。dispatch wiring 整段移到 Phase 6 (Codex Fork 1 ACCEPT)。

**Each Candidate carries**:
```
consumed_span: (start: u32, end: u32)
syllable_count: u8
display_text: String
score: f32
form: u8  // 1 = notone (Phase 5 唯一支援);0/2/3 (hanzi/numeric/abbrev) reserved for Phase 6+
```

`form = 1` hard-code 因 span-local lookup 永遠走 `tl:<toneless>` 前綴 (見 §Phase 1b),`DictionaryRecord` 沒有 form 欄位 (`engine/lexicon/src/dictionary_reader.rs:38-51`)。Phase 6 proto 落地時若需 multi-form,再從 prefix-index key 前綴推導 (Codex Fork 5 ACCEPT)。

**Tests**:`engine/lexicon/tests/span_local_fetch.rs`
- `tsua` → 候選列必含 {(紙, span=(0,4), syll=1), (珠仔, span=(0,4), syll=2), (珠, span=(0,3), syll=1)}
- `taigikhipuann` → 必含 {(台, syll=1), (台語, syll=2), (台語齒盤, syll=4)} (視 dict 是否有 4-syll 詞而定)
- `taixyz` → 只回單音節候選 (因 endings={3})
- 排序:同 span 內以 `score` desc 排序;跨 span 結果合併後一起 desc 排序 (test fixture 用 frequency 控制預期順序)
- Hermetic fixture builders 沿用 `tests/common/mod.rs::build_tkdb_v2` (Phase 1 引入) + 內聯 fst::SetBuilder pattern (見 `tests/syllables_fst.rs:186-207`)

**規模**:M (~400 LOC + tests)

---

### Phase 6 — Proto + dispatch RPC

> **Design vs. original spec(2026-05-10 pre-impl Codex co-decide,record at branch `v358-phase6-proto-dispatch`)**:
> 1. **`ComposingResponse` 不改 oneof**:本來計畫把 body 改成 4-variant oneof (`EnterContinuousResp` / `FetchAtPosResp` / ...);實際採 Codex Fork A3 — 在現有 flat `ComposingResponse` 多加一個 `optional ContinuousResponse continuous = 5;`。其他 12 個 method 全部 backward-compatible (proto3 zero-default safe)。
> 2. **`EnterContinuous {}` 無 payload**:原計畫 `EnterContinuous { raw, mode }`;實作對齊 Phase 4 嚴格前置條件 (transition.rs:484-490 — 只有 `Phase::Composing { raw }` 非空才轉),平台須先 `Start`/`Append` 再 `EnterContinuous`。`mode` 已在 `Request.config_snapshot.input_mode`,redundant。
> 3. **`CommitContinuous { display_text, consumed_bytes, syllable_count }`**:stateless,直接搬 Phase 4 Intent 形狀,不引入 server-side `last_candidates` 快取。平台契約:commit 時三個欄位必須複製對應的 `CandidateMessage.consumed_span_end` / `display_text` / `syllable_count`。
> 4. **`FetchAtPos { position }` 唯一響應 `ContinuousResponse`**:其他三個連續輸入 method (`EnterContinuous` / `CommitContinuous` / `ResetContinuous`) leave `continuous = None`(state-changing 走 Effect,平台後續 issue `FetchAtPos` 拿候選)。
> 5. **`ContinuousResponse` 只帶 `repeated CandidateMessage candidates`**:`pending_display` 不重複 (已在 `ComposingResponse.preedit.display_text`);`committed_display` 不重複 (committed segments 早已透過 Phase 4 mid-commit Effects 寫進文件,MOE-style UX 規格 `transition.rs:8-13`)。
> 6. **TPS → TL key mapping at dispatch boundary**:Phase 5 module 限制 `lexicon::fetch_candidates_for_endings` 只接受 canonical TL ASCII,Phase 6 在 `composing/src/dispatch.rs::build_keys_tps` 做 per-syllable `phonetics::tps_to_tl` + 累加 fused toneless key,對應 Phase 1b 的 fused FST 儲存。lexicon 加新 mode-agnostic 入口 `fetch_candidates_for_keys`,既有 `fetch_candidates_for_endings` 改為 thin wrapper。
> 7. **`syllable_inventory` 接入 lexicon Install**:`InstallRequest` 加 `string syllable_inventory_path = 5;`(optional,空字串視為未提供);`EngineState.syllable_inventory: Option<SyllableInventory>` Phase 7 / 8 平台 bundle 後填入。Phase 6 沒平台路徑時 `FetchAtPos` 回傳空 candidates(graceful degrade)。

**Files (確定 touch)**:
- `engine/protos/proto/composing.proto` — 4 個新 request message (`EnterContinuous` / `FetchAtPos` / `CommitContinuous` / `ResetContinuous`)、`ContinuousResponse` + `CandidateMessage`、`ComposingResponse.continuous` optional 欄位、`ComposingRequest.method` 30s 家族 4 個 variant (tags 30-33)
- `engine/protos/proto/lexicon.proto` — `InstallRequest.syllable_inventory_path` 第五欄位
- `ios/Sources/TaigiKeyboard/Engine/Generated/composing.pb.swift` + `lexicon.pb.swift` — regen
- `android/.../engine/proto/{Composing,Lexicon,...}.java` — regen + 12 個新 message class
- `engine/composing/src/api.rs` — `Intent::FetchAtPos { position: u32 }` 加進既有 enum
- `engine/composing/src/dispatch.rs` — 4 個新 method decode + `Intent::FetchAtPos` 在 dispatch 層 short-circuit (lexicon state 跨 crate 取得;TL / TPS 各自 key 構造);新增單元測試
- `engine/composing/src/transition.rs` — `Intent::FetchAtPos` defensive snapshot arm (production 走 dispatch);7 個既有 `ComposingResponse` constructor 補 `continuous: None`
- `engine/composing/tests/dispatch_continuous.rs` — decode + degraded-path 整合測試
- `engine/lexicon/src/paths.rs` — `LexiconPaths.syllables_fst: Option<PathBuf>` + 5-arg `validated()`
- `engine/lexicon/src/handle.rs` — `EngineState.syllable_inventory: Option<SyllableInventory>`
- `engine/lexicon/src/api.rs::install` — 路由新欄位
- `engine/lexicon/src/continuous.rs` — 新 `pub fn fetch_candidates_for_keys(keys, ...)` mode-agnostic 入口;既有 `fetch_candidates_for_endings` 改為 thin TL/POJ wrapper
- `engine/scripts/gen-platform-protos.sh` + `engine/protos/build.rs` — 不需手改 (只改 .proto 與已涵蓋的 generator script 不變)

**Phase 6 限制 (deferred to Phase 9 dogfood)**:
- TL/POJ 帶調符 (`pe̍h` / `chóa`) 的 continuous-input 不支援:per-syllable POJ→TL canonicalization 還沒整進 dispatch,`build_keys_tl` 只做 `to_ascii_lowercase`。POJ 使用者請用數字調 (`peh4`)
- TPS tone-1 (無調號) 隱式邊界不支援:`tps::valid_span_endings` 只看 tone-mark / 入聲韻尾,tone-1 syllables 不切。Phase 9 dogfood 後再決定要不要實作 next-initial-seen rule
- `enabled_sources_bitmask` / `user_freq_boost` 在 dispatch 端硬編 `u32::MAX` / `1.0`:Phase 7 / 8 平台 UI 整合決定要從 `FetchAtPos` 加欄位還是 `AppConfig` 帶下來

**Cross-platform parity 設計**:UI **不**重新計算 candidate `consumed_span` / `display_text`——engine 在 `ContinuousResponse.candidates` 直接給出,iOS/Android 只 render。

**規模**:M (~600 LOC handcoded — proto + dispatch + transition + lexicon paths/handle/continuous + tests),加 generated bindings (per roadmap.md:502 不計 review size)

---

### Phase 7 — iOS UI 整合

**Files**:
- `ios/Sources/Composition/CompositionRoot.swift` — 新增 Continuous mode 偵測:當 raw input 長度 ≥ 閾值 (估 ~3 chars) 且包含合法音節邊界時,先 `Append` / `Start` 把 buffer 餵滿,再呼 `EnterContinuous {}` (Phase 6 contract:no payload — `Phase::Composing { raw }` 必須非空才能轉 Continuous,見 `engine/composing/src/transition.rs:484-490`)
- `ios/Sources/Autocomplete/...` (KeyboardKit AutocompleteProvider 子類) — render candidate strip from `ContinuousResponse.candidates`;每次 candidate 列重整都要先發 `FetchAtPos { position: 0 }` 拿候選 (`continuous` 欄位只在 FetchAtPos 才有);tap candidate → 帶 `display_text` / `consumed_bytes = candidate.consumed_span_end` / `syllable_count = candidate.syllable_count` 發 `CommitContinuous`,然後再 issue 一次 `FetchAtPos` 刷新剩餘候選
- 新增 ComposingTextStrip 元件 (在 keyboard 上方一行) — `pending_display` 從 `ComposingResponse.preedit.display_text` 取;committed segments 已透過 mid-commit `CommitTextReplacingPreedit` Effect 寫進文件,UI strip 不重複渲染
- Backspace handler:Continuous mode 沿用既有 `Intent::DeleteBackward`,Phase 4 已實作 Continuous 折回 (`engine/composing/src/transition.rs:209-309`)
- `ContinuousResponse.candidates[i].form` 目前固定為 1 (FORM_NOTONE);Phase 7 UI 不需要分支,直接 render `display_text`

**iOS-specific gotchas**:
- KeyboardKit setMarkedText / commitText 對應:committed segments 走 `commitText` (透過 Phase 4 mid-commit Effect),pending 走 `setMarkedText` (透過 `UpdatePreedit` Effect)
- Settings 切換 (TL/POJ/TPS) mid-composition → `ResetContinuous {}`
- Stale tap rejection:每個 request 帶 generation,Phase 6 dispatch 的 generation 同步沿用既有 `EngineHandle::handle` 設計

**規模**:M-L (~500 LOC Swift + tests)

---

### Phase 8 — Android UI 整合

**Files**:
- `android/.../ime/core/TaigiKeyboard.kt` — 同 iOS,偵測 + 先 `Append` / `Start` 餵滿 buffer 再 `EnterContinuous {}` (no payload)
- `android/.../ime/text/smartbar/SmartbarManager.kt` — render candidate strip from `ContinuousResponse.candidates`;每次刷候選 issue `FetchAtPos { position: 0 }`;tap candidate → 帶 `display_text` / `consumed_bytes = candidate.consumed_span_end` / `syllable_count = candidate.syllable_count` 發 `CommitContinuous`,然後再 issue 一次 `FetchAtPos`
- `android/.../ime/core/InputView.kt` — 加 ComposingTextStrip composable;`pending_display` 從 `ComposingResponse.preedit.display_text` 取
- Backspace handler:沿用既有 `Intent::DeleteBackward` (Phase 4 已蓋 Continuous 折回邏輯)

**Android-specific gotchas**:
- `setComposingText` for pending、`commitText` for committed (committed segments 已透過 Phase 4 mid-commit Effect)
- 旋轉 / focus loss → `onFinishInput` / `onStartInput` → `ResetContinuous {}`
- IME mode swap → 同上 reset
- `setComposingText("")` 應在 commit 後立即清空 marked text 區
- **Mode 偵測**:平台 `RustEngineBridge.appConfig` 目前把 TPS 映到 `"tl"` (`android/.../RustEngineBridge.kt:1544-1558`),Phase 6 dispatch 因此用 `phonetics::contains_tps(raw)` 判斷而非 config string;Phase 8 不需要改 appConfig 行為

**規模**:M-L (~500 LOC Kotlin + tests)

---

### Phase 9 — Continuous-input ranking 修復 + 主流 IME 對齊 (FINALIZED 2026-05-11)

**Spec source-of-truth**:[`docs/engine/continuous-input-ranking.md`](engine/continuous-input-ranking.md)

**Plan finalization**:Codex 三輪 ANALYSIS-ONLY consult ── R1 framing(6 軸)、R2 design forks(Q1-Q8)、R3 final fork-clearing(Q9 custom_dict + Q10 nextword verify + Q11 sort_key)。Transcripts `/tmp/codex-v358-phase9-plan-r{1,2,3}-out.txt`。每 PR 仍要 Codex sandwich(pre + post)+ `/codex-pr-review`。

**目標(spec §7)**:
- **G1** Phrase-priority ranking ── 全 buffer exact match 入 Tier 1
- **G2** User-freq feedback loop closes ── `user_frequency.db` 接入 Continuous fetch,boost cap 5×
- **G3** Engine = ranking authority(已達成,preserve)
- **G4** Multi-vocab axis baseline ── `mode: HANT/TAILO/MIXED` carrier-only,**不**進 rank tie-break
- **G5** Caret + nail dual cursor ── **defer**,僅 docs 內把 `consumed_bytes` 稱作 "nail advance bytes"(R2 Q5,無 API surface)

**Non-goals**(R1/R2/R3 explicit reject):full Viterbi、Rime SchemaYAML、Google cloud-LM、MOE CompositioMode 雙軸、`words` 欄位(dict.bin v3 schema bump)、`vocabulary` 屬性 chip UI、`weight: f64` widening、6A 詞典 build-time freq ×100 hack、6B corpus-derived freq、no-op `MoveCaret` 占位 API

---

#### PR 拆分(9 PR,總 ~1350–2050 LOC)

PR 順序 = `9.1 → 9.2 → 9.3a → 9.3b → 9.3c → 9.4a → 9.4b → 9.5 → 9.6`(R2 Q7.c + R3 Q9 加 9.6)。

| PR | 範圍 | LOC | 主檔案 | Codex cite |
|---|---|---|---|---|
| **9.1** Ranking core | Tier 1 = `consumed_span_end == raw.len()`(R2 Q1.a);lexicographic sort_key 套用;`record_to_candidate`(`continuous.rs:237-245`)補通 `DictionaryRecord.bitmask`(R3 Q11.a 指出目前 discard) | 300–450 Rust | `engine/lexicon/src/continuous.rs`、`engine/ranking/src/score.rs` | R2 Q1.a / Q2.c / R3 Q11 |
| **9.2** mode carrier | `mode: HANT/TAILO/MIXED` 加進 `CandidateMessage`(`composing.proto:192`);Rust derive(hanzi 有無 + non-ASCII roman);iOS/Android binding regen;**不**進 rank tie-break(R2 Q3.a) | 200–300 Rust + bindings | `engine/protos/proto/composing.proto`、`engine/lexicon/src/continuous.rs` | R2 Q3.a |
| **9.3a** user-freq Rust + proto | 加 `FrequencyEntry[]`(複用 `lexicon.proto:333-340`)進 `FetchAtPos` request;`fetch_via_lexicon` 拿掉 `1.0` 寫死(`dispatch.rs:312-323`);adjusted_score 公式套 cap | 150–250 Rust | `engine/composing/src/dispatch.rs`、`engine/lexicon/src/continuous.rs`、`engine/ranking/src/score.rs` | R2 Q4.b / R3 Q11 |
| **9.3b** user-freq iOS plumb | 在 `FetchAtPos` 前 batch query `user_frequency.db` `WHERE word IN (...)` → 塞 `FrequencyEntry[]` 到 request | 200–350 Swift | iOS Autocomplete + Lexicon DB layer | R2 Q7.c |
| **9.3c** user-freq Android plumb | mirror 9.3b 在 Android `CandidateUpdateCoordinator` / Service layer | 200–350 Kotlin | Android Smartbar + Lexicon DB layer | mirror Phase 8 |
| **9.4a** TPS tone-1 | `syllabifier/tps.rs:65-93` 加 next-initial-seen rule(tone-1 隱式邊界) | 50–100 Rust | `engine/composing/src/syllabifier/tps.rs` | R2 Q5.b |
| **9.4b** Hyphen offset map | `build_keys_tl`(`dispatch.rs:181-207`)維護 shadow hyphenless buffer + `(shadow→raw)` offset map;`CandidateMessage.consumed_span_*` 保 raw byte offsets | 200–350 Rust | `engine/composing/src/dispatch.rs` | R2 Q5.b |
| **9.5** Data variant | 補「台灣台語」變體於 `dictionary/output/dictionary.csv:140250` 鄰近行;**僅** minimum,不做大規模 audit | ~50 data | `dictionary/output/dictionary.csv` 或上游 source | R2 Q6.a |
| **9.6** Custom dict in Continuous | 連續輸入 fetch 加平台側 custom_dict 查詢(custom_dictionary.db 維持 native);custom 來源 rank 高於 kautian(`custom=0, kautian=1, ...`);**無**強制 Tier 1 promotion(仍要 `consumed_span_end == raw.len()`);canonical key 用既有 `notone` 衍生欄 | 200–300 跨平台 | iOS/Android Autocomplete + Rust merge point | R3 Q9.A |

依賴鏈:9.1 lock rank-key API → 9.2 proto carrier 趁 metadata-only → 9.3a-c 跨平台同 release tag → 9.4a/b coverage 變更放後 → 9.5 data 最後(golden expectations 才不反覆改)→ 9.6 confirm custom 行為穩定後再 plumb。

---

#### Sort_key 公式(PR-9.1 source-of-truth)

```text
For each RawCandidate produced by fetch_candidates_for_keys:

  tier            = if consumed_span_end == raw.len() { 0 } else { 1 }
  coverage_bytes  = consumed_span_end - consumed_span_start
  syll_bias       = 1.0 + 0.1 × max(0, syllable_count − 1)         // 沿用 Phase 5
  user_freq_boost = min(1.0 + count × BOOST_ALPHA, MAX_BOOST)       // R2 Q4.b
                    // count = FrequencyEntry.count, 缺 entry 時 boost = 1.0
  adjusted_score  = freq × syll_bias × user_freq_boost              // R3 Q11.c multiplicative

  recency_rank    = if last_used_ms != 0 && now_ms − last_used_ms < RECENCY_WINDOW_MS { 0 } else { 1 }
  source_tier     = source_rank_for(bitmask, is_custom)
                    // custom=0, kautian=1, taigitv=2, stti=3, kungge=4, default=5

  sort_key = (
      tier,             // asc:  Tier 0 全 buffer 優先
      -coverage_bytes,  // desc: 同 tier 內,長 match 先
      recency_rank,     // asc:  同 (tier, coverage) 內,recent 先
      -adjusted_score,  // desc: freq × syll × boost
      -freq,            // desc: 原 freq 二次 tie-break
      source_tier,      // asc:  custom > kautian > taigitv > ... > default
      stable_idx,       // 插入順序,FST byte-sort deterministic
  )

Sort ascending by sort_key; first element = slot #1.
```

#### 跨平台 invariant 常數(Rust pinned,平台不可 override)

| 常數 | 值 | 來源 |
|---|---|---|
| `MAX_BOOST` | `5.0` | R2 Q4.b + Codex Q-E stale dominance warning |
| `BOOST_ALPHA` | `0.1` | R2 Q4.b(50 次後 boost 飽和) |
| `RECENCY_WINDOW_MS` | `3_600_000`(1h) | R3 Q11.b,reuse legacy `engine/ranking/src/score.rs:45` |
| `MAX_SYLLABLES` | `8` | 沿用 Phase 6 `engine/composing/src/dispatch.rs:35` |
| `SOURCE_TIERS` | `custom=0, kautian=1, taigitv=2, stti=3, kungge=4, default=5` | R3 Q11.a(新增 custom 在頂)+ legacy `score.rs:65-70` 既有四 tier 順序 |

依 `rules/cross-platform-alignment.md` §3a:`score.rs` 為單一 source of truth,iOS/Android **不**可重定義。

---

#### 三個 user-data DB 在 Phase 9 的角色(R3 Q10)

| DB | 角色 | Phase 9 動嗎? |
|---|---|---|
| `user_frequency.db` | 詞 → (count, last_used_ms);ranking boost 用 | **動** 9.3b/9.3c 平台 batch query;schema 不動 |
| `user_association.db` | (前詞 → 後詞 → count);nextword 用 | **不動**(Phase 4 handshake 已對齊;R3 Q10.a) |
| `custom_dictionary.db` | 使用者主動管理的私人詞典 | **動** 9.6 接入平台側 Continuous fetch 查詢;schema 不動 |

「連續打字記憶 = nextword 記憶」 status(R3 Q10):
- **語意層面 Phase 4 已對齊**:`transition.rs:644-673` mid + final commit 都發 `NextWord*` Effect;iOS `KeyboardViewController+TextInput.swift:60-72`、Android `SmartbarManager.kt:188-204` 路由
- **mid ≠ final 語意差**:mid-commit `UpdateLastSelectedWord` 更新 `last_selected_word`、寫 compound association,但**不**記 `prev→this`、不 bump generation、不觸發 prediction(`engine/nextword/src/decide.rs:253-289`);只有 final-commit 才完整(`decide.rs:130-179`)── 這是設計如此(中段 commit 時下一詞尚未打完,prev→this 不該記)
- **三 DB 不合併**(per `feedback_user_data_sqlite_stays_native.md` + `docs/architecture/data-artifacts-portability.md:16-20`)
- **Phase 9 無**對 nextword 內部修改

---

#### 回歸守護矩陣(每 PR 跑,9.1 為 acceptance 主)

| Input | 預期 #1 候選 | Tier | 守護原因 |
|---|---|---|---|
| `taiuantaigi` | 臺灣台語 / 台灣台語(9.5 後)| 0 | 主 acceptance |
| `e` | 的 | 0(full-buffer = 1 byte) | 短輸入不被誤埋 |
| `tsua` | 珠仔(syll=2, cov=4)| 0 | 多 syll 同 coverage 優先 |
| `taixyz` | Tier 0 空;Tier 1 「台」 | (空 0, 全 1) | 部分無效尾不誤升 Tier 0 |
| `tai5` | 「台」(numeric tone)| 0 | `is_false_toneless_boundary` 防誤切 |
| `gautsa`(custom)| 𠢕早(9.6 後)| 0 | custom 跨模式可達 |
| 重複選 5 次 X 後 X | X 排前 | (前)| user_freq boost 內生效 |
| 重複選 50+ 次 X | X 排前但不 dominate phrase | (前)| boost cap 5× 防 stale dominance |

---

#### 既有 dogfood 矩陣(release-prep,9.6 merge 後跑)

1. Tone 1/4 邊界:`taibak` / `bakkiann` / `khihthau` / `taigikhipuann`
2. TPS 全部音節:`ㄉㄞˊㄨㄢˊㄉㄞˊㆣㄧˋㄌㄛˊㄇㄚˋㆢㄧ˫` → 「臺灣台語羅馬字」
3. Mid-composition mode switch (TL ↔ POJ ↔ TPS) → reset 一致
4. Focus loss → buffer 清空,no zombie state
5. Stale candidate tap → response 帶舊 generation,平台忽略
6. Partial invalid tail:`taixyz` → 切出 `tai`,`xyz` 留在 pending
7. Uppercase / hyphen / apostrophe:`Tai-gi`、`pe̍h-ōe-jī`、`a'au`
8. Paste / emoji during composition → `ResetContinuous` 後 commit
9. Punctuation / space / Enter → commit boundary
10. Hardware keyboard arrow keys → reset(本輪不支援 mid-buffer 編輯;G5 已 defer)

Round-A/B/C dogfood:9.6 merge 後,iPhone + Android 實機 S1/S2/S3 + 上述 10 條矩陣 + 回歸守護矩陣全綠 → cut v3.5.8。

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
| 2 — syllable inventory FST | ~300 + tests | 3 | No | **Merged in PR #251** (squash `f4c2e52f`) |
| 3 — syllabifier (TL + TPS) | ~450 + tests | 4, 5 | No | **Merged in PR #252** (squash `2f7feac1`) |
| 4 — `Phase::Continuous` + nextword 邊界 | ~700 + tests | 6 | No | **Merged in PR #253** (squash `a69bfc75`) |
| 5 — span-local candidate fetch | ~400 + tests | 6 | No | **Merged in PR #254** (squash `cf813af4`) |
| 6 — proto + dispatch RPCs | ~400 + bindings | 7A, 8 | No | **Merged in PR #255** (squash `c6f2ca42`) |
| 7A — iOS bridge wiring (engine-facing) | ~250 Swift + tests | 7B | No | **Merged in PR #256** (squash `8c431af6`) |
| 7B — iOS UI integration (user-visible) | ~250 Swift + tests | — | **Yes** | **Merged in PR #257** (squash `65c2120c`) |
| 8 — Android UI 整合 | ~500 Kotlin + tests | — | **Yes** | **Merged in PR #258** (squash `9fed869b`) |
| 9.1 — Ranking core(tier + sort_key + bitmask plumb) | ~300-450 Rust | 9.2-9.6 | **Yes**(排序變)| **Pending** |
| 9.2 — `mode: HANT/TAILO/MIXED` carrier(無 UI chip)| ~200-300 Rust + bindings | 9.3 | No(metadata-only)| **Pending** |
| 9.3a — user-freq Rust + proto(`FrequencyEntry[]` in FetchAtPos)| ~150-250 Rust | 9.3b, 9.3c | No(Rust 內部)| **Pending** |
| 9.3b — user-freq iOS plumb(batch SQLite query)| ~200-350 Swift | — | **Yes**(boost 生效)| **Pending** |
| 9.3c — user-freq Android plumb(batch SQLite query)| ~200-350 Kotlin | — | **Yes**(boost 生效)| **Pending** |
| 9.4a — TPS tone-1 next-initial-seen rule | ~50-100 Rust | 9.4b | **Yes**(coverage)| **Pending** |
| 9.4b — Hyphen offset map(shadow buffer)| ~200-350 Rust | — | **Yes**(coverage)| **Pending** |
| 9.5 — 詞典補「台灣台語」變體 | ~50 data | — | **Yes**(acceptance)| **Pending** |
| 9.6 — custom_dict 接入 Continuous fetch(平台側查詢 + custom source rank)| ~200-300 跨平台 | — | **Yes** | **Pending** |

**Status legend**:Pending / In progress (PR #N) / Merged in PR #N / Blocked (reason)

**Active PR pointer**:next round = **Phase 9.1 — Ranking core**(branch suggestion: `v358-phase9.1-ranking-core`)。Phase 0 merged in PR #248,Phase 1 merged in PR #249,Phase 1b N/A merged in PR #250 (squash `2c826b96`),Phase 2 merged in PR #251 (squash `f4c2e52f`),Phase 3 merged in PR #252 (squash `2f7feac1`),Phase 4 merged in PR #253 (squash `a69bfc75`),Phase 5 merged in PR #254 (squash `cf813af4`),Phase 6 merged in PR #255 (squash `c6f2ca42`),Phase 7A merged in PR #256 (squash `8c431af6`),Phase 7B merged in PR #257 (squash `65c2120c`),Phase 8 merged in PR #258 (squash `9fed869b`)。

**總計**(含 Phase 9 finalized):20 個 PR(原 11 個 Phase 0-8 已 merge,加 Phase 9.1-9.6 共 9 個 sub-PR),加總約 6000-6500 LOC + tests。多數 hand-reviewed code PR 落在 200-450 LOC;Phase 6 + 9.2 的 generated bindings 不計入 review size。

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
