import Foundation

/// Helper for determining confirm key text based on settings
struct ConfirmKeyTextHelper {

    /// Get the appropriate text for the confirm/return key
    /// - Returns: The text to display on the confirm key
    static func getConfirmKeyText() -> String {
        return Tab4Texts.confirmKey.hanji
    }
}
