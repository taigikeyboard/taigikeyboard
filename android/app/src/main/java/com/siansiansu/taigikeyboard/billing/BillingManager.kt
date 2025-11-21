package com.siansiansu.taigikeyboard.billing

import android.app.Activity
import android.content.Context
import com.android.billingclient.api.*
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withContext
import kotlin.coroutines.resume
import kotlin.coroutines.suspendCoroutine

/**
 * 管理 Google Play Billing 的核心類別
 * 遵循 2025 年最佳實踐，使用 Billing Library 7.x
 */
class BillingManager(private val context: Context) : PurchasesUpdatedListener {

    companion object {
        // 產品 ID 定義
        const val PRODUCT_COFFEE = "com.siansiansu.taigikeyboard.donate.small"
        const val PRODUCT_MEAL = "com.siansiansu.taigikeyboard.donate.medium"
        const val PRODUCT_PREMIUM = "com.siansiansu.taigikeyboard.donate.large"

        val PRODUCT_IDS = listOf(PRODUCT_COFFEE, PRODUCT_MEAL, PRODUCT_PREMIUM)
    }

    // BillingClient 實例
    // 注意：對於一次性產品（one-time products），必須明確啟用 pending purchases
    @Suppress("DEPRECATION")
    private var billingClient: BillingClient = BillingClient.newBuilder(context)
        .setListener(this)
        .enablePendingPurchases()
        .build()

    // 產品詳情狀態
    private val _productDetails = MutableStateFlow<Map<String, ProductDetails>>(emptyMap())
    val productDetails: StateFlow<Map<String, ProductDetails>> = _productDetails.asStateFlow()

    // 已購買產品狀態
    private val _purchasedProductIds = MutableStateFlow<Set<String>>(emptySet())
    val purchasedProductIds: StateFlow<Set<String>> = _purchasedProductIds.asStateFlow()

    // 載入狀態
    private val _isLoading = MutableStateFlow(false)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    // 錯誤訊息
    private val _errorMessage = MutableStateFlow<String?>(null)
    val errorMessage: StateFlow<String?> = _errorMessage.asStateFlow()

    /**
     * 啟動 BillingClient 連接
     */
    suspend fun startConnection(): Boolean = suspendCoroutine { continuation ->
        billingClient.startConnection(object : BillingClientStateListener {
            override fun onBillingSetupFinished(billingResult: BillingResult) {
                if (billingResult.responseCode == BillingClient.BillingResponseCode.OK) {
                    continuation.resume(true)
                } else {
                    continuation.resume(false)
                }
            }

            override fun onBillingServiceDisconnected() {
                // 連接中斷，可能需要重新連接
                continuation.resume(false)
            }
        })
    }

    /**
     * 查詢產品詳情
     */
    suspend fun queryProductDetails() {
        _isLoading.value = true
        _errorMessage.value = null

        try {
            // 確保已連接
            if (!billingClient.isReady) {
                val connected = startConnection()
                if (!connected) {
                    _errorMessage.value = "Failed to connect to billing service"
                    _isLoading.value = false
                    return
                }
            }

            // 建立產品查詢參數
            val productList = PRODUCT_IDS.map { productId ->
                QueryProductDetailsParams.Product.newBuilder()
                    .setProductId(productId)
                    .setProductType(BillingClient.ProductType.INAPP)
                    .build()
            }

            val params = QueryProductDetailsParams.newBuilder()
                .setProductList(productList)
                .build()

            // 執行查詢
            withContext(Dispatchers.IO) {
                billingClient.queryProductDetailsAsync(params) { billingResult, productDetailsList ->
                    if (billingResult.responseCode == BillingClient.BillingResponseCode.OK) {
                        val detailsMap = productDetailsList.associateBy { it.productId }
                        _productDetails.value = detailsMap
                    } else {
                        _errorMessage.value = "Failed to load products: ${billingResult.debugMessage}"
                    }
                    _isLoading.value = false
                }
            }
        } catch (e: Exception) {
            _errorMessage.value = "Error querying products: ${e.message}"
            _isLoading.value = false
        }
    }

    /**
     * 啟動購買流程
     */
    suspend fun launchBillingFlow(activity: Activity, productDetails: ProductDetails): BillingResult {
        val productDetailsParamsList = listOf(
            BillingFlowParams.ProductDetailsParams.newBuilder()
                .setProductDetails(productDetails)
                .build()
        )

        val billingFlowParams = BillingFlowParams.newBuilder()
            .setProductDetailsParamsList(productDetailsParamsList)
            .build()

        return billingClient.launchBillingFlow(activity, billingFlowParams)
    }

    /**
     * 處理購買更新（PurchasesUpdatedListener 回調）
     */
    override fun onPurchasesUpdated(billingResult: BillingResult, purchases: MutableList<Purchase>?) {
        if (billingResult.responseCode == BillingClient.BillingResponseCode.OK && purchases != null) {
            for (purchase in purchases) {
                handlePurchase(purchase)
            }
        } else if (billingResult.responseCode == BillingClient.BillingResponseCode.USER_CANCELED) {
            // 使用者取消購買
            _errorMessage.value = null
        } else {
            _errorMessage.value = "Purchase failed: ${billingResult.debugMessage}"
        }
    }

    /**
     * 處理購買並進行驗證與確認
     */
    private fun handlePurchase(purchase: Purchase) {
        // 驗證購買狀態
        if (purchase.purchaseState == Purchase.PurchaseState.PURCHASED) {
            // 如果尚未確認，需要在 72 小時內確認
            if (!purchase.isAcknowledged) {
                acknowledgePurchase(purchase)
            }

            // 更新已購買產品清單
            val currentPurchased = _purchasedProductIds.value.toMutableSet()
            currentPurchased.addAll(purchase.products)
            _purchasedProductIds.value = currentPurchased
        }
    }

    /**
     * 確認購買（acknowledge）
     */
    private fun acknowledgePurchase(purchase: Purchase) {
        val acknowledgePurchaseParams = AcknowledgePurchaseParams.newBuilder()
            .setPurchaseToken(purchase.purchaseToken)
            .build()

        billingClient.acknowledgePurchase(acknowledgePurchaseParams) { billingResult ->
            if (billingResult.responseCode == BillingClient.BillingResponseCode.OK) {
                // 購買確認成功
            }
        }
    }

    /**
     * 查詢已購買的產品
     */
    suspend fun queryPurchases() {
        val params = QueryPurchasesParams.newBuilder()
            .setProductType(BillingClient.ProductType.INAPP)
            .build()

        withContext(Dispatchers.IO) {
            billingClient.queryPurchasesAsync(params) { billingResult, purchasesList ->
                if (billingResult.responseCode == BillingClient.BillingResponseCode.OK) {
                    val purchasedIds = purchasesList
                        .filter { it.purchaseState == Purchase.PurchaseState.PURCHASED }
                        .flatMap { it.products }
                        .toSet()
                    _purchasedProductIds.value = purchasedIds
                }
            }
        }
    }

    /**
     * 取得指定產品的詳情
     */
    fun getProductDetails(productId: String): ProductDetails? {
        return _productDetails.value[productId]
    }

    /**
     * 檢查產品是否已購買
     */
    fun isPurchased(productId: String): Boolean {
        return _purchasedProductIds.value.contains(productId)
    }

    /**
     * 格式化價格（移除台幣的 .00）
     */
    fun formatPrice(productDetails: ProductDetails): String {
        val price = productDetails.oneTimePurchaseOfferDetails?.formattedPrice ?: ""
        return if (productDetails.oneTimePurchaseOfferDetails?.priceCurrencyCode == "TWD") {
            price.replace(".00", "")
        } else {
            price
        }
    }

    /**
     * 清除錯誤訊息
     */
    fun clearError() {
        _errorMessage.value = null
    }

    /**
     * 結束 BillingClient
     */
    fun endConnection() {
        billingClient.endConnection()
    }
}
