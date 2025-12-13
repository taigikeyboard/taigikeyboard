# 輸入法 TODO

## 1. Trie 優化

### 1.1 InputNormalizer 統一正規化為 TL 數字聲調
- 任何輸入格式都正規化為 TL 數字聲調：
  - POJ 調符（hó）→ TL 數字（hoo2）
  - TL 調符（hóo）→ TL 數字（hoo2）
  - POJ 數字（ho2）→ TL 數字（hoo2）
  - TL 數字（hoo2）→ TL 數字（hoo2）
- pojToTl 轉換規則（參考 KeSi `tsuan_kongke`）：
  ```
  ch → ts
  ou → oo
  o͘ → oo
  ⁿ → nn
  oa → ua
  oe → ue
  eng → ing
  ek → ik
  ```
- 參考：`references/KeSi/kesi/susia/kongke.py`

### 1.2 InputNormalizer 單元測試（JUnit）
- 調符轉數字、大小寫、連字符、pojToTl 等
- `android/app/src/test/java/.../InputNormalizerTest.kt`

### 1.3 縮減 Trie key 種類：7 種 → 4 種
- 移除前綴（`poj:`/`tl:`/`hanzi:`）
- 只存 TL：數字聲調、無聲調、縮寫、漢字
- ~~無聲調欄位：音節數 = 1 時不產生~~（已改為全部產生，因 MARISA-trie 前綴搜尋長詞優先會截斷短詞）
- 應用層根據 InputMode 選欄位並去重
- 更新建構腳本：
  - `dictionary2/build/03_create_trie_db.sh`
  - `dictionary2/build/04_create_trie.py`

## 2. 詞頻優化

- [x] 改用線性加權排序
  - ~~目前問題：用過 1 次就排在所有沒用過的高頻詞前面~~
  - 公式：`score = min(userFreq, 50) * 100 + baseFreq`
  - 進階（未來）：時間衰減，最近用過的權重更高

- [x] 長度匹配優先排序
  - 公式：`score = matchBonus + min(userFreq, 50) * 100 + baseFreq`
  - matchBonus: 完全匹配 10000 / 短詞 5000 / 長詞 0
  - 參考：FREQ.md

## 3. 平台整合

- [x] Android 整合 MARISA-trie native library
- [ ] iOS 整合 MARISA-trie native library
- [ ] 詞庫管理 UI（啟用/停用詞庫）

## 4. 未來功能

- [ ] 下一詞預測：Bigram 統計（需語料 + 斷詞）
