// 載入共用 taigi-emojis dist/emoji.json,映射成 ISEmojiView 的分類模型(資料源替換,UI 不變)。

import CoreGraphics
import CoreText
import Foundation
import ISEmojiView

// Loads the bundled taigi-emojis emoji set and adapts it to ISEmojiView's [EmojiCategory] model.
enum TaigiEmojiData {
    // Slim Decodable shape — only the fields the emoji keyboard renders. emoji.json also carries
    // keywords / keywordsByLocale / cp / name / subgroup / version; ISEmojiView has no search, so
    // those are intentionally NOT decoded, keeping keyboard-extension memory lean.
    private struct Document: Decodable {
        let categories: [CategoryEntry]
    }

    private struct CategoryEntry: Decodable {
        let id: String
        let emoji: [EmojiEntry]
    }

    private struct EmojiEntry: Decodable {
        let base: String
        let variations: [String]
    }

    private static let resourceName = "emoji"
    private static let resourceExtension = "json"

    // ISEmojiView's category order + the taigi-emojis category ids that feed each. taigi-emojis
    // splits smileys_emotion + people_body; ISEmojiView merges both into .smileysAndPeople, so they
    // concatenate (smileys first). The remaining seven map 1:1. Order matches ISEmojiView's own
    // EmojiLoader.availableCategories so the category bar looks identical to the previous build.
    private static let categoryLayout: [(category: ISEmojiView.Category, sourceIDs: [String])] = [
        (.smileysAndPeople, ["smileys_emotion", "people_body"]),
        (.animalsAndNature, ["animals_nature"]),
        (.foodAndDrink, ["food_drink"]),
        (.activity, ["activities"]),
        (.travelAndPlaces, ["travel_places"]),
        (.objects, ["objects"]),
        (.symbols, ["symbols"]),
        (.flags, ["flags"]),
    ]

    /// Builds ISEmojiView categories from the bundled `emoji.json`, dropping any emoji the device
    /// font cannot render. `emoji.json` is the single source — a missing/unreadable resource is a
    /// packaging bug, so it asserts (loud in debug) and returns `[]`; NO plist fallback.
    static func loadISEmojiCategories() -> [EmojiCategory] {
        guard
            let url = Bundle.main.url(forResource: resourceName, withExtension: resourceExtension),
            let data = try? Data(contentsOf: url),
            let document = try? JSONDecoder().decode(Document.self, from: data)
        else {
            // 單一資料源 = emoji.json。缺失/壞檔 = 打包錯誤,直接 assert 把錯點炸出來,
            // 不做 plist fallback(USER:不要冗餘 fallback,否則看不到錯在哪)。
            assertionFailure("[EMOJI] emoji.json missing/unreadable — must be bundled in the keyboard-extension target")
            return []
        }

        let emojiBySourceID = Dictionary(
            document.categories.map { ($0.id, $0.emoji) },
            uniquingKeysWith: { first, _ in first },
        )

        return categoryLayout.compactMap { entry in
            let emojis = entry.sourceIDs
                .flatMap { emojiBySourceID[$0] ?? [] }
                .compactMap(makeRenderableEmoji)
            return emojis.isEmpty ? nil : EmojiCategory(category: entry.category, emojis: emojis)
        }
    }

    // Keep the base only when renderable; keep each skin-tone variation only when renderable.
    // Mirrors the Android loader's per-entry glyph gate (PaintCompat.hasGlyph).
    private static func makeRenderableEmoji(_ entry: EmojiEntry) -> Emoji? {
        guard isRenderable(entry.base) else { return nil }
        let renderableVariations = entry.variations.filter(isRenderable)
        return Emoji(emojis: [entry.base] + renderableVariations)
    }

    // Glyph availability: every UTF-16 unit of the cluster must map to a real glyph in the emoji
    // font. A brand-new emoji whose scalar the OS font lacks fails here, so it is dropped instead of
    // showing as tofu (replaces ISEmojiView's old per-iOS-version plist bucketing).
    private static let emojiFont = CTFontCreateWithName("AppleColorEmoji" as CFString, 0, nil)

    private static func isRenderable(_ string: String) -> Bool {
        let utf16 = Array(string.utf16)
        guard !utf16.isEmpty else { return false }
        var glyphs = [CGGlyph](repeating: 0, count: utf16.count)
        return CTFontGetGlyphsForCharacters(emojiFont, utf16, &glyphs, utf16.count)
    }
}
