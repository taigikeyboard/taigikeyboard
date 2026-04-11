# Implementation Plan: v3.4.8 Refactoring

## Overview
Cross-platform refactoring to reduce coupling, extract shared components, and improve code organization.
This refactoring aims to build the right foundation for the future — do the correct thing now, handle it carefully.

---

## Completed (v3.4.8)

### iOS
- Stage 1: App/ folder cleanup
- Stage 2: Extract Overlays from Autocomplete
- Stage 3: Extract Phonetics from Input
- Stage 4: Reduce Autocomplete cross-folder coupling
- Stage 5: Unify DEBUG logging (DebugLogger)
- Stage 6: KeyboardViewController review & cleanup
- Stage 7: Actions/ review & cleanup
- Stage 8: NextWord State deduplication
- Stage 8b: App/ simplify scan fixes
- Stage 9: Tab3 data view deduplication
- Stage 10: NextWord decoupling from ActionHandler
- Stage 11: Emojis/ cleanup
- Stage 12: Callouts/ cleanup
- Stage 13: Settings/ cleanup
- Stage 13b: TPS bidirectional sync cleanup
- Stage 14: Styling/ review & cleanup
- Stage 14b: Move LayoutPreviewAssets to Resources/Assets

### Android
- Stage 1: Cleanup & extract shared UI components

---

## Bugfix: SQLite WAL storage bloat (branch: `bugfix-wal`)

### Problem
App「文件與資料」佔用 8.25GB。根因：SQLite WAL 模式 + 缺少 checkpoint 邏輯。
iOS keyboard extension 生命週期短（隨時被系統殺掉），auto-checkpoint 來不及觸發，`.db-wal` 檔案無限增長。

### Affected databases

#### iOS — 4 個全部啟用 WAL，零 checkpoint
| 資料庫 | 用途 | 讀頻率 | 寫頻率 | WAL | 需要 WAL？ |
|--------|------|--------|--------|:---:|:---------:|
| `dictionary.db` | 主詞庫 | 每次按鍵 | 從不（唯讀） | ✅ | **否** — 唯讀，WAL 毫無意義 |
| `user_frequency.db` | 使用頻率 | 每次按鍵 | 每次選字 | ✅ | **否** — DispatchQueue 序列化，WAL 並行優勢無效 |
| `user_association.db` | 詞彙關聯 | 選字時 | 選字時 | ✅ | **否** — 同上 |
| `custom_dictionary.db` | 自訂詞庫 | 每次按鍵 | 使用者手動 | ✅ | **否** — 同上，寫入極少 |

#### Android — 僅 user_association.db 啟用 WAL
| 資料庫 | 用途 | 讀頻率 | 寫頻率 | WAL | 需要 WAL？ |
|--------|------|--------|--------|:---:|:---------:|
| `dictionary.db` | 主詞庫（唯讀） | 每次按鍵 | 從不 | ❌ | 否（正確） |
| `user_frequency.db` | 使用頻率 | 每次按鍵 | 每次選字 | ❌ | 否（正確） |
| `user_association.db` | 詞彙關聯 | 選字時 | 選字時 | ✅ | **否** — 見下方分析 |
| `custom_dictionary.db` | 自訂詞庫 | 每次按鍵 | 使用者手動 | ❌ | 否（正確） |

### WAL 需求分析

**iOS — 全部不需要 WAL**
- `SQLiteConnectionManager` 用 `DispatchQueue` 序列化所有讀寫
- WAL 的核心優勢（concurrent readers/writers）被完全抵消
- Extension 短命 → auto-checkpoint 無法觸發 → WAL 反而有害

**Android `user_association.db` — WAL 可移除**
- 用 `Dispatchers.IO` 有真正並行（`predict()` 讀 + `recordAssociation()` 寫）
- 但每個操作都是單筆 INSERT/UPDATE 或小型 SELECT（< 1ms）
- 碰撞窗口極小，`user_frequency.db` 讀寫頻率更高且沒用 WAL 也正常運作
- 移除後最差情況：偶爾 ~1ms 延遲，使用者無感

### Root cause
- iOS: `SQLiteConnectionManager.configure()` 對所有 DB 設定 `PRAGMA journal_mode=WAL;`，零 checkpoint
- Android: `NextWordService.connectUserDb()` 設定 `PRAGMA journal_mode=WAL;`，零 checkpoint
- iOS extension 反覆啟動/銷毀 → WAL 累積數月 → GB 級膨脹
- Android IME service 常駐 → auto-checkpoint 通常來得及，風險較低

### Decision: 全部改 DELETE mode

統一雙平台使用 DELETE journal mode：
- DELETE 模式的 journal 檔每次交易後自動清除，不會累積
- 雙平台邏輯統一，不需要分別處理 checkpoint
- 徹底消除 WAL 膨脹風險

### Fix plan

> **重要**：必須先改程式碼（移除 WAL pragma），再加遷移邏輯。
> 如果順序反了，遷移 checkpoint 完成後 configure() 又會把 journal mode 設回 WAL。
> （Codex review 指出的關鍵風險）

**Step 1 (iOS)**: 移除 WAL pragma
- `SQLiteConnectionManager.configure()` 中移除 `PRAGMA journal_mode=WAL;`
- SQLite 預設為 DELETE mode，移除即可（或改為明確 `PRAGMA journal_mode=DELETE;`）
- **修改檔案**: `ios/Sources/TaigiKeyboard/Lexicon/Database/SQLiteConnectionManager.swift`

**Step 2 (iOS)**: 一次性遷移 — 清理舊 WAL 檔
- 啟動時偵測 `.db-wal` 檔案是否存在
- 若存在：先用 WAL 模式 open → `PRAGMA wal_checkpoint(TRUNCATE)` → close
- 重新用 DELETE 模式 open → `.db-wal` 和 `.db-shm` 自動移除
- 遷移邏輯只跑一次
- **修改檔案**: `SQLiteConnectionManager.swift`（或獨立 migration helper）

**Step 3 (Android)**: 移除 NextWordService 的 WAL + 一次性遷移
- `NextWordService.connectUserDb()` 移除 `PRAGMA journal_mode=WAL;`
- 開啟時偵測 `.db-wal` 存在 → checkpoint → 切回 DELETE
- **修改檔案**: `android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/NextWordService.kt`

**Step 4**: 驗證（雙平台）
- [ ] 新安裝不產生 `.db-wal` / `.db-shm` 檔
- [ ] 升級用戶舊 WAL 被 checkpoint 清理、空間回收
- [ ] 讀寫功能正常（頻率記錄、自訂詞庫、關聯記錄、下一字預測）
- [ ] 無 SQLITE_BUSY crash（Android 特別注意）

### Review notes (Codex)
- 遷移順序是最高風險：必須先改 code 再跑 migration，否則 WAL 會被重新啟用
- Android 的 SQLITE_BUSY 處理目前只有 broad `catch Exception`，這是**既有問題**，移除 WAL 不會惡化
- 未發現多程序共用資料庫的風險

**Status**: Complete — 待手動 build 驗證 + 上版

---

## Future

### Android Stage 2: (TBD)
**Goal**: TBD
**Status**: Not Started
