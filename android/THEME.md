# Taigi Keyboard - SPY×FAMILY Theme Design

## 設計理念

採用《SPY×FAMILY》（間諜家家酒）的 1960 年代歐洲復古美學，營造溫暖優雅又帶有神秘感的氛圍。

### 核心原則
- **復古扁平設計**：無陰影、使用邊框取代 elevation
- **極簡化**：移除過度使用的 icon，減少 "AI 感"
- **溫暖質感**：米色背景、柔和配色、霧面質感
- **視覺階層**：通過色彩和排版建立清晰的功能優先級
- **僅支援 Light Mode**：App 強制使用淺色模式（鍵盤保留深色模式支援）

---

## 配色系統

### SPY×FAMILY 主題配色

#### Light Mode

**主要顏色：**
```xml
<!-- Primary: Loid's Teal Gray -->
<color name="modern_accent">#8da99b</color>

<!-- Secondary: Yor's Deep Red -->
<color name="modern_accent_secondary">#610a10</color>

<!-- Accent: Anya's Warm Pink -->
<color name="spyfamily_warm_pink">#fab3ad</color>
```

**背景與表面：**
```xml
<!-- 溫暖米色背景 -->
<color name="modern_surface_primary">#f5f0e8</color>

<!-- 卡片背景（溫暖白色） -->
<color name="modern_surface_card">#FBF9F5</color>
```

**文字顏色：**
```xml
<!-- 主要文字（深棕灰） -->
<color name="modern_text_primary">#2c2827</color>

<!-- 次要文字（深青灰） -->
<color name="modern_text_secondary">#57675c</color>
```

**邊框與裝飾：**
```xml
<!-- 卡片邊框 @ 20% opacity -->
<color name="spyfamily_card_stroke">#3357675c</color>
```

#### Dark Mode (已停用)

**App 已完全移除 Dark Mode 支援：**
- ✅ `SettingsTheme` 從 `Theme.Material3.DayNight` 改為 `Theme.Material3.Light`
- ✅ `values-night/themes.xml` 中的 SettingsTheme 已移除
- ✅ `values-night/colors.xml` 中的 app 配色已註解停用
- ✅ `SponsorshipActivity` 強制使用 Light Mode (`darkTheme = false`)
- ✅ **鍵盤仍保留 Dark Mode 支援**（KeyboardTheme 未變動）

**Dark Mode 配色（已註解，保留供未來參考）：**
```xml
<!-- 未啟用
<color name="modern_surface_primary">#2B3B54</color>
<color name="modern_accent">#a8c0b5</color>
<color name="modern_accent_secondary">#a8666b</color>
-->
```

---

## 卡片樣式

### ModernCard Style

**復古扁平設計：**
```xml
<style name="ModernCard" parent="Widget.Material3.CardView.Filled">
    <item name="cardCornerRadius">10dp</item>        <!-- 復古方正圓角 -->
    <item name="cardElevation">0dp</item>            <!-- 無陰影扁平設計 -->
    <item name="strokeWidth">1dp</item>              <!-- 復古邊框 -->
    <item name="strokeColor">@color/spyfamily_card_stroke</item>
    <item name="cardBackgroundColor">@color/modern_surface_card</item>
</style>
```

**設計要點：**
- **無陰影（elevation: 0dp）**：避免現代 Material Design 的浮動感
- **細邊框（1dp）**：復古文件檔案夾質感
- **方正圓角（10dp）**：比現代設計（16dp）更復古
- **米白色卡片**：非純白，更溫暖

---

## 頁面結構

### ContentFragment（主畫面）

**設計原則：**
- ✅ 移除 App Title，提供更簡潔的介面
- ✅ 頂部 padding 16dp，與卡片間距保持一致
- ✅ 卡片之間 margin-top 16dp（由 ModernCard style 控制）

**四卡片結構：**

#### 卡片 1 - Navigation（主要功能）
```
🟢 啟用方法 (青灰綠)
🔴 鍵盤設定 (深紅)
```

#### 卡片 2 - Sponsorship（贊助支持）
```
🔴 贊助支持 (深紅 - 強調重要性)
```

#### 卡片 3 - Resources（次要功能）
```
🟢 意見回饋 (青灰綠)
🌸 評分鼓勵 (粉色)
🟢 分享朋友 (青灰綠)
```

#### 卡片 4 - Copyright（版權聲明）
```
🌸 版權聲明 (粉色)
```

**色彩分布統計：**
- 青灰綠：3 個（43%）
- 深紅：2 個（29%）
- 粉色：2 個（29%）

---

## 視覺元素

### 左側色條設計

**規格：**
- 寬度：4dp
- 高度：match_parent
- 右側間距：20dp

**配色邏輯：**
- **青灰綠（#8da99b）**：主要功能、一般資源
- **深紅（#610a10）**：重要功能（鍵盤設定、贊助）
- **粉色（#fab3ad）**：溫暖點綴（評分、版權）

**XML 範例：**
```xml
<View
    android:layout_width="4dp"
    android:layout_height="match_parent"
    android:layout_marginEnd="20dp"
    android:background="@color/modern_accent" />
```

### 箭頭圖示

**淡化處理（減少視覺干擾）：**
```xml
<ImageView
    android:layout_width="20dp"
    android:layout_height="20dp"
    android:src="@drawable/ic_arrow_forward"
    android:tint="@color/modern_text_secondary"
    android:alpha="0.5" />
```

### 虛線分隔線

**復古虛線質感：**
```xml
<!-- drawable/retro_dotted_divider.xml -->
<shape xmlns:android="http://schemas.android.com/apk/res/android"
    android:shape="line">
    <stroke
        android:width="1dp"
        android:color="@color/modern_text_secondary"
        android:dashWidth="3dp"
        android:dashGap="3dp" />
</shape>
```

**Style：**
```xml
<style name="ModernDivider">
    <item name="android:layout_height">1dp</item>
    <item name="android:layout_marginStart">28dp</item>
    <item name="android:background">@drawable/retro_dotted_divider</item>
    <item name="android:alpha">0.3</item>
    <item name="android:layerType">software</item> <!-- 虛線渲染需要 -->
</style>
```

---

## 文字排版

### 標題

**主頁面已移除 App Title**
- ✅ 提供更簡潔的視覺體驗
- ✅ 減少視覺干擾，讓用戶專注於功能卡片

**其他頁面標題樣式（左對齊復古排版）：**
```xml
<TextView
    android:textSize="32sp"
    android:textStyle="bold"
    android:textColor="@color/modern_text_primary"
    android:gravity="start"
    android:letterSpacing="-0.02" />
```

### 列表項目文字

**統一規格（18sp bold）：**
```xml
<style name="ModernTitleText">
    <item name="android:textSize">18sp</item>
    <item name="android:textStyle">bold</item>
    <item name="android:textColor">@color/modern_text_primary</item>
</style>
```

**重要：**
- 所有列表項目使用相同字體大小（18sp）
- 不使用 16sp 或其他大小，避免視覺不一致

---

## 去除 "AI 感" 的設計決策

### 移除的元素

#### ✅ Icon Circle（圓形 icon 背景）
**移除前：**
```xml
<FrameLayout
    android:layout_width="44dp"
    android:layout_height="44dp"
    android:background="@drawable/icon_circle_background">
    <ImageView ... />
</FrameLayout>
```

**改為：左側色條**
```xml
<View
    android:layout_width="4dp"
    android:layout_height="match_parent"
    android:background="@color/modern_accent" />
```

#### ✅ 過多的 Icon
- Navigation 項目：移除所有左側 icon circles
- Resource 項目：移除所有左側 icon circles
- Copyright 頁面：移除 88dp 大型 icon circle
- Settings 頁面：移除 Input Mode 的 keyboard icon

#### ✅ 陰影效果
- 所有卡片：`cardElevation="2dp"` → `cardElevation="0dp"`
- Toolbar：保持 `elevation="0dp"`

#### ✅ 過度圓潤的圓角
- 卡片圓角：`16dp` → `10dp`

### 保留的 Icon（功能性需要）
- ✅ 右側箭頭（但淡化至 alpha 0.5）
- ✅ Settings 頁面的 toggle buttons icon
- ✅ Copyright 頁面的按鈕 icon

---

## 設計哲學

### 視覺階層

**主要功能 vs 次要功能：**
- 主要功能（Navigation）：使用多種顏色強調
- 次要功能（Resources）：使用統一粉色，較低調
- 重要強調（Sponsorship）：單獨卡片 + 深紅色

### 復古質感實現

**避免現代風格：**
- ❌ 大量陰影
- ❌ 浮動卡片效果
- ❌ 過度圓潤的圓角
- ❌ 統一的 icon circles
- ❌ 完美對稱的排版

**改用復古元素：**
- ✅ 扁平設計 + 細邊框
- ✅ 左側色條（檔案夾風格）
- ✅ 虛線分隔線
- ✅ 左對齊標題（非置中）
- ✅ 不對稱的色彩分布

### SPY×FAMILY 主題呼應

**色彩象徵：**
- **青灰綠（#8da99b）**：Loid 的冷靜專業
- **深紅（#610a10）**：Yor 的力量與重要性
- **粉色（#fab3ad）**：Anya 的溫暖可愛
- **米色背景（#f5f0e8）**：Forger 家的溫馨日常
- **深藍灰（Dark Mode）**：間諜任務的神秘氛圍

---

## Jetpack Compose 復古主題

### SponsorshipActivity 主題系統

**Color Scheme（Light Mode）：**
```kotlin
lightColorScheme(
    primary = Color(0xFF8da99b),           // Loid's Teal Gray Green
    secondary = Color(0xFF610a10),         // Yor's Deep Red
    tertiary = Color(0xFFfab3ad),          // Anya's Warm Pink
    background = Color(0xFFf5f0e8),        // 溫暖米色背景
    surface = Color(0xFFFBF9F5),           // 卡片背景（溫暖白色）
    surfaceVariant = Color(0xFFf0ebe3),    // 稍深的米色
    onSurface = Color(0xFF2c2827),         // 主要文字（深棕灰）
    onSurfaceVariant = Color(0xFF57675c)   // 次要文字（深青灰）
)
```

**Color Scheme（Dark Mode - 未啟用）：**
```kotlin
darkColorScheme(
    primary = Color(0xFFa8c0b5),           // 淺化青灰綠
    secondary = Color(0xFFa8666b),         // 淺化深紅
    tertiary = Color(0xFFfab3ad),          // Anya's Warm Pink
    background = Color(0xFF2B3B54),        // 深藍灰（間諜場景）
    surface = Color(0xFF3C4C64),           // 深青藍灰
)
```

### Retro Card Style (Compose)

**復古卡片設計：**
```kotlin
Card(
    shape = RoundedCornerShape(10.dp),  // 復古方正圓角
    elevation = CardDefaults.cardElevation(defaultElevation = 0.dp),  // 無陰影扁平設計
    border = BorderStroke(1.dp, MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.2f))  // 復古邊框
)
```

**設計原則：**
- ✅ 移除漸層背景，使用純色米色背景
- ✅ 移除所有 icon circles 的背景和陰影
- ✅ 所有卡片使用 10dp 圓角（比現代 16dp 更復古）
- ✅ 所有卡片 elevation 設為 0dp（扁平設計）
- ✅ 所有卡片添加 1dp 邊框（復古質感）
- ✅ Icons 直接顯示，不使用圓形背景容器
- ✅ 使用 SPY×FAMILY 配色取代硬編碼顏色

### Sponsor Tier Colors

**贊助層級配色：**
- **Coffee Tier**: #8da99b (Loid's Teal Gray Green)
- **Meal Tier**: #610a10 (Yor's Deep Red)
- **Premium Tier**: #fab3ad (Anya's Warm Pink)

---

## 檔案清單

### 已修改的檔案

**配色：**
- `app/src/main/res/values/colors.xml`
- `app/src/main/res/values-night/colors.xml` ✨（App 配色已停用，保留鍵盤配色）

**主題樣式：**
- `app/src/main/res/values/themes.xml` ✨（SettingsTheme 改為 Light Mode）
- `app/src/main/res/values-night/themes.xml` ✨（移除 SettingsTheme，保留 KeyboardTheme）
- `app/src/main/res/values/styles.xml`

**主畫面 Layout：**
- `app/src/main/res/layout/fragment_content.xml` ✨（移除 App Title）
- `app/src/main/res/layout/home_navigation_item_setup.xml`
- `app/src/main/res/layout/home_navigation_item_settings.xml`
- `app/src/main/res/layout/home_navigation_item_copyright.xml`
- `app/src/main/res/layout/home_resource_item_sponsorship.xml`
- `app/src/main/res/layout/home_resource_item_contact.xml`
- `app/src/main/res/layout/home_resource_item_rate.xml`
- `app/src/main/res/layout/home_resource_item_share.xml`

**子畫面 Layout：**
- `app/src/main/res/layout/activity_keyboard_settings.xml`
- `app/src/main/res/layout/activity_copyright.xml`
- `app/src/main/res/layout/copyright_page_item.xml`
- `app/src/main/res/layout/activity_contact.xml`
- `app/src/main/res/layout/settings_activity.xml`

**Onboarding 頁面 Layout：**
- `app/src/main/res/layout/activity_onboarding.xml`
- `app/src/main/res/layout/fragment_welcome.xml`
- `app/src/main/res/layout/fragment_setup.xml` ✨
- `app/src/main/res/layout/fragment_completed.xml`

**Drawable 資源：**
- `app/src/main/res/drawable/icon_circle_background.xml`
- `app/src/main/res/drawable/icon_circle_blue.xml`
- `app/src/main/res/drawable/icon_circle_red.xml`
- `app/src/main/res/drawable/icon_circle_yellow.xml`
- `app/src/main/res/drawable/icon_circle_green.xml`
- `app/src/main/res/drawable/footer_decoration_line.xml`
- `app/src/main/res/drawable/retro_dotted_divider.xml`（新建）

**Kotlin 代碼：**
- `app/src/main/java/com/siansiansu/taigikeyboard/settings/CopyrightPagerAdapter.kt`
- `app/src/main/java/com/siansiansu/taigikeyboard/settings/ContentFragment.kt` ✨（移除 headerTitle 引用）

**Jetpack Compose (Sponsorship 頁面)：**
- `app/src/main/java/com/siansiansu/taigikeyboard/sponsorship/SponsorshipActivity.kt` ✨（強制 Light Mode）
- `app/src/main/java/com/siansiansu/taigikeyboard/sponsorship/SponsorshipScreen.kt` ✨

---

## 未來改進方向

### 可選的進階優化

**紙張質感：**
- 加入微妙的噪點紋理（noise texture）
- 或使用極淺的漸層模擬紙張

**不對稱元素：**
- Footer 裝飾線偏左或偏右
- 使用不同大小的色塊裝飾

**手繪風格裝飾：**
- 手繪風格的下劃線
- 紙張摺角效果

**復古字體：**
- 考慮使用更具復古感的字體（需評估可讀性）

---

## 參考

**設計靈感來源：**
- 1960 年代歐洲文件檔案夾
- 間諜情報文件美學
- SPY×FAMILY 動畫的視覺風格
- 復古打字機排版

**配色參考：**
```
#8da99b  (141,169,155)  - 青灰綠
#fab3ad  (250,179,173)  - 淺粉紅
#2c2827  (44,40,39)     - 深棕灰
#610a10  (97,10,16)     - 深紅
#57675c  (87,103,92)    - 深青灰
```

---

**最後更新：** 2025-10-19
**設計版本：** 1.3 - SPY×FAMILY Retro Theme (Minimalist)

---

## 更新歷史

### v1.3 - 2025-10-19
- ✅ 移除主頁面 App Title，提供更簡潔的介面
- ✅ 調整頂部間距為 16dp（從 24dp）
- ✅ 修正 ContentFragment.kt 移除 headerTitle 引用

### v1.2 - 2025-10-19
- ✅ 移除 App 的 Dark Mode 支援，強制使用 Light Mode
- ✅ 保留鍵盤的 Dark Mode 功能
- ✅ SettingsTheme 改用 `Theme.Material3.Light.NoActionBar`
- ✅ SponsorshipActivity 強制使用 Light Mode
- ✅ 停用 values-night 中的 app 配色（保留鍵盤配色）

### v1.1 - 2025-10-19
- ✅ 將 SPY×FAMILY 復古主題應用到所有子頁面
- ✅ 更新 Onboarding、Contact、Sponsorship 頁面
- ✅ Jetpack Compose 主題系統完整實現

### v1.0 - 2025-01-XX
- ✅ 初始 SPY×FAMILY 復古主題設計
- ✅ 主畫面 ContentFragment 重新設計
- ✅ Settings 和 Copyright 頁面樣式更新
