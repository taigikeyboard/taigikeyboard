# 使用者資料跨輸入模式 + 單→三索引相容性稽核 (v3.6.1 重點修復候選)

- **Date**: 2026-06-03
- **Status**: Analysis-only. NO code written. USER 將嚴格 review 多次後再決定修復方向 (USER 指示 2026-06-03)。
- **Scope tag**: USER 明確指示「將以上列為 v3.6.1 的重點修復項目」(2026-06-03)。其餘 release scope / timing / tag 仍 user-gated。
- **Method**: 兩平台 + 引擎並行 read-only audit (6 個 general-purpose agent) + Codex ANALYSIS-ONLY 二意見 ×2 (詞關聯根因 + DB 生命週期/最佳實踐)。所有結論均 file:line 佐證,empirically traced,非 code-reading 推論。
- **Trigger**: USER 回報「詞關聯紀錄和舊版無通,連續打字同一漢字詞之前的關聯詞跑不出來」。
- **Sections**: §1-5 四功能跨 mode + 三索引相容 (USER 第一批);§6 v3.6.1 項目;§7 DB 生命週期 + 最佳實踐 (USER 第二批:升級安全 / 膨脹 / 冗餘清理 / 最佳實踐)。**ship 優先序見 §7.5。**

---

## 0. TL;DR

四項使用者資料功能的「跨輸入模式共享 (tl/poj/tps)」+「單索引→三索引相容性」稽核結果:

| 功能 | 跨 mode 共享? | 單→三索引破壞? | v3.6.1 需修? |
|---|---|---|---|
| **詞頻紀錄** `user_frequency.db` | ✅ 漢字詞全共享 / 羅馬字詞 dict 命中也共享 | ❌ 未碰 (跨 mode 統一早於三索引,PR #310) | 🟡 僅 hanji-absent 自訂 TPS 詞一個 sliver (刻意 carve-out) |
| **詞關聯紀錄** `user_association.db` | ❌ **`prev_tl` 形式不一致 → 查不到** | ❌ 非三索引引起 (prev_tl filter 自 v3.4.7) | 🔴 **確認真 bug,主修項** |
| **自訂詞庫** `custom_dictionary.db` | ❌ **TL/POJ↔TPS 硬 miss;TL↔POJ 靠拼寫巧合** | ❌ 未碰自訂詞路徑 (但造成系統字典 tri-index、自訂詞單-index 的不對稱) | 🔴 **真 cross-mode 缺口** |
| **備份復原** `.taigi` JSON | ✅ restore 重新 derive,re-key 到新世界 | ❌ 格式與索引解耦,未碰 | 🟢 相容,不需修 (依賴上面修好) |

**核心結論(三索引價值評估)**:**四個問題沒有一個由三索引造成,放棄/回退三索引修不好任何一個。** 真正的橫切問題在 **使用者資料層的 key 一致性**(關聯 / 自訂詞 的 key 含 mode-dependent surface form)。三索引把**系統字典**做成三軸對稱,反而**凸顯**使用者資料層還沒對齊的不對稱 —— 暴露而非肇因。

---

## 1. 詞頻紀錄 (Frequency) — ✅ 基本已跨 mode 共享

### Key 機制
- `user_frequency.db` 單欄 key:`word TEXT UNIQUE`(iOS `UserFrequencySchema.swift:30-36`;Android `UserFrequencyService.kt:514-525`)。
- `word` = `displayText` = **漢字優先**(漢字非空則漢字,否則羅馬字)。引擎權威:`engine/ranking/src/score.rs:173-186` `FrequencyMap` key = `display_text_key`;`engine/lexicon/src/continuous.rs:1424` `display_text = hanzi.unwrap_or(tl)`。

### 跨 mode 判定
- **漢字詞:完全 mode-independent**。key = 漢字,與 tl/poj/tps 無關。POJ/TPS 只改 presentation `roman`,不進 key。**(主流案例全部 OK)**
- **羅馬字詞(無漢字)**:
  - dict.bin 命中 → key = `record.tl`(canonical TL,所有 mode 一致)→ 共享 ✅。POJ/TPS surface (`recase_tl_as_poj_display`) presentation-only,不進 key(`composing/continuous.rs:23-24` 註明)。
  - 自訂詞命中 → `canonical_tl_form` 把 POJ surface 折成 TL(`engine/phonetics/src/api.rs:296-309`)→ **TL↔POJ 共享 ✅**;**TPS 走 identity(注音字形非拉丁)→ 獨立 bucket**(刻意 carve-out,`api.rs:299-307`,僅影響 hanji-absent 自訂詞,極罕見)。

### 寫入點(兩平台一致)
- iOS:`ActionHandler+Suggestions.swift:108`(連續)+ `:140`(一般)→ `recordUsage(for: displayText)`。`:100-106` 註明用 sidechannel displayText 而非 TPS surface form。
- Android:`CandidateClickHandler.kt:160/267/383`,`:329-336` 註明「canonical DISPLAY_TEXT sidechannel → user_frequency.db + NextWord key 保持 mode-independent (decision b)」。

### 三索引影響
**無**。跨 mode freq-key 統一在 **PR #310 `ad085b3b` (2026-05-21)** 就完成,**早於**三索引 cluster (#333-#340)。三索引 PR 只改 FST key family 選擇,未碰 `display_text`/`roman`/`record_to_candidate`(`git show 7ef27025` continuous.rs 0 行命中 display_text)。

### v3.6.1 結論
🟡 基本不需修。唯一 sliver = hanji-absent 自訂詞且 roman 存成 TPS 注音 → 與 TL/POJ 不同 bucket。屬已記錄的刻意邊界,非回歸。**若要 100% 對齊,跟著自訂詞修法一起處理(見 §3)。**

---

## 2. 詞關聯紀錄 (Association) — 🔴 確認真 bug(主修項)

### Key 機制
- `user_association.db`:`prev_word`(漢字)、`next_word`(漢字)、`prev_tl`/`next_tl`(羅馬字)、`count`、`UNIQUE(prev_word, next_word, next_tl)`。
- 查詢(**兩平台一致**):`WHERE prev_word = ? AND (prev_tl = ? OR prev_tl = '')`(iOS `NextWordRepository.swift:164-188`;Android `NextWordService.kt:248-260`)。`prev_tl` 是 **hard filter**(羅馬字 exact-match),`OR prev_tl=''` 只救「儲存時 prev_tl 為空」的舊列。
- bundled `association.bin` 對比:prev 只用 **漢字 bytes** key,**不含 prev_tl**(`engine/lexicon/src/association_reader.rs:112`)。

### 根因(empirically traced + Codex 確認)
**`prev_tl` 被當 hard filter,但其來源羅馬字形式因提交路徑/版本而異:**

| 提交路徑 | 傳給 NextWord 的 roman | 例(台語) | file:line |
|---|---|---|---|
| 一般候選 commit | 候選 canonical TL(帶聲調符號+連字) | `tâi-gí` | `ActionHandler+Suggestions.swift:158` |
| **連續輸入** commit | `raw_text` = 字面原始打字 slice(刻意不正規化) | `taigi` | `engine/composing/src/transition.rs:813,842,862` |

兩者經 `poj_display_to_tl_display`(`engine/nextword/src/decide.rs:122`)後仍不同 → 同漢字詞,`prev_tl` 存成 `tâi-gí`(一般學的)但連續輸入查詢用 `taigi` → **exact-match miss** → 學過的關聯詞跑不出來。

**加重因子**:`ON CONFLICT(prev_word,next_word,next_tl) DO UPDATE SET prev_tl = excluded.prev_tl`(`NextWordRepository.swift:47-48,76-77`)。UNIQUE **不含** prev_tl → 同一筆 bigram 的 prev_tl 被**最後寫入的路徑覆蓋**,normal/continuous 互洗。hard filter 更脆弱。

**鐵證**:`transition.rs:781-783` 註解自打臉 —「`canonical_text` 用 canonical dictionary key 讓**學習 mode-independent**」。漢字維度做到,但羅馬字維度傳 `raw_text` 違背同一意圖。

### 三索引影響
**非三索引引起。** `prev_tl` filter 自 v3.4.7 (`9f13f505`) 就在;連續輸入 raw_text 路徑自 v3.5.8。#334 沒碰 NextWordSchema/Service/Repository/decide.rs/associations.py。(註:`association.bin` 內容 SHA 因字典重建而變 `02d6d0f2`→`ce7422c8`,但 **key schema 仍 Hanji-only**,user-learned DB 完全沒動。)用戶資料是在單索引版本累積的,跨 mode/跨路徑的 roman 不一致才使其失效。

### Core Principle #7 怎麼套(關鍵設計答案,Codex + code 一致)
**#7 (漢字,羅馬字) pair-key 對 bigram 的 *next*(候選)端是硬約束**(UNIQUE 含 next_tl、後處理以 (hanzi,tl) merge)。**但 *prev*(上下文)端現況本就不是 pair-key** —— bundled bin 只用漢字、user DB 的 UNIQUE 也不含 prev_tl。**正確方向 = prev_tl 降為 disambiguating / ranking 訊號,不當 hard filter。**

### 候選修法(Codex 排 C > A > B;未定,USER review 後決定)
- **C(Codex 推薦)**:查詢改 `WHERE prev_word = ?`;`prev_tl` 移到 `ORDER BY`(exact-match 優先 > empty > mismatch)。同時修跨版本/跨路徑 recall + 保留讀音作排序訊號。範圍 = 兩平台 SQL(`NextWordRepository.swift` + `NextWordService.kt`),**非 engine**(user_association.db 留平台,rust-migration-policy §6)。
- **A(最小)**:純漢字 lookup(對齊 bundled bin)。最貼「配合舊版格式」,但丟掉 prev 讀音區分。
- **B(不適合單獨用)**:連續輸入改傳 canonical TL + DB migration 重正規化既有列。但 continuous effect 拿的是 raw slice,`canonical_tl_form("taigi")` 不會變 `tâi-gí`;舊 raw 列無法無歧義重建。**列為 C 之後的完整對齊 follow-up。**

### 第二 co-bug(同寫入路徑;最終 review 升級)
同一條 raw-roman 寫入路徑造成**第二個資料模型 bug**:連續輸入把 **`next_tl` 也存成 raw/toneless**(`taigi`),一般 commit 存 canonical(`tâi-gí`)。`UNIQUE(prev_word,next_word,next_tl)` 含 next_tl → **同一個 next word 變兩列**;且 nextword `filter.rs:57,68` 按 **`(hanzi, tl)` merge**(非 hanzi)→ 兩列變**兩個預測 → 預測列重複顯示同詞**。

這不是取代主 root cause,而是讓「R1 純查詢放寬」從「乾淨修好」變成「會召回但可能召回成重複」。**修法**:R1 在 `filter.rs` merge 層加 **toneless-collapse**(toneless 列折進同漢字 toned 列;2 guardrail:分隔符+聲調不敏感、不做歧義一對多)讀層消重;R2 讓連續 commit 寫 canonical TL,寫層根治未來 fragmentation(亦為 R5 foundational)。hanzi-only dedup 不可(違 #7 真多音字 重/tāng vs 重/tàng)。詳見 §8 R1/R2。

---

## 3. 自訂詞庫 (Custom Dictionary) — 🔴 真 cross-mode 缺口

### Key 機制
- `custom_dictionary` 表(兩平台同):`id, roman, hanzi, notone, abbrev, roman_num, ...`(iOS `CustomDictionarySchema.swift:51-83`;Android `CustomDictionaryService.kt:400-419`)。dedup key = `(roman, hanzi)` pair(符合 #7)。
- **只存一個 roman 形式**(原樣輸入,raw,非 canonical TL,`ComposingManager.swift:411` / `continuous.rs:1514-1516`)。`notone`/`abbrev`/`roman_num` 三個衍生欄由該單一 roman 經 Rust `derive_notone`/`derive_abbrev`/`normalize_input` 生成(`engine/phonetics/src/derivation.rs:20-64`)。

### 跨 mode 缺口(根因)
1. **單一存形 + 衍生不跨家族**:`derive_notone`(`derivation.rs:20-39`)只小寫化、轉鼻音記號、去聲調,**不折 `ch↔ts` / `oa↔ua`,也不轉拉丁↔注音**。POJ 存 `chiah` → `notone="chiah"`;TL 存 `tsiah` → `notone="tsiah"` → 不同 bucket。
2. **查詢 SQLite mode-blind**:`searchPrefix`(`CustomDictionaryDerivation.swift:49-59`)無 `mode` 參數;query `WHERE notone LIKE ?||'%' OR abbrev LIKE ?||'%'`(`CustomDictionaryRepository.swift:312-344`)純字串前綴比對,不建 `tl:`/`poj:`/`tps:` 家族鍵。
3. **mode-aware 的 `custom_toneless_key`(`shadow.rs:534-572`)在 mode-blind SQLite gate 之後** → 只能 re-key 已通過比對的列,救不回被 SQLite 濾掉的跨家族列。

### 跨 mode 判定
- **TL/POJ ↔ TPS:硬 miss**(注音字形 prefix 永遠不匹配拉丁 `notone`)。
- **TL ↔ POJ:只在兩拼寫巧合相同時命中**(`chiah` vs `tsiah` miss),非設計保證。

### 三索引影響
**未碰自訂詞路徑**(三索引只重建系統字典 `dictionary.bin` + FST;`engine/lexicon/src/search.rs` 的 tl:/poj:/tps: 三索引從不讀 `custom_dictionary.db`)。自訂詞在三索引**前後都**是 mode-fragmented。**但**:三索引把系統字典做成三軸 first-class,自訂詞仍單軸 → **不對稱**(系統字典 tps 查得到、自訂詞 tps 查不到)。

### v3.6.1 結論
🔴 自訂詞**不是真正跨 mode**。修法方向(未定):讓自訂詞像系統字典一樣多家族可查 —— 寫入時 emit 三家族衍生鍵,或查詢時把輸入 + 既有列都 canonicalize 到統一家族再比對。範圍橫跨平台 SQLite + 引擎 derive。**待 USER review 後決定是否納入 v3.6.1 + 方向。**

---

## 4. 備份復原 (Backup/Restore) — 🟢 相容,本身不需修

### 機制
- `.taigi` = 純 **JSON**,有 `version` 欄(目前 = 1,兩平台同:iOS `BackupService.swift:87`;Android `BackupService.kt:50`)。是 **projection**(只存 `{roman,hanzi}` / `{word,count,lastUsed}` / `{prevWord,prevTl,nextWord,nextTl,count,lastUsed}`),非 SQLite dump,**不含**衍生鍵欄、不含 FST。
- **Restore 重新 derive,非 verbatim insert**:
  - 自訂詞 → `save()` 重算 `generateNotone`/`generateRomanNum`(現行 Rust 引擎)。
  - 關聯 → `prevTl`/`nextTl` 過 `pojToTl` 折成 canonical TL(`BackupService.swift:178-181`)。
  - 詞頻 → verbatim merge `ON CONFLICT(word) DO UPDATE SET count=MAX(...)`。
- Restore 只檢查 backup JSON `version >= 1`,**無 DB schema 版本邏輯**(schema migration 各 repo lazy 跑,獨立於 restore;舊 backup 缺欄如 `prevTl` 由 `?? ""` / `optString` 容錯)。

### 三索引影響
**無**。格式與索引解耦。三索引 PR 0 行碰 BackupService/user-DB/derivation 檔。

### v3.6.1 結論
🟢 單→三索引 backup **可乾淨 restore**,re-derive 自動 re-key 到新世界。**但** restore 後的自訂詞繼承 §3 的 mode-blind 限制(restore 不是 bug,但修好 §3 後 restore 的資料才真跨 mode)。**附帶觀察**:iOS 自訂詞 `schemaVersion=1` vs Android 有到 v4 migration —— derived-key 重生歷史的平台差異,不影響 restore(restore 一律 re-derive),僅 parity note。

---

## 5. 三索引機制價值評估(USER 指令 #4)

### 三索引是什麼
系統字典的**搜尋索引**升級:`tl:`/`poj:`/`tps:` 三個 FST key family first-class 並列(`create_fst.py`),退役 `is_tps` short-circuit、`tps_or_mapped_to_er` runtime branch、`tps_to_tl` canonicalize chain。= TPS 成為 first-class 可搜尋 mode + 架構去耦。Tagged v3.5.9 `3c8bec16`(2026-05-29),v3.6.0 `66520db8`(2026-05-31)為對使用者的正式 release。

### 是否仍有價值 — **是,且與本次四 bug 正交**
1. **四個問題沒有一個由三索引造成**(§1-4 逐一 empirically 證明)。回退三索引 **修不好** 任何一個。
2. 三索引解決的是**系統字典查詢**的真實架構債(TPS 半成品 short-circuit、散落特例),這價值獨立成立(eval doc `2026-05-20-triple-index-eval.md` 已評,USER 已選 C/D)。
3. 三索引讓系統字典三軸對稱,**反而凸顯**使用者資料層(關聯 prev_tl、自訂詞單軸)還沒對齊 —— 這是「暴露問題」不是「製造問題」。USER 的直覺(三索引相關)方向上可理解(時間點吻合:升級後資料失效),但**機制上錯位** —— 痛點在 user-data key 層,非 system-dict index 層。

### 真正的橫切原則(本次稽核提煉)
> **使用者歷史 key 必須是 candidate identity(漢字,或 canonical roman),不得含 surface form / mode / input code。**

這正是 `2026-05-20-triple-index-eval.md` §硬約束 #2 早已寫下的契約。詞頻 + 備份**遵守**了(key=漢字 / restore re-derive);**詞關聯(prev_tl)+ 自訂詞(單一 raw roman + mode-blind query)違反**。v3.6.1 的真正主題 = **把這條契約補齊到關聯 + 自訂詞**,而非動三索引。

---

## 6. v3.6.1 重點修復項目(USER 指示列入;方向待 USER 多次 review 後定)

| # | 項目 | 性質 | 範圍 | 候選方向 | 優先 |
|---|---|---|---|---|---|
| 1 | 詞關聯 prev_tl hard-filter → ranking 訊號 | 行為變更 (parity tier) | 兩平台 SQL(非 engine) | C(推薦)/ A / +B follow-up | 🔴 高 |
| 2 | 連續輸入 commit 帶 canonical TL(修 next_tl 污染 + 完整對齊 #7) | 行為變更 | engine `transition.rs` payload + 平台 | = #1 的 B 部分,可後續 | 🟡 中 |
| 3 | 自訂詞跨 mode 可查(多家族衍生鍵 or 查詢端 canonicalize) | 行為變更 | 平台 SQLite + engine derive | 待設計 | 🟡 條件(見 §7.5:非 v3.6.0 回歸,Codex 建議延後,USER 拍板) |
| 4 | 詞頻 hanji-absent 自訂 TPS sliver(隨 #3 一起) | 行為變更 | 同 #3 | 隨 #3 | 🟢 低 |
| 5 | 備份復原 | 不需修 | — | 依賴 #1/#3 修好 | 🟢 — |

### 共同必備(任一行為變更項落地時)
- 跨平台一致:iOS + Android + `behavioral-invariants.md` 新 `INVARIANT_*` + invariant test 同 PR(cross-platform-alignment §3a)。
- Parity tier 標記 + 前後行為描述(§1b)。
- 新增 S 系列 dogfood 驗收(關聯跨 mode recall / 自訂詞跨 mode 命中)。
- Codex pre/post sandwich + `/simplify`。
- **不動三索引。**

### 待 USER 拍板(本文件不替 USER 決定)
1. v3.6.1 收哪幾項(#1 必 / #3 是否同版 / #2 是否拆後續)?
2. #1 走 C / A / 含 B?#3 走「寫入多鍵」or「查詢端 canonicalize」?
3. 是否要先補一個一次性 DB 修復 migration(把既有 prev_tl 正規化或清空),還是純靠查詢放寬向後相容?
4. 拆 PR 邊界(一 PR 一功能 vs 一 PR 一原則)?

---

## 7. 使用者資料 DB 生命週期 + 最佳實踐稽核 (USER 指令 2026-06-03 第二批)

兩平台三個 user-writable SQLite DB 的遷移安全 / 成長上限 / 冗餘清理 / SQLite 最佳實踐 audit。全 file:line traced;Codex ANALYSIS-ONLY 二意見驗證策略(prompt `/tmp/userdata-lifecycle-codex.txt`)。

### 7.1 升級安全 (USER 指令 #2「用戶能否順利升級到 v3.6.1」)

| DB | iOS 遷移 | Android 遷移 | 破壞性? | 舊資料存活 |
|---|---|---|---|---|
| custom_dictionary.db | schemaVersion=1,僅 ALTER+backfill | DATABASE_VERSION=5,僅 ALTER+UPDATE | ❌ 非破壞 | ✅ 全存活 |
| user_frequency.db | 無遷移(CREATE IF NOT EXISTS) | DATABASE_VERSION=1,onUpgrade no-op | ❌ 非破壞 | ✅ 全存活 |
| user_association.db | v<3 → **DROP**;v3→v4 ALTER prev_tl | v0/v1→v2 copy;**v2→v3 DROP 無 copy**;v3→v4 ALTER | ⚠ **pre-v3 破壞** | 僅 v3+ 存活 |

**結論**:
- **現役 v3.6.0 用戶(都在 v4)升 v3.6.1 = 安全**,前提:**推薦修法是純查詢改動,不 bump schema、不 DROP**。
- ⚠ **Codex 修正措辭**:不能宣稱「所有升級路徑安全」。**長期未更新、直接從 pre-v3(單索引時代)跳 v3.6.1 的休眠用戶,仍會被既有 `user_association` pre-v3 DROP 清空關聯**(歷史遺留,非 v3.6.1 新增)。要宣稱全路徑安全,得另修舊遷移。
- **硬約束**:v3.6.1 若加一次性清理 migration,**必須 UPDATE/DELETE,絕不可 DROP/recreate**。

### 7.2 無限膨脹 (USER 指令 #4「用戶資料是否無限制膨脹」)

| DB | iOS | Android | eviction |
|---|---|---|---|
| user_frequency.db | ~22k bounded(cap 20k + batch 2k) | ~22k bounded | count ASC, last_used ASC (LRU) |
| user_association.db | ~55k bounded(cap 50k + batch 5k) | ~55k bounded | 同上 |
| **custom_dictionary.db** | **30000 上限,滿則 throw** | **無上限 ❌** | 無(throw,不驅逐) |

- 詞頻 / 關聯:兩平台 row-count bounded ✅。⚠ **Codex 補**:row-count bound ≠ byte-size hard bound(SQLite freelist、WAL/journal sidecar、字串長度仍影響檔案大小)。
- **自訂詞 Android 無上限 = 真 gap**(parity + 理論膨脹),但 user-authored / self-limiting,優先級**低於** recall 主 bug。
- ⚠ **UX 正解(Codex)**:user-authored 資料**不應 LRU silent-evict**(會誤刪用戶自己的詞)。正確 = hard cap + 明確 error/warning + 管理/刪除/匯出路徑。iOS 的 throw-at-cap 方向對;Android 對齊時要 grandfather 既有 >30000 列、擋新增,**不自動驅逐**。

### 7.3 冗餘 / 未使用資料清理 (USER 指令 #3)

- **Dead-row hazard(兩平台確認)**:`user_association` UNIQUE key = `(prev_word, next_word, next_tl)`,**不含 prev_tl**;但 `ON CONFLICT DO UPDATE SET prev_tl = excluded.prev_tl` 覆寫 prev_tl,且讀取 `WHERE ... prev_tl = ? OR prev_tl = ''`。→ 儲存 prev_tl 為非空且不等於查詢 roman 的列**不可讀但仍佔 50k 額度、仍被 count++**。
- **Codex 關鍵結論**:**推薦修法(`WHERE prev_word = ?`)直接消滅整個 dead-row class** —— 每列都能被 prev_word 讀到,dead 只是 read predicate 造成的不可見,**非真損壞**。
- **Codex 明確警告**:relaxed query 下這些列**不再 dead**,**絕不可 DELETE「dead rows」**(= 刪用戶學過的關聯)。
- **是否需 data migration**:**不需要。查詢改動本身即把歷史列全部恢復可讀。** 加回歸測試驗證 mismatched 非空 prev_tl 也能回傳即可。
- **orphan 清理**:目前**完全無**(詞 key 是字串,與 bundled dict 解耦,被移除的 dict 詞對應的詞頻/關聯永不清)。Codex 警告:orphan 清理要極小心 —— 不能只用 bundled dict 判定(會誤刪自訂詞);台語 identity 必須 `(hanzi, tl)` pair。低優先。
- **空間回收**:三 DB **無 VACUUM / auto_vacuum / integrity_check**(兩平台)→ prune 刪除後空頁進 freelist、檔案不縮(可重用)。⚠ **Codex**:`VACUUM` 重建 DB 需約 2x 暫存空間,**不可在鍵盤啟動 / 遷移路徑跑**;屬 maintenance 非 hotfix。

### 7.4 最佳實踐 (USER 指令 #1 + #5)

**遵循 ✅**:全 parameterized SQL(無注入,合 security-rules.md)、三 DB 都有 index、批次匯入包 transaction、iOS 單一序列化連線/DB + WAL→DELETE 遷移、詞頻/關聯 LRU eviction。

**缺口 / 非最佳實踐**:
- 無 VACUUM / integrity_check / `PRAGMA optimize`(Codex 建議 schema/index 變更後跑 optimize,低頻診斷用 quick_check)。
- 無 age / orphan 清理(僅 count-cap)。
- 關聯 dead-row 累積(§7.3,推薦修法消滅)。
- Android 自訂詞無上限(§7.2)。
- record path per-row autocommit 且**錯誤被吞**(log-only,靜默寫入失敗)。
- 冗餘 index:iOS `idx_word`(被 UNIQUE 遮蔽)、`idx_user_prev_word`(composite 左前綴)。
- cap policy 三處不同結構家(CustomDictionaryCapacityPolicy / UserFrequencyPruner / NextWordService inline)。
- **iOS/Android divergence**:自訂詞上限(iOS 30k throw / Android 無)、schema 版號(iOS 1 / Android 5)、關聯遷移(iOS DROP / Android copy-then-DROP)、journal mode(iOS 三 DB pin DELETE / Android 僅關聯 pin)。

**Codex 補充風險(本輪未列、值得記)**:
- **詞頻 key = displayText(漢字優先)可能違反 Core Principle #7**:多音字(重/tîng vs 重/tāng)合併到同一 `word=重` 列 → 詞頻 ranking 合併污染。比冗餘 index 重要,非 v3.6.1 hotfix。
- **備份/隱私政策**:自訂詞應可備份;詞頻/關聯 = learned typing behavior,或應 exclude from iCloud / Android Auto Backup,或至少明確決策。
- **iOS app-extension 多進程**:containing app + keyboard extension 共用 App Group DB → 避免長 write transaction(process suspension 風險,Apple TN2408)。
- **Android 遷移執行緒**:`SQLiteOpenHelper` upgrade 勿在 main thread 跑長遷移;`onConfigure` 才是設 WAL/foreign keys 的位置。

### 7.5 v3.6.1 ship 優先序(Codex 收斂;USER 拍板 scope)

| 優先 | 項目 | Codex 建議 |
|---|---|---|
| **1 — ship now** | 詞關聯 recall fix(`WHERE prev_word=?` + prev_tl 降 ranking hint)+ 回歸測試 | **hotfix 正解。同時修 recall + 消滅 dead-row class。無需 data migration。** |
| 2 — 條件 ship | 自訂詞跨 mode | **僅當確認是 v3.6.0 使用者可見回歸才進 v3.6.1。本稽核顯示自訂詞 mode-fragmentation 在三索引前就存在 → 非 v3.6.0 回歸 → Codex 建議延後;USER 拍板。** |
| 3 — later | Android 自訂詞上限對齊(grandfather + 擋新增,不自動驅逐) | 下一輪 |
| 4 — later | VACUUM / integrity / orphan 清理 / `PRAGMA optimize` / 詞頻多音字 key / 備份隱私 / journal-mode 對齊 | maintenance 輪,逐項評估 |

**Net(Codex)**:v3.6.1 保持 **query-only association fix + 回歸測試**,不加 cleanup migration。其餘放下一輪。

---

## 8. v3.6.1 多 PR 計畫 (USER 拍板 2026-06-03:一併處理全部,well-planned PRs,context 清除可獨立 debug)

**USER 決策**: v3.6.1 處理**全部**稽核項目;每 PR 自足、PR 間清 context、出問題好定位。大部分 auto mode(依 Claude+Codex 建議);commit to main。

**一 round = 一 PR**(`~/.claude/rules/round-workflow.md`);PR sizing 200-500 LOC;每個行為變更 PR 跨 iOS+Android + `behavioral-invariants.md` 新 INVARIANT + invariant test + S 系列 dogfood 同 PR(cross-platform-alignment §3a),parity tier 標記。

| Round | Scope | 範圍 | 風險 | auto mode? |
|---|---|---|---|---|
| **R0 admin** (本輪) | roadmap + 本報告 + memory plan,commit to main | docs | none | — |
| **R1** 🔴 | 詞關聯 recall + dedup:(a) 查詢 `WHERE prev_word = ?` + prev_tl 移 `ORDER BY` ranking(CASE exact>empty>mismatch,**rank-before-truncate**,overfetch limit×2);(b) **`filter.rs` merge 加 toneless-collapse** — toneless next_tl 列折進同漢字 toned 列。**2 guardrail**:① 分隔符+聲調皆不敏感(復用 `roman_reading_eq` PR #380 continuous.rs:298);② **僅當恰好一個** toned 列匹配才折,多個(重/tāng+重/tàng)保留不歸併。回歸測試(mismatched 非空 prev_tl 仍回傳 + dup-collapse + LIMIT)+ `INVARIANT_NEXTWORD_PREV_HANJI_LOOKUP` + S dogfood | iOS+Android SQL + engine `filter.rs`(無 schema,無 migration) | **低** | ✅ auto + Codex |
| **R2** | 連續輸入 commit 帶 canonical TL(非 raw_text)→ 修 next_tl/prev_tl 來源 fragmentation(寫層根治)+ 顯示;完整對齊 #7 next 端。**設計 fork→Codex 先**:`CommitContinuous` 加 `association_tl` 欄(**proto triple-touch** rust-migration-policy §4)+ 連續候選帶 candidate TL sidechannel + NailedSegment 欄 + emit **僅進 NextWord effect 不進 lattice key**(lattice 保 raw `entry.roman` continuous.rs:264-291) | engine `transition.rs`+proto + iOS/Android wiring + autocomplete suggestion TL | 中 | ⚠ Codex 設計 fork 先 |
| **R3** | 自訂詞 cross-mode lookup:**設計 fork → Codex consult 先;偏好 (a) 寫入多家族衍生鍵 +backfill**(Codex:查詢端 canonicalize 風險破壞 custom lattice byte identity) | engine `derivation.rs` + 平台 query + 非破壞 migration(多家族 backfill) | 中-高 | ⚠ Codex 設計 fork 先 |
| **R4** | Android 自訂詞 cap parity:hard cap 30000 + grandfather 既有 + 擋新增明確 error(**不自動驅逐** user-authored);cap-policy 結構整併 | Android only | 低 | ✅ auto |
| **R5** ⚠ | 多音字 frequency `(hanji, tl)` pair-key:修 #7 違反(重/tîng+重/tāng 合併)。**全範圍非單純 ALTER**:user_frequency schema ALTER+tl + **`FrequencyEntry` proto +tl** + ranking `FrequencyMap` key(score.rs:173) + candidate key extraction(`buildFrequencyEntries` dedup ComposingManager) + **backup/import migration** + tolerant(舊 tl='' 仍配,比照 R1)。**依賴 R2**(需 canonical TL 於連續 commit,否則詞頻同樣 raw/canonical fragmentation) | engine `score.rs`+proto + iOS/Android schema(**非破壞 ALTER,絕不 DROP**) | **高**(re-key 全詞頻 + ranking + 高風險 migration) | ⚠ Codex pre/post,謹慎,**必在 R2 後** |
| **R6** | SQLite hygiene:VACUUM on-demand(**非啟動路徑**,prune 後/用戶壓縮鍵)、`PRAGMA optimize`(schema/index 變後)、integrity_check 診斷、journal-mode parity、移除冗餘 index、record-path 錯誤上拋(停止吞錯) | iOS+Android | 低 | ✅ auto |
| **R7** | 備份/隱私政策:learned data(詞頻/關聯)是否 exclude iCloud/Android Auto Backup(自訂詞保持可備份) | iOS backup attrs / Android `fullBackupContent` | 低(**需 USER 產品決策**) | ⛔ STOP — USER 產品決策後才 impl |

**刻意不做(YAGNI,Codex)**: orphan 清理(被移除 bundled dict 詞對應的詞頻/關聯)—— risk > value,且須 `(hanzi,tl)`-safe predicate + 保留自訂詞才安全。v3.6.1 不做,未來有明確 corruption predicate 再議。

**總計**: **7 個實作 round (R1-R7)** + R0 admin(本輪 docs commit)。

**Round 排序理由**: R1 最先(用戶實際回報的 bug;讀層自足 — 查詢放寬 + toneless-collapse 同時修 recall + 消滅 dead-row + 防重複顯示,Codex 確認可獨立 ship)。R2 寫層根治 fragmentation + 為 R5 foundational。R3 自訂詞獨立。R4 Android-only 小。R5 最高風險(re-key 全詞頻 + 高風險 migration)**必在 R2 後**。R6 hygiene 低風險。R7 需產品決策殿後(與 R5 的 backup migration 協調)。每 round 之間 context 清除,memory `project_user_data_cross_mode_audit.md` + 本報告 = 唯一 hand-off。

**依賴**: R5 → 依賴 R2(canonical TL)。R1 讀層自足(不需 R2 先)。R3/R4/R6 獨立。R7 產品決策 + 與 R5 backup 協調。

**auto mode 例外(真 BLOCK,需 USER 介入)**: R2/R3 設計 fork(Codex consult 後若仍兩案相當)、R5 若 review 發現 ranking 回歸風險過高、R7 備份隱私產品決策。其餘 auto。

**最終 review 收斂(2026-06-03)**: 此計畫經 Codex ANALYSIS-ONLY **對抗 review + 確認 pass** 兩輪。Codex 起初 7 項 objection(R1 非乾淨單獨 / 重複曝光 / LIMIT 飢餓 / 多音字 prev 錯讀音 / R2 需 proto / 平台替換較差 / R5 under-scoped),全部以上述修正 resolved。**最終 Codex「no remaining objection」** —— 條件為 R1 toneless-collapse 含 2 guardrail(分隔符+聲調不敏感、不做歧義一對多)。Claude 亦無異議。雙簽核完成。Prompts: `/tmp/v361-plan-adversarial-codex.txt` + `/tmp/v361-plan-confirm-codex.txt`。

## 9. 三索引繼續實作價值 (USER 指令 #5)

**三索引已 100% 完成,無 pending 實作項。** 三軸全 first-class 並列(empirically 確認):
- **TL**: 本來就是 first-class。
- **POJ**: B-1 #308 (`9b365c9c`) + B-2 #309 (`f2a4f4e1`) — POJ first-class key emission + mode-aware lattice/guard。
- **TPS**: D #334-340 — `tps:` FST family + 退役 is_tps short-circuit / tps_or_mapped_to_er / tps_to_tl chain。
- `create_fst.py` 確認 emit tl:/poj:/tps: 三家族。

**結論**:
- **沒有「繼續實作」的 backlog** —— eval doc 的 C 計畫(POJ+TPS 都 first-class)已透過 B 系列 + D 系列達成。
- **價值已交付且應保留**:三軸對稱搜尋(TPS/POJ 教學/學習者生態可搜可學可排序)+ 架構去耦(退役大量 ad-hoc special case)。回退 = 失去 TPS first-class 搜尋 + 重新引入已退役的架構債。
- **與 v3.6.1 正交**:本次四 bug 全在**使用者資料層**(關聯/詞頻/自訂詞 key),三索引在**系統字典搜尋層**,零交集。三索引把系統字典做對稱反而**凸顯**使用者資料層未對齊 —— v3.6.1 正是補齊使用者資料層。
- **不需也不應為 v3.6.1 動三索引。**

---

## Appendix — 稽核方法與證據鏈

- 6 個 read-only general-purpose agent 並行 audit(iOS/Android/engine × 兩批),每結論 file:line。
- Codex `codex exec` ANALYSIS-ONLY 二意見 ×2:
  - 詞關聯根因 + 修法排名 + #7 適用範圍,prompt `/tmp/nextword-assoc-codex.txt`。修正一處:`association.bin` 非 byte-identical(SHA 變),但 key schema/user-DB 未動的結論成立。
  - DB 生命週期 + 最佳實踐 verdict(§7),prompt `/tmp/userdata-lifecycle-codex.txt`。確認 query-only fix 無需 data migration、dead-row 不可 DELETE、VACUUM 屬 maintenance 非 hotfix、自訂詞 user-authored 不可 LRU 驅逐;補多音字詞頻 key / 備份隱私 / 多進程 / 遷移執行緒風險。
  - **計畫對抗 review**(§8,USER 要求「直到雙方無異議」),prompt `/tmp/v361-plan-adversarial-codex.txt`。Codex 7 objection:發現 `next_tl` fragmentation 第二 co-bug + filter `(hanzi,tl)` merge 重複 + R1 LIMIT 飢餓 + R2 需 proto + R5 under-scoped。
  - **計畫確認 pass**,prompt `/tmp/v361-plan-confirm-codex.txt`。全 7 項 resolved;最終「no remaining objection」(條件:R1 toneless-collapse 2 guardrail — 分隔符+聲調不敏感、不做歧義一對多)。雙簽核完成。
- 三索引 ship 範圍:roadmap `docs/roadmap.md` + eval `docs/reports/2026-05-20-triple-index-eval.md` + memory `project_v359_d_tps_triindex_plan.md`。
- 所有「跨 mode 共享?」判定均 trace 實際 key 構造 + lookup query,非 code-reading 推論。
