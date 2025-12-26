# 說明文字與圖片修改指引

## 說明文字

### iOS
- **Tab1 頭頁文字**：`ios/Sources/TaigiKeyboard/Localization/Tab1Texts.swift`
- **Tab2 佈局文字**：`ios/Sources/TaigiKeyboard/Localization/Tab2Texts.swift`
- **Tab3 詞庫文字**：`ios/Sources/TaigiKeyboard/Localization/Tab3Texts.swift`
- **Tab4 設定文字**：`ios/Sources/TaigiKeyboard/Localization/Tab4Texts.swift`
- **版權聲明文字**：`ios/Sources/TaigiKeyboard/Model/CopyrightDataSource.swift`

### Android
- **Tab1 頭頁文字**：`android/app/src/main/java/com/siansiansu/taigikeyboard/localization/Tab1Texts.kt`
- **Tab2 佈局文字**：`android/app/src/main/java/com/siansiansu/taigikeyboard/localization/Tab2Texts.kt`
- **Tab3 詞庫文字**：`android/app/src/main/java/com/siansiansu/taigikeyboard/localization/Tab3Texts.kt`
- **Tab4 設定文字**：`android/app/src/main/java/com/siansiansu/taigikeyboard/localization/Tab4Texts.kt`
- **版權聲明文字**：`android/app/src/main/java/com/siansiansu/taigikeyboard/model/CopyrightDataSource.kt`
- **字串資源**：`android/app/src/main/res/values/strings.xml`

---

## 圖片資源

### iOS
- **啟用方法截圖**：`ios/Sources/TaigiKeyboard/App/Assets/Assets.xcassets/setup_step1.imageset/`、`setup_step2.imageset/`
- **佈局預覽圖**：`ios/Sources/TaigiKeyboard/App/Assets/Assets.xcassets/layout_*.imageset/`
- **功能說明圖**：`ios/Sources/TaigiKeyboard/App/Assets/Assets.xcassets/nextword_*.imageset/`
- **FAQ 截圖**：`ios/Sources/TaigiKeyboard/App/Assets/Assets.xcassets/faq_*.imageset/`

### Android
- **啟用方法截圖（Light）**：`android/app/src/main/res/drawable-xxxhdpi/setup_step1.png`、`setup_step2.png`
- **啟用方法截圖（Dark）**：`android/app/src/main/res/drawable-night-xxxhdpi/setup_step1.png`、`setup_step2.png`
- **佈局預覽圖**：`android/app/src/main/res/drawable-xxxhdpi/layout_*.png`
- **功能說明圖**：`android/app/src/main/res/drawable-xxxhdpi/nextword_*.png`
- **FAQ 截圖**：`android/app/src/main/res/drawable-xxxhdpi/faq_*.png`
- **圖示 (Icon)**：`android/app/src/main/res/drawable/*.xml`

---

## 注意事項

1. **文字修改**：iOS 和 Android 需同步修改，確保兩平台內容一致
2. **圖片尺寸**：
   - iOS：提供 1x、2x、3x 三種尺寸
   - Android：xxxhdpi (4x) 為主，寬度建議 1080px
3. **Dark Mode**：Android 需同時更新 `drawable-xxxhdpi` 和 `drawable-night-xxxhdpi`
