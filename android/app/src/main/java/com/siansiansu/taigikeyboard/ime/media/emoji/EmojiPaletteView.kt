package com.siansiansu.taigikeyboard.ime.media.emoji

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.grid.rememberLazyGridState
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.shape.GenericShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Tab
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.TabRowDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Popup
import kotlinx.coroutines.launch
import kotlin.math.ceil

private val EmojiCategoryValues = EmojiCategory.values()
private val EmojiBaseWidth = 70.dp
private val EmojiDefaultFontSize = 35.sp

// 變體指示器三角形形狀（左到右佈局）
private val VariantsTriangleShapeLtr = GenericShape { size, _ ->
    moveTo(x = size.width, y = 0f)
    lineTo(x = size.width, y = size.height)
    lineTo(x = 0f, y = size.height)
}

// 變體指示器三角形形狀（右到左佈局）
private val VariantsTriangleShapeRtl = GenericShape { size, _ ->
    moveTo(x = 0f, y = 0f)
    lineTo(x = size.width, y = size.height)
    lineTo(x = 0f, y = size.height)
}

/**
 * Emoji 面板主 Composable
 *
 * @param fullEmojiMappings 所有分類的 emoji 資料
 * @param preferredSkinTone 使用者偏好的膚色設定
 * @param onEmojiClick Emoji 點擊回調
 * @param onSkinToneSelected 膚色選擇回調
 * @param modifier Modifier
 */
@Composable
fun EmojiPaletteView(
    fullEmojiMappings: EmojiLayoutDataMap,
    preferredSkinTone: com.siansiansu.taigikeyboard.ime.keyboard.EmojiSkinTone,
    onEmojiClick: (EmojiKeyData) -> Unit,
    onSkinToneSelected: (com.siansiansu.taigikeyboard.ime.keyboard.EmojiSkinTone) -> Unit,
    modifier: Modifier = Modifier
) {
    var activeCategory by remember { mutableStateOf(EmojiCategory.SMILEYS_EMOTION) }
    val scope = rememberCoroutineScope()

    Column(modifier = modifier) {
        val pagerState = rememberPagerState(pageCount = { EmojiCategoryValues.size })

        // Tab 列
        EmojiCategoriesTabRow(
            activeCategory = activeCategory,
            onCategoryChange = { category ->
                activeCategory = category
                scope.launch {
                    pagerState.animateScrollToPage(EmojiCategoryValues.indexOf(category))
                }
            }
        )

        // 分頁內容
        HorizontalPager(
            state = pagerState,
            beyondViewportPageCount = 2
        ) { page ->
            val lazyGridState = rememberLazyGridState()

            // 同步 pager 與 activeCategory
            LaunchedEffect(pagerState.currentPage) {
                activeCategory = EmojiCategoryValues[pagerState.currentPage]
            }

            val category = EmojiCategoryValues[page]

            // 根據分類取得 emoji 資料
            val emojiList = remember(category, fullEmojiMappings) {
                fullEmojiMappings[category] ?: emptyList()
            }

            // 使用 key() 包裹確保 Compose 正確追蹤和複用
            key(category) {
                CompositionLocalProvider(LocalLayoutDirection provides LayoutDirection.Ltr) {
                    // Emoji Grid - 設定固定高度避免佔滿整個螢幕
                    LazyVerticalGrid(
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(240.dp),
                        columns = GridCells.Fixed(7),
                        state = lazyGridState,
                        horizontalArrangement = Arrangement.spacedBy(2.dp),
                        verticalArrangement = Arrangement.spacedBy(2.dp)
                    ) {
                        items(emojiList) { emojiSet ->
                            EmojiKey(
                                emojiSet = emojiSet,
                                preferredSkinTone = preferredSkinTone,
                                onEmojiClick = onEmojiClick,
                                onSkinToneSelected = onSkinToneSelected
                            )
                        }
                    }
                }
            }
        }
    }
}

/**
 * Emoji 分類 Tab 列
 */
@Composable
private fun EmojiCategoriesTabRow(
    activeCategory: EmojiCategory,
    onCategoryChange: (EmojiCategory) -> Unit
) {
    val selectedTabIndex = EmojiCategoryValues.indexOf(activeCategory)

    PrimaryTabRow(
        modifier = Modifier
            .fillMaxWidth()
            .height(48.dp),
        selectedTabIndex = selectedTabIndex,
        containerColor = androidx.compose.material3.MaterialTheme.colorScheme.surface,
        contentColor = androidx.compose.material3.MaterialTheme.colorScheme.onSurface,
        indicator = {
            TabRowDefaults.PrimaryIndicator(
                modifier = Modifier.tabIndicatorOffset(selectedTabIndex, matchContentSize = false),
                height = 4.dp,
                color = androidx.compose.material3.MaterialTheme.colorScheme.primary
            )
        }
    ) {
        EmojiCategoryValues.forEachIndexed { index, category ->
            Tab(
                selected = activeCategory == category,
                onClick = { onCategoryChange(category) },
                icon = {
                    Icon(
                        imageVector = category.icon(),
                        contentDescription = category.toString(),
                        modifier = Modifier.size(24.dp)
                    )
                }
            )
        }
    }
}

/**
 * 單個 Emoji 按鈕
 */
@Composable
private fun EmojiKey(
    emojiSet: EmojiSet,
    preferredSkinTone: com.siansiansu.taigikeyboard.ime.keyboard.EmojiSkinTone,
    onEmojiClick: (EmojiKeyData) -> Unit,
    onSkinToneSelected: (com.siansiansu.taigikeyboard.ime.keyboard.EmojiSkinTone) -> Unit
) {
    // 使用 remember 快取計算結果，避免每次重組都重新計算
    val variations = remember(emojiSet) { emojiSet.variations() }
    val hasVariations = remember(emojiSet) { variations.isNotEmpty() }
    val base = remember(emojiSet, preferredSkinTone, hasVariations) {
        if (hasVariations && preferredSkinTone != com.siansiansu.taigikeyboard.ime.keyboard.EmojiSkinTone.DEFAULT) {
            emojiSet.base(withSkinTone = preferredSkinTone.codePoint)
        } else {
            emojiSet.base()
        }
    }
    var showVariantsPopup by remember { mutableStateOf(false) }

    Box(
        modifier = Modifier
            .aspectRatio(1f)
            .pointerInput(Unit) {
                detectTapGestures(
                    onTap = {
                        onEmojiClick(base)
                    },
                    onLongPress = {
                        if (hasVariations) {
                            showVariantsPopup = true
                        }
                    }
                )
            }
    ) {
        // Emoji 文字
        Text(
            modifier = Modifier.align(Alignment.Center),
            text = base.getCodePointsAsString(),
            fontSize = EmojiDefaultFontSize,
            color = androidx.compose.material3.MaterialTheme.colorScheme.onSurface
        )

        // 變體指示器
        if (hasVariations) {
            val shape = when (LocalLayoutDirection.current) {
                LayoutDirection.Ltr -> VariantsTriangleShapeLtr
                LayoutDirection.Rtl -> VariantsTriangleShapeRtl
            }
            Box(
                modifier = Modifier
                    .align(Alignment.BottomEnd)
                    .offset(x = (-4).dp, y = (-4).dp)
                    .size(4.dp)
                    .background(
                        androidx.compose.material3.MaterialTheme.colorScheme.onSurfaceVariant,
                        shape
                    )
            )
        }

        // 變體彈窗
        if (showVariantsPopup) {
            EmojiVariationsPopup(
                variations = variations,
                onEmojiTap = { emoji, skinTone ->
                    onEmojiClick(emoji)
                    // 記錄使用者選擇的膚色
                    if (skinTone != null) {
                        onSkinToneSelected(skinTone)
                    }
                    showVariantsPopup = false
                },
                onDismiss = {
                    showVariantsPopup = false
                }
            )
        }
    }
}

/**
 * Emoji 變體彈窗（膚色選擇）
 */
@Composable
private fun EmojiVariationsPopup(
    variations: List<EmojiKeyData>,
    onEmojiTap: (EmojiKeyData, com.siansiansu.taigikeyboard.ime.keyboard.EmojiSkinTone?) -> Unit,
    onDismiss: () -> Unit
) {
    val emojiKeyHeight = 48.dp

    Popup(
        alignment = Alignment.TopCenter,
        offset = with(LocalDensity.current) {
            val y = -emojiKeyHeight * ceil(variations.size / 6f)
            IntOffset(x = 0, y = y.toPx().toInt())
        },
        onDismissRequest = onDismiss
    ) {
        Box(
            modifier = Modifier
                .widthIn(max = EmojiBaseWidth * 6)
                .background(androidx.compose.material3.MaterialTheme.colorScheme.surfaceVariant)
                .padding(8.dp)
        ) {
            Column {
                variations.chunked(6).forEach { row ->
                    androidx.compose.foundation.layout.Row {
                        row.forEach { emoji ->
                            // 偵測 emoji 的膚色修飾符
                            val skinTone = detectSkinToneFromEmoji(emoji)

                            Box(
                                modifier = Modifier
                                    .width(EmojiBaseWidth)
                                    .height(emojiKeyHeight)
                                    .pointerInput(Unit) {
                                        detectTapGestures { onEmojiTap(emoji, skinTone) }
                                    }
                            ) {
                                Text(
                                    modifier = Modifier.align(Alignment.Center),
                                    text = emoji.getCodePointsAsString(),
                                    fontSize = EmojiDefaultFontSize,
                                    color = androidx.compose.material3.MaterialTheme.colorScheme.onSurface
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

/**
 * 偵測 emoji 中的膚色修飾符
 * 返回對應的 EmojiSkinTone，如果沒有膚色修飾符則返回 null
 */
private fun detectSkinToneFromEmoji(emoji: EmojiKeyData): com.siansiansu.taigikeyboard.ime.keyboard.EmojiSkinTone? {
    val skinToneCodePoints = com.siansiansu.taigikeyboard.ime.keyboard.EmojiSkinTone.availableTones()
        .map { it.codePoint }

    // 檢查 emoji 的 codePoints 中是否包含膚色修飾符
    for (codePoint in emoji.codePoints) {
        if (codePoint in skinToneCodePoints) {
            return com.siansiansu.taigikeyboard.ime.keyboard.EmojiSkinTone.fromCodePoint(codePoint)
        }
    }

    return null
}
