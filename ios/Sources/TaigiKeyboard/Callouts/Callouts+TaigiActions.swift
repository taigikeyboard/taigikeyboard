import Foundation
import KeyboardKit

/// Taigi-specific callout actions
public extension Callouts {
    /// Long-press callout builder: layout-specific → symbol → tone variations
    static let taigiToneActions: ActionsBuilder = { params in
        guard case let .character(char) = params.action else {
            return Callouts.Actions.english.actions(for: params.action)
        }

        // Layout-specific callouts take priority over symbol/tone callouts
        let layoutType = SharedSettings.shared.keyboardLayoutType
        switch layoutType {
        case .tps:
            if let actions = TPSCallouts.actions[char] {
                return actions.map { .character($0) }
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
