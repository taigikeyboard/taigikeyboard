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
| 資料庫 | 用途 | 寫入頻率 |
|--------|------|---------|
| `dictionary.db` | 主詞庫（唯讀查詢） | 低 |
| `user_frequency.db` | 使用頻率 | 每次打字 |
| `user_association.db` | 詞彙關聯 | 每次選字 |
| `custom_dictionary.db` | 自訂詞庫 | 使用者手動 |

### Root cause
- `SQLiteConnectionManager.configure()` 設定 `PRAGMA journal_mode=WAL;`
- 整個 codebase **零** checkpoint 呼叫（`wal_checkpoint` / `wal_autocheckpoint` 搜尋結果為空）
- Extension 反覆啟動/銷毀 → WAL 累積數月 → GB 級膨脹

### Why WAL is wrong for keyboard extension
WAL 優勢（並行讀寫、寫入速度）在 extension 場景幾乎無用：
- 生命週期短（秒級~分鐘級）
- 寫入量小（每次一筆頻率更新）
- 讀寫幾乎不同時
- 但 WAL 的代價（需要 checkpoint）在 extension 特別嚴重

### Fix plan
**Step 1**: 改 journal mode `WAL` → `DELETE`
- `SQLiteConnectionManager.configure()` 中 `PRAGMA journal_mode=DELETE;`
- DELETE 模式的 journal 檔每次交易後自動清除，不會累積

**Step 2**: 一次性清理舊 WAL 檔（為現有用戶回收空間）
- App 啟動時檢測 `.db-wal` 檔案是否存在
- 若存在：先用 WAL 模式 open → `PRAGMA wal_checkpoint(TRUNCATE)` → close → 再用 DELETE 模式 open
- 清理後 `.db-wal` 和 `.db-shm` 自動移除

**Step 3**: 驗證
- 確認新安裝不產生 WAL 檔
- 確認升級用戶 WAL 被清理、空間回收
- 確認讀寫功能正常（頻率記錄、自訂詞庫、關聯記錄）

### Files to modify
- `Lexicon/Database/SQLiteConnectionManager.swift` — journal mode + migration checkpoint
- 可能需要 `KeyboardViewController` 或 `AppDelegate` 加啟動時 migration hook

**Status**: Not Started

---

## Future

### Android Stage 2: (TBD)
**Goal**: TBD
**Status**: Not Started
