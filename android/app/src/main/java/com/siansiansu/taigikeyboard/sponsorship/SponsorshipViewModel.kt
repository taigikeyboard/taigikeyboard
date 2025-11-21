package com.siansiansu.taigikeyboard.sponsorship

import android.app.Activity
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.android.billingclient.api.ProductDetails
import com.siansiansu.taigikeyboard.billing.BillingManager
import com.siansiansu.taigikeyboard.settings.LocalizedText
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch

/**
 * 贊助頁面的 ViewModel
 * 管理購買狀態和使用者互動
 */
class SponsorshipViewModel(
    private val billingManager: BillingManager
) : ViewModel() {

    // 產品詳情
    val productDetails: StateFlow<Map<String, ProductDetails>> = billingManager.productDetails

    // 已購買產品
    val purchasedProductIds: StateFlow<Set<String>> = billingManager.purchasedProductIds

    // 載入狀態
    val isLoading: StateFlow<Boolean> = billingManager.isLoading

    // 錯誤訊息
    val errorMessage: StateFlow<String?> = billingManager.errorMessage

    // 成功購買提示
    private val _showThankYou = MutableStateFlow(false)
    val showThankYou: StateFlow<Boolean> = _showThankYou.asStateFlow()

    init {
        // 初始化時載入產品和查詢購買記錄
        viewModelScope.launch {
            billingManager.startConnection()
            billingManager.queryProductDetails()
            billingManager.queryPurchases()
        }
    }

    /**
     * 啟動購買流程
     */
    fun purchaseProduct(activity: Activity, productId: String) {
        val productDetails = billingManager.getProductDetails(productId)
        if (productDetails == null) {
            return
        }

        viewModelScope.launch {
            val result = billingManager.launchBillingFlow(activity, productDetails)
            if (result.responseCode == com.android.billingclient.api.BillingClient.BillingResponseCode.OK) {
                _showThankYou.value = true
            }
        }
    }

    /**
     * 關閉感謝訊息對話框
     */
    fun dismissThankYou() {
        _showThankYou.value = false
    }

    /**
     * 清除錯誤訊息
     */
    fun clearError() {
        billingManager.clearError()
    }

    /**
     * 取得產品詳情
     */
    fun getProductDetails(productId: String): ProductDetails? {
        return billingManager.getProductDetails(productId)
    }

    /**
     * 檢查產品是否已購買
     */
    fun isPurchased(productId: String): Boolean {
        return billingManager.isPurchased(productId)
    }

    /**
     * 格式化價格
     */
    fun formatPrice(productDetails: ProductDetails): String {
        return billingManager.formatPrice(productDetails)
    }

    override fun onCleared() {
        super.onCleared()
        billingManager.endConnection()
    }
}
