# P3c R6-1 — Pe̍h-ōe-jī (POJ) draft review sheet (2026-06-22)

**Status: REVIEW-PENDING. Mechanically derived from the (also review-pending) Tâi-lô draft — NOT yet linguistically verified.**

## Why this exists

`poj` (Pe̍h-ōe-jī romanization) was added for all 211 UI keys to wire the GeneratedMap display-language path (Android `GeneratedTaigiStrings.kt` poj map + iOS `nan-Latn-TW-x-poj.lproj`). POJ is **DEBUG-selectable only** this round — it is NOT in the production picker (`productionLanguages` unchanged = hanji/en/ja). Promotion to production (R6-2) is gated on the review below **and** on the Tâi-lô review (R5-2), since POJ is derived from TL.

## How the draft was produced

- POJ is a **deterministic transliteration of TL** with no semantics of its own (`ts→ch`, `tsh→chh`, `oo→o͘`, `nn→ⁿ`, `ua→oa`, `ue→oe`, `ing→eng`, `ik→ek`, tone-position rules, …). So it is **derived-and-stored**, not hand-authored: `tools/i18n/derive_poj.py` (`make i18n-derive-poj`) runs every authored `tailo` value through the canonical `taigi-converter` bridge (`dictionary/common/taigi_bridge.convert_tl_to_poj_strict` — the same converter the dictionary pipeline uses) and writes the result back as the `poj` value in `i18n/*.json`. `make i18n` then emits it like any other language.
- The converter is the project's **single POJ authority** (CP#3 — authoritative-source-only; `.claude/rules/phonetics.md` §4). NO POJ was invented.
- tailo↔poj are kept in **lockstep**: `validate_generated_map_completeness` (`tools/i18n/i18n_lib.py`) fails the build if a key authors one but not the other. Re-run `make i18n-derive-poj` after any tailo correction.

## ⚠ Dependency on the Tâi-lô draft (read first)

**POJ correctness = TL correctness × converter correctness.** The converter is trusted (canonical reference); the TL draft is NOT yet reviewed (`docs/reports/2026-06-22-i18n-tl-draft-review.md`). So **every POJ row inherits the review status of its TL source**:

- Do **NOT** hand-edit a `poj` value to "fix" it. If a reading is wrong, fix the `tailo` value in `i18n/*.json`, then `make i18n-derive-poj` + `make i18n` — POJ re-derives automatically.
- The ONLY reason to touch POJ directly would be a genuine **converter defect** (TL correct, POJ wrong) — that is a `taigi-converter` bug or the first real `pojOverride` use case, not a row edit. None were found in the audit below.
- Promotion (R6-2) is gated on BOTH the TL review (R5-2) and a pass over this sheet.

## Override schema deliberately NOT built (plan D4 superseded)

The original plan (D4) proposed `pojOverride` / `derivePoj:false` / protected-span machinery to guard brand/URL/placeholder tokens. An audit of **all 211 real strings** (independently reproduced by Codex pre-impl) showed the converter rewrites **only** tokens it parses as valid TL syllables and preserves everything else **verbatim** — so that machinery is YAGNI and was not built. A minimal override would be added only if a real exception surfaces in review.

**Corruption audit — zero corruption:**

- **Brand / acronym / proper-noun / tech tokens preserved exactly**: `iTaigi`, `CSV`, `MB`, `iOS`, `Android`, `App`, `Enter`, `Email`, `ChhoeTaigi`, `ButTaiwan`, `Lohankha`, `PhahTaigi`, `justfont`.
- **`{placeholder}` spans preserved exactly**: `{imported}`, `{skipped}`, `{customDict}`, `{frequency}`, `{association}`.
- **Punctuation / CJK / digits** pass through untouched.

## Distribution

211 keys: **183 changed** (TL→POJ phoneme transform applied), **28 unchanged** (`= TL` rows — the TL spelling already has no POJ-differing phoneme, e.g. `hó`, `san-tî`, `ian-sui`; sanity-check these are genuinely diff-free, not a skipped conversion).

## What to review

For each row: is the **derived POJ** the correct POJ rendering of the **Tâi-lô** in the same row? (The `= TL` note flags rows where TL==POJ.) If a row looks wrong, decide whether the TL is wrong (fix tailo → re-derive) or the converter is wrong (rare — file against `taigi-converter`). The `note` column flags `= TL` rows and any preserved brand token.

## `common` (21 keys)

| key | 漢字 | Tâi-lô | derived POJ | note |
|---|---|---|---|---|
| `cancel` | 取消 | tshú-siau | chhú-siau |  |
| `ok` | 好 | hó | hó | = TL (no POJ-differing phoneme) |
| `delete` | 刪除 | san-tî | san-tî | = TL (no POJ-differing phoneme) |
| `back` | 返回 | tńg huê | tńg hôe |  |
| `viewWebsite` | 官方網站 | kuann-hong bāng-tsām | koaⁿ-hong bāng-chām |  |
| `exportFailed` | 匯出失敗 | huē-tshut sit-pāi | hōe-chhut sit-pāi |  |
| `importFailed` | 匯入失敗 | huē-ji̍p sit-pāi | hōe-ji̍p sit-pāi |  |
| `moeDict` | 教育部臺灣台語常用詞辭典 | kàu-io̍k-pōo tâi-uân-tâi-gí siâng-iōng-sû sû-tián | kàu-io̍k-pō͘ tâi-oân-tâi-gí siâng-iōng-sû sû-tián |  |
| `newwordDict` | 公視台語台台語新詞辭庫 | kong-sī tâi-gí-tâi tâi-gí sin sû sî khòo | kong-sī tâi-gí-tâi tâi-gí sin sû sî khò͘ |  |
| `kunggeDict` | 工藝中心臺灣台語工藝詞庫 | kang-gē tiong-sim tâi-uân-tâi-gí kang-gē sû khòo | kang-gē tiong-sim tâi-oân-tâi-gí kang-gē sû khò͘ |  |
| `iTaigiDict` | iTaigi愛台語 | iTaigi ài-tâi-gí | iTaigi ài-tâi-gí | = TL (no POJ-differing phoneme); brand kept: iTaigi |
| `taiwanJapanDict` | 臺日大辭典台語譯本 | tâi ji̍t tuā sû-tián tâi-gí i̍k-pún | tâi ji̍t tōa sû-tián tâi-gí e̍k-pún |  |
| `taiHuaDict` | 台華線頂對照典 | tâi huâ suànn-tíng tuì-tsiàu tián | tâi hôa sòaⁿ-téng tùi-chiàu tián |  |
| `taiwanPlantDict` | 台灣植物名彙 | tâi-uân tsi̍t-bu̍t miâ luī | tâi-oân chi̍t-bu̍t miâ lūi |  |
| `sttiDict` | 教育部學科術語臺灣台語對譯 | kàu-io̍k-pōo ha̍k-kho su̍t-gí tâi-uân-tâi-gí tuì-i̍k | kàu-io̍k-pō͘ ha̍k-kho su̍t-gí tâi-oân-tâi-gí tùi-e̍k |  |
| `accentDict` | 腔口差 | khiunn-kháu tsha | khiuⁿ-kháu chha |  |
| `fontSystemDefault` | 系統 | hē-thóng | hē-thóng | = TL (no POJ-differing phoneme) |
| `fontOpenHuninn` | 粉圓 | hún-înn | hún-îⁿ |  |
| `fontIansui` | 芫荽 | ian-sui | ian-sui | = TL (no POJ-differing phoneme) |
| `fontGenYoMin` | 源樣明體 | guân iūnn bîng-thé | goân iūⁿ bêng-thé |  |
| `fontGenYoGothic` | 源樣烏體 | guân iūnn oo thé | goân iūⁿ o͘ thé |  |

## `dictionary` (73 keys)

| key | 漢字 | Tâi-lô | derived POJ | note |
|---|---|---|---|---|
| `clear` | 清除 | tshing-tî | chheng-tî |  |
| `save` | 儉起來 | khiām khí-lâi | khiām khí-lâi | = TL (no POJ-differing phoneme) |
| `customDictionary` | 自訂詞庫 | tsū tīng sû khòo | chū tēng sû khò͘ |  |
| `customDictEnabled` | 啟用自訂詞庫 | khé-iōng tsū tīng sû khòo | khé-iōng chū tēng sû khò͘ |  |
| `customDictEnabledInfo` | 拍開了後，家己加入 ê 詞會出現佇候選詞內底。關起來了後，自訂詞就袂閣出現。 | phah-khui liáu-āu， ka-kī ka-ji̍p ê sû huē tshut-hiān tī hāu-suán sû lāi-té。 kuainn--khí-lâi liáu-āu， tsū tīng sû tsiū buē koh tshut-hiān。 | phah-khui liáu-āu， ka-kī ka-ji̍p ê sû hōe chhut-hiān tī hāu-soán sû lāi-té。 koaiⁿ--khí-lâi liáu-āu， chū tēng sû chiū bōe koh chhut-hiān。 |  |
| `dataManagement` | 個人資料 | kò-jîn-tsu-liāu | kò-jîn-chu-liāu |  |
| `variantDictionary` | 異用字 | ī iōng-jī | ī iōng-jī | = TL (no POJ-differing phoneme) |
| `khiin` | 在來字 | tsāi-lâi jī | chāi-lâi jī |  |
| `customDictDescription` | 自訂詞庫使用 CSV 純文字檔案，第 1 欄囥欲拍 ê 羅馬字，第 2 欄囥漢字，毋免囥標題。 | tsū tīng sû khòo sú-iōng CSV sûn bûn-jī-tòng àn， tē 1 nuâ khǹg beh phah ê lô-má-jī， tē 2 nuâ khǹg hàn-jī， m̄-bián khǹg piau-tuê。 | chū tēng sû khò͘ sú-iōng CSV sûn bûn-jī-tòng àn， tē 1 nôa khǹg beh phah ê lô-má-jī， tē 2 nôa khǹg hàn-jī， m̄-bián khǹg piau-tôe。 | brand kept: CSV |
| `customDictEmpty` | 揤 + 符號加入自訂詞 | tshi̍h + hû-hō ka-ji̍p tsū tīng sû | chhi̍h + hû-hō ka-ji̍p chū tēng sû |  |
| `addEntry` | 增加詞 | tsing-ka sû | cheng-ka sû |  |
| `editEntry` | 編輯詞 | pian-tsi̍p sû | pian-chi̍p sû |  |
| `romanLabel` | 拍字 | phah-jī | phah-jī | = TL (no POJ-differing phoneme) |
| `romanPlaceholder` | 見本：gâu-tsá | kiàn-pún：gâu-tsá | kiàn-pún：gâu-chá |  |
| `hanziLabel` | 對應 | tuì-ìng | tùi-èng |  |
| `hanziPlaceholder` | 見本：𠢕早 | kiàn-pún： gâu-tsá | kiàn-pún： gâu-chá |  |
| `deleteAll` | 刪除全部自訂詞 | san-tî tsuân-pōo tsū tīng sû | san-tî choân-pō͘ chū tēng sû |  |
| `deleteAllMessage` | 敢確定欲刪除所有自訂詞？ | kám khak-tīng beh san-tî sóo-iú tsū tīng sû？ | kám khak-tēng beh san-tî só͘-iú chū tēng sû？ |  |
| `customDictPrivacyWarning` | 請毋通佇自訂詞庫囥敏感 ê 個人資料，親像身分證字號、口座密碼、信用卡號碼，請注意家己 ê 資訊安全。 | tshiánn m̄-thang tī tsū tīng sû khòo khǹg bín-kám ê kò-jîn-tsu-liāu， tshin-tshiūnn sin-hūn-tsìng jī-hō、 kháu-tsō bi̍t-bé、 sìn-iōng-khah hō-bé， tshiánn tsù-ì ka-kī ê tsu-sìn an-tsuân。 | chhiáⁿ m̄-thang tī chū tēng sû khò͘ khǹg bín-kám ê kò-jîn-chu-liāu， chhin-chhiūⁿ sin-hūn-chèng jī-hō、 kháu-chō bi̍t-bé、 sìn-iōng-khah hō-bé， chhiáⁿ chù-ì ka-kī ê chu-sìn an-choân。 |  |
| `importCSV` | 匯入詞庫 | huē-ji̍p sû khòo | hōe-ji̍p sû khò͘ |  |
| `exportCSV` | 匯出詞庫 | huē-tshut sû khòo | hōe-chhut sû khò͘ |  |
| `exportSuccess` | CSV 順利匯出 | CSV sūn-lī huē-tshut | CSV sūn-lī hōe-chhut | brand kept: CSV |
| `importResult` | 匯入 {imported} 項成功，{skipped} 項重複 | huē-ji̍p {imported} hāng sîng-kong，{skipped} hāng-tāng ho̍k | hōe-ji̍p {imported} hāng sêng-kong，{skipped} hāng-tāng ho̍k |  |
| `invalidCSVFormat` | 檔案格式無正確，請使用 CSV 格式 | tòng-àn keh-sik bô tsìng-khak， tshiánn sú-iōng CSV keh-sik | tòng-àn keh-sek bô chèng-khak， chhiáⁿ sú-iōng CSV keh-sek | brand kept: CSV |
| `fileTooLarge` | 檔案傷大（上限 5 MB） | tòng-àn siong tuā（siōng hān 5 MB） | tòng-àn siong tōa（siōng hān 5 MB） | brand kept: MB |
| `tooManyEntries` | 詞傷濟（上限 30,000 項） | sû siunn-tse（siōng hān 30,000 hāng） | sû siuⁿ-che（siōng hān 30,000 hāng） |  |
| `importExportTitle` | 匯出匯入 | huē-tshut huē-ji̍p | hōe-chhut hōe-ji̍p |  |
| `frequencyExportCSV` | 匯出詞頻紀錄 | huē-tshut sû pîn kì-lio̍k | hōe-chhut sû pîn kì-lio̍k |  |
| `frequencyImportCSV` | 匯入詞頻紀錄 | huē-ji̍p sû pîn kì-lio̍k | hōe-ji̍p sû pîn kì-lio̍k |  |
| `frequencyDescription` | 詞頻紀錄使用 CSV 純文字檔案，第 1 欄囥詞，第 2 欄囥次數，毋免囥標題。 | sû pîn kì-lio̍k sú-iōng CSV sûn bûn-jī-tòng àn， tē 1 nuâ khǹg sû， tē 2 nuâ khǹg tshù siàu， m̄-bián khǹg piau-tuê。 | sû pîn kì-lio̍k sú-iōng CSV sûn bûn-jī-tòng àn， tē 1 nôa khǹg sû， tē 2 nôa khǹg chhù siàu， m̄-bián khǹg piau-tôe。 | brand kept: CSV |
| `associationExportCSV` | 匯出詞關聯紀錄 | huē-tshut sû kuan-liân kì-lio̍k | hōe-chhut sû koan-liân kì-lio̍k |  |
| `associationImportCSV` | 匯入詞關聯紀錄 | huē-ji̍p sû kuan-liân kì-lio̍k | hōe-ji̍p sû koan-liân kì-lio̍k |  |
| `associationDescription` | 詞關聯紀錄使用 CSV 純文字檔案，共 5 欄：頭前詞、頭前拍字、後壁詞、後壁拍字、次數，毋免囥標題。 | sû kuan-liân kì-lio̍k sú-iōng CSV sûn bûn-jī-tòng àn， kā 5 nuâ： thâu-tsîng sû、 thâu-tsîng phah-jī、 āu-piah sû、 āu-piah phah-jī、 tshù siàu， m̄-bián khǹg piau-tuê。 | sû koan-liân kì-lio̍k sú-iōng CSV sûn bûn-jī-tòng àn， kā 5 nôa： thâu-chêng sû、 thâu-chêng phah-jī、 āu-piah sû、 āu-piah phah-jī、 chhù siàu， m̄-bián khǹg piau-tôe。 | brand kept: CSV |
| `moeSectionTitle` | 教育部用字 | kàu-io̍k-pōo iōng-jī | kàu-io̍k-pō͘ iōng-jī |  |
| `otherSectionTitle` | 其他辭典 | kî-thann sû-tián | kî-thaⁿ sû-tián |  |
| `supplementSectionTitle` | 補充資料 | póo-tshiong tsu-liāu | pó͘-chhiong chu-liāu |  |
| `lkkDict` | 漢羅合用建議用字 | hàn-lô ha̍h-īng kiàn-gī iōng-jī | hàn-lô ha̍h-ēng kiàn-gī iōng-jī |  |
| `devSupplementDict` | 詞庫增補檔案 | sû khòo tsing-póo tòng-àn | sû khò͘ cheng-pó͘ tòng-àn |  |
| `kautianAccentLukang` | 鹿港偏泉腔 | lo̍k-káng phian tsuân-khiunn | lo̍k-káng phian choân-khiuⁿ |  |
| `kautianAccentSansia` | 三峽偏泉腔 | sam-kiap phian tsuân-khiunn | sam-kiap phian choân-khiuⁿ |  |
| `kautianAccentTaipak` | 臺北偏泉腔 | tâi-pak phian tsuân-khiunn | tâi-pak phian choân-khiuⁿ |  |
| `kautianAccentGilan` | 宜蘭偏漳腔 | gî-lân phian tsiang-khiunn | gî-lân phian chiang-khiuⁿ |  |
| `kautianAccentTainan` | 臺南混合腔 | tâi-lâm hūn-ha̍p khiunn | tâi-lâm hūn-ha̍p khiuⁿ |  |
| `kautianAccentKaohsiung` | 高雄混合腔 | ko-hiông hūn-ha̍p khiunn | ko-hiông hūn-ha̍p khiuⁿ |  |
| `kautianAccentKinmen` | 金門偏泉腔 | kim-mn̂g phian tsuân-khiunn | kim-mn̂g phian choân-khiuⁿ |  |
| `kautianAccentMakung` | 馬公偏泉腔 | bé-kang phian tsuân-khiunn | bé-kang phian choân-khiuⁿ |  |
| `kautianAccentSintik` | 新竹偏泉腔 | sin-tik phian tsuân-khiunn | sin-tek phian choân-khiuⁿ |  |
| `kautianAccentTaichung` | 臺中偏漳腔 | tâi-tiong phian tsiang-khiunn | tâi-tiong phian chiang-khiuⁿ |  |
| `kautianNameAppendix` | 姓名附錄 | sènn-miâ hù-lio̍k | sèⁿ-miâ hù-lio̍k |  |
| `searchPlaceholder` | 拍字揣詞 | phah-jī tshuē sû | phah-jī chhōe sû |  |
| `noResults` | 揣無結果 | tshuē-bô kiat-kó | chhōe-bô kiat-kó |  |
| `lookupChhoe` | ChhoeTaigi 辭典 | ChhoeTaigi sû-tián | ChhoeTaigi sû-tián | = TL (no POJ-differing phoneme); brand kept: ChhoeTaigi |
| `lookupMoe` | 教育部辭典 | kàu-io̍k-pōo sû-tián | kàu-io̍k-pō͘ sû-tián |  |
| `frequencyManagement` | 詞頻紀錄 | sû pîn kì-lio̍k | sû pîn kì-lio̍k | = TL (no POJ-differing phoneme) |
| `frequencyRecordingEnabled` | 開啟詞頻紀錄 | khai-khé sû pîn kì-lio̍k | khai-khé sû pîn kì-lio̍k | = TL (no POJ-differing phoneme) |
| `frequencyRecordingEnabledInfo` | 拍開了後，齒盤會記錄你揀過 ê 詞幾擺，予候選詞排序做參考，定定揀 ê 詞就會排較頭前，按呢候選詞就會愈來愈準。 | phah-khui liáu-āu， khí-puânn huē kì-lio̍k lí kíng kuè ê sû kuí-pái， hōo hāu-suán sû pâi-sī tsuè tsham-khó， tiānn-tiānn kíng ê sû tsiū huē pâi khah thâu-tsîng， án-ne hāu-suán sû tsiū huē jú-lâi-jú tsún。 | phah-khui liáu-āu， khí-pôaⁿ hōe kì-lio̍k lí kéng kòe ê sû kúi-pái， hō͘ hāu-soán sû pâi-sī chòe chham-khó， tiāⁿ-tiāⁿ kéng ê sû chiū hōe pâi khah thâu-chêng， án-ne hāu-soán sû chiū hōe jú-lâi-jú chún。 |  |
| `associationManagement` | 詞關聯紀錄 | sû kuan-liân kì-lio̍k | sû koan-liân kì-lio̍k |  |
| `associationRecordingEnabled` | 開啟詞關聯紀錄 | khai-khé sû kuan-liân kì-lio̍k | khai-khé sû koan-liân kì-lio̍k |  |
| `associationRecordingEnabledInfo` | 拍開了後，齒盤會記錄頭前、後壁 ê 關聯詞，予連紲建議愈來愈準。 | phah-khui liáu-āu， khí-puânn huē kì-lio̍k thâu-tsîng、 āu-piah ê kuan-liân sû， hōo liân-suà kiàn-gī jú-lâi-jú tsún。 | phah-khui liáu-āu， khí-pôaⁿ hōe kì-lio̍k thâu-chêng、 āu-piah ê koan-liân sû， hō͘ liân-sòa kiàn-gī jú-lâi-jú chún。 |  |
| `frequencyPrivacyWarning` | 詞頻紀錄對台語研究來講是真有價值 ê 資料。若欲提供予人研究訓練模型，請先刪除敏感 ê 私人資料。紀錄功能嘛會使關起來，毋過按呢候選詞 ê 排序就會較無準。 | sû pîn kì-lio̍k tuì tâi-gí gián-kiù lâi kóng sī tsin ū-kè ta̍t ê tsu-liāu。 nā-beh thê-kiong hōo-lâng gián-kiù hùn-liān bôo-hîng， tshiánn sian san-tî bín-kám ê su-jîn tsu-liāu。 kì-lio̍k kong-lîng mā ē-sái kuainn--khí-lâi， m̄-koh án-ne hāu-suán sû ê pâi-sī tsiū huē khah bô tsún。 | sû pîn kì-lio̍k tùi tâi-gí gián-kiù lâi kóng sī chin ū-kè ta̍t ê chu-liāu。 nā-beh thê-kiong hō͘-lâng gián-kiù hùn-liān bô͘-hêng， chhiáⁿ sian san-tî bín-kám ê su-jîn chu-liāu。 kì-lio̍k kong-lêng mā ē-sái koaiⁿ--khí-lâi， m̄-koh án-ne hāu-soán sû ê pâi-sī chiū hōe khah bô chún。 |  |
| `clearAllFrequency` | 刪除所有詞頻紀錄 | san-tî sóo-iú sû pîn kì-lio̍k | san-tî só͘-iú sû pîn kì-lio̍k |  |
| `associationPrivacyWarning` | 詞關聯紀錄對台語研究來講是真有價值 ê 資料。若欲提供予人做研究，請先刪除敏感 ê 內容。紀錄功能嘛會使關起來，毋過後一詞預測會較無準。 | sû kuan-liân kì-lio̍k tuì tâi-gí gián-kiù lâi kóng sī tsin ū-kè ta̍t ê tsu-liāu。 nā-beh thê-kiong hōo-lâng tsuè gián-kiù， tshiánn sian san-tî bín-kám ê luē-iông。 kì-lio̍k kong-lîng mā ē-sái kuainn--khí-lâi， m̄-koh āu tsi̍t sû ī-tshik huē khah bô tsún。 | sû koan-liân kì-lio̍k tùi tâi-gí gián-kiù lâi kóng sī chin ū-kè ta̍t ê chu-liāu。 nā-beh thê-kiong hō͘-lâng chòe gián-kiù， chhiáⁿ sian san-tî bín-kám ê lōe-iông。 kì-lio̍k kong-lêng mā ē-sái koaiⁿ--khí-lâi， m̄-koh āu chi̍t sû ī-chhek hōe khah bô chún。 |  |
| `clearAllAssociation` | 刪除所有詞關聯紀錄 | san-tî sóo-iú sû kuan-liân kì-lio̍k | san-tî só͘-iú sû koan-liân kì-lio̍k |  |
| `clearFrequencyMessage` | 確定欲刪除所有詞頻紀錄？ | khak-tīng beh san-tî sóo-iú sû pîn kì-lio̍k？ | khak-tēng beh san-tî só͘-iú sû pîn kì-lio̍k？ |  |
| `clearAssociationMessage` | 確定欲刪除所有詞關聯紀錄？ | khak-tīng beh san-tî sóo-iú sû kuan-liân kì-lio̍k？ | khak-tēng beh san-tî só͘-iú sû koan-liân kì-lio̍k？ |  |
| `noData` | 無資料 | bô tsu-liāu | bô chu-liāu |  |
| `filterHint` | 頂面上濟顯示 100 个詞，若揣無詞，請用下跤 ê「拍字揣詞」功能搜揣。 | tíng-bīn siāng tsuē hián-sī 100 ê sû， nā tshuē-bô sû， tshiánn iōng ē-kha ê「phah-jī tshuē sû」 kong-lîng tshiau-tshuē。 | téng-bīn siāng chōe hián-sī 100 ê sû， nā chhōe-bô sû， chhiáⁿ iōng ē-kha ê「phah-jī chhōe sû」 kong-lêng chhiau-chhōe。 |  |
| `backupRestore` | 備份復原 | pī-hūn ho̍k-guân | pī-hūn ho̍k-goân |  |
| `exportBackup` | 一擺全出 | tsi̍t-pái tsuân tshut | chi̍t-pái choân chhut |  |
| `importBackup` | 一擺全入 | tsi̍t-pái tsuân ji̍p | chi̍t-pái choân ji̍p |  |
| `exportBackupSuccess` | 備份順利匯出 | pī-hūn sūn-lī huē-tshut | pī-hūn sūn-lī hōe-chhut |  |
| `importBackupResult` | 匯入 {customDict} 項自訂詞、{frequency} 項詞頻、{association} 項詞關聯 | huē-ji̍p {customDict} hāng tsū tīng sû、{frequency} hāng sû pîn、{association} hāng sû kuan-liân | hōe-ji̍p {customDict} hāng chū tēng sû、{frequency} hāng sû pîn、{association} hāng sû koan-liân | brand kept: customDict |
| `backupPrivacyWarning` | 備份檔案內底包含你所有 ê 使用紀錄，佇 iOS 佮 Android 攏通用，準講換機仔嘛毋免煩惱。 | pī-hūn tòng-àn lāi-té pau-hâm lí sóo-iú ê sú-iōng kì-lio̍k， tī iOS kah Android láng thong-iōng， tsún-kóng uānn ki-á mā m̄-bián huân-ló。 | pī-hūn tòng-àn lāi-té pau-hâm lí só͘-iú ê sú-iōng kì-lio̍k， tī iOS kah Android láng thong-iōng， chún-kóng ōaⁿ ki-á mā m̄-bián hoân-ló。 | brand kept: Android/iOS |

## `home` (34 keys)

| key | 漢字 | Tâi-lô | derived POJ | note |
|---|---|---|---|---|
| `appHeaderTitle` | 台語齒盤 | tâi-gí khí-puânn | tâi-gí khí-pôaⁿ |  |
| `setupKeyboard` | 齒盤愛拍開才會當使用 | khí-puânn ài phah-khui tsiah ē tng sú-iōng | khí-pôaⁿ ài phah-khui chiah ē tng sú-iōng |  |
| `typingGuide` | 拍字說明 | phah-jī suat-bîng | phah-jī soat-bêng |  |
| `newFeatures` | 功能設定 | kong-lîng siat-tīng | kong-lêng siat-tēng |  |
| `faq` | 其他 | kî-thann | kî-thaⁿ |  |
| `setupGuide` | 啟用方法 | khé-iōng hong-huat | khé-iōng hong-hoat |  |
| `setupGuideDescription` | 手機仔系統規定第三方齒盤愛手動啟用才會當使用，請照下跤 ê 說明完成設定。 | tshiú-ki-á hē-thóng kui-tīng tē-sann hong khí-puânn ài tshiú-tōng khé-iōng tsiah ē tng sú-iōng， tshiánn tsiàu ē-kha ê suat-bîng uân-sîng siat-tīng。 | chhiú-ki-á hē-thóng kui-tēng tē-saⁿ hong khí-pôaⁿ ài chhiú-tōng khé-iōng chiah ē tng sú-iōng， chhiáⁿ chiàu ē-kha ê soat-bêng oân-sêng siat-tēng。 |  |
| `setupInfoMessage` | 「允准完整取用」意思是予齒盤會當捌你揤 ê 動作，成做你拍 ê 字。請放心，App 袂紀錄你 ê 資料。 | 「ún-tsún uân-tsíng tshú-iōng」 ì-sù sī hōo khí-puânn ē-tàng bat lí tshi̍h ê tōng-tsok， tsiânn-tsuè lí phah ê jī。 tshiánn hòng-sim，App buē kì-lio̍k lí ê tsu-liāu。 | 「ún-chún oân-chéng chhú-iōng」 ì-sù sī hō͘ khí-pôaⁿ ē-tàng bat lí chhi̍h ê tōng-chok， chiâⁿ-chòe lí phah ê jī。 chhiáⁿ hòng-sim，App bōe kì-lio̍k lí ê chu-liāu。 | brand kept: App |
| `setupBrandWarning` | 無仝牌子 ê 手機仔，設定 ê 方式可能會淡薄仔無仝款，毋過方式應該攏差不多。 | bô kâng pâi-tsú ê tshiú-ki-á， siat-tīng ê hong-sik khó-lîng huē tām-po̍h-á bô-kāng-khuán， m̄-koh hong-sik ìng-kai láng tsha-put-to。 | bô kâng pâi-chú ê chhiú-ki-á， siat-tēng ê hong-sek khó-lêng hōe tām-po̍h-á bô-kāng-khoán， m̄-koh hong-sek èng-kai láng chha-put-to。 |  |
| `setupGuideCompletedMessage` | 完成了後，重開你目前使用 ê App，予 App 重掠新 ê 齒盤清單。紲落來，佇會當拍字 ê 所在，揤牢地球圖示切去台語齒盤 | uân-sîng liáu-āu， tîng-khui lí ba̍k-tsîng sú-iōng ê App， hōo App tāng lia̍h sin ê khí-puânn tshing-tuann。 suà--lo̍h-lâi， tī ē-tàng phah-jī ê sóo-tsāi， tshi̍h tiâu tuē-kiû-tôo sī tshiat khì tâi-gí khí-puânn | oân-sêng liáu-āu， têng-khui lí ba̍k-chêng sú-iōng ê App， hō͘ App tāng lia̍h sin ê khí-pôaⁿ chheng-toaⁿ。 sòa--lo̍h-lâi， tī ē-tàng phah-jī ê só͘-chāi， chhi̍h tiâu tōe-kiû-tô͘ sī chhiat khì tâi-gí khí-pôaⁿ | brand kept: App |
| `setupGuideGoToSettings` | 去設定頁 | khì siat-tīng ia̍h | khì siat-tēng ia̍h |  |
| `setupGuideCloseButton` | 關閉 | kuan pì | koan pì |  |
| `setupGuideStep1Settings` | 點揤「齒盤」 | tiám-tshi̍h「khí-puânn」 | tiám-chhi̍h「khí-pôaⁿ」 |  |
| `setupGuideStep2AddKeyboard` | 點揤「增加齒盤」、「允准完整取用」 | tiám-tshi̍h「tsing-ka khí-puânn」、「ún-tsún uân-tsíng tshú-iōng」 | tiám-chhi̍h「cheng-ka khí-pôaⁿ」、「ún-chún oân-chéng chhú-iōng」 |  |
| `userGuide` | 網站紹介 | bāng-tsām siāu-kài | bāng-chām siāu-kài |  |
| `rateUs` | 為阮評分 | uî guán phîng-hun | ûi goán phêng-hun |  |
| `aboutDeveloper` | 關於 | kuan-î | koan-î |  |
| `privacyPolicy` | 隱私權政策 | ín-su-khuân tsìng-tshik | ín-su-khoân chèng-chhek |  |
| `freePromise` | 台語齒盤保證永遠免費，嘛袂做付費功能。台語是咱 ê 母語，無應該因為錢 ê 問題用袂著好家私。我向望逐家想欲學台語、寫台語 ê 人攏會當無負擔來使用，這是我做這个齒盤上重要 ê 心願。 | tâi-gí khí-puânn pó-tsìng íng-uán bián-huì， mā buē tsuè hù-huì kong-lîng。 tâi-gí sī lán ê bó-gí， bô ìng-kai in-uī tsînn ê būn-tê iōng buē tio̍h hó ke-si。 guá ǹg-bāng ta̍k-ke siūnn-beh ha̍k tâi-gí、 siá tâi-gí ê jîn láng ē-tàng bô hū-tam lâi sú-iōng， tse-sī guá tsuè tsit ê khí-puânn siōng tiōng-iàu ê sim-guān。 | tâi-gí khí-pôaⁿ pó-chèng éng-oán bián-hùi， mā bōe chòe hù-hùi kong-lêng。 tâi-gí sī lán ê bó-gí， bô èng-kai in-ūi chîⁿ ê būn-tê iōng bōe tio̍h hó ke-si。 góa ǹg-bāng ta̍k-ke siūⁿ-beh ha̍k tâi-gí、 siá tâi-gí ê jîn láng ē-tàng bô hū-tam lâi sú-iōng， che-sī góa chòe chit ê khí-pôaⁿ siōng tiōng-iàu ê sim-goān。 |  |
| `version` | 當前版本 | tong-tsiân pán-pún | tong-chiân pán-pún |  |
| `versionHistory` | 版本紀錄 | pán-pún kì-lio̍k | pán-pún kì-lio̍k | = TL (no POJ-differing phoneme) |
| `copyrightNotice` | 致謝 | tì-siā | tì-siā | = TL (no POJ-differing phoneme) |
| `viewLicense` | 授權條款 | siū-khuân tiâu-khuán | siū-khoân tiâu-khoán |  |
| `moeCopyright` | © 教育部 | © kàu-io̍k-pōo | © kàu-io̍k-pō͘ |  |
| `iTaigiCopyright` | © iTaigi愛台語 | © iTaigi ài-tâi-gí | © iTaigi ài-tâi-gí | = TL (no POJ-differing phoneme); brand kept: iTaigi |
| `newwordCopyright` | © 公視台語台 | © kong-sī tâi-gí-tâi | © kong-sī tâi-gí-tâi | = TL (no POJ-differing phoneme) |
| `openFontCopyright` | © justfont | © justfont | © justfont | = TL (no POJ-differing phoneme); brand kept: justfont |
| `butTaiwanCopyright` | © ButTaiwan | © ButTaiwan | © ButTaiwan | = TL (no POJ-differing phoneme); brand kept: ButTaiwan |
| `taiwanPlantCopyright` | © 佐佐木舜一 | © tsò-tsò-bo̍k sùn tsi̍t | © chò-chò-bo̍k sùn chi̍t |  |
| `taiHuaCopyright` | © 鄭良偉 | © tēnn liông uí | © tēⁿ liông úi |  |
| `taiwanJapanCopyright` | © 小川尚義 | © siáu-tshuan siōng gī | © siáu-chhoan siōng gī |  |
| `kunggeCopyright` | © 國立臺灣工藝研究發展中心 | © kok-li̍p tâi-uân kang-gē gián-kiù huat-tián tiong-sim | © kok-li̍p tâi-oân kang-gē gián-kiù hoat-tián tiong-sim |  |
| `accentDictCredit` | 實齋整理、提供 | si̍t tse tsíng-lí、 thê-kiong | si̍t che chéng-lí、 thê-kiong |  |
| `devSupplementCredit` | 建中整理、提供 | kiàn-tiong tsíng-lí、 thê-kiong | kiàn-tiong chéng-lí、 thê-kiong |  |

## `layout` (8 keys)

| key | 漢字 | Tâi-lô | derived POJ | note |
|---|---|---|---|---|
| `romanizationKeyboard` | 羅馬字齒盤 | lô-má-jī khí-puânn | lô-má-jī khí-pôaⁿ |  |
| `taigiPhonetic` | 方音符號 | hong-im-hû-hō | hong-im-hû-hō | = TL (no POJ-differing phoneme) |
| `standardLayout` | Lohankha | Lohankha | Lohankha | = TL (no POJ-differing phoneme); brand kept: Lohankha |
| `phahTaigiLayout` | PhahTaigi | PhahTaigi | PhahTaigi | = TL (no POJ-differing phoneme); brand kept: PhahTaigi |
| `tpsLayout` | 方音符號齒佈 | hong-im-hû-hō khí pòo | hong-im-hû-hō khí pò͘ |  |
| `moe1Layout` | 教育部輸入法齒佈1 | kàu-io̍k-pōo su-ji̍p-hoat khí pòo1 | kàu-io̍k-pō͘ su-ji̍p-hoat khí pòo1 |  |
| `moe2Layout` | 教育部輸入法齒佈2 | kàu-io̍k-pōo su-ji̍p-hoat khí pòo2 | kàu-io̍k-pō͘ su-ji̍p-hoat khí pòo2 |  |
| `comingSoon` | 連鞭上市 | liâm-mi tsiūnn-tshī | liâm-mi chiūⁿ-chhī |  |

## `settings` (39 keys)

| key | 漢字 | Tâi-lô | derived POJ | note |
|---|---|---|---|---|
| `reset` | 恢復 | hue-ho̍k | hoe-ho̍k |  |
| `inputMode` | 輸入模式 | su-ji̍p bôo-sik | su-ji̍p bô͘-sek |  |
| `displayLanguage` | 顯示語言 | hián-sī gí-giân | hián-sī gí-giân | = TL (no POJ-differing phoneme) |
| `displayLanguageAutomatic` | 自動 | tsū-tōng | chū-tōng |  |
| `pojMode` | 白話字 | pe̍h-uē-jī | pe̍h-ōe-jī |  |
| `tlMode` | 台羅 | tâi-lô | tâi-lô | = TL (no POJ-differing phoneme) |
| `englishMode` | 英文 | ing-bûn | eng-bûn |  |
| `tpsMode` | 方音符號 | hong-im-hû-hō | hong-im-hû-hō | = TL (no POJ-differing phoneme) |
| `typingSectionTitle` | 拍字設定 | phah-jī siat-tīng | phah-jī siat-tēng |  |
| `outputBothScripts` | 括號標註 | kuat-hō phiau-tsù | koat-hō phiau-chù |  |
| `literalRomanCandidate` | 顯示羅馬字 | hián-sī lô-má-jī | hián-sī lô-má-jī | = TL (no POJ-differing phoneme) |
| `literalRomanCandidateInfo` | 候選詞列第一个位囥羅馬字，會當用手點抑是揤 Enter 送出，若關，干焦會當揤 Enter 送出，袂當用手點，但是候選詞列空間較大。 | hāu-suán sû lia̍t tē-it ê uī khǹg lô-má-jī， ē-tàng iōng tshiú tiám ah-sī tshi̍h Enter sàng-tshut， nā kuan， kan-tann-ē-tàng tshi̍h Enter sàng-tshut， buē-tàng iōng tshiú tiám， tān-sī hāu-suán sû lia̍t khang-king khah-tōa。 | hāu-soán sû lia̍t tē-it ê ūi khǹg lô-má-jī， ē-tàng iōng chhiú tiám ah-sī chhi̍h Enter sàng-chhut， nā koan， kan-taⁿ-ē-tàng chhi̍h Enter sàng-chhut， bōe-tàng iōng chhiú tiám， tān-sī hāu-soán sû lia̍t khang-keng khah-tōa。 | brand kept: Enter |
| `autoCapitalization` | 自動大本字 | tsū-tōng tuā-pún-jī | chū-tōng tōa-pún-jī |  |
| `autoSpace` | 自動空白 | tsū-tōng khàng-pe̍h | chū-tōng khàng-pe̍h |  |
| `keyboardSectionTitle` | 齒盤設定 | khí-puânn siat-tīng | khí-pôaⁿ siat-tēng |  |
| `toolbarAutoCollapse` | 自動隱藏工具列 | tsū-tōng ún-tsông kang-khū-lia̍t | chū-tōng ún-chông kang-khū-lia̍t |  |
| `toolbarAutoCollapseInfo` | 選字了後工具列會自動合起來，予齒盤面頂空間較大。 | suán jī liáu-āu kang-khū-lia̍t huē tsū-tōng ha̍p--khí-lâi， hōo khí-puânn bīn-tíng khang-king khah-tōa。 | soán jī liáu-āu kang-khū-lia̍t hōe chū-tōng ha̍p--khí-lâi， hō͘ khí-pôaⁿ bīn-téng khang-keng khah-tōa。 |  |
| `globeKey` | 齒盤切換揤鈕 | khí-puânn tshiat-uānn tshi̍h-liú | khí-pôaⁿ chhiat-ōaⁿ chhi̍h-liú |  |
| `globeKeyInfo` | 佇齒盤面頂加 1 粒地球揤鈕，揤著會使切換去其他齒盤。 | tī khí-puânn bīn-tíng ka 1 lia̍p tuē-kiû tshi̍h-liú， tshi̍h tio̍h ē-sái tshiat-uānn khì kî-thann khí-puânn。 | tī khí-pôaⁿ bīn-téng ka 1 lia̍p tōe-kiû chhi̍h-liú， chhi̍h tio̍h ē-sái chhiat-ōaⁿ khì kî-thaⁿ khí-pôaⁿ。 |  |
| `feedbackSectionTitle` | 拍字反應 | phah-jī huán-ìng | phah-jī hoán-èng |  |
| `soundFeedback` | 揤仔聲 | tshi̍h á siann | chhi̍h á siaⁿ |  |
| `vibrationFeedback` | 震動反應 | tín-tāng huán-ìng | tín-tāng hoán-èng |  |
| `pojSettingsSectionTitle` | 白話字 | pe̍h-uē-jī | pe̍h-ōe-jī |  |
| `doubleTapOO` | 連紲拍 oo → o͘ | liân-suà phah oo → o͘ | liân-sòa phah o͘ → o͘ |  |
| `doubleTapNN` | 連紲拍 nn → ⁿ | liân-suà phah nn → ⁿ | liân-sòa phah nn → ⁿ |  |
| `tpsSettingsSectionTitle` | 方音符號 | hong-im-hû-hō | hong-im-hû-hō | = TL (no POJ-differing phoneme) |
| `tpsOrMapsToER` | or 對應 ㄜ | or tuì-ìng ㄜ | or tùi-èng ㄜ |  |
| `tpsOrMapsToERInfo` | 台羅 or 毋是正式寫法，方音符號 ㄜ 正式干焦對應 er。本設定只控制候選詞按怎顯示;字典揣詞已經共 er 佮 or 攏對應做仝一个音位，無論本設定開抑無開攏揣會著。

開啟（預設）：or 顯示做 ㄜ。
關閉：or 顯示做 ㄛ（恢復台羅 o）。 | tâi-lô or m̄-sī tsìng-sik siá-huat， hong-im-hû-hō ㄜ tsìng-sik kan-na tuì-ìng er。 pún siat-tīng tsí khòng-tsè hāu-suán sû án-nuá hián-sī; jī-tián tshuē sû í-king kā er kah or láng tuì-ìng tsuè kāng tsi̍t-ê im-uī， bô-lūn pún siat-tīng khui ah-bô khui láng tshuē huē tio̍h。 khai-khé（ī-siat）：or hián-sī tsuè ㄜ。 kuan pì：or hián-sī tsuè ㄛ（hue-ho̍k tâi-lô o）。 | tâi-lô or m̄-sī chèng-sek siá-hoat， hong-im-hû-hō ㄜ chèng-sek kan-na tùi-èng er。 pún siat-tēng chí khòng-chè hāu-soán sû án-nóa hián-sī; jī-tián chhōe sû í-keng kā er kah or láng tùi-èng chòe kāng chi̍t-ê im-ūi， bô-lūn pún siat-tēng khui ah-bô khui láng chhōe hōe tio̍h。 khai-khé（ī-siat）：or hián-sī chòe ㄜ。 koan pì：or hián-sī chòe ㄛ（hoe-ho̍k tâi-lô o）。 |  |
| `resetSettings` | 恢復設定 | hue-ho̍k siat-tīng | hoe-ho̍k siat-tēng |  |
| `resetSettingsMessage` | 這个動作會恢復所有設定，敢欲繼續？ | tsit ê tōng-tsok huē hue-ho̍k sóo-iú siat-tīng， kám beh kè-sio̍k？ | chit ê tōng-chok hōe hoe-ho̍k só͘-iú siat-tēng， kám beh kè-sio̍k？ |  |
| `resetSuccess` | 設定已恢復 | siat-tīng í hue-ho̍k | siat-tēng í hoe-ho̍k |  |
| `resetFailed` | 恢復設定失敗，請重試 | hue-ho̍k siat-tīng sit-pāi， tshiánn tāng tshì | hoe-ho̍k siat-tēng sit-pāi， chhiáⁿ tāng chhì |  |
| `noEmailApp` | 揣無 Email App | tshuē-bô Email App | chhōe-bô Email App | brand kept: App/Email |
| `diagnosticSectionTitle` | 裝置資訊 | tsong-tì tsu-sìn | chong-tì chu-sìn |  |
| `diagnosticCopy` | Khó͘-phih 裝置資訊 | Khó͘-phih tsong-tì tsu-sìn | Khó͘-phih chong-tì chu-sìn | brand kept: Kh |
| `diagnosticCopied` | 已 khó͘-phih | í khó͘-phih | í khó͘-phih | = TL (no POJ-differing phoneme) |
| `diagnosticShare` | 分享裝置資訊 | hun-hióng tsong-tì tsu-sìn | hun-hióng chong-tì chu-sìn |  |
| `diagnosticEmail` | Email 回報問題 | Email huê-pò būn-tê | Email hôe-pò būn-tê | brand kept: Email |
| `openApp` | 去APP調整 | khìAPP tiâu-tsíng | khìAPP tiâu-chéng | brand kept: APP |

## `theme` (36 keys)

| key | 漢字 | Tâi-lô | derived POJ | note |
|---|---|---|---|---|
| `customFont` | 字型設定 | jī-hîng siat-tīng | jī-hêng siat-tēng |  |
| `customThemesSection` | 自訂主題 | tsū tīng tsú-tê | chū tēng chú-tê |  |
| `createNewTheme` | 新主題… | sin tsú-tê… | sin chú-tê… |  |
| `keyboardSection` | 齒盤介面 | khí-puânn kài-bīn | khí-pôaⁿ kài-bīn |  |
| `colorKeySection` | 揤鈕介面 | tshi̍h-liú kài-bīn | chhi̍h-liú kài-bīn |  |
| `candidateSection` | 候選詞介面 | hāu-suán sû kài-bīn | hāu-soán sû kài-bīn |  |
| `colorKeyboardBackground` | 齒盤色水 | khí-puânn sik-tsuí | khí-pôaⁿ sek-chúi |  |
| `colorKeyText` | 揤鈕文字 | tshi̍h-liú bûn-jī | chhi̍h-liú bûn-jī |  |
| `colorNormalKeyFill` | 一般揤鈕色水 | it-puann tshi̍h-liú sik-tsuí | it-poaⁿ chhi̍h-liú sek-chúi |  |
| `colorSpecialKeyFill` | 特殊揤鈕色水 | ti̍k-sû tshi̍h-liú sik-tsuí | te̍k-sû chhi̍h-liú sek-chúi |  |
| `colorCandidateText` | 候選詞文字 | hāu-suán sû bûn-jī | hāu-soán sû bûn-jī |  |
| `colorCandidateBackground` | 候選詞背景 | hāu-suán sû puē-kíng | hāu-soán sû pōe-kéng |  |
| `keyHeight` | 齒盤懸度 | khí-puânn kuân-tōo | khí-pôaⁿ koân-tō͘ |  |
| `keyFontSize` | 揤鈕字大細 | tshi̍h-liú jī tuā-sè | chhi̍h-liú jī tōa-sè |  |
| `candidateTextSize` | 候選詞大細 | hāu-suán sû tuā-sè | hāu-soán sû tōa-sè |  |
| `keyCornerRadius` | 揤鈕圓角 | tshi̍h-liú înn kak | chhi̍h-liú îⁿ kak |  |
| `keyBorderWidth` | 揤鈕邊粗幼 | tshi̍h-liú pinn tshoo-iù | chhi̍h-liú piⁿ chho͘-iù |  |
| `keyShadow` | 揤鈕陰影 | tshi̍h-liú im-iánn | chhi̍h-liú im-iáⁿ |  |
| `editorTitleNew` | 新主題 | sin tsú-tê | sin chú-tê |  |
| `editorTitleEdit` | 編輯主題 | pian-tsi̍p tsú-tê | pian-chi̍p chú-tê |  |
| `nameHeader` | 主題名稱 | tsú-tê miâ-tshing | chú-tê miâ-chheng |  |
| `namePlaceholder` | 輸入主題名稱 | su-ji̍p tsú-tê miâ-tshing | su-ji̍p chú-tê miâ-chheng |  |
| `editorSave` | 儲存 | thú-tsûn | thú-chûn |  |
| `editorResetAll` | 恢復預設設定 | hue-ho̍k ī-siat siat-tīng | hoe-ho̍k ī-siat siat-tēng |  |
| `defaultName` | 新主題 | sin tsú-tê | sin chú-tê |  |
| `cardMenuApply` | 套用 | thò-iōng | thò-iōng | = TL (no POJ-differing phoneme) |
| `cardMenuEdit` | 編輯 | pian-tsi̍p | pian-chi̍p |  |
| `capReachedTitle` | 已達主題數量上限 | í ta̍t tsú-tê sòo-liōng siōng hān | í ta̍t chú-tê sò͘-liōng siōng hān |  |
| `capReachedMessage` | 自訂主題上限是 5 个,請先刪除一个才會使閣新增。 | tsū tīng tsú-tê siōng hān sī 5 ê, tshiánn sian san-tî tsi̍t-ê tsiah-ē-sái koh sin-tsing。 | chū tēng chú-tê siōng hān sī 5 ê, chhiáⁿ sian san-tî chi̍t-ê chiah-ē-sái koh sin-cheng。 |  |
| `cardMenu` | 主題選項 | tsú-tê suán-hāng | chú-tê soán-hāng |  |
| `colorPickerGrid` | 格仔 | keh-á | keh-á | = TL (no POJ-differing phoneme) |
| `colorPickerSpectrum` | 光譜 | kong phóo | kong phó͘ |  |
| `colorPickerSliders` | 滑桿 | ku̍t kuáinn | ku̍t koáiⁿ |  |
| `colorRed` | 紅色 | âng-sik | âng-sek |  |
| `colorGreen` | 綠色 | li̍k-sik | le̍k-sek |  |
| `colorBlue` | 藍色 | nâ-sik | nâ-sek |  |