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

## Bugfix: SQLite WAL storage bloat (separate branch)

### Problem
App「文件與資料」佔用 8.25GB。根因：4 個 SQLite 資料庫啟用 WAL 模式但**沒有 checkpoint 邏輯**。

Keyboard extension 生命週期短（隨時被系統殺掉），SQLite 自動 checkpoint 來不及觸發，導致 `.db-wal` 檔案無限增長。

### Affected databases

#### iOS — 4 個全部啟用 WAL，零 checkpoint
| 資料庫 | 用途 | 寫入頻率 | WAL |
|--------|------|---------|:---:|
| `dictionary.db` | 主詞庫（唯讀查詢） | 低 | ✅ |
| `user_frequency.db` | 使用頻率 | 每次打字 | ✅ |
| `user_association.db` | 詞彙關聯 | 每次選字 | ✅ |
| `custom_dictionary.db` | 自訂詞庫 | 使用者手動 | ✅ |

#### Android — 僅 user_association.db 啟用 WAL
| 資料庫 | 用途 | 寫入頻率 | WAL |
|--------|------|---------|:---:|
| dictionary.db (LexiconService) | 主詞庫（唯讀） | 低 | ❌ |
| user_frequency.db (UserFrequencyService) | 使用頻率 | 每次打字 | ❌ |
| user_association.db (NextWordService) | 詞彙關聯 | 每次選字 | ✅ |
| custom_dictionary.db (CustomDictionaryService) | 自訂詞庫 | 使用者手動 | ❌ |

Android 風險較低（IME service 常駐，不像 iOS extension 頻繁被殺），但應一併修正保持一致。

### Root cause
- iOS: `SQLiteConnectionManager.configure()` 對所有 DB 設定 `PRAGMA journal_mode=WAL;`，零 checkpoint
- Android: `NextWordService.initializeUserDb()` 設定 `PRAGMA journal_mode=WAL;`，零 checkpoint
- iOS extension 反覆啟動/銷毀 → WAL 累積數月 → GB 級膨脹
- Android IME service 常駐 → 自動 checkpoint 有機會觸發，但不保證

### Why WAL is wrong for keyboard IME
WAL 優勢（並行讀寫、寫入速度）在 IME 場景幾乎無用：
- 寫入量小（每次一筆頻率更新）
- 讀寫幾乎不同時
- iOS extension 生命週期短（秒級~分鐘級），WAL checkpoint 無法可靠觸發
- Android IME service 常駐但也可能被系統回收

### Fix plan

**Step 1 (iOS)**: 改 journal mode `WAL` → `DELETE`
- `SQLiteConnectionManager.configure()` 中 `PRAGMA journal_mode=DELETE;`
- DELETE 模式的 journal 檔每次交易後自動清除，不會累積

**Step 2 (iOS)**: 一次性清理舊 WAL 檔（為現有用戶回收空間）
- App 啟動時檢測 `.db-wal` 檔案是否存在
- 若存在：先用 WAL 模式 open → `PRAGMA wal_checkpoint(TRUNCATE)` → close → 再用 DELETE 模式 open
- 清理後 `.db-wal` 和 `.db-shm` 自動移除

**Step 3 (Android)**: 移除 NextWordService 的 WAL
- `NextWordService.initializeUserDb()` 移除 `db.rawQuery("PRAGMA journal_mode=WAL;", null)?.close()`
- SQLite 預設 journal mode 為 DELETE，移除即可
- 同樣需要一次性清理舊 WAL：開啟時若偵測到 `.db-wal` 存在，先 checkpoint 再切回 DELETE

**Step 4**: 驗證（雙平台）
- 確認新安裝不產生 WAL 檔
- 確認升級用戶 WAL 被清理、空間回收
- 確認讀寫功能正常（頻率記錄、自訂詞庫、關聯記錄）

### Files to modify
- **iOS**: `Lexicon/Database/SQLiteConnectionManager.swift` — journal mode + migration checkpoint
- **iOS**: 可能需要 `KeyboardViewController` 或 `AppDelegate` 加啟動時 migration hook
- **Android**: `ime/dictionary/NextWordService.kt` — 移除 WAL pragma + 一次性 checkpoint migration

**Status**: Not Started

---

## Future

### Android Stage 2: (TBD)
**Goal**: TBD
**Status**: Not Started
