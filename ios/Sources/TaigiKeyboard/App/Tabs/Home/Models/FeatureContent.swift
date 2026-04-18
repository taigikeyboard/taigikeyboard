import Foundation

// MARK: - HomeTab content models (parsed from tab1-features.json / tab1-faq.json)

/// Root container for the features JSON file.
struct FeaturesFile: Codable {
    let features: [FeatureContent]
}

/// Root container for the FAQ JSON file.
struct FAQsFile: Codable {
    let faqs: [FeatureContent]
}

/// JSON stores localized text as {"hanji": "..."} for cross-platform compatibility.
/// iOS only uses the hanji value, so we decode it into a plain String.
private struct HanjiText: Decodable {
    let hanji: String
}

/// A single feature description entry.
struct FeatureContent: Codable, Identifiable {
    let id: String
    let title: String
    let icon: PlatformIcon
    let summary: String?
    let paragraphs: [FeatureParagraph]

    private enum CodingKeys: String, CodingKey {
        case id, title, icon, summary, paragraphs
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        title = try container.decode(HanjiText.self, forKey: .title).hanji
        icon = try container.decode(PlatformIcon.self, forKey: .icon)
        summary = try container.decodeIfPresent(HanjiText.self, forKey: .summary)?.hanji
        paragraphs = try container.decode([FeatureParagraph].self, forKey: .paragraphs)
    }
}

/// A paragraph with optional media attachment.
struct FeatureParagraph: Codable {
    let text: String
    let attachment: ParagraphAttachment?

    private enum CodingKeys: String, CodingKey {
        case text, attachment
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        text = try container.decode(HanjiText.self, forKey: .text).hanji
        attachment = try container.decodeIfPresent(ParagraphAttachment.self, forKey: .attachment)
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
    case link(text: String, url: String)
    case navigation(text: String, destination: String, icon: PlatformIcon)

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
            let text = try container.decode(HanjiText.self, forKey: .text).hanji
            let url = try container.decode(String.self, forKey: .url)
            self = .link(text: text, url: url)
        case "navigation":
            let text = try container.decode(HanjiText.self, forKey: .text).hanji
            let destination = try container.decode(String.self, forKey: .destination)
            let icon = try container.decode(PlatformIcon.self, forKey: .icon)
            self = .navigation(text: text, destination: destination, icon: icon)
        default:
            throw DecodingError.dataCorruptedError(
                forKey: .type, in: container,
                debugDescription: "Unknown attachment type: \(type)",
            )
        }
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case let .slideshow(images, interval):
            try container.encode("slideshow", forKey: .type)
            try container.encode(images, forKey: .images)
            try container.encode(interval, forKey: .interval)
        case let .image(name):
            try container.encode("image", forKey: .type)
            try container.encode(name, forKey: .name)
        case let .link(text, url):
            try container.encode("link", forKey: .type)
            try container.encode(text, forKey: .text)
            try container.encode(url, forKey: .url)
        case let .navigation(text, destination, icon):
            try container.encode("navigation", forKey: .type)
            try container.encode(text, forKey: .text)
            try container.encode(destination, forKey: .destination)
            try container.encode(icon, forKey: .icon)
        }
    }
}
