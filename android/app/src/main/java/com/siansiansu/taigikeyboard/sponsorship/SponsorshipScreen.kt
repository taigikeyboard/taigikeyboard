package com.siansiansu.taigikeyboard.sponsorship

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowForward
import androidx.compose.material.icons.automirrored.filled.MenuBook
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.livedata.observeAsState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.blur
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.android.billingclient.api.ProductDetails
import com.siansiansu.taigikeyboard.billing.BillingManager
import com.siansiansu.taigikeyboard.settings.AppTexts
import com.siansiansu.taigikeyboard.settings.LanguageManager
import com.siansiansu.taigikeyboard.settings.LocalizedText

/**
 * 贊助頁面主畫面
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SponsorshipScreen(
    viewModel: SponsorshipViewModel,
    onDismiss: () -> Unit
) {
    val context = LocalContext.current
    val languageManager = LanguageManager.getInstance(context)
    val displayLanguage by languageManager.currentDisplayLanguage.observeAsState(
        com.siansiansu.taigikeyboard.settings.DisplayLanguage.HANJI
    )

    val productDetails by viewModel.productDetails.collectAsState()
    val purchasedProductIds by viewModel.purchasedProductIds.collectAsState()
    val isLoading by viewModel.isLoading.collectAsState()
    val showThankYou by viewModel.showThankYou.collectAsState()
    val errorMessage by viewModel.errorMessage.collectAsState()

    // 錯誤對話框
    if (errorMessage != null) {
        AlertDialog(
            onDismissRequest = { viewModel.clearError() },
            title = { Text(AppTexts.purchaseFailed.text(displayLanguage)) },
            text = { Text(errorMessage ?: "") },
            confirmButton = {
                TextButton(onClick = { viewModel.clearError() }) {
                    Text(AppTexts.confirm.text(displayLanguage))
                }
            }
        )
    }

    // 感謝對話框
    if (showThankYou) {
        AlertDialog(
            onDismissRequest = { viewModel.dismissThankYou() },
            title = { Text("") },
            text = { Text(AppTexts.thankYouMessage.text(displayLanguage)) },
            confirmButton = {
                TextButton(onClick = { viewModel.dismissThankYou() }) {
                    Text(AppTexts.done.text(displayLanguage))
                }
            }
        )
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { },
                navigationIcon = {
                    IconButton(onClick = onDismiss) {
                        Icon(Icons.Default.Close, contentDescription = "Close")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = Color.Transparent
                )
            )
        }
    ) { paddingValues ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
        ) {
            // 背景漸層
            GradientBackground()

            // 主要內容
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = 20.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(32.dp)
            ) {
                Spacer(modifier = Modifier.height(8.dp))

                // Hero Section
                HeroSection(displayLanguage)

                // App Introduction Section
                AppIntroductionSection(displayLanguage)

                // Sponsor Tiers Section
                SponsorTiersSection(
                    displayLanguage = displayLanguage,
                    productDetails = productDetails,
                    purchasedProductIds = purchasedProductIds,
                    isLoading = isLoading,
                    viewModel = viewModel
                )

                Spacer(modifier = Modifier.height(16.dp))
            }
        }
    }
}

/**
 * 復古扁平背景 - SPY×FAMILY Retro Theme
 */
@Composable
fun GradientBackground() {
    Box(modifier = Modifier.fillMaxSize()) {
        // 溫暖米色背景（復古扁平設計，無漸層）
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background)
        )
    }
}

/**
 * Hero Section - 頂部愛心圖示和標題 (Retro Style)
 */
@Composable
fun HeroSection(displayLanguage: com.siansiansu.taigikeyboard.settings.DisplayLanguage) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 8.dp),
        shape = RoundedCornerShape(10.dp),  // Retro square corners
        elevation = CardDefaults.cardElevation(defaultElevation = 0.dp),  // Flat design
        border = androidx.compose.foundation.BorderStroke(1.dp, MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.2f)),  // Retro border
        colors = CardDefaults.cardColors(
            containerColor = Color(0xFFFBF9F5)  // 溫暖白色背景，與版權聲明頁面一致
        )
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(28.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(24.dp)
        ) {
            // 愛心圖示（簡化，移除圓形背景和模糊效果）
            Icon(
                imageVector = Icons.Default.Favorite,
                contentDescription = "Heart",
                modifier = Modifier
                    .padding(top = 12.dp)
                    .size(64.dp),
                tint = Color(0xFFF5BAB4)  // Soft Pink
            )

            // 標題和副標題
            Column(
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(14.dp)
            ) {
                Text(
                    text = AppTexts.sponsorTitle.text(displayLanguage),
                    style = MaterialTheme.typography.headlineMedium,
                    color = MaterialTheme.colorScheme.onSurface
                )

                Text(
                    text = AppTexts.sponsorSubtitle.text(displayLanguage),
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                    lineHeight = MaterialTheme.typography.bodyLarge.lineHeight.times(1.2f)
                )
            }
        }
    }
}

/**
 * App Introduction Section - 承諾列表 (Retro Style)
 */
@Composable
fun AppIntroductionSection(displayLanguage: com.siansiansu.taigikeyboard.settings.DisplayLanguage) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 4.dp),
        shape = RoundedCornerShape(10.dp),  // Retro square corners
        elevation = CardDefaults.cardElevation(defaultElevation = 0.dp),  // Flat design
        border = androidx.compose.foundation.BorderStroke(1.dp, MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.2f)),  // Retro border
        colors = CardDefaults.cardColors(
            containerColor = Color(0xFFFBF9F5)  // 溫暖白色背景，與版權聲明頁面一致
        )
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(24.dp),
            verticalArrangement = Arrangement.spacedBy(20.dp)
        ) {
            Text(
                text = AppTexts.aboutApp.text(displayLanguage),
                style = MaterialTheme.typography.titleLarge,
                color = MaterialTheme.colorScheme.onSurface
            )

            Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
                CommitmentRow(
                    icon = Icons.Default.Favorite,
                    text = AppTexts.futurePlan4,
                    displayLanguage = displayLanguage,
                    color = MaterialTheme.colorScheme.secondary  // Yor's Deep Red
                )
                CommitmentRow(
                    icon = Icons.Default.Forum,
                    text = AppTexts.futurePlan1,
                    displayLanguage = displayLanguage,
                    color = MaterialTheme.colorScheme.primary  // Loid's Teal Gray
                )
                CommitmentRow(
                    icon = Icons.Default.Security,
                    text = AppTexts.futurePlan2,
                    displayLanguage = displayLanguage,
                    color = MaterialTheme.colorScheme.primary  // Loid's Teal Gray
                )
                CommitmentRow(
                    icon = Icons.AutoMirrored.Filled.MenuBook,
                    text = AppTexts.futurePlan3,
                    displayLanguage = displayLanguage,
                    color = MaterialTheme.colorScheme.tertiary  // Anya's Warm Pink
                )
            }
        }
    }
}

/**
 * 承諾項目行
 */
@Composable
fun CommitmentRow(
    icon: ImageVector,
    text: LocalizedText,
    displayLanguage: com.siansiansu.taigikeyboard.settings.DisplayLanguage,
    color: Color
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 6.dp),
        horizontalArrangement = Arrangement.spacedBy(16.dp),
        verticalAlignment = Alignment.Top
    ) {
        Icon(
            imageVector = icon,
            contentDescription = null,
            modifier = Modifier.size(24.dp),
            tint = MaterialTheme.colorScheme.primary
        )

        Text(
            text = text.text(displayLanguage),
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.weight(1f)
        )
    }
}

/**
 * Sponsor Tiers Section - 贊助層級卡片
 */
@Composable
fun SponsorTiersSection(
    displayLanguage: com.siansiansu.taigikeyboard.settings.DisplayLanguage,
    productDetails: Map<String, ProductDetails>,
    purchasedProductIds: Set<String>,
    isLoading: Boolean,
    viewModel: SponsorshipViewModel
) {
    Column(
        modifier = Modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Text(
            text = AppTexts.oneTimeSponsor.text(displayLanguage),
            style = MaterialTheme.typography.titleLarge,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.padding(horizontal = 4.dp)
        )

        // 三個贊助層級
        SponsorTierCard(
            tier = SponsorTier.Coffee,
            productDetails = productDetails[BillingManager.PRODUCT_COFFEE],
            isPurchased = purchasedProductIds.contains(BillingManager.PRODUCT_COFFEE),
            isLoading = isLoading,
            displayLanguage = displayLanguage,
            viewModel = viewModel
        )

        SponsorTierCard(
            tier = SponsorTier.Meal,
            productDetails = productDetails[BillingManager.PRODUCT_MEAL],
            isPurchased = purchasedProductIds.contains(BillingManager.PRODUCT_MEAL),
            isLoading = isLoading,
            displayLanguage = displayLanguage,
            viewModel = viewModel
        )

        SponsorTierCard(
            tier = SponsorTier.Premium,
            productDetails = productDetails[BillingManager.PRODUCT_PREMIUM],
            isPurchased = purchasedProductIds.contains(BillingManager.PRODUCT_PREMIUM),
            isLoading = isLoading,
            displayLanguage = displayLanguage,
            viewModel = viewModel
        )
    }
}

/**
 * 贊助層級卡片 (Retro Style)
 */
@Composable
fun SponsorTierCard(
    tier: SponsorTier,
    productDetails: ProductDetails?,
    isPurchased: Boolean,
    isLoading: Boolean,
    displayLanguage: com.siansiansu.taigikeyboard.settings.DisplayLanguage,
    viewModel: SponsorshipViewModel
) {
    val context = LocalContext.current
    val activity = context as? androidx.activity.ComponentActivity

    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(10.dp),  // Retro square corners
        elevation = CardDefaults.cardElevation(defaultElevation = 0.dp),  // Flat design
        border = androidx.compose.foundation.BorderStroke(1.dp, MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.2f)),  // Retro border
        colors = CardDefaults.cardColors(
            containerColor = Color(0xFFFBF9F5),  // 溫暖白色背景，與版權聲明頁面一致
            disabledContainerColor = Color(0xFFFBF9F5)  // disabled 狀態也使用相同背景
        ),
        onClick = {
            if (!isPurchased && !isLoading && productDetails != null && activity != null) {
                viewModel.purchaseProduct(activity, tier.productId)
            }
        },
        enabled = !isPurchased && !isLoading && productDetails != null
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(20.dp),
            horizontalArrangement = Arrangement.spacedBy(18.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // 圖示（簡化，移除圓形背景）
            Icon(
                imageVector = tier.icon,
                contentDescription = null,
                modifier = Modifier.size(32.dp),
                tint = tier.color
            )

            // 內容
            Column(
                modifier = Modifier.weight(1f),
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Text(
                    text = tier.title.text(displayLanguage),
                    style = MaterialTheme.typography.titleMedium,
                    color = MaterialTheme.colorScheme.onSurface
                )

                // 價格或狀態顯示
                when {
                    isPurchased -> {
                        Text(
                            text = AppTexts.alreadySponsored.text(displayLanguage),
                            style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.primary
                        )
                    }
                    productDetails != null -> {
                        Text(
                            text = viewModel.formatPrice(productDetails),
                            style = MaterialTheme.typography.headlineSmall,
                            color = tier.color
                        )
                    }
                    isLoading -> {
                        CircularProgressIndicator(
                            modifier = Modifier.size(20.dp),
                            strokeWidth = 2.dp
                        )
                    }
                }
            }

            // 購買指示器
            Icon(
                imageVector = if (isPurchased) Icons.Default.CheckCircle else Icons.AutoMirrored.Filled.ArrowForward,
                contentDescription = null,
                modifier = Modifier.size(26.dp),
                tint = tier.color
            )
        }
    }
}

/**
 * 贊助層級定義 - SPY×FAMILY 配色
 */
enum class SponsorTier(
    val productId: String,
    val title: LocalizedText,
    val icon: ImageVector,
    val color: Color
) {
    Coffee(
        productId = BillingManager.PRODUCT_COFFEE,
        title = AppTexts.coffeeTier,
        icon = Icons.Default.LocalCafe,
        color = Color(0xFF8da99b)  // Loid's Teal Gray Green
    ),
    Meal(
        productId = BillingManager.PRODUCT_MEAL,
        title = AppTexts.mealTier,
        icon = Icons.Default.Restaurant,
        color = Color(0xFF610a10)  // Yor's Deep Red
    ),
    Premium(
        productId = BillingManager.PRODUCT_PREMIUM,
        title = AppTexts.premiumTier,
        icon = Icons.Default.Stars,
        color = Color(0xFFfab3ad)  // Anya's Warm Pink
    )
}
