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

Located in the **Settings** tab, "Device Info" section.

| Action | Label | Mechanism |
|--------|-------|-----------|
| Copy | "Copy Device Info" | Pasteboard/Clipboard, with visual feedback |
| Share | "Share Device Info" | Native share sheet |
| Email | "Email a Bug Report" | Pre-filled mailto with subject "TaigiKeyboard Bug Report (v{version})" |

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
| Strings | `StringKey.settings*` (i18n resolver) | `L10n` (generated — settings namespace migrated to i18n codegen) |
| Pattern | Enum with static methods | Singleton object |
