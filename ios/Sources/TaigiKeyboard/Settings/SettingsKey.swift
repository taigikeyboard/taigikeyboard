// 中文: SharedSettings 與 UserDefaults 之間的 typed-key 描述符。每個鍵 = key string + 預設值 + 編解碼器。
// 中文: 公開 SettingsKey<T> 給設定 facade,以及 UserDefaults 的 typed read/write/remove 擴充。

import CoreGraphics
import Foundation

/// Typed descriptor for a single persisted `UserDefaults` key, used by
/// `SharedSettings` to centralize the read / default-fallback / write
/// boilerplate. Each descriptor carries:
///
/// - `key` — the raw `UserDefaults` key string (frozen for binary compat).
/// - `defaultValue` — the value returned when the key is absent or its
///   stored representation cannot be decoded back into `T`.
/// - `read` — pulls `T?` from the supplied `UserDefaults`; `nil` means
///   "absent or malformed" and the typed accessor returns `defaultValue`.
/// - `write` — encodes `T` into `UserDefaults`.
///
/// Construct descriptors via the typed factories (`.bool(_:default:)`,
/// `.rawRep(_:default:)`, `.double(_:default:)`, `.cgFloat(_:default:)`,
/// `.codable(_:default:)`) so each codec lives in one place.
// 中文: 持久化單一 UserDefaults 鍵的型別描述符。key 字串、預設值、讀寫 closure 全封裝。
// 中文: 一律走 .bool / .rawRep / .double / .cgFloat / .codable factory 建構,讓每種編碼一處到位。
struct SettingsKey<T> {
    let key: String
    let defaultValue: T
    let read: (UserDefaults) -> T?
    let write: (UserDefaults, T) -> Void

    /// Intentionally `fileprivate`: forces all construction through the
    /// typed factories below so each codec (Bool / Double / CGFloat /
    /// rawRep / codable) lives in exactly one place.
    fileprivate init(
        key: String,
        defaultValue: T,
        read: @escaping (UserDefaults) -> T?,
        write: @escaping (UserDefaults, T) -> Void,
    ) {
        self.key = key
        self.defaultValue = defaultValue
        self.read = read
        self.write = write
    }
}

extension SettingsKey where T == Bool {
    /// `UserDefaults`-backed `Bool` with "missing-or-non-Bool → default"
    /// semantics. Uses `object(forKey:) as? Bool` rather than
    /// `bool(forKey:)` so an unset key returns `defaultValue` instead
    /// of `false`.
    static func bool(_ key: String, default defaultValue: Bool) -> Self {
        Self(
            key: key,
            defaultValue: defaultValue,
            read: { $0.object(forKey: key) as? Bool },
            write: { $0.set($1, forKey: key) },
        )
    }
}

extension SettingsKey where T == Int {
    /// `UserDefaults`-backed `Int` with "missing-or-non-Int → default"
    /// semantics. Uses `object(forKey:) as? Int` so an unset key returns
    /// `defaultValue` instead of `0`.
    static func int(_ key: String, default defaultValue: Int) -> Self {
        Self(
            key: key,
            defaultValue: defaultValue,
            read: { $0.object(forKey: key) as? Int },
            write: { $0.set($1, forKey: key) },
        )
    }
}

extension SettingsKey where T == String {
    /// `UserDefaults`-backed `String` with "missing-or-non-String → default"
    /// semantics.
    static func string(_ key: String, default defaultValue: String) -> Self {
        Self(
            key: key,
            defaultValue: defaultValue,
            read: { $0.string(forKey: key) },
            write: { $0.set($1, forKey: key) },
        )
    }
}

extension SettingsKey where T == Double {
    /// `UserDefaults`-backed `Double` with "missing-or-non-Double → default"
    /// semantics.
    static func double(_ key: String, default defaultValue: Double) -> Self {
        Self(
            key: key,
            defaultValue: defaultValue,
            read: { $0.object(forKey: key) as? Double },
            write: { $0.set($1, forKey: key) },
        )
    }
}

extension SettingsKey where T == CGFloat {
    /// Stored on disk as `Double`; the public property exposes `CGFloat`.
    /// "missing-or-non-Double → default" semantics.
    static func cgFloat(_ key: String, default defaultValue: CGFloat) -> Self {
        Self(
            key: key,
            defaultValue: defaultValue,
            read: { ($0.object(forKey: key) as? Double).map { CGFloat($0) } },
            write: { $0.set(Double($1), forKey: key) },
        )
    }
}

extension SettingsKey {
    /// `String`-backed `RawRepresentable` enums (e.g. `InputMode`,
    /// `FontType`, `KeyboardLayoutType`). Malformed or unknown stored
    /// raw strings fall back to the descriptor's default.
    static func rawRep<R>(_ key: String, default defaultValue: R) -> SettingsKey<R>
        where R: RawRepresentable, R.RawValue == String
    {
        SettingsKey<R>(
            key: key,
            defaultValue: defaultValue,
            read: { defaults in
                guard let raw = defaults.string(forKey: key) else { return nil }
                return R(rawValue: raw)
            },
            write: { $0.set($1.rawValue, forKey: key) },
        )
    }

    /// JSON-encoded `Codable` values (e.g. `KeyboardColorSettings`).
    /// Decode failures (missing key, corrupted blob, schema drift) fall
    /// back to the descriptor's default; encode failures silently no-op,
    /// matching the pre-wrapper behaviour for the colour settings field.
    /// TODO(B8a-followup): route encode failure through the iOS logger so
    /// a future Codable conformance with a throwing `encode(to:)` does
    /// not silently lose user data.
    static func codable<V>(_ key: String, default defaultValue: V) -> SettingsKey<V>
        where V: Codable
    {
        SettingsKey<V>(
            key: key,
            defaultValue: defaultValue,
            read: { defaults in
                guard let data = defaults.data(forKey: key) else { return nil }
                return try? JSONDecoder().decode(V.self, from: data)
            },
            write: { defaults, value in
                if let data = try? JSONEncoder().encode(value) {
                    defaults.set(data, forKey: key)
                }
            },
        )
    }
}

extension UserDefaults {
    /// Reads through a `SettingsKey<T>`; returns `defaultValue` when the
    /// key is absent or its stored representation cannot be decoded.
    func value<T>(for key: SettingsKey<T>) -> T {
        key.read(self) ?? key.defaultValue
    }

    /// Writes through a `SettingsKey<T>`; identical to the pre-wrapper
    /// codec path for each typed factory.
    func set<T>(_ value: T, for key: SettingsKey<T>) {
        key.write(self, value)
    }

    /// Removes the stored value for a `SettingsKey<T>`, restoring the
    /// "absent → defaultValue" behaviour on subsequent reads. Used by
    /// `SharedSettings.resetToDefaults()` for descriptors whose default
    /// is computed (e.g. the device-aware globe key).
    func remove<T>(_ key: SettingsKey<T>) {
        removeObject(forKey: key.key)
    }

    /// Reads the raw stored object for a `SettingsKey<T>` without
    /// applying the descriptor's codec. Used when a caller needs to
    /// distinguish "absent" from "stored default value" (e.g. the
    /// device-aware globe-key getter, which falls back to a computed
    /// default only when nothing has been written yet).
    // 中文: 跳過 descriptor codec,直接拿 raw Any?。供 device-aware 預設值場景 (例:地球鍵) 判別「沒寫過」與「寫過跟預設一樣」。
    func storedObject<T>(for key: SettingsKey<T>) -> Any? {
        object(forKey: key.key)
    }
}
