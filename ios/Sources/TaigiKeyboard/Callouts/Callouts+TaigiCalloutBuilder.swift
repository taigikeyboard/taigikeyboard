// Callouts+TaigiCalloutBuilder.swift
// Entry point for long-press callout actions.
// Lookup order: layout-specific (TPS/MOE1/MOE2) → symbol → tone variations (POJ/TL).
// Maps are defined in Callouts+TaigiCalloutMaps.swift.

// 中文: 長按 callout 動作的進入點。
// 中文: 查詢順序:layout-specific(TPS / MOE1 / MOE2)→ Symbol → 調符變體(POJ / TL)。
// 中文: callout 資料表定義於 Callouts+TaigiCalloutMaps.swift。

import Foundation
import KeyboardKit

public extension Callouts {
    /// Long-press callout builder: layout-specific → symbol → tone variations
    // 中文: 長按 callout 的 ActionsBuilder — 依 layout-specific → symbol → 調符變體順序查表。
    static let taigiCalloutActions: ActionsBuilder = { params in
        guard case let .character(char) = params.action else {
            return Callouts.Actions.english.actions(for: params.action)
        }

        // Layout-specific callouts take priority over symbol/tone callouts
        let layoutType = SharedSettings.shared.keyboardLayoutType
        switch layoutType {
        case .tps:
            if let chars = TPSCallouts.calloutChars(for: char) {
                return chars.map { .character($0) }
            }
        case .moe1:
            if let actions = MOE1Callouts.actions[char] {
                return actions.map { .character($0) }
            }
        case .moe2:
            if let actions = MOE2Callouts.actions[char] {
                return actions.map { .character($0) }
            }
        default:
            break
        }

        // Symbol callouts: checked after layout-specific so TPS/MOE punctuation takes priority,
        // while symbol-only characters always get callouts regardless of layout
        if let actions = SymbolCallouts.actions[char] {
            return actions.map { .character($0) }
        }

        let toneMap = SharedSettings.shared.inputMode == .poj
            ? TaigiToneMaps.poj
            : TaigiToneMaps.tl

        return toneMap[char]?.map { .character($0) }
    }
}
