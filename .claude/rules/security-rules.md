---
paths: ["ios/**", "android/**", "macos/**", "windows/**", "linux/**", "desktop/**", "engine/**"]
---

# Security Rules (Cross-Platform)

Security and privacy rules for every platform.
When adding or modifying code, check this guide first.

## Logging

### Release builds must have zero logs

All logging must be guarded so that **no log output appears in production/release builds**.

| Platform | Guard | Example |
|----------|-------|---------|
| iOS / macOS | `DebugLogger` wrapper | `logger.debug("[SEARCH] query='\(query)'")`|
| Android | `LoggerBackend` (`AndroidLoggerBackend` gates every level on `BuildConfig.DEBUG`) | `logger.d(TAG, "[SEARCH] query='$input'")` |

### iOS specifics

- Use `DebugLogger(category:)` — wrapper around `os.Logger` that is a complete no-op in release builds
- No `#if DEBUG` needed at call sites — the wrapper handles it internally via `@autoclosure` (message string is never constructed in release)
- No `privacy: .public` needed at call sites — the wrapper applies it automatically
- Do NOT use `os.Logger` directly — always use `DebugLogger`

### Android specifics

- Log through the injected `LoggerBackend` (`ime/core/logging/`); only `AndroidLoggerBackend` imports `android.util.Log`, and it gates all levels on `BuildConfig.DEBUG`
- Hot paths: check `logger.isDebugEnabled` (or the inline `logger.debug(TAG) { ... }` extension) so the message string is never built in release
- ProGuard (`proguard-rules.pro`) strips `Log.d/v/i/w/e` via `-assumenosideeffects` as a secondary defense only

### What must never be logged (even in debug)

- Passwords, tokens, API keys
- Full user text input in aggregate (individual keystrokes for debugging are acceptable in debug)

## SQL

- All queries must use **parameterized binding** (`?` placeholders + `sqlite3_bind_*` / `rawQuery` with args)
- Never use string interpolation for values in SQL: `WHERE name = '\(column)'` is forbidden
- DDL statements (ALTER TABLE, CREATE INDEX) that require dynamic identifiers must validate against a **hardcoded whitelist** before interpolation
- Every platform's user-data SQL is the engine's (`engine/userdata`, rusqlite `params!` binding)

## Network

- All URLs must use HTTPS where the server supports it
- No `NSAllowsArbitraryLoads` in Info.plist unless strictly necessary with per-domain exceptions
- Android: Do not enable `android:usesCleartextTraffic`

## Data Storage

- User frequency data, custom dictionary, and associations: store in app-private directories only
  - iOS: App Group shared container
  - Android: `context.getDatabasePath()` / `context.filesDir`
- No `MODE_WORLD_READABLE` / `MODE_WORLD_WRITABLE` (Android)
- Consider `FileProtection` attributes on sensitive database files (iOS)
- Backup files (`.taigi`) contain user data — document as sensitive

## Permissions

- Request only the minimum permissions required
- iOS: Only enable `RequestsOpenAccess` if the keyboard extension genuinely needs network access
- Android: Do not request `INTERNET`, `READ_CONTACTS`, or other dangerous permissions unless justified

## Exported Components (Android)

- All exported services must declare `android:permission` (e.g., `BIND_INPUT_METHOD`)
- Activities that don't need external access must set `android:exported="false"`

## Pasteboard / Clipboard

- Auto-clear clipboard after copying sensitive data (e.g., diagnostic info)
- Do not read clipboard contents without explicit user action
