# 三索引可行性評估 (POJ + TL + TPS first-class in lattice)

- **Date**: 2026-05-20
- **Status**: Analysis-only, not implementation. User-gated for scope / timing / tag ([[feedback_no_unilateral_release_scope]]).
- **User decision (2026-05-20)**: 走 **C (三索引)** — TPS 是 first-class user mode (教學 / 教會 / 學習者生態都要支援可搜尋、可學習、可排序)。本文件記錄評估推導與 risk register;**impl 由 user 另行排程**。
- **Related**: [[project_v359_triple_index_eval]] [[project_v359_backlog_handoff]] [[project_v359_refactor_plan_draft]] [[project_continuous_poj_ascii_canonicalize]] [[project_continuous_poj_render]] [[reference_mainstream_ime_comparison]]

---

## 1. TL;DR

引擎內部對 TL / POJ / TPS 三軸處理不一致,屬 architectural debt:

| 路徑 | TL | POJ | TPS |
|---|---|---|---|
| 字典查詢 (`lexicon::search`) | first-class `tl:` | **first-class `poj:`** | disguised — `tps_to_tl` → `tl:` + bolted `tps_or_mapped_to_er` |
| 鍵盤連續輸入 lattice/walker | first-class `tl:` | surface — `canonicalize_poj_shadow` → `tl:` | **完全跳過 walker** (`is_tps` short-circuit) |
| 自家 syllabifier | `syllabifier/tl/` | 無 | `syllabifier/tps.rs` 已存在但只服務 endings,沒接 walker |

評估了 4 個選項 (A=現況 / B=雙索引 / C=三索引 / D=雙+TPS 顯式 adapter),經 Codex ANALYSIS-ONLY 第二意見壓力測試,**排序 D > C > B > A**(D 有硬條件)。User 拍板 **C**(TPS = first-class user mode)。

**走 C 的硬約束**(3 條,impl 前必先 verify):
1. **mode-axis 分層**:Input + Key + FST 三層 mode-aware,Walker + Scorer **永遠 mode-blind**。
2. **user-history key 不得含 surface form / mode / input code**,只能 candidate identity (hanji or canonical roman)。**已 grep 確認現況符合此約束**(`continuous.rs:484` + `TaigiWord.kt:6`),但 C 改 RawCandidate.roman 形式時要保留此契約。
3. **dict.bin / FST size 增量必測非推論**;build pipeline POC 量化作為 go/no-go gate。

---

## 2. Background — 為什麼要評估

2026-05-20 策略對話:user 質疑「POJ 連續輸入為何要 POJ→TL canonicalize」。深入後發現 POJ 在 search 已 first-class (commit `5e13d5dc`, v3.5.6) 但 continuous 卻是 surface form (commit `cf48bbae`, Phase 9 連續輸入時代引入);TPS 全程是 architectural 半成品 — 有自家 syllabifier 沒接 walker、無 FST index、bolted-on `tps_or_mapped_to_er`。

User 問:三軸全升 first-class 是否值得?Goal = 對未來架構乾淨清楚不複雜。

---

## 3. Evidence (已 grep 驗證 2026-05-20)

### 3.1 索引族 emit
- `dictionary/output/dictionary.csv` 欄位 = `hanzi,tl,frequency,poj,tl_num,poj_num,tl_notone,poj_notone,tl_abbrev,poj_abbrev,kautian,...` — **沒有** `tps_notone` / `tps_num` / `tps_abbrev`
- `dictionary/build/create_fst.py:123,127,134` emit `tl:` / `poj:` / `hanzi:` 三族,**沒** emit `tps:`

### 3.2 key normalizer fall-through
- `engine/lexicon/src/key_normalizer.rs:34` `KeyMode::Tps => format!("tl:{normalized}")` (fall through,前提是 `phonetics::tps_to_tl` 已先轉成 TL ASCII 形)

### 3.3 TPS canonicalize tax
- `engine/lexicon/src/classification.rs:20,77` `if contains_tps(raw) { tps_to_tl(raw) }` 是 search-path 入口轉換
- `engine/composing/src/continuous.rs:259-310` `build_keys_tps` 內部呼叫 `phonetics::tps_to_tl` 把 bopomofo → numeric-tone TL 再組 `tl:` key
- `engine/lexicon/src/search.rs:132-145` `tps_or_mapped_to_er` expansion — TPS 模式且 input 含 `"er"` 時做 `key.replace("er","or")` 平行 rowid Σ (search-time bolted-on)

### 3.4 walker short-circuit
- `engine/composing/src/continuous.rs:803` `if is_tps { (build_keys_tps(raw), None) } else { ... }` — TPS 用自家 key builder,**shadow lattice = None**
- `engine/composing/src/continuous.rs:892` `if !is_tps { fetch_walker_slot0_inner(...) }` — TPS 整條 lattice 跳過

### 3.5 syllabifier 既有資產
- `engine/composing/src/syllabifier/tl/` — TL 已有自家 syllabifier
- `engine/composing/src/syllabifier/tps.rs` — 已存在,line 93 `pub fn valid_span_endings(input: &str, pos: usize) -> Vec<usize>` 含 tone-1 implicit-boundary 規則 (Phase 9 Item 7 "next-initial-seen")。**已接 endings 路徑;未接 walker。**

### 3.6 scorer/walker mode-blind 範圍 (Codex 校正,本輪 grep verified)
- `engine/ranking/src/score.rs:186` `pub type FrequencyMap = std::collections::HashMap<String, FrequencyData>` — value type 與 mode 無關
- `engine/ranking/src/score.rs:173-174` key = "`TaigiWord.displayText` (= `hanji` if non-empty else `roman`)"
- `engine/composing/src/continuous.rs:484` `let display_text = entry.hanji.clone().unwrap_or_else(|| entry.roman.clone());` — **引擎側候選建立時就 hanji-優先**
- `engine/composing/src/continuous.rs:698` 同樣 hanji-優先 fallback
- `android/.../TaigiWord.kt:6` 中文註:「displayText 漢字優先,其次羅馬字」— 平台側對齊
- `android/.../proto/FrequencyEntry.java:12,212` `display_text_key matches TaigiWord.displayText (= hanji ?? roman)` — 平台側 user_frequency.db 也用同 key

**結論**:Scorer / walker / user-freq 都用 **candidate identity (hanji-優先)**,**不** 含 surface form / mode / input code。**B/C/D 升索引對 hanji-bearing candidate 的 user-history 零影響**(無 learning split)。

### 3.7 POJ→TL canonicalize 起源
- Commit `cf48bbae` Item 9: POJ diacritic canonicalize (Phase 9 連續輸入時代引入)
- 字典 search 的 POJ direct lookup 自 v3.5.6 `5e13d5dc` 就在,**沒被取代**
- continuous 走 canonicalize 是為了 reuse 既有的 `tl:` lattice 機制,**不是 POJ 不能 first-class**

### 3.8 連續輸入 `tl:`-only 契約
- `lexicon/src/continuous.rs:5,80,442,870-887` 寫:「continuous only ever builds `tl:` keys; `poj:` / `hanzi:` pass through untouched」
- **這是 contract 不是 limit** — 升 POJ first-class 就是把 contract 改成「mode-aware」

---

## 4. 四個策略選項

| 選項 | 描述 | 增量 LOC (engine) | 改 platform? | 拆架構稅 |
|---|---|---|---|---|
| **A** | TL 單索引維持 (現況) | 0 | 0 | 不拆 — 永久 canonicalize tax + 內部不一致 |
| **B** | 雙索引 (POJ first-class lattice) + TPS 現狀 | ~+1200 | 0 | 拆 POJ canonicalize ambiguity class;TPS short-circuit 仍在 |
| **C** | 三索引 (POJ + TPS 都 first-class) | ~+1450 = B + ~250 incremental | 0 | 拆**所有** POJ canonicalize + TPS short-circuit + `tps_or_mapped_to_er` |
| **D** | 雙 + TPS 整理成顯式 surface adapter (不升 first-class) | ≈ B + ~150 拆除 | 0 | 拆 POJ canonicalize 半邊 + TPS short-circuit;TPS canonicalize chain 顯式化 |

### Cost scaling 觀察

- 1 → 2 索引 = **~+1200 LOC engine** (架構大改造:mode-aware lattice、新 pattern)
- 2 → 3 索引 = **~+250-300 LOC engine** (incremental:沿用 pattern + 拆 `is_tps` short-circuit / `tps_or_mapped_to_er` 抵消)
- 3 → N 索引 = **~+200/mode** (機制成熟,純加數據軸)

**1→2 是 architecture cost,2→3 是 data cost**。若決定走 B,加 C 的邊際 cost 最低窗口就是同一輪。

---

## 5. Scorer / walker 零改動修正

Walker / scorer / user-freq 都 mode-blind (`engine/ranking/src/score.rs`),所有方案下 **scorer/walker layer 零改動**:
- `record_to_candidate` / `calculate_continuous_score` / `edge_cost` 都吃 frequency,不吃 key form
- 同一 dict entry 命中時,不管 `tl:tsiah` / `poj:chiah` / `tps:ㄐㄧㄚ`,frequency 一致
- `CORPUS_TOTAL_FREQ` / `BOOST_ALPHA` / `OOV_PER_CHAR_PENALTY` 都是 frequency 上的常數,key prefix 無關
- `FrequencyMap` key 是 `displayText` (hanji-優先 fallback roman),不含 mode

改動全在:**syllabifier / inventory FST / lattice edge construction / key_normalizer / build pipeline**。

⚠️ **本輪修正一個前一輪誤講**:曾說「scorer 要為 POJ 重 tune」— **不對**。Scorer 本就 mode-blind,改動只在 inventory + edge 構造。

---

## 6. Codex ANALYSIS-ONLY 第二意見 (2026-05-20)

- 第二意見 verbatim transcript: `/tmp/v359-triple-index-codex-prompt.txt` + Codex output
- Codex 最終排序 **D > C > B > A**,但 D 有硬條件
- 校正了 3 個我的盲點:

### 6.1 校正 #1 — D 中間態硬條件
> "D 不是天然乾淨;D 只有在你把 TPS 定義成『非正字法 peer,而是 phonetic surface adapter』時才乾淨。也就是 TPS 進來後立刻變成明確的 TL key stream,後面和 TL/POJ 共用 edge construction,不再有 walker bypass、rowid union、散落的 `er/or` 特例。若 D 只是把現有 special cases 改名成 chain,那你對自己的質疑成立:它會變成『拆得不徹底但宣稱拆了』。"

意義:走 D 必須**一次收緊** TPS adapter 為單一可測點,否則 D 退化為 maintenance debt 重包裝;此時排序變成 **C > B > D > A**。

### 6.2 校正 #2 — scorer mode-blind 範圍要收窄
> "scorer/walker mode-blind 大方向可以,但範圍要收窄:只對『同 rowid、同 segmentation、同 user-history key、同候選集合』成立。hidden mode cost 可能藏在 edge inventory、OOV 路徑、未知音節 penalty 的計量單位、render 後 dedupe、recency/user frequency key。如果 recency key 是 candidate/rowid,問題小;如果含 input code、surface form、mode,POJ first-class 可能造成 learning split 或重複加權。"

**本輪追加 grep verification**(§3.6):`FrequencyMap` key = `displayText` (hanji-優先 fallback roman),不含 mode → **hanji-bearing candidate 零 learning split**;副路徑 (純羅馬字無 hanji) 有 surface split 可能 — 詳見 §8.2 risk #2。

### 6.3 校正 #3 — librime Spelling Algebra prior art framing
> "Rime 不是單純『都在 input layer』;它有 Spelling Algebra,明確用來把編碼映射到新拼寫形式,包含同音系的注音/拼音/國語羅馬字互轉、簡拼、模糊音、回顯碼等。這支持 D 的『surface chain』思路,也支持『不要為每個 spelling 都手寫一套完整索引』。但 Taigi POJ/TL 是正字法,不只是 fuzzy variant;若 POJ/TL 的縮寫、容錯、學習、候選排序都要各自自然,Rime prior art 不能直接推出『單索引一定對』。"

意義:**Spelling Algebra 是 D 的 surface-chain 思路的直接 prior art;C 的多索引在 mainstream 沒直接 precedent;B 是中間態**。我原本說「mainstream IME 都在 input layer」太強;Rime Spelling Algebra 就是「在 storage 之外另開一層 mapping」。

### 6.4 校正 #4 — dict size 估算必測非推論
> "dict size 估算偏樂觀的地方在 raw text 單位。TPS 若是注音 UTF-8,平均『6 chars』不是 6 bytes;而 runtime 成本也不等於 CSV raw size。真正要看的是新增 key 數、rowid postings/value payload、FST 是否 mmap、查詢會觸碰多少 page。500KB-1MB 增量『可能合理』,但不能當決策依據;我會把它列成必測,而不是推論。"

意義:走 C 必須跑 build pipeline POC 量化 `dictionary/output/dictionary.bin` + `syllables.fst` 增量、引擎啟動 mmap profile,作為 go/no-go gate。

---

## 7. 4 criteria 評估矩陣 (Codex 共識)

| Criterion | A 現況 | B 雙索引 | C 三索引 | D 雙+TPS adapter |
|---|---|---|---|---|
| **對未來架構有利** | ✗ 永久 canonicalize tax + 內部不一致 永久存在 | ◯ POJ 對稱;TPS 仍 ad-hoc | ✓ 三軸全對稱;新 mode 純加數據軸 | ✓ POJ 對稱 + TPS 顯式 surface (一致但非升格);保留升 C 可選性 |
| **可讀性高** | ✗ offset map / Phase 1 NFD walk / Phase 2 substitution / mode gate 大量隱含規則 | ◯ POJ 路徑乾淨;TPS short-circuit 仍藏 | ✓ 模式統一 mental model (mode-aware lattice 一處查到底) | ✓ TPS 從 ad-hoc 變顯式;POJ first-class |
| **程式碼乾淨清楚** | ✗ `is_tps` short-circuit 兩三處 + `tps_or_mapped_to_er` bolted-on + `canonicalize_poj_shadow` mode gate | ◯ POJ canonicalize 拆;TPS special case 不動 | ✓ 拆掉**所有** TPS special case + POJ canonicalize 整條 | ✓ 拆 TPS short-circuit + POJ canonicalize 半 (顯式 chain);TPS-canonicalize chain 顯式單一 |
| **邏輯不複雜** | ✗ 多層 fallback + mode gate + offset map | ◯ POJ 邏輯簡化;TPS 多層仍在 | ✓ mode-aware lattice 統一範式;canonicalize 整條退役 | ◯ 兩個 canonicalize chain (POJ ASCII 退役 + TPS 顯式留存),但都顯式 |

**Codex 與我共識排序**:
- **若 D 達硬條件**:**D > C > B > A**
- **若 D 達不到硬條件**:**C > B > D > A** (D 退化為 maintenance debt 重包裝)
- 決定軸 = **TPS 的產品定位**:
  - TPS 是 first-class user mode → **C**(本案 user 決定)
  - TPS 是輸入表面 adapter → **D** (達硬條件)
  - POJ 拆,TPS 不碰 → **B**
  - 都不碰 → **A**

---

## 8. Risk Register (走 C 必處理)

### 8.1 v3.5.8-hot code 衝擊
- C 會碰 `composing::continuous` + `composing::shadow` (A1/A2 才剛動過)
- **S0 golden 必跑且要擴 matrix** 含 POJ continuous + TPS continuous 新 cases。**不能 `UPDATE_GOLDEN` 蓋掉**(會吞 byte-non-identical 真實行為差)
- POJ continuous 升 first-class **不是 behavior-neutral refactor**(候選順序 / recase 行為極可能變)→ **feature change tier**,**不在 v3.5.9 Tier-A 安全傘下**
- 影響 release 排程:C 應作為獨立 minor feature 而非 refactor slice

### 8.2 副路徑 learning split (純羅馬字無 hanji)
- **主路徑 zero risk** (§3.6 grep 驗證):`FrequencyMap` key = `displayText` = hanji-優先,B/C/D 升索引對 hanji-bearing candidate 的 user-history 零影響
- ⚠️ **副路徑**:`hanji == None` 時 fallback to `roman` (`continuous.rs:484/698`)
  - 現況:continuous walker 跑 TL canonical form,RawCandidate.roman 是 TL form,即 POJ 輸入也記在 TL roman key
  - C 之後:若 POJ continuous 直接 `poj:` 索引,RawCandidate.roman 可能變 POJ surface → 同一純羅馬字 entry POJ 學一次 + TL 學一次 = split
- **硬約束**:走 C 時 `RawCandidate.roman` 在 fallback 場景必須保持 **canonical TL form**(或 mode-aware 統一規定一個 canonical form)。**impl 時必須在 PR body 顯式 record 此契約**
- 影響面有限:絕大多數 dict entry 都有 hanji;custom_dict + NextWord + OOV 才會 hit

### 8.3 dict.bin / FST size 增量必測非推論
- C 需 dict.csv 加 3 column (`tps_notone` / `tps_num` / `tps_abbrev`)、create_fst.py emit `tps:`、`engine/protos/build.rs` 對應 → 全平台 .bin/.fst regen
- **必跑 POC** 量化:
  - `dictionary/output/dictionary.bin` 增量 (bytes)
  - `dictionary/output/syllables.fst` 增量 (bytes)
  - 引擎 startup mmap profile (page touch 數)
  - iOS keyboard extension memory pressure (32 MB hard cap)
  - Android keyboard service memory pressure
- POC 結果作為 **go/no-go gate**,不能用 ~500 KB-1 MB 推論值決策

### 8.4 MOE-audit #5 強相關
- Mode-axis 設計**必須一併拍板** fused 整句 vs 逐音節 conversion granularity (原 user-scoped 待議,見 [[project_continuous_poj_ascii_canonicalize]])
- 不能 surface 抑制 — 走 C 時要顯式決定切點

### 8.5 build pipeline 改動的回滾性
- C 是 dict schema 永久 forward change → 一旦 ship,**回滾代價高**(用戶端 .bin 已含 `tps:` 族,舊版引擎讀新版 dict.bin 行為未定)
- 緩解:dict.bin schema 加版號(若尚未),引擎 startup verify;rollback strategy 在 impl 前決定

### 8.6 TPS 自動校正 3 layer impact(本輪追加 grep 確認)

TPS 自動校正在引擎 / 平台兩端分 3 個獨立 layer,**C 走後行為差別 layer 各異**(不全是無痛):

| Layer | 規則範例 | 機制 / 位置 | C 之後 impact |
|---|---|---|---|
| **L1 即時鍵入調整**(用戶最有感) | `ㄗ+ㄧ→ㄐ` / `ㄘ+ㄧ→ㄑ` / `ㄙ+ㄧ→ㄒ` / `ㆡ+ㄧ→ㆢ`(顎化);`{ㄇ,ㄫ}` 鼻音替換;初聲/終聲鍵形狀切換;鼻化母音 auto-update | `engine/phonetics/src/tps_adjust.rs` (196 LOC, 4 fns 收成 `pub(crate) fn adjust`) + `Intent::ReplaceLast` proto + iOS `TPSInputAdjuster.swift` / Android `CharacterInputPipeline.kt` 平台 caller | ✅ **零 impact**。L1 在 raw input → composing buffer 階段觸發,**早於** FST 索引族查詢,與 `tl:`/`poj:`/`tps:` 索引族選擇完全分離 |
| **L2 er↔or 方言變體查詢**(海口/內埔腔容錯) | 輸入 `er` 結尾詞時,平行查 `or` 結尾,結果 union | `engine/lexicon/src/search.rs:132-145` `key.replace("er","or")` 平行 rowid Σ + `SearchParams.tps_or_mapped_to_er` flag + proto `SearchRequest.tps_or_mapped_to_er` (`engine/protos/proto/lexicon.proto:118,129`) | ⚙️ **規則上移 build pipeline**:dict.csv 含 `er` 的 entry 在 build 階段同時 emit `tps:<er>` 與 `tps:<or>` 兩 FST key 指向同 rowid → runtime 邏輯消失、變單次查詢;**proto field 變 obsolete**(保留 wire 兼容、實際 ignored);⚠️ user toggle 變 **always-on**(無法 runtime 關閉,見下方 product decision) |
| **L3 編碼變體 normalization** | NFD 第 8 聲點 `U+0307` ↔ encode-safe `U+02D9`、其他注音符號編碼變體 | `engine/phonetics/src/tps.rs` 的 `ZHUYIN_TONES` + `ZHUYIN_TONES_ENCODE_SAFE` 兩張表 + `syllabifier/tps.rs:43-62` `classify` 同等接受兩種編碼 | ✅ **零 impact**。在 syllabifier 內,獨立於索引族 |

**Narrative clarification**(`tps_adjust.rs:10-11` 註解):
> "Engine does not gate because Android `InputMode` (POJ/TL) has no `.tps` case — TPS is a **layout**, not a mode."

跟「TPS = first-class user mode」**不衝突**,講不同層:
- `tps_adjust.rs` 的 "mode" = `protos::InputMode` (POJ / TL / Hanzi 三 case;**沒** `.tps`)。TPS 是 keyboard layout 切換,輸出可以是 POJ/TL romanization
- 本評估的 first-class 軸 = `KeyMode { Tl, Poj, Tps }` (`key_normalizer.rs:54` **已存在**) = FST **索引族**
- C 後仍維持 「TPS 是 keyboard layout」narrative;FST 查詢時 `KeyMode::Tps` 改 emit `tps:` 而非 fall through 到 `tl:`

**Product decision 待 user 拍板(L2 toggle 政策)**:

走 C 後 `tps_or_mapped_to_er` 從 runtime flag 變 baked-into-FST。三個處理方式:

| 選項 | 內容 | 代價 | 評估 |
|---|---|---|---|
| **A always-on** | 所有 TPS 用戶永遠 er↔or 容錯 | 簡單,FST 一份 | ✓ 推薦 — 極少用戶會主動關方言容錯 |
| **B 兩 FST variant** | 一個含 er↔or expansion 一個不含,runtime 切 | dict.bin 體積 x2 + 啟動兩份 mmap | ✗ 反推 — memory cost 高 |
| **C runtime post-filter** | 查到 `or` form candidate 時若 toggle off 就 drop | runtime 邏輯反而比現況複雜 | ✗ 反 pattern |

**建議走 A always-on**;這是 product 決策(進 §11)。

---

## 9. 走 C 的硬約束 (impl 前必達)

1. **mode-axis 分層**:Input + Key + FST 三層 mode-aware,Walker + Scorer **永遠 mode-blind**
2. **user-history key 不得含 surface form / mode / input code**:
   - 引擎側 `RawCandidate.display_text` 在 POJ first-class 後保持 `hanji ?? canonical_roman` 契約
   - `RawCandidate.roman` 在 hanji-absent fallback 場景保持 canonical TL form (或預先決定 mode-aware 統一 canonical)
   - 平台側 `user_frequency.db` insert 走 `displayText` (hanji-優先) — 已符合契約
3. **dict.bin / FST size POC** 通過 go/no-go gate(8.3)
4. **mode-axis 用 enum 而非 bool**:`is_poj: bool` 形態的 API 必須改 `enum InputMode { Tl, Poj, Tps, ... }`,N-script-ready
5. **MOE-audit #5 fused-vs-逐音節 切點** 一併決定
6. **S0 golden matrix 擴展** 含 POJ continuous + TPS continuous,不蓋舊 cases
7. **dict.bin schema 版號 + 引擎 verify** 機制(rollback strategy)

---

## 10. 後續步驟 (User-gated)

**本輪 (2026-05-20) deliverable**:此 doc + memory update。不開 round / 不寫 code / 不開 branch / 不 commit。

**Impl round 預期 shape** (供 user 排程參考,**不替 user 拍板時機 / tag**):

| Step | 內容 | 估算 LOC | 性質 |
|---|---|---|---|
| C-0 | dict.csv schema 加 3 column + create_fst.py emit `tps:` + build.rs 更新 + POC measure (8.3 go/no-go gate) | ~+150 build | data pipeline |
| C-1 | `KeyMode` API mode-aware: `key_normalizer::build` TPS 改 `format!("tps:{normalized}")` | ~+30 engine | refactor |
| C-2 | POJ continuous first-class: 拆 `canonicalize_poj_shadow`,walker mode-aware shadow → POJ form 直查 `poj:` | ~+800-1000 engine | feature change |
| C-3 | TPS continuous first-class: 接 `syllabifier/tps.rs` 到 walker (移除 `is_tps` short-circuit) + `tps_or_mapped_to_er` 規則進 build pipeline 階段(`tps:<er>` 與 `tps:<or>` 同時 emit;proto field 變 obsolete,wire 兼容保留) + **L1 `tps_adjust.rs` / L3 編碼 normalization 零改動**(見 §8.6) | ~+200-250 engine | feature change |
| C-4 | `render_roman_for_mode` 退役 (每個 mode 直接顯示 mode-form 結果) | ~-150 engine | cleanup |
| C-5 | golden matrix 擴 + on-device dogfood (POJ + TPS 連續輸入) | tests | acceptance |

**Scope / timing / tag** (v3.5.9 vs v3.5.10 vs v3.6 哪一版做 C-0 ~ C-5,以及是否拆 sub-PR) = **fully user-gated** ([[feedback_no_unilateral_release_scope]])。

⚠️ **不該與 Tier B 平台 inventory 同期做** — C 是 engine feature change,Tier B 是平台 cleanup,搶 v3.5.9-hot composing path 的可能性高,建議 C 自己一輪 (或自己一個 minor version)。User 自決。

---

## 11. 我**不**替 user 決定的事

- **Scope / timing / tag** (v3.5.9 / v3.5.10 / v3.6 哪一版做)= user-gated
- **TPS 的產品定位**(本案 user 已答 first-class)= user 已答
- **是否拆 C-0 ~ C-5 進不同 PR / 不同 release** = user-gated
- **與 Tier B / dogfood / MOE-audit #5 的排程交織** = user-gated
- **rollback strategy 細節**(dict.bin 版號設計 / 引擎 verify 形態)= 待 impl round 設計時定
- **L2 er↔or user toggle 政策**(§8.6 product decision)= user-gated;我建議 always-on(選項 A),但走 C 前 user 拍板

---

## Appendix A — Codex transcript reference

Codex `codex exec` ANALYSIS-ONLY second opinion 2026-05-20:
- Prompt: `/tmp/v359-triple-index-codex-prompt.txt`
- Output 已 inline 進 §6 (verbatim quotes for校正 #1-4 + final 排序)
- Codex did NOT propose any code changes (ANALYSIS-ONLY 守住)
- Codex Sources cited: Rime librime README on Spelling Algebra, Rime schema documentation on compiled prism/table binaries
