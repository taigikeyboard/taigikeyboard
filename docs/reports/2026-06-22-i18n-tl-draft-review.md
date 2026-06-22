# P3b R5-1 — Tâi-lô draft review sheet (2026-06-22)

**Status: REVIEW-PENDING. Mechanical first pass — NOT yet linguistically verified.**

## Why this exists

`tailo` (Tâi-lô romanization) was added for all 211 UI keys to wire the GeneratedMap display-language path (Android `GeneratedTaigiStrings.kt` + iOS `nan-Latn-TW-x-tailo.lproj`). tailo is **DEBUG-selectable only** this round — it is NOT in the production picker (`productionLanguages` unchanged = hanji/en/ja). Promotion to production (R5-2) is gated on the manual review below.

## How the draft was produced (CP#3 — authoritative-source-only)

- Source = the project's MOE-derived `dictionary/output/dictionary.csv` (`hanzi → tl`, `kautian_main`, frequency). NO reading was invented.

- Per hanji string: greedy longest-match dictionary segmentation; per matched word the reading is picked by (1) `kautian_main==True`, (2) a 教育部-leaning tie-break heuristic (penalize central-vowel `ir`/`er` + caron variants; prefer `j-` over `l-` for the 字/日/入 class), (3) highest frequency. Latin / `{placeholders}` / punctuation pass through verbatim.

- ⚠ The tie-break is a HEURISTIC, **not** a true 教育部第一推薦音讀 ordering. The dictionary itself often flags multiple readings as `kautian_main` (e.g. 字 `jī`/`lī`), and segmentation / context 多音 / compound hyphenation / tone-sandhi are NOT resolved mechanically.

## ⚠ Review gate (Codex-flagged)

- **Review ALL 211 rows, not only MED/LOW.** A HIGH row can still be wrong (wrong segmentation, context-dependent reading, dictionary gap).

- The tailo map + `.lproj` ship in the release binary even while DEBUG-only-selectable. **Do NOT release (and do NOT run R5-2 promote) before this sheet is manually reviewed + corrected.**

- Edit the corrected reading directly in `i18n/*.json` (`values.tailo`), then `make i18n` to regenerate.

## Confidence distribution

`{'HIGH': 63, 'LOW': 85, 'MED': 63}` (HIGH = clean single-reading match; MED = multi-`kautian_main` word, 正音-leaning pick; LOW = single-char fallback used somewhere). MED/LOW = needs-eyeball, NOT necessarily wrong.

## Flag legend

- `X=>r:AMB` — word X had multiple `kautian_main` readings; draft picked `r`.
- `X=>r:1char` — single-char fallback (no compound dict entry matched); reading `r` is the char's standalone reading, may need compound hyphenation.
- `X:MISS` — not in dictionary (should be none).


## `common` (21 keys)

| key | 漢字 (hanji) | drafted Tâi-lô | conf | flags |
|---|---|---|---|---|
| `cancel` | 取消 | tshú-siau | HIGH |  |
| `ok` | 好 | hó | LOW | 好=>hó:1char |
| `delete` | 刪除 | san-tî | MED | 刪除=>san-tî:AMB |
| `back` | 返回 | tńg huê | LOW | 返=>tńg:1char; 回=>huê:1char |
| `viewWebsite` | 官方網站 | kuann-hong bāng-tsām | HIGH |  |
| `exportFailed` | 匯出失敗 | huē-tshut sit-pāi | HIGH |  |
| `importFailed` | 匯入失敗 | huē-ji̍p sit-pāi | MED | 匯入=>huē-ji̍p:AMB |
| `moeDict` | 教育部臺灣台語常用詞辭典 | kàu-io̍k-pōo tâi-uân-tâi-gí siâng-iōng-sû sû-tián | MED | 常用詞=>siâng-iōng-sû:AMB |
| `newwordDict` | 公視台語台台語新詞辭庫 | kong-sī tâi-gí-tâi tâi-gí sin sû sî khòo | MED | 台語台=>tâi-gí-tâi:AMB; 台語=>tâi-gí:AMB; 新=>sin:1char; 詞=>sû:1char; 辭=>sî:1char; 庫=>khòo:1char |
| `kunggeDict` | 工藝中心臺灣台語工藝詞庫 | kang-gē tiong-sim tâi-uân-tâi-gí kang-gē sû khòo | LOW | 詞=>sû:1char; 庫=>khòo:1char |
| `iTaigiDict` | iTaigi愛台語 | iTaigi ài-tâi-gí | HIGH |  |
| `taiwanJapanDict` | 臺日大辭典台語譯本 | tâi ji̍t tuā sû-tián tâi-gí i̍k-pún | LOW | 臺=>tâi:1char; 日=>ji̍t:1char; 大=>tuā:1char; 台語=>tâi-gí:AMB |
| `taiHuaDict` | 台華線頂對照典 | tâi huâ suànn-tíng tuì-tsiàu tián | LOW | 台=>tâi:1char; 華=>huâ:1char; 典=>tián:1char |
| `taiwanPlantDict` | 台灣植物名彙 | tâi-uân tsi̍t-bu̍t miâ luī | MED | 植物=>tsi̍t-bu̍t:AMB; 名=>miâ:1char; 彙=>luī:1char |
| `sttiDict` | 教育部學科術語臺灣台語對譯 | kàu-io̍k-pōo ha̍k-kho su̍t-gí tâi-uân-tâi-gí tuì-i̍k | MED | 術語=>su̍t-gí:AMB |
| `accentDict` | 腔口差 | khiunn-kháu tsha | LOW | 差=>tsha:1char |
| `fontSystemDefault` | 系統 | hē-thóng | HIGH |  |
| `fontOpenHuninn` | 粉圓 | hún-înn | HIGH |  |
| `fontIansui` | 芫荽 | ian-sui | MED | 芫荽=>ian-sui:AMB |
| `fontGenYoMin` | 源樣明體 | guân iūnn bîng-thé | LOW | 源=>guân:1char; 樣=>iūnn:1char |
| `fontGenYoGothic` | 源樣烏體 | guân iūnn oo thé | LOW | 源=>guân:1char; 樣=>iūnn:1char; 烏=>oo:1char; 體=>thé:1char |

## `dictionary` (73 keys)

| key | 漢字 (hanji) | drafted Tâi-lô | conf | flags |
|---|---|---|---|---|
| `clear` | 清除 | tshing-tî | MED | 清除=>tshing-tî:AMB |
| `save` | 儉起來 | khiām khí-lâi | LOW | 儉=>khiām:1char |
| `customDictionary` | 自訂詞庫 | tsū tīng sû khòo | LOW | 自=>tsū:1char; 訂=>tīng:1char; 詞=>sû:1char; 庫=>khòo:1char |
| `customDictEnabled` | 啟用自訂詞庫 | khé-iōng tsū tīng sû khòo | LOW | 自=>tsū:1char; 訂=>tīng:1char; 詞=>sû:1char; 庫=>khòo:1char |
| `customDictEnabledInfo` | 拍開了後，家己加入 ê 詞會出現佇候選詞內底。關起來了後，自訂詞就袂閣出現。 | phah-khui liáu-āu， ka-kī ka-ji̍p ê sû huē tshut-hiān tī hāu-suán sû lāi-té。 kuainn--khí-lâi liáu-āu， tsū tīng sû tsiū buē koh tshut-hiān。 | MED | 家己=>ka-kī:AMB; 加入=>ka-ji̍p:AMB; 詞=>sû:1char; 會=>huē:1char; 佇=>tī:1char; 詞=>sû:1char; 內底=>lāi-té:AMB; 自=>tsū:1char; 訂=>tīng:1char; 詞=>sû:1char; 就=>tsiū:1char; 袂=>buē:1char; 閣=>koh:1char |
| `dataManagement` | 個人資料 | kò-jîn-tsu-liāu | HIGH |  |
| `variantDictionary` | 異用字 | ī iōng-jī | LOW | 異=>ī:1char |
| `khiin` | 在來字 | tsāi-lâi jī | LOW | 字=>jī:1char |
| `customDictDescription` | 自訂詞庫使用 CSV 純文字檔案，第 1 欄囥欲拍 ê 羅馬字，第 2 欄囥漢字，毋免囥標題。 | tsū tīng sû khòo sú-iōng CSV sûn bûn-jī-tòng àn， tē 1 nuâ khǹg beh phah ê lô-má-jī， tē 2 nuâ khǹg hàn-jī， m̄-bián khǹg piau-tuê。 | LOW | 自=>tsū:1char; 訂=>tīng:1char; 詞=>sû:1char; 庫=>khòo:1char; 純=>sûn:1char; 文字檔=>bûn-jī-tòng:AMB; 案=>àn:1char; 第=>tē:1char; 欄=>nuâ:1char; 囥=>khǹg:1char; 欲=>beh:1char; 拍=>phah:1char; 羅馬字=>lô-má-jī:AMB; 第=>tē:1char; 欄=>nuâ:1char; 囥=>khǹg:1char; 漢字=>hàn-jī:AMB; 囥=>khǹg:1char; 標題=>piau-tuê:AMB |
| `customDictEmpty` | 揤 + 符號加入自訂詞 | tshi̍h + hû-hō ka-ji̍p tsū tīng sû | LOW | 揤=>tshi̍h:1char; 加入=>ka-ji̍p:AMB; 自=>tsū:1char; 訂=>tīng:1char; 詞=>sû:1char |
| `addEntry` | 增加詞 | tsing-ka sû | LOW | 詞=>sû:1char |
| `editEntry` | 編輯詞 | pian-tsi̍p sû | LOW | 詞=>sû:1char |
| `romanLabel` | 拍字 | phah-jī | MED | 拍字=>phah-jī:AMB |
| `romanPlaceholder` | 見本：gâu-tsá | kiàn-pún：gâu-tsá | HIGH |  |
| `hanziLabel` | 對應 | tuì-ìng | HIGH |  |
| `hanziPlaceholder` | 見本：𠢕早 | kiàn-pún： gâu-tsá | HIGH |  |
| `deleteAll` | 刪除全部自訂詞 | san-tî tsuân-pōo tsū tīng sû | MED | 刪除=>san-tî:AMB; 自=>tsū:1char; 訂=>tīng:1char; 詞=>sû:1char |
| `deleteAllMessage` | 敢確定欲刪除所有自訂詞？ | kám khak-tīng beh san-tî sóo-iú tsū tīng sû？ | LOW | 敢=>kám:1char; 欲=>beh:1char; 刪除=>san-tî:AMB; 所有=>sóo-iú:AMB; 自=>tsū:1char; 訂=>tīng:1char; 詞=>sû:1char |
| `customDictPrivacyWarning` | 請毋通佇自訂詞庫囥敏感 ê 個人資料，親像身分證字號、口座密碼、信用卡號碼，請注意家己 ê 資訊安全。 | tshiánn m̄-thang tī tsū tīng sû khòo khǹg bín-kám ê kò-jîn-tsu-liāu， tshin-tshiūnn sin-hūn-tsìng jī-hō、 kháu-tsō bi̍t-bé、 sìn-iōng-khah hō-bé， tshiánn tsù-ì ka-kī ê tsu-sìn an-tsuân。 | LOW | 請=>tshiánn:1char; 佇=>tī:1char; 自=>tsū:1char; 訂=>tīng:1char; 詞=>sû:1char; 庫=>khòo:1char; 囥=>khǹg:1char; 親像=>tshin-tshiūnn:AMB; 字號=>jī-hō:AMB; 請=>tshiánn:1char; 家己=>ka-kī:AMB |
| `importCSV` | 匯入詞庫 | huē-ji̍p sû khòo | MED | 匯入=>huē-ji̍p:AMB; 詞=>sû:1char; 庫=>khòo:1char |
| `exportCSV` | 匯出詞庫 | huē-tshut sû khòo | LOW | 詞=>sû:1char; 庫=>khòo:1char |
| `exportSuccess` | CSV 順利匯出 | CSV sūn-lī huē-tshut | HIGH |  |
| `importResult` | 匯入 {imported} 項成功，{skipped} 項重複 | huē-ji̍p {imported} hāng sîng-kong，{skipped} hāng-tāng ho̍k | MED | 匯入=>huē-ji̍p:AMB; 項=>hāng:1char; 複=>ho̍k:1char |
| `invalidCSVFormat` | 檔案格式無正確，請使用 CSV 格式 | tòng-àn keh-sik bô tsìng-khak， tshiánn sú-iōng CSV keh-sik | MED | 檔案=>tòng-àn:AMB; 無=>bô:1char; 請=>tshiánn:1char |
| `fileTooLarge` | 檔案傷大（上限 5 MB） | tòng-àn siong tuā（siōng hān 5 MB） | MED | 檔案=>tòng-àn:AMB; 傷=>siong:1char; 大=>tuā:1char; 上=>siōng:1char; 限=>hān:1char |
| `tooManyEntries` | 詞傷濟（上限 30,000 項） | sû siunn-tse（siōng hān 30,000 hāng） | LOW | 詞=>sû:1char; 傷濟=>siunn-tse:AMB; 上=>siōng:1char; 限=>hān:1char; 項=>hāng:1char |
| `importExportTitle` | 匯出匯入 | huē-tshut huē-ji̍p | MED | 匯入=>huē-ji̍p:AMB |
| `frequencyExportCSV` | 匯出詞頻紀錄 | huē-tshut sû pîn kì-lio̍k | LOW | 詞=>sû:1char; 頻=>pîn:1char; 紀錄=>kì-lio̍k:AMB |
| `frequencyImportCSV` | 匯入詞頻紀錄 | huē-ji̍p sû pîn kì-lio̍k | MED | 匯入=>huē-ji̍p:AMB; 詞=>sû:1char; 頻=>pîn:1char; 紀錄=>kì-lio̍k:AMB |
| `frequencyDescription` | 詞頻紀錄使用 CSV 純文字檔案，第 1 欄囥詞，第 2 欄囥次數，毋免囥標題。 | sû pîn kì-lio̍k sú-iōng CSV sûn bûn-jī-tòng àn， tē 1 nuâ khǹg sû， tē 2 nuâ khǹg tshù siàu， m̄-bián khǹg piau-tuê。 | LOW | 詞=>sû:1char; 頻=>pîn:1char; 紀錄=>kì-lio̍k:AMB; 純=>sûn:1char; 文字檔=>bûn-jī-tòng:AMB; 案=>àn:1char; 第=>tē:1char; 欄=>nuâ:1char; 囥=>khǹg:1char; 詞=>sû:1char; 第=>tē:1char; 欄=>nuâ:1char; 囥=>khǹg:1char; 次=>tshù:1char; 數=>siàu:1char; 囥=>khǹg:1char; 標題=>piau-tuê:AMB |
| `associationExportCSV` | 匯出詞關聯紀錄 | huē-tshut sû kuan-liân kì-lio̍k | LOW | 詞=>sû:1char; 紀錄=>kì-lio̍k:AMB |
| `associationImportCSV` | 匯入詞關聯紀錄 | huē-ji̍p sû kuan-liân kì-lio̍k | MED | 匯入=>huē-ji̍p:AMB; 詞=>sû:1char; 紀錄=>kì-lio̍k:AMB |
| `associationDescription` | 詞關聯紀錄使用 CSV 純文字檔案，共 5 欄：頭前詞、頭前拍字、後壁詞、後壁拍字、次數，毋免囥標題。 | sû kuan-liân kì-lio̍k sú-iōng CSV sûn bûn-jī-tòng àn， kā 5 nuâ： thâu-tsîng sû、 thâu-tsîng phah-jī、 āu-piah sû、 āu-piah phah-jī、 tshù siàu， m̄-bián khǹg piau-tuê。 | LOW | 詞=>sû:1char; 紀錄=>kì-lio̍k:AMB; 純=>sûn:1char; 文字檔=>bûn-jī-tòng:AMB; 案=>àn:1char; 共=>kā:1char; 欄=>nuâ:1char; 頭前=>thâu-tsîng:AMB; 詞=>sû:1char; 頭前=>thâu-tsîng:AMB; 拍字=>phah-jī:AMB; 詞=>sû:1char; 拍字=>phah-jī:AMB; 次=>tshù:1char; 數=>siàu:1char; 囥=>khǹg:1char; 標題=>piau-tuê:AMB |
| `moeSectionTitle` | 教育部用字 | kàu-io̍k-pōo iōng-jī | HIGH |  |
| `otherSectionTitle` | 其他辭典 | kî-thann sû-tián | HIGH |  |
| `supplementSectionTitle` | 補充資料 | póo-tshiong tsu-liāu | HIGH |  |
| `lkkDict` | 漢羅合用建議用字 | hàn-lô ha̍h-īng kiàn-gī iōng-jī | HIGH |  |
| `devSupplementDict` | 詞庫增補檔案 | sû khòo tsing-póo tòng-àn | LOW | 詞=>sû:1char; 庫=>khòo:1char; 檔案=>tòng-àn:AMB |
| `kautianAccentLukang` | 鹿港偏泉腔 | lo̍k-káng phian tsuân-khiunn | LOW | 偏=>phian:1char |
| `kautianAccentSansia` | 三峽偏泉腔 | sam-kiap phian tsuân-khiunn | LOW | 偏=>phian:1char |
| `kautianAccentTaipak` | 臺北偏泉腔 | tâi-pak phian tsuân-khiunn | LOW | 偏=>phian:1char |
| `kautianAccentGilan` | 宜蘭偏漳腔 | gî-lân phian tsiang-khiunn | LOW | 偏=>phian:1char; 漳腔=>tsiang-khiunn:AMB |
| `kautianAccentTainan` | 臺南混合腔 | tâi-lâm hūn-ha̍p khiunn | LOW | 腔=>khiunn:1char |
| `kautianAccentKaohsiung` | 高雄混合腔 | ko-hiông hūn-ha̍p khiunn | LOW | 腔=>khiunn:1char |
| `kautianAccentKinmen` | 金門偏泉腔 | kim-mn̂g phian tsuân-khiunn | LOW | 偏=>phian:1char |
| `kautianAccentMakung` | 馬公偏泉腔 | bé-kang phian tsuân-khiunn | MED | 馬公=>bé-kang:AMB; 偏=>phian:1char |
| `kautianAccentSintik` | 新竹偏泉腔 | sin-tik phian tsuân-khiunn | LOW | 偏=>phian:1char |
| `kautianAccentTaichung` | 臺中偏漳腔 | tâi-tiong phian tsiang-khiunn | LOW | 偏=>phian:1char; 漳腔=>tsiang-khiunn:AMB |
| `kautianNameAppendix` | 姓名附錄 | sènn-miâ hù-lio̍k | MED | 姓名=>sènn-miâ:AMB; 附錄=>hù-lio̍k:AMB |
| `searchPlaceholder` | 拍字揣詞 | phah-jī tshuē sû | MED | 拍字=>phah-jī:AMB; 揣=>tshuē:1char; 詞=>sû:1char |
| `noResults` | 揣無結果 | tshuē-bô kiat-kó | HIGH |  |
| `lookupChhoe` | ChhoeTaigi 辭典 | ChhoeTaigi sû-tián | HIGH |  |
| `lookupMoe` | 教育部辭典 | kàu-io̍k-pōo sû-tián | HIGH |  |
| `frequencyManagement` | 詞頻紀錄 | sû pîn kì-lio̍k | LOW | 詞=>sû:1char; 頻=>pîn:1char; 紀錄=>kì-lio̍k:AMB |
| `frequencyRecordingEnabled` | 開啟詞頻紀錄 | khai-khé sû pîn kì-lio̍k | LOW | 詞=>sû:1char; 頻=>pîn:1char; 紀錄=>kì-lio̍k:AMB |
| `frequencyRecordingEnabledInfo` | 拍開了後，齒盤會記錄你揀過 ê 詞幾擺，予候選詞排序做參考，定定揀 ê 詞就會排較頭前，按呢候選詞就會愈來愈準。 | phah-khui liáu-āu， khí-puânn huē kì-lio̍k lí kíng kuè ê sû kuí-pái， hōo hāu-suán sû pâi-sī tsuè tsham-khó， tiānn-tiānn kíng ê sû tsiū huē pâi khah thâu-tsîng， án-ne hāu-suán sû tsiū huē jú-lâi-jú tsún。 | LOW | 會=>huē:1char; 記錄=>kì-lio̍k:AMB; 你=>lí:1char; 揀=>kíng:1char; 過=>kuè:1char; 詞=>sû:1char; 予=>hōo:1char; 詞=>sû:1char; 排序=>pâi-sī:AMB; 做=>tsuè:1char; 揀=>kíng:1char; 詞=>sû:1char; 就=>tsiū:1char; 會=>huē:1char; 排=>pâi:1char; 較=>khah:1char; 頭前=>thâu-tsîng:AMB; 按呢=>án-ne:AMB; 詞=>sû:1char; 就=>tsiū:1char; 會=>huē:1char; 準=>tsún:1char |
| `associationManagement` | 詞關聯紀錄 | sû kuan-liân kì-lio̍k | LOW | 詞=>sû:1char; 紀錄=>kì-lio̍k:AMB |
| `associationRecordingEnabled` | 開啟詞關聯紀錄 | khai-khé sû kuan-liân kì-lio̍k | LOW | 詞=>sû:1char; 紀錄=>kì-lio̍k:AMB |
| `associationRecordingEnabledInfo` | 拍開了後，齒盤會記錄頭前、後壁 ê 關聯詞，予連紲建議愈來愈準。 | phah-khui liáu-āu， khí-puânn huē kì-lio̍k thâu-tsîng、 āu-piah ê kuan-liân sû， hōo liân-suà kiàn-gī jú-lâi-jú tsún。 | LOW | 會=>huē:1char; 記錄=>kì-lio̍k:AMB; 頭前=>thâu-tsîng:AMB; 詞=>sû:1char; 予=>hōo:1char; 準=>tsún:1char |
| `frequencyPrivacyWarning` | 詞頻紀錄對台語研究來講是真有價值 ê 資料。若欲提供予人研究訓練模型，請先刪除敏感 ê 私人資料。紀錄功能嘛會使關起來，毋過按呢候選詞 ê 排序就會較無準。 | sû pîn kì-lio̍k tuì tâi-gí gián-kiù lâi kóng sī tsin ū-kè ta̍t ê tsu-liāu。 nā-beh thê-kiong hōo-lâng gián-kiù hùn-liān bôo-hîng， tshiánn sian san-tî bín-kám ê su-jîn tsu-liāu。 kì-lio̍k kong-lîng mā ē-sái kuainn--khí-lâi， m̄-koh án-ne hāu-suán sû ê pâi-sī tsiū huē khah bô tsún。 | LOW | 詞=>sû:1char; 頻=>pîn:1char; 紀錄=>kì-lio̍k:AMB; 對=>tuì:1char; 台語=>tâi-gí:AMB; 來=>lâi:1char; 講=>kóng:1char; 是=>sī:1char; 真=>tsin:1char; 值=>ta̍t:1char; 請=>tshiánn:1char; 先=>sian:1char; 刪除=>san-tî:AMB; 私人=>su-jîn:AMB; 紀錄=>kì-lio̍k:AMB; 嘛=>mā:1char; 會使=>ē-sái:AMB; 毋過=>m̄-koh:AMB; 按呢=>án-ne:AMB; 詞=>sû:1char; 排序=>pâi-sī:AMB; 就=>tsiū:1char; 會=>huē:1char; 較=>khah:1char; 無=>bô:1char; 準=>tsún:1char |
| `clearAllFrequency` | 刪除所有詞頻紀錄 | san-tî sóo-iú sû pîn kì-lio̍k | MED | 刪除=>san-tî:AMB; 所有=>sóo-iú:AMB; 詞=>sû:1char; 頻=>pîn:1char; 紀錄=>kì-lio̍k:AMB |
| `associationPrivacyWarning` | 詞關聯紀錄對台語研究來講是真有價值 ê 資料。若欲提供予人做研究，請先刪除敏感 ê 內容。紀錄功能嘛會使關起來，毋過後一詞預測會較無準。 | sû kuan-liân kì-lio̍k tuì tâi-gí gián-kiù lâi kóng sī tsin ū-kè ta̍t ê tsu-liāu。 nā-beh thê-kiong hōo-lâng tsuè gián-kiù， tshiánn sian san-tî bín-kám ê luē-iông。 kì-lio̍k kong-lîng mā ē-sái kuainn--khí-lâi， m̄-koh āu tsi̍t sû ī-tshik huē khah bô tsún。 | LOW | 詞=>sû:1char; 紀錄=>kì-lio̍k:AMB; 對=>tuì:1char; 台語=>tâi-gí:AMB; 來=>lâi:1char; 講=>kóng:1char; 是=>sī:1char; 真=>tsin:1char; 值=>ta̍t:1char; 做=>tsuè:1char; 請=>tshiánn:1char; 先=>sian:1char; 刪除=>san-tî:AMB; 內容=>luē-iông:AMB; 紀錄=>kì-lio̍k:AMB; 嘛=>mā:1char; 會使=>ē-sái:AMB; 毋過=>m̄-koh:AMB; 後=>āu:1char; 一=>tsi̍t:1char; 詞=>sû:1char; 預測=>ī-tshik:AMB; 會=>huē:1char; 較=>khah:1char; 無=>bô:1char; 準=>tsún:1char |
| `clearAllAssociation` | 刪除所有詞關聯紀錄 | san-tî sóo-iú sû kuan-liân kì-lio̍k | MED | 刪除=>san-tî:AMB; 所有=>sóo-iú:AMB; 詞=>sû:1char; 紀錄=>kì-lio̍k:AMB |
| `clearFrequencyMessage` | 確定欲刪除所有詞頻紀錄？ | khak-tīng beh san-tî sóo-iú sû pîn kì-lio̍k？ | LOW | 欲=>beh:1char; 刪除=>san-tî:AMB; 所有=>sóo-iú:AMB; 詞=>sû:1char; 頻=>pîn:1char; 紀錄=>kì-lio̍k:AMB |
| `clearAssociationMessage` | 確定欲刪除所有詞關聯紀錄？ | khak-tīng beh san-tî sóo-iú sû kuan-liân kì-lio̍k？ | LOW | 欲=>beh:1char; 刪除=>san-tî:AMB; 所有=>sóo-iú:AMB; 詞=>sû:1char; 紀錄=>kì-lio̍k:AMB |
| `noData` | 無資料 | bô tsu-liāu | LOW | 無=>bô:1char |
| `filterHint` | 頂面上濟顯示 100 个詞，若揣無詞，請用下跤 ê「拍字揣詞」功能搜揣。 | tíng-bīn siāng tsuē hián-sī 100 ê sû， nā tshuē-bô sû， tshiánn iōng ē-kha ê「phah-jī tshuē sû」 kong-lîng tshiau-tshuē。 | MED | 上濟=>siāng tsuē:AMB; 个=>ê:1char; 詞=>sû:1char; 若=>nā:1char; 詞=>sû:1char; 請=>tshiánn:1char; 用=>iōng:1char; 拍字=>phah-jī:AMB; 揣=>tshuē:1char; 詞=>sû:1char; 搜揣=>tshiau-tshuē:AMB |
| `backupRestore` | 備份復原 | pī-hūn ho̍k-guân | HIGH |  |
| `exportBackup` | 一擺全出 | tsi̍t-pái tsuân tshut | LOW | 全=>tsuân:1char; 出=>tshut:1char |
| `importBackup` | 一擺全入 | tsi̍t-pái tsuân ji̍p | LOW | 全=>tsuân:1char; 入=>ji̍p:1char |
| `exportBackupSuccess` | 備份順利匯出 | pī-hūn sūn-lī huē-tshut | HIGH |  |
| `importBackupResult` | 匯入 {customDict} 項自訂詞、{frequency} 項詞頻、{association} 項詞關聯 | huē-ji̍p {customDict} hāng tsū tīng sû、{frequency} hāng sû pîn、{association} hāng sû kuan-liân | MED | 匯入=>huē-ji̍p:AMB; 項=>hāng:1char; 自=>tsū:1char; 訂=>tīng:1char; 詞=>sû:1char; 項=>hāng:1char; 詞=>sû:1char; 頻=>pîn:1char; 項=>hāng:1char; 詞=>sû:1char |
| `backupPrivacyWarning` | 備份檔案內底包含你所有 ê 使用紀錄，佇 iOS 佮 Android 攏通用，準講換機仔嘛毋免煩惱。 | pī-hūn tòng-àn lāi-té pau-hâm lí sóo-iú ê sú-iōng kì-lio̍k， tī iOS kah Android láng thong-iōng， tsún-kóng uānn ki-á mā m̄-bián huân-ló。 | MED | 檔案=>tòng-àn:AMB; 內底=>lāi-té:AMB; 你=>lí:1char; 所有=>sóo-iú:AMB; 紀錄=>kì-lio̍k:AMB; 佇=>tī:1char; 佮=>kah:1char; 攏=>láng:1char; 換=>uānn:1char; 嘛=>mā:1char |

## `home` (34 keys)

| key | 漢字 (hanji) | drafted Tâi-lô | conf | flags |
|---|---|---|---|---|
| `appHeaderTitle` | 台語齒盤 | tâi-gí khí-puânn | MED | 台語=>tâi-gí:AMB |
| `setupKeyboard` | 齒盤愛拍開才會當使用 | khí-puânn ài phah-khui tsiah ē tng sú-iōng | LOW | 愛=>ài:1char; 當=>tng:1char |
| `typingGuide` | 拍字說明 | phah-jī suat-bîng | MED | 拍字=>phah-jī:AMB; 說明=>suat-bîng:AMB |
| `newFeatures` | 功能設定 | kong-lîng siat-tīng | HIGH |  |
| `faq` | 其他 | kî-thann | HIGH |  |
| `setupGuide` | 啟用方法 | khé-iōng hong-huat | HIGH |  |
| `setupGuideDescription` | 手機仔系統規定第三方齒盤愛手動啟用才會當使用，請照下跤 ê 說明完成設定。 | tshiú-ki-á hē-thóng kui-tīng tē-sann hong khí-puânn ài tshiú-tōng khé-iōng tsiah ē tng sú-iōng， tshiánn tsiàu ē-kha ê suat-bîng uân-sîng siat-tīng。 | LOW | 方=>hong:1char; 愛=>ài:1char; 當=>tng:1char; 請=>tshiánn:1char; 照=>tsiàu:1char; 說明=>suat-bîng:AMB |
| `setupInfoMessage` | 「允准完整取用」意思是予齒盤會當捌你揤 ê 動作，成做你拍 ê 字。請放心，App 袂紀錄你 ê 資料。 | 「ún-tsún uân-tsíng tshú-iōng」 ì-sù sī hōo khí-puânn ē-tàng bat lí tshi̍h ê tōng-tsok， tsiânn-tsuè lí phah ê jī。 tshiánn hòng-sim，App buē kì-lio̍k lí ê tsu-liāu。 | MED | 允准=>ún-tsún:AMB; 是=>sī:1char; 予=>hōo:1char; 會當=>ē-tàng:AMB; 捌=>bat:1char; 你=>lí:1char; 揤=>tshi̍h:1char; 成做=>tsiânn-tsuè:AMB; 你=>lí:1char; 拍=>phah:1char; 字=>jī:1char; 請=>tshiánn:1char; 袂=>buē:1char; 紀錄=>kì-lio̍k:AMB; 你=>lí:1char |
| `setupBrandWarning` | 無仝牌子 ê 手機仔，設定 ê 方式可能會淡薄仔無仝款，毋過方式應該攏差不多。 | bô kâng pâi-tsú ê tshiú-ki-á， siat-tīng ê hong-sik khó-lîng huē tām-po̍h-á bô-kāng-khuán， m̄-koh hong-sik ìng-kai láng tsha-put-to。 | MED | 無仝=>bô kâng:AMB; 可能=>khó-lîng:AMB; 會=>huē:1char; 毋過=>m̄-koh:AMB; 應該=>ìng-kai:AMB; 攏=>láng:1char |
| `setupGuideCompletedMessage` | 完成了後，重開你目前使用 ê App，予 App 重掠新 ê 齒盤清單。紲落來，佇會當拍字 ê 所在，揤牢地球圖示切去台語齒盤 | uân-sîng liáu-āu， tîng-khui lí ba̍k-tsîng sú-iōng ê App， hōo App tāng lia̍h sin ê khí-puânn tshing-tuann。 suà--lo̍h-lâi， tī ē-tàng phah-jī ê sóo-tsāi， tshi̍h tiâu tuē-kiû-tôo sī tshiat khì tâi-gí khí-puânn | LOW | 你=>lí:1char; 目前=>ba̍k-tsîng:AMB; 予=>hōo:1char; 重=>tāng:1char; 掠=>lia̍h:1char; 新=>sin:1char; 佇=>tī:1char; 會當=>ē-tàng:AMB; 拍字=>phah-jī:AMB; 揤=>tshi̍h:1char; 牢=>tiâu:1char; 示=>sī:1char; 切=>tshiat:1char; 去=>khì:1char; 台語=>tâi-gí:AMB |
| `setupGuideGoToSettings` | 去設定頁 | khì siat-tīng ia̍h | LOW | 去=>khì:1char; 頁=>ia̍h:1char |
| `setupGuideCloseButton` | 關閉 | kuan pì | LOW | 關=>kuan:1char; 閉=>pì:1char |
| `setupGuideStep1Settings` | 點揤「齒盤」 | tiám-tshi̍h「khí-puânn」 | HIGH |  |
| `setupGuideStep2AddKeyboard` | 點揤「增加齒盤」、「允准完整取用」 | tiám-tshi̍h「tsing-ka khí-puânn」、「ún-tsún uân-tsíng tshú-iōng」 | MED | 允准=>ún-tsún:AMB |
| `userGuide` | 網站紹介 | bāng-tsām siāu-kài | HIGH |  |
| `rateUs` | 為阮評分 | uî guán phîng-hun | LOW | 為=>uî:1char; 阮=>guán:1char |
| `aboutDeveloper` | 關於 | kuan-î | HIGH |  |
| `privacyPolicy` | 隱私權政策 | ín-su-khuân tsìng-tshik | MED | 隱私權=>ín-su-khuân:AMB |
| `freePromise` | 台語齒盤保證永遠免費，嘛袂做付費功能。台語是咱 ê 母語，無應該因為錢 ê 問題用袂著好家私。我向望逐家想欲學台語、寫台語 ê 人攏會當無負擔來使用，這是我做這个齒盤上重要 ê 心願。 | tâi-gí khí-puânn pó-tsìng íng-uán bián-huì， mā buē tsuè hù-huì kong-lîng。 tâi-gí sī lán ê bó-gí， bô ìng-kai in-uī tsînn ê būn-tê iōng buē tio̍h hó ke-si。 guá ǹg-bāng ta̍k-ke siūnn-beh ha̍k tâi-gí、 siá tâi-gí ê jîn láng ē-tàng bô hū-tam lâi sú-iōng， tse-sī guá tsuè tsit ê khí-puânn siōng tiōng-iàu ê sim-guān。 | MED | 台語=>tâi-gí:AMB; 嘛=>mā:1char; 袂=>buē:1char; 做=>tsuè:1char; 台語=>tâi-gí:AMB; 是=>sī:1char; 咱=>lán:1char; 母語=>bó-gí:AMB; 無=>bô:1char; 應該=>ìng-kai:AMB; 錢=>tsînn:1char; 問題=>būn-tê:AMB; 用=>iōng:1char; 袂=>buē:1char; 著=>tio̍h:1char; 好=>hó:1char; 我=>guá:1char; 想欲=>siūnn-beh:AMB; 學=>ha̍k:1char; 台語=>tâi-gí:AMB; 寫=>siá:1char; 台語=>tâi-gí:AMB; 人=>jîn:1char; 攏=>láng:1char; 會當=>ē-tàng:AMB; 無=>bô:1char; 來=>lâi:1char; 我=>guá:1char; 做=>tsuè:1char; 上=>siōng:1char |
| `version` | 當前版本 | tong-tsiân pán-pún | HIGH |  |
| `versionHistory` | 版本紀錄 | pán-pún kì-lio̍k | MED | 紀錄=>kì-lio̍k:AMB |
| `copyrightNotice` | 致謝 | tì-siā | HIGH |  |
| `viewLicense` | 授權條款 | siū-khuân tiâu-khuán | MED | 授權=>siū-khuân:AMB; 條款=>tiâu-khuán:AMB |
| `moeCopyright` | © 教育部 | © kàu-io̍k-pōo | HIGH |  |
| `iTaigiCopyright` | © iTaigi愛台語 | © iTaigi ài-tâi-gí | HIGH |  |
| `newwordCopyright` | © 公視台語台 | © kong-sī tâi-gí-tâi | MED | 台語台=>tâi-gí-tâi:AMB |
| `openFontCopyright` | © justfont | © justfont | HIGH |  |
| `butTaiwanCopyright` | © ButTaiwan | © ButTaiwan | HIGH |  |
| `taiwanPlantCopyright` | © 佐佐木舜一 | © tsò-tsò-bo̍k sùn tsi̍t | MED | 佐佐木=>tsò-tsò-bo̍k:AMB; 舜=>sùn:1char; 一=>tsi̍t:1char |
| `taiHuaCopyright` | © 鄭良偉 | © tēnn liông uí | LOW | 鄭=>tēnn:1char; 良=>liông:1char; 偉=>uí:1char |
| `taiwanJapanCopyright` | © 小川尚義 | © siáu-tshuan siōng gī | LOW | 尚=>siōng:1char; 義=>gī:1char |
| `kunggeCopyright` | © 國立臺灣工藝研究發展中心 | © kok-li̍p tâi-uân kang-gē gián-kiù huat-tián tiong-sim | HIGH |  |
| `accentDictCredit` | 實齋整理、提供 | si̍t tse tsíng-lí、 thê-kiong | LOW | 實=>si̍t:1char; 齋=>tse:1char |
| `devSupplementCredit` | 建中整理、提供 | kiàn-tiong tsíng-lí、 thê-kiong | HIGH |  |

## `layout` (8 keys)

| key | 漢字 (hanji) | drafted Tâi-lô | conf | flags |
|---|---|---|---|---|
| `romanizationKeyboard` | 羅馬字齒盤 | lô-má-jī khí-puânn | MED | 羅馬字=>lô-má-jī:AMB |
| `taigiPhonetic` | 方音符號 | hong-im-hû-hō | HIGH |  |
| `standardLayout` | Lohankha | Lohankha | HIGH |  |
| `phahTaigiLayout` | PhahTaigi | PhahTaigi | HIGH |  |
| `tpsLayout` | 方音符號齒佈 | hong-im-hû-hō khí pòo | LOW | 齒=>khí:1char; 佈=>pòo:1char |
| `moe1Layout` | 教育部輸入法齒佈1 | kàu-io̍k-pōo su-ji̍p-hoat khí pòo1 | LOW | 齒=>khí:1char; 佈=>pòo:1char |
| `moe2Layout` | 教育部輸入法齒佈2 | kàu-io̍k-pōo su-ji̍p-hoat khí pòo2 | LOW | 齒=>khí:1char; 佈=>pòo:1char |
| `comingSoon` | 連鞭上市 | liâm-mi tsiūnn-tshī | MED | 連鞭=>liâm-mi:AMB |

## `settings` (39 keys)

| key | 漢字 (hanji) | drafted Tâi-lô | conf | flags |
|---|---|---|---|---|
| `reset` | 恢復 | hue-ho̍k | MED | 恢復=>hue-ho̍k:AMB |
| `inputMode` | 輸入模式 | su-ji̍p bôo-sik | MED | 輸入=>su-ji̍p:AMB |
| `displayLanguage` | 顯示語言 | hián-sī gí-giân | MED | 語言=>gí-giân:AMB |
| `displayLanguageAutomatic` | 自動 | tsū-tōng | HIGH |  |
| `pojMode` | 白話字 | pe̍h-uē-jī | MED | 白話字=>pe̍h-uē-jī:AMB |
| `tlMode` | 台羅 | tâi-lô | HIGH |  |
| `englishMode` | 英文 | ing-bûn | HIGH |  |
| `tpsMode` | 方音符號 | hong-im-hû-hō | HIGH |  |
| `typingSectionTitle` | 拍字設定 | phah-jī siat-tīng | MED | 拍字=>phah-jī:AMB |
| `outputBothScripts` | 括號標註 | kuat-hō phiau-tsù | HIGH |  |
| `literalRomanCandidate` | 顯示羅馬字 | hián-sī lô-má-jī | MED | 羅馬字=>lô-má-jī:AMB |
| `literalRomanCandidateInfo` | 候選詞列第一个位囥羅馬字，會當用手點抑是揤 Enter 送出，若關，干焦會當揤 Enter 送出，袂當用手點，但是候選詞列空間較大。 | hāu-suán sû lia̍t tē-it ê uī khǹg lô-má-jī， ē-tàng iōng tshiú tiám ah-sī tshi̍h Enter sàng-tshut， nā kuan， kan-tann-ē-tàng tshi̍h Enter sàng-tshut， buē-tàng iōng tshiú tiám， tān-sī hāu-suán sû lia̍t khang-king khah-tōa。 | LOW | 詞=>sû:1char; 列=>lia̍t:1char; 第一=>tē-it:AMB; 个=>ê:1char; 位=>uī:1char; 囥=>khǹg:1char; 羅馬字=>lô-má-jī:AMB; 會當=>ē-tàng:AMB; 用=>iōng:1char; 手=>tshiú:1char; 點=>tiám:1char; 抑是=>ah-sī:AMB; 揤=>tshi̍h:1char; 若=>nā:1char; 關=>kuan:1char; 揤=>tshi̍h:1char; 袂當=>buē-tàng:AMB; 用=>iōng:1char; 手=>tshiú:1char; 點=>tiám:1char; 詞=>sû:1char; 列=>lia̍t:1char; 空間=>khang-king:AMB |
| `autoCapitalization` | 自動大本字 | tsū-tōng tuā-pún-jī | MED | 大本字=>tuā-pún-jī:AMB |
| `autoSpace` | 自動空白 | tsū-tōng khàng-pe̍h | HIGH |  |
| `keyboardSectionTitle` | 齒盤設定 | khí-puânn siat-tīng | HIGH |  |
| `toolbarAutoCollapse` | 自動隱藏工具列 | tsū-tōng ún-tsông kang-khū-lia̍t | MED | 工具列=>kang-khū-lia̍t:AMB |
| `toolbarAutoCollapseInfo` | 選字了後工具列會自動合起來，予齒盤面頂空間較大。 | suán jī liáu-āu kang-khū-lia̍t huē tsū-tōng ha̍p--khí-lâi， hōo khí-puânn bīn-tíng khang-king khah-tōa。 | LOW | 選=>suán:1char; 字=>jī:1char; 工具列=>kang-khū-lia̍t:AMB; 會=>huē:1char; 予=>hōo:1char; 空間=>khang-king:AMB |
| `globeKey` | 齒盤切換揤鈕 | khí-puânn tshiat-uānn tshi̍h-liú | HIGH |  |
| `globeKeyInfo` | 佇齒盤面頂加 1 粒地球揤鈕，揤著會使切換去其他齒盤。 | tī khí-puânn bīn-tíng ka 1 lia̍p tuē-kiû tshi̍h-liú， tshi̍h tio̍h ē-sái tshiat-uānn khì kî-thann khí-puânn。 | LOW | 佇=>tī:1char; 加=>ka:1char; 粒=>lia̍p:1char; 地球=>tuē-kiû:AMB; 揤=>tshi̍h:1char; 著=>tio̍h:1char; 會使=>ē-sái:AMB; 去=>khì:1char |
| `feedbackSectionTitle` | 拍字反應 | phah-jī huán-ìng | MED | 拍字=>phah-jī:AMB |
| `soundFeedback` | 揤仔聲 | tshi̍h á siann | LOW | 揤=>tshi̍h:1char; 仔=>á:1char; 聲=>siann:1char |
| `vibrationFeedback` | 震動反應 | tín-tāng huán-ìng | MED | 震動=>tín-tāng:AMB |
| `pojSettingsSectionTitle` | 白話字 | pe̍h-uē-jī | MED | 白話字=>pe̍h-uē-jī:AMB |
| `doubleTapOO` | 連紲拍 oo → o͘ | liân-suà phah oo → o͘ | LOW | 拍=>phah:1char |
| `doubleTapNN` | 連紲拍 nn → ⁿ | liân-suà phah nn → ⁿ | LOW | 拍=>phah:1char |
| `tpsSettingsSectionTitle` | 方音符號 | hong-im-hû-hō | HIGH |  |
| `tpsOrMapsToER` | or 對應 ㄜ | or tuì-ìng ㄜ | HIGH |  |
| `tpsOrMapsToERInfo` | 台羅 or 毋是正式寫法，方音符號 ㄜ 正式干焦對應 er。本設定只控制候選詞按怎顯示;字典揣詞已經共 er 佮 or 攏對應做仝一个音位，無論本設定開抑無開攏揣會著。

開啟（預設）：or 顯示做 ㄜ。
關閉：or 顯示做 ㄛ（恢復台羅 o）。 | tâi-lô or m̄-sī tsìng-sik siá-huat， hong-im-hû-hō ㄜ tsìng-sik kan-na tuì-ìng er。 pún siat-tīng tsí khòng-tsè hāu-suán sû án-nuá hián-sī; jī-tián tshuē sû í-king kā er kah or láng tuì-ìng tsuè kāng tsi̍t-ê im-uī， bô-lūn pún siat-tīng khui ah-bô khui láng tshuē huē tio̍h。 khai-khé（ī-siat）：or hián-sī tsuè ㄜ。 kuan pì：or hián-sī tsuè ㄛ（hue-ho̍k tâi-lô o）。 | MED | 毋是=>m̄-sī:AMB; 干焦=>kan-na:AMB; 本=>pún:1char; 只=>tsí:1char; 詞=>sû:1char; 按怎=>án-nuá:AMB; 字典=>jī-tián:AMB; 揣=>tshuē:1char; 詞=>sû:1char; 共=>kā:1char; 佮=>kah:1char; 攏=>láng:1char; 做=>tsuè:1char; 仝=>kāng:1char; 一个=>tsi̍t-ê:AMB; 本=>pún:1char; 開=>khui:1char; 抑無=>ah-bô:AMB; 開=>khui:1char; 攏=>láng:1char; 揣=>tshuē:1char; 會=>huē:1char; 著=>tio̍h:1char; 預設=>ī-siat:AMB; 做=>tsuè:1char; 關=>kuan:1char; 閉=>pì:1char; 做=>tsuè:1char; 恢復=>hue-ho̍k:AMB |
| `resetSettings` | 恢復設定 | hue-ho̍k siat-tīng | MED | 恢復=>hue-ho̍k:AMB |
| `resetSettingsMessage` | 這个動作會恢復所有設定，敢欲繼續？ | tsit ê tōng-tsok huē hue-ho̍k sóo-iú siat-tīng， kám beh kè-sio̍k？ | LOW | 會=>huē:1char; 恢復=>hue-ho̍k:AMB; 所有=>sóo-iú:AMB; 敢=>kám:1char; 欲=>beh:1char |
| `resetSuccess` | 設定已恢復 | siat-tīng í hue-ho̍k | LOW | 已=>í:1char; 恢復=>hue-ho̍k:AMB |
| `resetFailed` | 恢復設定失敗，請重試 | hue-ho̍k siat-tīng sit-pāi， tshiánn tāng tshì | MED | 恢復=>hue-ho̍k:AMB; 請=>tshiánn:1char; 重=>tāng:1char; 試=>tshì:1char |
| `noEmailApp` | 揣無 Email App | tshuē-bô Email App | HIGH |  |
| `diagnosticSectionTitle` | 裝置資訊 | tsong-tì tsu-sìn | HIGH |  |
| `diagnosticCopy` | Khó͘-phih 裝置資訊 | Khó͘-phih tsong-tì tsu-sìn | HIGH |  |
| `diagnosticCopied` | 已 khó͘-phih | í khó͘-phih | LOW | 已=>í:1char |
| `diagnosticShare` | 分享裝置資訊 | hun-hióng tsong-tì tsu-sìn | MED | 分享=>hun-hióng:AMB |
| `diagnosticEmail` | Email 回報問題 | Email huê-pò būn-tê | MED | 回報=>huê-pò:AMB; 問題=>būn-tê:AMB |
| `openApp` | 去APP調整 | khìAPP tiâu-tsíng | LOW | 去=>khì:1char |

## `theme` (36 keys)

| key | 漢字 (hanji) | drafted Tâi-lô | conf | flags |
|---|---|---|---|---|
| `customFont` | 字型設定 | jī-hîng siat-tīng | MED | 字型=>jī-hîng:AMB |
| `customThemesSection` | 自訂主題 | tsū tīng tsú-tê | LOW | 自=>tsū:1char; 訂=>tīng:1char; 主題=>tsú-tê:AMB |
| `createNewTheme` | 新主題… | sin tsú-tê… | LOW | 新=>sin:1char; 主題=>tsú-tê:AMB |
| `keyboardSection` | 齒盤介面 | khí-puânn kài-bīn | HIGH |  |
| `colorKeySection` | 揤鈕介面 | tshi̍h-liú kài-bīn | HIGH |  |
| `candidateSection` | 候選詞介面 | hāu-suán sû kài-bīn | LOW | 詞=>sû:1char |
| `colorKeyboardBackground` | 齒盤色水 | khí-puânn sik-tsuí | HIGH |  |
| `colorKeyText` | 揤鈕文字 | tshi̍h-liú bûn-jī | MED | 文字=>bûn-jī:AMB |
| `colorNormalKeyFill` | 一般揤鈕色水 | it-puann tshi̍h-liú sik-tsuí | HIGH |  |
| `colorSpecialKeyFill` | 特殊揤鈕色水 | ti̍k-sû tshi̍h-liú sik-tsuí | HIGH |  |
| `colorCandidateText` | 候選詞文字 | hāu-suán sû bûn-jī | LOW | 詞=>sû:1char; 文字=>bûn-jī:AMB |
| `colorCandidateBackground` | 候選詞背景 | hāu-suán sû puē-kíng | LOW | 詞=>sû:1char |
| `keyHeight` | 齒盤懸度 | khí-puânn kuân-tōo | HIGH |  |
| `keyFontSize` | 揤鈕字大細 | tshi̍h-liú jī tuā-sè | LOW | 字=>jī:1char; 大細=>tuā-sè:AMB |
| `candidateTextSize` | 候選詞大細 | hāu-suán sû tuā-sè | LOW | 詞=>sû:1char; 大細=>tuā-sè:AMB |
| `keyCornerRadius` | 揤鈕圓角 | tshi̍h-liú înn kak | LOW | 圓=>înn:1char; 角=>kak:1char |
| `keyBorderWidth` | 揤鈕邊粗幼 | tshi̍h-liú pinn tshoo-iù | LOW | 邊=>pinn:1char |
| `keyShadow` | 揤鈕陰影 | tshi̍h-liú im-iánn | HIGH |  |
| `editorTitleNew` | 新主題 | sin tsú-tê | LOW | 新=>sin:1char; 主題=>tsú-tê:AMB |
| `editorTitleEdit` | 編輯主題 | pian-tsi̍p tsú-tê | MED | 主題=>tsú-tê:AMB |
| `nameHeader` | 主題名稱 | tsú-tê miâ-tshing | MED | 主題=>tsú-tê:AMB |
| `namePlaceholder` | 輸入主題名稱 | su-ji̍p tsú-tê miâ-tshing | MED | 輸入=>su-ji̍p:AMB; 主題=>tsú-tê:AMB |
| `editorSave` | 儲存 | thú-tsûn | HIGH |  |
| `editorResetAll` | 恢復預設設定 | hue-ho̍k ī-siat siat-tīng | MED | 恢復=>hue-ho̍k:AMB; 預設=>ī-siat:AMB |
| `defaultName` | 新主題 | sin tsú-tê | LOW | 新=>sin:1char; 主題=>tsú-tê:AMB |
| `cardMenuApply` | 套用 | thò-iōng | HIGH |  |
| `cardMenuEdit` | 編輯 | pian-tsi̍p | HIGH |  |
| `capReachedTitle` | 已達主題數量上限 | í ta̍t tsú-tê sòo-liōng siōng hān | LOW | 已=>í:1char; 達=>ta̍t:1char; 主題=>tsú-tê:AMB; 上=>siōng:1char; 限=>hān:1char |
| `capReachedMessage` | 自訂主題上限是 5 个,請先刪除一个才會使閣新增。 | tsū tīng tsú-tê siōng hān sī 5 ê, tshiánn sian san-tî tsi̍t-ê tsiah-ē-sái koh sin-tsing。 | LOW | 自=>tsū:1char; 訂=>tīng:1char; 主題=>tsú-tê:AMB; 上=>siōng:1char; 限=>hān:1char; 是=>sī:1char; 个=>ê:1char; 請=>tshiánn:1char; 先=>sian:1char; 刪除=>san-tî:AMB; 一个=>tsi̍t-ê:AMB; 閣=>koh:1char |
| `cardMenu` | 主題選項 | tsú-tê suán-hāng | MED | 主題=>tsú-tê:AMB |
| `colorPickerGrid` | 格仔 | keh-á | HIGH |  |
| `colorPickerSpectrum` | 光譜 | kong phóo | LOW | 光=>kong:1char; 譜=>phóo:1char |
| `colorPickerSliders` | 滑桿 | ku̍t kuáinn | LOW | 滑=>ku̍t:1char; 桿=>kuáinn:1char |
| `colorRed` | 紅色 | âng-sik | HIGH |  |
| `colorGreen` | 綠色 | li̍k-sik | HIGH |  |
| `colorBlue` | 藍色 | nâ-sik | MED | 藍色=>nâ-sik:AMB |
