# Security Rules (Cross-Platform)

Project-specific security and privacy rules for every platform. Baseline practice — HTTPS only, least privilege, app-private storage, nothing world-readable — applies without being listed here.

## Logging — release builds emit nothing

| Platform | Guard | Example |
|----------|-------|---------|
| iOS / macOS | `DebugLogger` wrapper | `logger.debug("[SEARCH] query='\(query)'")`|
| Android | `LoggerBackend` (`AndroidLoggerBackend` gates every level on `BuildConfig.DEBUG`) | `logger.d(TAG, "[SEARCH] query='$input'")` |

- **iOS / macOS**: `DebugLogger(category:)` wraps `os.Logger` and is a complete no-op in release. Its `@autoclosure` message is never built there and it applies `privacy: .public` itself, so call sites need neither `#if DEBUG` nor a privacy annotation. Never call `os.Logger` directly; a shared-core candidate uses `LoggerFactory.make(category:)` (`docs/architecture/behavioral-invariants.md` §12), whose installed factory produces a `DebugLogger`.
- **Android**: log through the injected `LoggerBackend` (`ime/core/logging/`); only `AndroidLoggerBackend` imports `android.util.Log`. On hot paths check `logger.isDebugEnabled` (or use the inline `logger.debug(TAG) { ... }` extension) so the message string is never built in release. ProGuard (`proguard-rules.pro`) strips `Log.d/v/i/w/e` via `-assumenosideeffects` as a secondary defence only.
- **Never logged, even in debug**: passwords, tokens, API keys, and user text in aggregate (single keystrokes are acceptable in debug).

## SQL

- Every platform's user-data SQL is the engine's (`engine/userdata`); values are bound with `?` placeholders through rusqlite `params!`, never interpolated (`format!("… WHERE name = '{name}'")`).
- DDL that needs a dynamic identifier (ALTER TABLE, CREATE INDEX) validates it against a hardcoded whitelist before interpolation.

## Network and permissions

- No `NSAllowsArbitraryLoads` in `Info.plist` without per-domain exceptions; no `android:usesCleartextTraffic`.
- iOS `RequestsOpenAccess` (Full Access) is on only because the App Group container is read-only to the keyboard without it (`docs/contributing/ios-guidelines.md`); it is not a licence to add network access.
- Android requests no `INTERNET`, `READ_CONTACTS` or other dangerous permission without a stated reason.

## Data storage

- User frequency data, the custom dictionary and associations live in the iOS App Group shared container and in Android `context.getDatabasePath()` / `context.filesDir`.
- Backup files (`.taigi`) contain user data — treat them as sensitive.

## Exported components (Android)

- Every exported service declares `android:permission` (e.g. `BIND_INPUT_METHOD`).
- An Activity that needs no external access sets `android:exported="false"`.

## Pasteboard / clipboard

- Auto-clear the clipboard after copying sensitive data (e.g. diagnostic info).
- Never read clipboard contents without an explicit user action.
