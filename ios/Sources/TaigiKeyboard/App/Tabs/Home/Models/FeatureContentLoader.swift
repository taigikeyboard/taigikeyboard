import Foundation

/// Loads HomeTab content from bundled JSON files.
enum FeatureContentLoader {
    /// Cached features, loaded once from JSON.
    static let features: [FeatureContent] = {
        guard let url = Bundle.main.url(forResource: "tab1-features", withExtension: "json"),
              let data = try? Data(contentsOf: url),
              let file = try? JSONDecoder().decode(FeaturesFile.self, from: data) else {
            return []
        }
        return file.features
    }()

    /// Cached FAQs, loaded once from JSON.
    static let faqs: [FeatureContent] = {
        guard let url = Bundle.main.url(forResource: "tab1-faq", withExtension: "json"),
              let data = try? Data(contentsOf: url),
              let file = try? JSONDecoder().decode(FAQsFile.self, from: data) else {
            return []
        }
        return file.faqs
    }()
}
