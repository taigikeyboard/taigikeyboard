import Foundation
import KeyboardKit

/// Taigi-specific tone mapping for callout actions
/// Extends KeyboardKit's Callouts namespace following framework conventions
public extension Callouts {
    /// Provides tone variation mappings for POJ and TL input modes
    /// Maps base characters to their tone variations, sorted by tone number
    enum TaigiToneMaps {
        /// POJ mode: Base character to tone variations mapping
        /// Example: "a" -> ["á", "à", "â", "ǎ", "ā", "a̍", "ă"]
        static let poj: [String: [String]] = buildToneMap(
            toneToBase: ToneMappings.pojToneToBase,
            toneToNumber: ToneMappings.pojToneToNumber
        )

        /// TL mode: Base character to tone variations mapping
        /// Example: "a" -> ["á", "à", "â", "ǎ", "ā", "a̍", "a̋"]
        static let tl: [String: [String]] = buildToneMap(
            toneToBase: ToneMappings.tlToneToBase,
            toneToNumber: ToneMappings.tlToneToNumber
        )

        /// Builds a tone map from tone-to-base and tone-to-number mappings
        /// - Parameters:
        ///   - toneToBase: Maps tone characters to their base forms
        ///   - toneToNumber: Maps tone characters to their tone numbers
        /// - Returns: Dictionary mapping base characters to sorted tone variations
        private static func buildToneMap(
            toneToBase: [String: String],
            toneToNumber: [String: Int]
        ) -> [String: [String]] {
            var mapping: [String: [String]] = [:]

            // Group tone characters by their base character
            for (toneChar, baseChar) in toneToBase {
                if mapping[baseChar] == nil {
                    mapping[baseChar] = []
                }
                mapping[baseChar]?.append(toneChar)
            }

            // Add nasal marker ⁿ to 'n' key
            mapping["n"] = (mapping["n"] ?? []) + ["ⁿ"]

            // Sort tone variations by tone number
            for key in mapping.keys {
                mapping[key]?.sort { first, second in
                    let firstTone = toneToNumber[first] ?? 1
                    let secondTone = toneToNumber[second] ?? 1
                    return firstTone < secondTone
                }
            }

            return mapping
        }
    }
}
