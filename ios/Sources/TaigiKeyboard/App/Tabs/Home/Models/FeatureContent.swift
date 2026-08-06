// 中文: HomeTab 的 features / FAQ 內容資料模型。
// 中文: JSON 檔內以 {"hanji": "...", "en": ..., "ja": ...} 物件包裝在地化文字(iOS / Android 共用),
// 中文: 解碼成 LocalizedContentText;render 時依顯示語言 resolve,未授權語言 fallback 回 hanji。

import Foundation

// MARK: - HomeTab content models (parsed from features.json / faq.json)

/// Root container for the features JSON file.
// 中文: features.json 的根容器。
struct FeaturesFile: Codable {
    let features: [FeatureContent]
}

/// Root container for the FAQ JSON file.
// 中文: faq.json 的根容器。
struct FAQsFile: Codable {
    let faqs: [FeatureContent]
}

/// A localized content string from the shared JSON, e.g. `{"hanji": "...", "en": "...", "ja": "..."}`.
/// Only `hanji` is required; the other languages are authored later (C2). `resolve(for:)` returns the
/// active language's string, falling back to `hanji` for any language not yet authored — so until C2
/// fills the keys, every effective language renders Hanji (display unchanged).
/// `Codable` (not Decodable-only) so the enclosing `Codable` models keep their synthesized `Encodable`.
// 中文: 跨平台 JSON 共用格式 — Android 也讀同份檔。hanji 必填,其餘語言之後補(C2);未授權 fallback hanji。
struct LocalizedContentText: Codable {
    let hanji: String
    let tailo: String?
    let poj: String?
    let ja: String?
    let en: String?

    /// The string for `language`, falling back to `hanji` when that language is unauthored. `language`
    /// is the EFFECTIVE display language (never `.system`) — render sites pass `DisplayLanguageStore.language`,
    /// already resolved away from `.system`. Mirrors StringResolver.swift's fallback contract (assert the
    /// `.system` boundary in DEBUG, degrade to Hanji in release rather than crash).
    /// CROSS-PLATFORM INVARIANT — mirrors
    /// android .../content/FeatureContent.kt `LocalizedContentText.resolve`. Drift causes silent divergence.
    func resolve(for language: DisplayLanguage) -> String {
        switch language {
        case .hanji: return hanji
        case .tailo: return tailo ?? hanji
        case .poj: return poj ?? hanji
        case .japanese: return ja ?? hanji
        case .english: return en ?? hanji
        case .system:
            assertionFailure("LocalizedContentText.resolve(for:) must receive an effective language, never .system")
            return hanji
        }
    }
}

/// A single feature description entry.
// 中文: 單筆 feature / FAQ 項目。title / summary 走 hanji 欄位解碼。
struct FeatureContent: Codable, Identifiable {
    let id: String
    let title: LocalizedContentText
    let icon: PlatformIcon
    let summary: LocalizedContentText?
    let paragraphs: [FeatureParagraph]

    private enum CodingKeys: String, CodingKey {
        case id, title, icon, summary, paragraphs
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        title = try container.decode(LocalizedContentText.self, forKey: .title)
        icon = try container.decode(PlatformIcon.self, forKey: .icon)
        summary = try container.decodeIfPresent(LocalizedContentText.self, forKey: .summary)
        paragraphs = try container.decode([FeatureParagraph].self, forKey: .paragraphs)
    }
}

/// A paragraph with optional media attachment.
// 中文: 一個段落:文字 + 可選 media / link / navigation attachment。
struct FeatureParagraph: Codable {
    let text: LocalizedContentText
    let attachment: ParagraphAttachment?

    private enum CodingKeys: String, CodingKey {
        case text, attachment
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        text = try container.decode(LocalizedContentText.self, forKey: .text)
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
    case link(text: LocalizedContentText, url: String)
    case navigation(text: LocalizedContentText, destination: String, icon: PlatformIcon)

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
            let text = try container.decode(LocalizedContentText.self, forKey: .text)
            let url = try container.decode(String.self, forKey: .url)
            self = .link(text: text, url: url)
        case "navigation":
            let text = try container.decode(LocalizedContentText.self, forKey: .text)
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
