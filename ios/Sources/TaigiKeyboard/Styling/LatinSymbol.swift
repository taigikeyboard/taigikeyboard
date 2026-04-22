import SwiftUI
import UIKit

extension Image {
    /// SF Symbol rendered with the Latin locale regardless of the device language.
    ///
    /// `Image(systemName:)` follows the device locale and can produce CJK,
    /// Arabic, Hebrew, or Thai glyph variants for locale-sensitive symbols
    /// (`number`, quote bubbles, certain `character.*` symbols). This
    /// initializer injects `UIImage.SymbolConfiguration(locale:)` with
    /// `Locale(identifier: "en")` so the rendered glyph stays Latin.
    ///
    /// Accessibility caveat: `Image(uiImage:)` does not carry the accessibility
    /// label that `Image(systemName:)` auto-derives from the symbol name. When
    /// the symbol is the only content of a control, add an explicit
    /// `.accessibilityLabel(...)` at the call site.
    ///
    /// Rendering mode: the UIImage is forced to `.alwaysTemplate` so
    /// `.foregroundColor` / `.foregroundStyle` tint the glyph as callers expect
    /// from `Image(systemName:)`. This helper does not support palette,
    /// hierarchical, or multicolor rendering — use `Image(systemName:)`
    /// directly if a future call site needs those.
    init(latinSystemName name: String) {
        if let uiImage = UIImage(systemName: name, withConfiguration: Self.latinLocaleConfig) {
            self.init(uiImage: uiImage.withRenderingMode(.alwaysTemplate))
        } else {
            assertionFailure("Invalid SF Symbol name: \(name)")
            self.init(systemName: name)
        }
    }

    private static let latinLocaleConfig = UIImage.SymbolConfiguration(
        locale: Locale(identifier: "en"),
    )
}
