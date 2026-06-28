// 中文: HomeTab 內容 loader。從 bundle 內 features.json / faq.json 一次性讀入並快取。

import Foundation

/// Loads HomeTab content from bundled JSON files.
// 中文: HomeTab features / FAQ 的 lazy 快取容器。讀檔失敗回傳空陣列,不拋例外。
enum FeatureContentLoader {
    /// Cached features, loaded once from JSON.
    // 中文: features.json 解碼後的 features 清單,首次存取時讀檔並快取。
    static let features: [FeatureContent] = {
        guard let url = Bundle.main.url(forResource: "features", withExtension: "json"),
              let data = try? Data(contentsOf: url),
              let file = try? JSONDecoder().decode(FeaturesFile.self, from: data)
        else {
            return []
        }
        return file.features
    }()

    /// Cached FAQs, loaded once from JSON.
    // 中文: faq.json 解碼後的 FAQ 清單,首次存取時讀檔並快取。
    static let faqs: [FeatureContent] = {
        guard let url = Bundle.main.url(forResource: "faq", withExtension: "json"),
              let data = try? Data(contentsOf: url),
              let file = try? JSONDecoder().decode(FAQsFile.self, from: data)
        else {
            return []
        }
        return file.faqs
    }()
}
