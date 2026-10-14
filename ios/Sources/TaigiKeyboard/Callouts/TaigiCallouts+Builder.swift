// TaigiCallouts+Builder.swift
// Entry point for long-press callout actions.
// Lookup order: layout-specific (TPS/MOE1/MOE2) → symbol → tone variations (POJ/TL).
// Maps are defined in TaigiCallouts+Maps.swift.

import Foundation
import KeyboardKit

public extension TaigiCallouts {
    /// Long-press callout builder: layout-specific → symbol → tone variations
    static let taigiCalloutActions: KeyboardCalloutActions.Builder = { params in
        guard case let .character(char) = params.action else {
            return KeyboardCalloutActions.english.actions(for: params.action)
        }
        let settings = SharedSettings.shared
        return calloutChars(
            for: char,
            inputMode: settings.inputMode,
            layoutType: settings.keyboardLayoutType,
        )?.map { .character($0) }
    }

    /// Callout characters for `char`, or nil for none.
    // CROSS-PLATFORM INVARIANT — English mode offers symbol callouts only, as
    // Android `LayoutManager.loadExtendedPopups` loads no Taigi popups there.
    internal static func calloutChars(
        for char: String,
        inputMode: InputMode,
        layoutType: KeyboardLayoutType,
    ) -> [String]? {
        if inputMode == .english {
            return SymbolCallouts.actions[char]
        }

        // Layout-specific callouts take priority over symbol/tone callouts
        switch layoutType {
        case .tps:
            if let chars = TPSCallouts.calloutChars(for: char) {
                return chars
            }
        case .moe1:
            if let actions = MOE1Callouts.actions[char] {
                return actions
            }
        case .moe2:
            if let actions = MOE2Callouts.actions[char] {
                return actions
            }
        default:
            break
        }

        // Symbol callouts: checked after layout-specific so TPS/MOE punctuation takes priority,
        // while symbol-only characters always get callouts regardless of layout
        if let actions = SymbolCallouts.actions[char] {
            return actions
        }

        let toneMap = inputMode == .poj
            ? TaigiToneMaps.poj
            : TaigiToneMaps.tl
        return toneMap[char]
    }
}
