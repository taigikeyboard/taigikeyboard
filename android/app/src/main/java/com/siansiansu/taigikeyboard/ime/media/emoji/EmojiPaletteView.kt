package com.siansiansu.taigikeyboard.ime.media.emoji

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.aspectRatio
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
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.Tab
import androidx.compose.material3.TabRowDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Popup
import kotlinx.coroutines.launch
import kotlin.math.ceil

private val EmojiCategoryValues = EmojiCategory.entries
private val EmojiBaseWidth = 70.dp
private val EmojiDefaultFontSize = 35.sp

private val VariantsTriangleShapeLtr = GenericShape { size, _ ->
    moveTo(x = size.width, y = 0f)
    lineTo(x = size.width, y = size.height)
    lineTo(x = 0f, y = size.height)
}

private val VariantsTriangleShapeRtl = GenericShape { size, _ ->
    moveTo(x = 0f, y = 0f)
    lineTo(x = size.width, y = size.height)
    lineTo(x = 0f, y = size.height)
}

@Composable
fun EmojiPaletteView(
    fullEmojiMappings: EmojiLayoutDataMap,
    preferredSkinTone: com.siansiansu.taigikeyboard.ime.keyboard.EmojiSkinTone,
    onEmojiClick: (EmojiKeyData) -> Unit,
    onSkinToneSelected: (com.siansiansu.taigikeyboard.ime.keyboard.EmojiSkinTone) -> Unit,
    modifier: Modifier = Modifier,
) {
    var activeCategory by remember { mutableStateOf(EmojiCategory.SMILEYS_EMOTION) }
    val scope = rememberCoroutineScope()

    Column(modifier = modifier) {
        val pagerState = rememberPagerState(pageCount = { EmojiCategoryValues.size })

        EmojiCategoriesTabRow(
            activeCategory = activeCategory,
            onCategoryChange = { category ->
                activeCategory = category
                scope.launch {
                    pagerState.animateScrollToPage(EmojiCategoryValues.indexOf(category))
                }
            },
        )

        HorizontalPager(
            state = pagerState,
            beyondViewportPageCount = 2,
        ) { page ->
            val lazyGridState = rememberLazyGridState()

            LaunchedEffect(pagerState.currentPage) {
                activeCategory = EmojiCategoryValues[pagerState.currentPage]
            }

            val category = EmojiCategoryValues[page]

            val emojiList = remember(category, fullEmojiMappings) {
                fullEmojiMappings[category] ?: emptyList()
            }

            // key() keeps Compose tracking and reusing grid state per category.
            key(category) {
                CompositionLocalProvider(LocalLayoutDirection provides LayoutDirection.Ltr) {
                    // Fixed height so the grid does not fill the whole screen.
                    LazyVerticalGrid(
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(240.dp),
                        columns = GridCells.Fixed(7),
                        state = lazyGridState,
                        horizontalArrangement = Arrangement.spacedBy(2.dp),
                        verticalArrangement = Arrangement.spacedBy(2.dp),
                    ) {
                        items(emojiList) { emojiSet ->
                            EmojiKey(
                                emojiSet = emojiSet,
                                preferredSkinTone = preferredSkinTone,
                                onEmojiClick = onEmojiClick,
                                onSkinToneSelected = onSkinToneSelected,
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun EmojiCategoriesTabRow(
    activeCategory: EmojiCategory,
    onCategoryChange: (EmojiCategory) -> Unit,
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
                color = androidx.compose.material3.MaterialTheme.colorScheme.primary,
            )
        },
    ) {
        EmojiCategoryValues.forEachIndexed { index, category ->
            Tab(
                selected = activeCategory == category,
                onClick = { onCategoryChange(category) },
                icon = {
                    Icon(
                        imageVector = category.icon(),
                        contentDescription = category.toString(),
                        modifier = Modifier.size(24.dp),
                    )
                },
            )
        }
    }
}

@Composable
private fun EmojiKey(
    emojiSet: EmojiSet,
    preferredSkinTone: com.siansiansu.taigikeyboard.ime.keyboard.EmojiSkinTone,
    onEmojiClick: (EmojiKeyData) -> Unit,
    onSkinToneSelected: (com.siansiansu.taigikeyboard.ime.keyboard.EmojiSkinTone) -> Unit,
) {
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
                    },
                )
            },
    ) {
        Text(
            modifier = Modifier.align(Alignment.Center),
            text = base.getCodePointsAsString(),
            fontSize = EmojiDefaultFontSize,
            color = androidx.compose.material3.MaterialTheme.colorScheme.onSurface,
        )

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
                        shape,
                    ),
            )
        }

        if (showVariantsPopup) {
            EmojiVariationsPopup(
                variations = variations,
                onEmojiTap = { emoji, skinTone ->
                    onEmojiClick(emoji)
                    if (skinTone != null) {
                        onSkinToneSelected(skinTone)
                    }
                    showVariantsPopup = false
                },
                onDismiss = {
                    showVariantsPopup = false
                },
            )
        }
    }
}

@Composable
private fun EmojiVariationsPopup(
    variations: List<EmojiKeyData>,
    onEmojiTap: (EmojiKeyData, com.siansiansu.taigikeyboard.ime.keyboard.EmojiSkinTone?) -> Unit,
    onDismiss: () -> Unit,
) {
    val emojiKeyHeight = 48.dp

    Popup(
        alignment = Alignment.TopCenter,
        offset = with(LocalDensity.current) {
            val y = -emojiKeyHeight * ceil(variations.size / 6f)
            IntOffset(x = 0, y = y.toPx().toInt())
        },
        onDismissRequest = onDismiss,
    ) {
        Box(
            modifier = Modifier
                .widthIn(max = EmojiBaseWidth * 6)
                .background(androidx.compose.material3.MaterialTheme.colorScheme.surfaceVariant)
                .padding(8.dp),
        ) {
            Column {
                variations.chunked(6).forEach { row ->
                    androidx.compose.foundation.layout.Row {
                        row.forEach { emoji ->
                            val skinTone = detectSkinToneFromEmoji(emoji)

                            Box(
                                modifier = Modifier
                                    .width(EmojiBaseWidth)
                                    .height(emojiKeyHeight)
                                    .pointerInput(Unit) {
                                        detectTapGestures { onEmojiTap(emoji, skinTone) }
                                    },
                            ) {
                                Text(
                                    modifier = Modifier.align(Alignment.Center),
                                    text = emoji.getCodePointsAsString(),
                                    fontSize = EmojiDefaultFontSize,
                                    color = androidx.compose.material3.MaterialTheme.colorScheme.onSurface,
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

private fun detectSkinToneFromEmoji(emoji: EmojiKeyData): com.siansiansu.taigikeyboard.ime.keyboard.EmojiSkinTone? {
    val skinToneCodePoints = com.siansiansu.taigikeyboard.ime.keyboard.EmojiSkinTone
        .availableTones()
        .map { it.codePoint }

    for (codePoint in emoji.codePoints) {
        if (codePoint in skinToneCodePoints) {
            return com.siansiansu.taigikeyboard.ime.keyboard.EmojiSkinTone
                .fromCodePoint(codePoint)
        }
    }

    return null
}
