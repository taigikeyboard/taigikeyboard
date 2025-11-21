import StoreKit
import Foundation

/// StoreKit 管理器（處理 IAP 購買與交易）
@MainActor
class StoreKitManager: ObservableObject {
    static let shared = StoreKitManager()

    @Published var products: [Product] = []
    @Published var purchasedProductIDs = Set<String>()
    @Published var isLoading = false
    @Published var errorMessage: LocalizedText?

    private var updateListenerTask: Task<Void, Error>?

    private init() {
        updateListenerTask = listenForTransactions()
        Task {
            await loadProducts()
            await updatePurchasedProducts()
        }
    }

    deinit {
        updateListenerTask?.cancel()
    }

    /// 載入可購買的產品列表
    func loadProducts() async {
        isLoading = true
        errorMessage = nil

        do {
            products = try await Product.products(for: ProductConfiguration.productIDs)
            products.sort { $0.price < $1.price }
            isLoading = false
        } catch {
            errorMessage = AppTexts.loadingProductsFailed
            isLoading = false
        }
    }

    /// 執行購買流程
    func purchase(_ product: Product) async throws -> Transaction? {
        isLoading = true
        errorMessage = nil

        do {
            let result = try await product.purchase()

            switch result {
            case .success(let verification):
                let transaction = try checkVerified(verification)
                await updatePurchasedProducts()
                await transaction.finish()
                isLoading = false
                return transaction

            case .userCancelled:
                isLoading = false
                return nil

            case .pending:
                errorMessage = AppTexts.purchasePending
                isLoading = false
                return nil

            @unknown default:
                errorMessage = AppTexts.unknownError
                isLoading = false
                return nil
            }
        } catch {
            errorMessage = AppTexts.purchaseFailed
            isLoading = false
            throw error
        }
    }

    /// 驗證交易結果
    private func checkVerified<T>(_ result: VerificationResult<T>) throws -> T {
        switch result {
        case .unverified:
            throw StoreError.failedVerification
        case .verified(let safe):
            return safe
        }
    }

    /// 更新已購買產品清單
    func updatePurchasedProducts() async {
        var purchasedProducts = Set<String>()

        for await result in Transaction.currentEntitlements {
            if case .verified(let transaction) = result {
                purchasedProducts.insert(transaction.productID)
            }
        }

        self.purchasedProductIDs = purchasedProducts
    }

    /// 監聽交易更新
    private func listenForTransactions() -> Task<Void, Error> {
        return Task.detached {
            for await result in Transaction.updates {
                do {
                    let transaction = try await MainActor.run {
                        try self.checkVerified(result)
                    }
                    await self.updatePurchasedProducts()
                    await transaction.finish()
                } catch {}
            }
        }
    }

    /// 根據 ID 取得產品
    func product(for productID: String) -> Product? {
        return products.first { $0.id == productID }
    }

    /// 檢查產品是否已購買
    func isPurchased(_ productID: String) -> Bool {
        return purchasedProductIDs.contains(productID)
    }

    /// 格式化產品價格（TWD 移除小數點）
    func formattedPrice(for product: Product) -> String {
        return PriceFormatter.formattedPrice(for: product)
    }

    enum StoreError: Error {
        case failedVerification
    }
}
