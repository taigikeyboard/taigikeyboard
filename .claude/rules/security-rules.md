# Security Rules (Cross-Platform)

Mandatory rules for security and privacy. Both platforms MUST follow these rules.
When adding or modifying code, check this guide first.

## Logging

### Release builds must have zero logs

All logging must be guarded so that **no log output appears in production/release builds**.

| Platform | Guard | Example |
|----------|-------|---------|
| iOS | `DebugLogger` wrapper | `logger.debug("[SEARCH] query='\(query)'")`|
| Android | `if (BuildConfig.DEBUG)` | `if (BuildConfig.DEBUG) Log.d(TAG, "[SEARCH] query='$input'")` |

### iOS specifics

- Use `DebugLogger(category:)` — wrapper around `os.Logger` that is a complete no-op in release builds
- No `#if DEBUG` needed at call sites — the wrapper handles it internally via `@autoclosure` (message string is never constructed in release)
- No `privacy: .public` needed at call sites — the wrapper applies it automatically
- Do NOT use `os.Logger` directly — always use `DebugLogger`

### Android specifics

- ProGuard (`proguard-rules.pro`) strips `Log.d/v/i/w/e` via `-assumenosideeffects` as a secondary defense
- Do NOT rely solely on ProGuard — always add `if (BuildConfig.DEBUG)` as the primary guard
- For multiple consecutive Log calls, group in a single `if (BuildConfig.DEBUG) { ... }` block
- Ensure `import com.siansiansu.taigikeyboard.BuildConfig` is present when using the guard

### What must never be logged (even in debug)

- Passwords, tokens, API keys
- Full user text input in aggregate (individual keystrokes for debugging are acceptable in debug)

## SQL

- All queries must use **parameterized binding** (`?` placeholders + `sqlite3_bind_*` / `rawQuery` with args)
- Never use string interpolation for values in SQL: `WHERE name = '\(column)'` is forbidden
- DDL statements (ALTER TABLE, CREATE INDEX) that require dynamic identifiers must validate against a **hardcoded whitelist** before interpolation
- iOS: Use `SQLiteConnectionManager.sqliteTransient` for all `sqlite3_bind_text` destructor parameters

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
