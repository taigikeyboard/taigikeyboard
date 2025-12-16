# 台語鍵盤 - 技術規格

確保 iOS / Android 雙平台實作一致。

---

## 文件列表

| 檔案 | 說明 |
|------|------|
| `files.md` | 雙平台檔案對應 |
| `composing.md` | 雙狀態組字（rawInput / composingText） |
| `tone.md` | 聲調處理（輸入、顯示、搜尋、修正） |
| `autocomplete.md` | 自動完成服務 |
| `sort.md` | 候選詞排序（v3 公式） |
| `trie.md` | MARISA-trie 整合 |
| `theme.md` | 主題設計 |
| `log.md` | Debug Log 規範 |
| `khiin.md` | 起引輸入法參考 |

---

## 使用方式

1. **新增功能前** - 先查閱相關 spec，確認雙平台邏輯
2. **修改一側後** - 更新 spec，確保另一側同步
3. **發現差異時** - 在 spec 記錄，討論是否對齊

---

## 維護原則

- 繁體中文
- 程式碼範例標註來源檔案
- 保持文件與程式碼同步
- 記錄「為什麼」而非「是什麼」
