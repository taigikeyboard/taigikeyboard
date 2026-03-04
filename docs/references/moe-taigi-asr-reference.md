# MOE Taigi IME - ASR Implementation Analysis
> **Type**: Reference
> **Keywords**: `MOE`, `ASR`, `VoiceInput`, `SpeechRecognition`

> 分析日期：2026-01-15
> APK 來源：Google Play (tw.moe.taiwanese.taigi)
> 反編譯工具：jadx 1.5.3

## 架構概覽

```
┌─────────────────┐     ┌──────────────────┐     ┌───────────────────┐
│  VoiceInputView │────▶│   VoiceViewModel  │────▶│  VoiceRepository  │
│  (UI Layer)     │     │   (P.N)           │     │  (D4.C0148a)      │
└─────────────────┘     └──────────────────┘     └─────────┬─────────┘
                                                           │
                         ┌─────────────────────────────────┼─────────────────────────────────┐
                         │                                 │                                 │
                         ▼                                 ▼                                 ▼
                ┌─────────────────┐             ┌─────────────────┐             ┌─────────────────┐
                │  REST API Client│             │   AudioRecorder │             │ WebSocket Client│
                │  (h.C1075d)     │             │   (D4.D)        │             │ (h.C1076e)      │
                └─────────────────┘             └─────────────────┘             └─────────────────┘
```

## 服務提供商

- **公司**：[長問科技 Bronci](https://www.bronci.com.tw/)
- **服務網址**：https://speech.bronci.com.tw/
- **服務類型**：雲端 ASR（非本地模型）

### 長問科技技術特點

| 項目 | 說明 |
|------|------|
| 支援語言 | 國語、台語、客語、英語 |
| 混合辨識 | 支援台文、台羅拼音、客語漢字與拼音輸出 |
| 部署方式 | 雲端 / 地端 |
| 專屬模型 | 可在一個月內建立特定領域模型，辨識率 > 90% |

### 訓練方法（推測）

長問科技未公開詳細訓練方法，根據業界常見做法推測：

1. **基礎架構**：可能基於 Transformer (Whisper、Conformer) 或 HuBERT
2. **語料來源**：台語廣播、Podcast、教育部台語辭典語音、眾包錄音
3. **Fine-tuning**：在大型中文/多語言模型上微調
4. **混合語言處理**：台語常混用華語，需特殊處理

---

## 1. REST API

### Base URL

```
https://iqt.bronci.com.tw
```

### 端點

| 端點 | 方法 | Header | Body | 說明 |
|------|------|--------|------|------|
| `/api/v1/app/users` | POST | `device-id: <id>` | - | 註冊設備，取得 UserCred |
| `/api/v1/app/login` | POST | - | `UserCred` | 登入取得 AuthToken |
| `/api/v1/app/models` | GET | `Authorization: <token>` | - | 取得可用 ASR 模型列表 |

### 資料模型

```kotlin
// 使用者憑證
data class UserCred(
    // 欄位未完全反編譯
)

// 登入回應
data class LoginResponse(
    // 欄位未完全反編譯
)

// 啟動資訊
data class Bootstrap(
    val authToken: String,
    val moduleList: List<ModelItem>
)

// ASR 模型項目
data class ModelItem(
    @SerializedName("name")
    val name: String,

    @SerializedName("version")
    val version: String?,

    @SerializedName("displayName")
    val displayName: String?,

    @SerializedName("isDefaultModel")
    val isDefaultModel: Int?,

    @SerializedName("description")
    val description: String?,

    @SerializedName("modelStatus")
    val modelStatus: String?,

    @SerializedName("customized")
    val customized: Boolean?
)
```

---

## 2. WebSocket 串流辨識

### 端點

```
wss://iqt.bronci.com.tw/ws/v1/app/transcript
```

### 連線參數 (Query String)

| 參數 | 值 | 說明 |
|------|-----|------|
| `token` | `<authToken>` | 認證 token |
| `type` | `raw` | 音訊格式 |
| `rate` | `16000` | 取樣率 16kHz |
| `channel` | `1` | 單聲道 |
| `noSpeechTimeout` | `5` | 靜音超時秒數 |
| `modelName` | `<name>` | ASR 模型名稱 |

### 連線範例

```
wss://iqt.bronci.com.tw/ws/v1/app/transcript?token=xxx&type=raw&rate=16000&channel=1&noSpeechTimeout=5&modelName=xxx
```

### WebSocket Headers

```http
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: <random-base64>
Sec-WebSocket-Version: 13
Sec-WebSocket-Extensions: permessage-deflate
```

### 訊息格式

- **發送**：Binary (PCM 16-bit raw audio)
- **接收**：Text (辨識結果，格式未完全分析)
- **結束訊號**：發送 `"EOS"` 字串

---

## 3. 音訊錄製參數

```kotlin
AudioRecord(
    audioSource = MediaRecorder.AudioSource.VOICE_RECOGNITION,  // 6
    sampleRateInHz = 16000,                                     // 16 kHz
    channelConfig = AudioFormat.CHANNEL_IN_MONO,                // 16
    audioFormat = AudioFormat.ENCODING_PCM_16BIT,               // 2
    bufferSize = max(AudioRecord.getMinBufferSize(...), 320)
)
```

### 音訊規格

| 參數 | 值 |
|------|-----|
| 取樣率 | 16000 Hz |
| 位元深度 | 16-bit |
| 聲道 | Mono (單聲道) |
| 編碼 | PCM (無壓縮) |
| 音源 | VOICE_RECOGNITION |

---

## 4. 網路庫

- **HTTP Client**: OkHttp 4.12.0
- **REST API**: Retrofit2 + Gson
- **WebSocket**: OkHttp WebSocket
- **Ping Interval**: 15 秒

---

## 5. 流程說明

```
1. 初始化
   └── POST /api/v1/app/users (device-id) → UserCred

2. 登入
   └── POST /api/v1/app/login (UserCred) → Bootstrap(authToken, modelList)

3. 取得模型列表
   └── GET /api/v1/app/models (Authorization) → List<ModelItem>

4. 開始語音輸入
   ├── 建立 WebSocket 連線 (帶 token, modelName)
   ├── 請求 Audio Focus
   └── 啟動 AudioRecord

5. 串流傳輸
   └── 持續將 PCM 音訊透過 WebSocket Binary Frame 發送

6. 接收辨識結果
   └── WebSocket 回傳辨識文字 (Text Frame)

7. 結束
   ├── 發送 "EOS" 訊號
   ├── 停止 AudioRecord
   ├── 釋放 Audio Focus
   └── 關閉 WebSocket
```

---

## 6. 核心類別對照表

| 混淆名稱 | 推測原名 | 檔案位置 | 用途 |
|----------|----------|----------|------|
| `h.C1075d` | VoiceApiClient | `sources/h/C1075d.java` | Retrofit REST API 客戶端 |
| `h.C1076e` | WebSocketClient | `sources/h/C1076e.java` | WebSocket 連線管理 |
| `h.InterfaceC1073b` | VoiceApiService | `sources/h/InterfaceC1073b.java` | Retrofit API 介面定義 |
| `D4.C0148a` | VoiceRepository | `sources/D4/C0148a.java` | 語音服務整合層 |
| `D4.D` | AudioRecordManager | `sources/D4/D.java` | 音訊錄製管理 |
| `P.N` | VoiceViewModel | `sources/P/N.java` | 語音輸入 ViewModel |
| `P.AbstractC0426m` | VoiceInputViewBase | `sources/P/AbstractC0426m.java` | 語音輸入 UI 基類 |
| `A0.H` | AudioFocusManager | - | Audio Focus 管理 |

---

## 7. UI 元件

### 軟鍵盤模式
- `VoiceInputViewSoft` - 軟體鍵盤的語音輸入 View
- Layout: `R.layout.voice_input_view`

### 硬體鍵盤模式
- `VoiceInputViewHard` - 硬體鍵盤的語音輸入 View

### 共用元件
- `MicWaveButton` - 麥克風波形動畫按鈕
- `VoicePermissionActivity` - 麥克風權限請求 Activity

---

## 8. 權限

```xml
<uses-permission android:name="android.permission.RECORD_AUDIO" />
<uses-permission android:name="android.permission.INTERNET" />
```

---

## 9. 檔案位置

反編譯後的原始碼位於：

```
references/moe_taigi_apk/
├── base.apk                    # 原始 APK
├── extracted/                  # 解壓後的資源
└── decompiled/
    ├── resources/              # 資源檔案
    └── sources/
        ├── android/moe/taiwanese/taigi/  # 主應用程式碼
        │   ├── ui/
        │   │   ├── ime/TaigiIME.java     # 輸入法主服務
        │   │   └── voice/                # 語音輸入 UI
        │   └── data/local/model/voice/   # 語音相關資料模型
        ├── D4/                           # 語音核心邏輯
        ├── h/                            # API 客戶端
        └── P/                            # ViewModel 層
```

---

## 10. 未來實作參考

### 法律注意事項

> **警告**：未經授權使用長問科技 API 屬於違法行為
> - 直接呼叫其 API 可能構成「未經授權存取電腦系統」
> - 這是商業服務，需簽約付費
> - 建議：聯繫長問科技詢問合作，或使用開源替代方案

---

## 11. On-Device ASR 方案

### 可行性評估

On-device ASR 可行，但台語模型選擇有限。

### 推薦方案

| 方案 | 台語支援 | 模型大小 | 平台 | 備註 |
|------|----------|----------|------|------|
| [Sherpa-ONNX](https://github.com/k2-fsa/sherpa-onnx) + Whisper | ⚠️ 需微調 | 39MB-1.5GB | iOS/Android | 離線、開源、支援多語言 |
| [Whisper.cpp](https://github.com/ggerganov/whisper.cpp) | ⚠️ 需微調 | 39MB-1.5GB | 多平台 | 輕量 C++ 實作 |
| [ChineseTaiwaneseWhisper](https://github.com/sandy1990418/ChineseTaiwaneseWhisper) | ✅ 專為台語 | 需訓練 | 需整合 | 專為台語優化的訓練框架 |
| [CLiFT-ASR](https://arxiv.org/html/2511.06860) | ✅ 專為台語 | 較小 | 研究中 | 2025 論文，跨語言微調框架 |

### Whisper 台語支援

Whisper 支援 `min_nan`（閩南語）語言代碼，但原生辨識率有限，建議微調：

```python
# Whisper 語言代碼
"min_nan"  # Min Nan (閩南語/台語)
```

### Sherpa-ONNX 特點

- 支援 iOS、Android、HarmonyOS、Raspberry Pi
- 完全離線，無需網路
- 支援 12 種程式語言 (C++, Python, Kotlin, Swift, etc.)
- 可整合 Whisper 模型
- 提供預編譯 APK 範例

---

## 12. 自架 ASR Server 資源需求

### 硬體需求

| 模型 | GPU | 記憶體 | 同時使用者 | 月成本估計 |
|------|-----|--------|------------|------------|
| Whisper tiny (39MB) | 不需 | 1GB | 5-10 | ~$20 (CPU VPS) |
| Whisper small (244MB) | 建議 | 2GB | 10-20 | ~$50-100 |
| Whisper medium (769MB) | 需要 | 5GB | 20-50 | ~$200-500 |
| Whisper large (1.5GB) | 必須 | 10GB+ | 50+ | ~$500+ |

### Serverless 方案（按使用量計費）

| 服務 | 計價 | 台語支援 |
|------|------|----------|
| Google Speech-to-Text | $0.006/15秒 | ❌ 無 |
| Azure Speech | $1/小時 | ❌ 無 |
| AWS Transcribe | $0.024/分鐘 | ❌ 無 |
| 長問科技 | 需洽詢 | ✅ 有 |

### 自架建議

若要自架台語 ASR 服務：

1. **開發階段**：使用 Whisper + Fine-tuning
2. **部署階段**：
   - 小規模：CPU VPS + Whisper tiny/small
   - 中規模：GPU Server + Whisper medium
   - 大規模：Kubernetes + 多 GPU 節點
3. **優化**：使用 Sherpa-ONNX 轉換模型，提升推論速度

---

## 13. 實作步驟建議

若要在本專案實作語音輸入功能：

### 方案 A：On-Device（離線）

```
1. 選擇模型
   └── Whisper small + 台語微調

2. 整合 Sherpa-ONNX
   ├── iOS: sherpa_onnx_ios (Swift)
   └── Android: sherpa-onnx-android (Kotlin)

3. 實作音訊錄製
   ├── iOS: AVAudioEngine
   └── Android: AudioRecord (16kHz, 16-bit, Mono)

4. 處理辨識結果
   └── 即時顯示並送入輸入框
```

### 方案 B：雲端服務

```
1. 選擇服務
   ├── 商業：長問科技（需洽詢）
   └── 自架：Whisper + WebSocket Server

2. 實作音訊串流
   └── WebSocket Binary Frame

3. 處理辨識結果
   └── WebSocket Text Frame → 輸入框
```

---

---

## 14. 現成台語 ASR 模型（可直接使用）

> 不需要自己訓練，開源社群已有現成模型

### Hugging Face 預訓練模型

| 模型 | 基礎 | 訓練資料 | 連結 |
|------|------|----------|------|
| whisper-small-chinese-tw-minnan-hanzi | Whisper Small | Common Voice 11.0 | [HuggingFace](https://huggingface.co/TSukiLen/whisper-small-chinese-tw-minnan-hanzi) |
| whisper-small-nan-tw | Whisper Small | Common Voice 15.0/16.1 | [HuggingFace](https://huggingface.co/linshoufan/linshoufanfork-whisper-small-nan-tw) |

### 快速測試

```python
from transformers import pipeline

pipe = pipeline(
    "automatic-speech-recognition",
    model="linshoufan/linshoufanfork-whisper-small-nan-tw"
)

result = pipe("台語音檔.wav")
print(result["text"])
```

### 注意事項

- 目前 CER（字元錯誤率）約 50%，品質中等
- 適合 MVP 測試或要求不高的場景
- 如需更高品質，可考慮微調或使用付費服務

---

## 15. 開源台語語料庫

### 可用於訓練/微調的資料集

| 語料庫 | 說明 | 規模 | 連結 |
|--------|------|------|------|
| **Mozilla Common Voice** | 群眾募集語音 | 持續增長 | [commonvoice.mozilla.org](https://commonvoice.mozilla.org/datasets) |
| **Taiwan Tongues** | 政府/民間合作 (2025) | 目標 1000 萬字 | [HuggingFace](https://huggingface.co/datasets/adi-gov-tw/Taiwan-Tongues-ASR-CE-dataset-hokkien) |
| **台語語料彙整** | GitHub 整理清單 | 多來源 | [Taiwanese-Corpus](https://github.com/Taiwanese-Corpus/hue7jip8) |

### Taiwan Tongues 計畫

- 2025 年由 IMA 資訊經理人協會推動
- 目標：建立台灣本土語言語料庫
- 已有超過 10 人授權台語文學作品，累積上百萬字
- 廖元甫教授展示 600 萬字台語語料訓練的 LLM 原型

---

## 16. 決策建議

### 選擇矩陣

| 你的情況 | 建議方案 | 理由 |
|----------|----------|------|
| 想快速驗證 | 直接用現成模型 | 零成本，1-2 天可測試 |
| 免費 App + 品質要求中等 | 現成模型 + Sherpa-ONNX | 離線、免費 |
| 免費 App + 品質要求高 | 微調模型 | 用 Common Voice 資料 |
| 付費 App | 長問科技 API | 品質保證、省開發時間 |
| 重視隱私 | On-Device | 音訊不上傳 |

### 建議路線

```
Phase 1: 驗證需求（1-2 週）
├── 下載 whisper-small-nan-tw
├── 本地測試辨識率
└── 收集用戶回饋

Phase 2: 決策（根據 Phase 1 結果）
├── 品質可接受 → 整合 Sherpa-ONNX 上線
├── 品質不足 → 用 Common Voice 微調
└── 時間緊迫 → 評估付費 API

Phase 3: 正式實作
├── On-Device: Sherpa-ONNX 整合
└── 雲端: WebSocket 串流實作
```

---

## 參考資源

### 服務商
- [長問科技 Bronci](https://www.bronci.com.tw/)
- [長問 Speech](https://speech.bronci.com.tw/)

### On-Device 框架
- [Sherpa-ONNX GitHub](https://github.com/k2-fsa/sherpa-onnx)
- [Sherpa-ONNX 文件](https://k2-fsa.github.io/sherpa/onnx/index.html)
- [Whisper.cpp](https://github.com/ggerganov/whisper.cpp)

### 預訓練模型
- [whisper-small-nan-tw](https://huggingface.co/linshoufan/linshoufanfork-whisper-small-nan-tw)
- [whisper-small-chinese-tw-minnan-hanzi](https://huggingface.co/TSukiLen/whisper-small-chinese-tw-minnan-hanzi)
- [OpenAI Whisper](https://github.com/openai/whisper)

### 訓練框架
- [ChineseTaiwaneseWhisper](https://github.com/sandy1990418/ChineseTaiwaneseWhisper)
- [CLiFT-ASR 論文](https://arxiv.org/html/2511.06860)

### 語料庫
- [Mozilla Common Voice](https://commonvoice.mozilla.org/datasets)
- [Taiwan Tongues Dataset](https://huggingface.co/datasets/adi-gov-tw/Taiwan-Tongues-ASR-CE-dataset-hokkien)
- [台語語料彙整](https://github.com/Taiwanese-Corpus/hue7jip8)
