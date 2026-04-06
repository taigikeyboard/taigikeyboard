import Foundation

// MARK: - Tab1 content models (parsed from tab1-features.json / tab1-faq.json)

/// Root container for the features JSON file.
struct FeaturesFile: Codable {
    let features: [FeatureContent]
}

/// Root container for the FAQ JSON file.
struct FAQsFile: Codable {
    let faqs: [FeatureContent]
}

/// A single feature description entry.
struct FeatureContent: Codable, Identifiable {
    let id: String
    let title: LocalizedTextData
    let icon: PlatformIcon
    let summary: LocalizedTextData?
    let paragraphs: [FeatureParagraph]
}

/// A paragraph with optional media attachment.
struct FeatureParagraph: Codable {
    let text: LocalizedTextData
    let attachment: ParagraphAttachment?
}

/// Codable wrapper for localized text (mirrors LocalizedText).
struct LocalizedTextData: Codable {
    let hanji: String

    var asLocalizedText: LocalizedText {
        LocalizedText(hanji: hanji)
    }
}

/// Platform-specific icon names.
struct PlatformIcon: Codable {
    let ios: String
    let android: String
}

/// Media attachment on a paragraph.
enum ParagraphAttachment: Codable {
    case slideshow(images: [String], interval: Double)
    case image(name: String)
    case link(text: LocalizedTextData, url: String)
    case navigation(text: LocalizedTextData, destination: String, icon: PlatformIcon)

    // MARK: - Codable

    private enum CodingKeys: String, CodingKey {
        case type, images, interval, name, text, url, destination, icon
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let type = try container.decode(String.self, forKey: .type)

        switch type {
        case "slideshow":
            let images = try container.decode([String].self, forKey: .images)
            let interval = try container.decodeIfPresent(Double.self, forKey: .interval) ?? 2.0
            self = .slideshow(images: images, interval: interval)
        case "image":
            let name = try container.decode(String.self, forKey: .name)
            self = .image(name: name)
        case "link":
            let text = try container.decode(LocalizedTextData.self, forKey: .text)
            let url = try container.decode(String.self, forKey: .url)
            self = .link(text: text, url: url)
        case "navigation":
            let text = try container.decode(LocalizedTextData.self, forKey: .text)
            let destination = try container.decode(String.self, forKey: .destination)
            let icon = try container.decode(PlatformIcon.self, forKey: .icon)
            self = .navigation(text: text, destination: destination, icon: icon)
        default:
            throw DecodingError.dataCorruptedError(
                forKey: .type, in: container,
                debugDescription: "Unknown attachment type: \(type)"
            )
        }
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .slideshow(let images, let interval):
            try container.encode("slideshow", forKey: .type)
            try container.encode(images, forKey: .images)
            try container.encode(interval, forKey: .interval)
        case .image(let name):
            try container.encode("image", forKey: .type)
            try container.encode(name, forKey: .name)
        case .link(let text, let url):
            try container.encode("link", forKey: .type)
            try container.encode(text, forKey: .text)
            try container.encode(url, forKey: .url)
        case .navigation(let text, let destination, let icon):
            try container.encode("navigation", forKey: .type)
            try container.encode(text, forKey: .text)
            try container.encode(destination, forKey: .destination)
            try container.encode(icon, forKey: .icon)
        }
    }
}
