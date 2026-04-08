import Foundation
import KeyboardKit

extension ActionHandler {
    func handleCustomAction(_ name: String) {
        switch name {
        case "translate":
            handleTranslateToggle()
        default:
            break
        }
    }

    /// Toggle romanization / Hanji (漢字) display mode
    func handleTranslateToggle() {
        keyboardContext.toggleTranslateSwapped()
        if feedbackContext.settings.isHapticFeedbackEnabled {
            triggerHapticFeedback(.lightImpact)
        }
    }

    /// Open main app settings page
    func openMainAppSettings() {
        guard let url = URL(string: "taigikeyboard://"),
              let controller = keyboardController else { return }
        controller.openUrl(url)
    }
}
