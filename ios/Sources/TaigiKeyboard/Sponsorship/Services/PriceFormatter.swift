import StoreKit

/// 價格格式化工具
enum PriceFormatter {
    /// 格式化產品價格（TWD 移除小數點）
    static func formattedPrice(for product: Product) -> String {
        let displayPrice = product.displayPrice
        let currencyCode = product.priceFormatStyle.currencyCode

        if currencyCode == "TWD" {
            return displayPrice.replacingOccurrences(of: ".00", with: "")
        }

        return displayPrice
    }
}
