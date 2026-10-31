// TaigiCallouts+Builder.swift
// Entry point for long-press callout actions.
// Lookup order: the active custom layout's own list (Taigi modes) → layout-specific (TPS/MOE1/MOE2)
// → symbol → tone variations (POJ/TL).
// Maps are defined in TaigiCallouts+Maps.swift.

import Foundation
import KeyboardKit

public extension TaigiCallouts {
    /// Long-press callout builder: custom layout list → layout-specific → symbol → tone variations
    static let taigiCalloutActions: KeyboardCalloutActions.Builder = { params in
        guard case let .character(char) = params.action else {
            return KeyboardCalloutActions.english.actions(for: params.action)
        }
        let settings = SharedSettings.shared
        if let entries = customCalloutChars(
            for: char,
            inputMode: settings.inputMode,
            keyboardType: params.context.keyboardType,
            customLayout: settings.activeCustomLayout,
        ) {
            return entries.map { .character($0) }
        }
        return calloutChars(
            for: char,
            inputMode: settings.inputMode,
            layoutType: settings.keyboardLayoutType,
        )?.map { .character($0) }
    }

    /// The active custom layout's own long-press list for `char`, which replaces the whole built-in
    /// list; nil when the key keeps the built-in one. Letter page of a Taigi mode only: English and the
    /// symbol pages keep their own tables, as on Android (`LayoutManager.computeLayoutFor`).
    internal static func customCalloutChars(
        for char: String,
        inputMode: InputMode,
        keyboardType: Keyboard.KeyboardType,
        customLayout: CustomLayout?,
    ) -> [String]? {
        guard inputMode != .english, keyboardType == .alphabetic else { return nil }
        return customLayout?.calloutOverride(for: char)
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
