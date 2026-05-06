// 中文: HomeTab 的 features / FAQ 內容資料模型。
// 中文: JSON 檔內以 {"hanji": "..."} 包裝在地化文字以同時支援 iOS / Android,
// 中文: iOS 端只取 hanji 欄位解出 String。

import Foundation

// MARK: - HomeTab content models (parsed from tab1-features.json / tab1-faq.json)

/// Root container for the features JSON file.
// 中文: tab1-features.json 的根容器。
struct FeaturesFile: Codable {
    let features: [FeatureContent]
}

/// Root container for the FAQ JSON file.
// 中文: tab1-faq.json 的根容器。
struct FAQsFile: Codable {
    let faqs: [FeatureContent]
}

/// JSON stores localized text as {"hanji": "..."} for cross-platform compatibility.
/// iOS only uses the hanji value, so we decode it into a plain String.
// 中文: 跨平台 JSON 共用格式 — Android 也讀同份檔。iOS 只取 hanji。
private struct HanjiText: Decodable {
    let hanji: String
}

/// A single feature description entry.
// 中文: 單筆 feature / FAQ 項目。title / summary 走 hanji 欄位解碼。
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
// 中文: 一個段落:文字 + 可選 media / link / navigation attachment。
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
// 中文: 跨平台 icon 名稱對照(iOS = SF Symbol;Android = Material name)。
struct PlatformIcon: Codable {
    let ios: String
    let android: String
}

/// Media attachment on a paragraph.
// 中文: 段落附件:slideshow(輪播)/ image / link(外部連結)/ navigation(in-app 導覽)。
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
