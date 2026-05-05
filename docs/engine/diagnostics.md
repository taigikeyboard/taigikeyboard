# Diagnostic Service

> **Type**: Feature
> **Keywords**: `Diagnostics`, `DeviceInfo`, `BugReport`
> **Related**: app-ui.md

---

## Summary

- Collects minimal device/app metadata for bug reporting
- User-initiated only — no automatic collection or transmission
- Three sharing methods: Copy, Share, Email

---

## Data Model

| Field | Type | Description |
|-------|------|-------------|
| `appVersion` | String | App version (e.g., "3.4.5") |
| `buildNumber` | Int | Build number |
| `osVersion` | String | OS version |
| `deviceModel` | String | Hardware model identifier |

**Formatted output**:
```
App: v3.4.5 (42)
OS: iOS 17.3.1
Device: iPhone14,5
```

---

## Collection Sources

| Field | iOS | Android |
|-------|-----|---------|
| appVersion | `CFBundleShortVersionString` | `PackageManager.versionName` |
| buildNumber | `CFBundleVersion` | `BuildConfig.VERSION_CODE` |
| osVersion | `UIDevice.current.systemVersion` | `Build.VERSION.RELEASE` + `SDK_INT` |
| deviceModel | `utsname()` syscall | `Build.MANUFACTURER` + `Build.MODEL` |

---

## Data Flow

```
User taps button (Copy/Share/Email)
  → DiagnosticService.gather()
  → DiagnosticInfo struct created
  → .formatted() → plain text
  → Platform sharing mechanism
    ├─ Copy: UIPasteboard / ClipboardManager
    ├─ Share: ShareLink / Intent.ACTION_SEND
    └─ Email: mailto URL → info@taigikeyboard.tw
```

---

## User Interface

Located in the **Settings** tab, "裝置資訊" section.

| Action | Label | Mechanism |
|--------|-------|-----------|
| Copy | "Khó͘-phih 裝置資訊" | Pasteboard/Clipboard, with visual feedback |
| Share | "分享裝置資訊" | Native share sheet |
| Email | "Email 回報問題" | Pre-filled mailto with subject "Taigi Keyboard Bug Report (v{version})" |

---

## Privacy

- **No data transmission** — user controls when and how to share
- **User-initiated only** — no background collection or analytics
- **Minimal scope** — only device/OS/app metadata; no input history, settings, dictionary, or user identification
- **No special permissions** required on either platform

---

## Platform Correspondence

| Function | iOS | Android |
|----------|-----|---------|
| Service | `DiagnosticService.swift` | `DiagnosticService.kt` |
| UI | `SettingsTab.swift` (lines 92-130) | `InputSettingsScreen.kt` (lines 204-257) |
| Strings | `SettingsTexts.swift` | `SettingsTexts.kt` |
| Pattern | Enum with static methods | Singleton object |
