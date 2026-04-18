import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Input mode selected by the user: POJ (Pe̍h-ōe-jī), TL (Tâi-lô),
/// English passthrough, or TPS (Taiwanese Phonetic Symbols).
///
/// The enum is a pure value type over `String` so it can live in shared core;
/// the `displayName` localization sibling stays platform-side and is provided
/// via an extension in `SettingsModels.swift`.
enum InputMode: String, CaseIterable {
    case poj // Pe̍h-ōe-jī
    case tl // Tâi-lô
    case english
    case tps // Taiwanese Phonetic Symbols
}
