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

// MARK: - Service Implementation

/// Taigi callout service for KeyboardKit integration
/// Provides tone variation callouts for POJ and TL input modes
class TaigiCalloutService: CalloutService {
    static let shared = TaigiCalloutService()

    private init() {}

    /// Returns callout actions for the given keyboard action
    /// - Parameter action: The keyboard action to get callouts for
    /// - Returns: Array of keyboard actions to show in callout
    func calloutActions(for action: KeyboardAction) -> [KeyboardAction] {
        Callouts.taigiToneActions(.init(action: action)) ?? []
    }

    /// Handles feedback when callout selection changes
    /// Protocol compliance - delegates to KeyboardKit for system haptic settings
    func triggerFeedbackForSelectionChange() {
        // Let KeyboardKit handle feedback according to system settings
    }
}

// MARK: - Backward Compatibility

/// Legacy name for TaigiCalloutService
/// Maintained for backward compatibility with existing code
typealias CustomCalloutService = TaigiCalloutService

/// Legacy builder for backward compatibility
/// Use Callouts.taigiToneActions directly in new code
enum CustomCalloutActions {
    /// Direct builder function - delegates to Callouts.taigiToneActions
    static let directBuilder: Callouts.ActionsBuilder = Callouts.taigiToneActions
}
