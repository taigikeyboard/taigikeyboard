import Foundation
import KeyboardKit

/// Taigi-specific callout actions
/// Extends KeyboardKit's Callouts namespace following framework conventions
public extension Callouts {
    /// Builder function for Taigi tone callout actions
    /// Provides tone variations based on current input mode (POJ/TL)
    ///
    /// Usage:
    /// ```swift
    /// .keyboardCalloutActions(Callouts.taigiToneActions)
    /// ```
    static let taigiToneActions: ActionsBuilder = { params in
        guard case let .character(char) = params.action else {
            // Non-character actions: use English standard callouts
            return Callouts.Actions.english.actions(for: params.action)
        }

        // Get tone map based on current input mode
        let toneMap = SharedSettings.shared.inputMode == .poj
            ? TaigiToneMaps.poj
            : TaigiToneMaps.tl

        // Return tone variations if available
        return toneMap[char]?.map { .character($0) }
    }
}
