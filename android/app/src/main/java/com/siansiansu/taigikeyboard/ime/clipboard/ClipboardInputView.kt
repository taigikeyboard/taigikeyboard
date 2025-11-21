package com.siansiansu.taigikeyboard.ime.clipboard

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.staggeredgrid.LazyVerticalStaggeredGrid
import androidx.compose.foundation.lazy.staggeredgrid.StaggeredGridCells
import androidx.compose.foundation.lazy.staggeredgrid.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.Backspace
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.DeleteSweep
import androidx.compose.material.icons.outlined.ContentPaste
import androidx.compose.material.icons.outlined.Inbox
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.livedata.observeAsState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import androidx.compose.ui.graphics.Color as ComposeColor
import android.util.TypedValue

/**
 * 剪貼簿輸入視圖（Compose UI）
 * 參考 FlorisBoard 設計風格
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ClipboardInputView(
    modifier: Modifier = Modifier,
    onBackClick: () -> Unit = {},
    onBackspaceClick: () -> Unit = {}
) {
    val context = LocalContext.current
    val clipboardManager = ClipboardManager.getInstance(context)
    val languageManager = com.siansiansu.taigikeyboard.settings.LanguageManager.getInstance(context)
    val currentLanguage by languageManager.currentDisplayLanguage.observeAsState(com.siansiansu.taigikeyboard.settings.DisplayLanguage.HANJI)
    val items by clipboardManager.getAllItemsLive().observeAsState(emptyList())

    // 從 theme 讀取顏色（與預設鍵盤一致）
    val typedValue = remember { TypedValue() }
    val keyboardBgColor = remember(context.theme) {
        context.theme.resolveAttribute(R.attr.keyboard_bgColor, typedValue, true)
        ComposeColor(typedValue.data)
    }
    val keyBgColor = remember(context.theme) {
        context.theme.resolveAttribute(R.attr.key_bgColor, typedValue, true)
        ComposeColor(typedValue.data)
    }
    val textColor = remember(context.theme) {
        context.theme.resolveAttribute(android.R.attr.textColor, typedValue, true)
        ComposeColor(typedValue.data)
    }

    // 計算鍵盤高度（與 text_input 一致）
    val keyboardHeight = calculateKeyboardHeight()

    Column(
        modifier = modifier
            .fillMaxWidth()
            .height(keyboardHeight)
            .background(keyboardBgColor)
    ) {
        // 標題列
        ClipboardHeader(
            backgroundColor = keyboardBgColor,
            textColor = textColor,
            onBackClick = onBackClick,
            onClearAllClick = { clipboardManager.clearAll() },
            onBackspaceClick = onBackspaceClick,
            itemCount = items.size
        )

        // 內容區
        if (items.isEmpty()) {
            EmptyState(textColor = textColor)
        } else {
            // 剪貼簿列表（交錯網格，類似 FlorisBoard）
            LazyVerticalStaggeredGrid(
                columns = StaggeredGridCells.Adaptive(160.dp),
                modifier = Modifier
                    .fillMaxWidth()
                    .weight(1f),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
                verticalItemSpacing = 6.dp,
                contentPadding = PaddingValues(8.dp)
            ) {
                items(items, key = { it.id }) { item ->
                    ClipboardItemCard(
                        item = item,
                        backgroundColor = keyBgColor,
                        textColor = textColor,
                        currentLanguage = currentLanguage,
                        onItemClick = {
                            clipboardManager.pasteItem(item)
                            onBackClick()
                        },
                        onDeleteClick = {
                            clipboardManager.deleteItem(item.id)
                        }
                    )
                }
            }
        }
    }
}

/**
 * 標題列（參考 FlorisBoard ClipboardHeader）
 */
@Composable
private fun ClipboardHeader(
    backgroundColor: ComposeColor,
    textColor: ComposeColor,
    onBackClick: () -> Unit,
    onClearAllClick: () -> Unit,
    onBackspaceClick: () -> Unit,
    itemCount: Int
) {
    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .height(48.dp),
        color = backgroundColor,
        tonalElevation = 0.dp
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 4.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // 返回按鈕
            IconButton(
                onClick = onBackClick,
                modifier = Modifier.size(48.dp)
            ) {
                Icon(
                    imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                    contentDescription = "返回",
                    tint = textColor
                )
            }

            // 剪貼簿圖示
            Icon(
                imageVector = Icons.Outlined.ContentPaste,
                contentDescription = "剪貼簿",
                tint = textColor.copy(alpha = 0.7f),
                modifier = Modifier.size(24.dp)
            )

            Spacer(modifier = Modifier.weight(1f))

            // 清除全部按鈕
            IconButton(
                onClick = onClearAllClick,
                enabled = itemCount > 0,
                modifier = Modifier.size(48.dp)
            ) {
                Icon(
                    imageVector = Icons.Default.DeleteSweep,
                    contentDescription = "清除全部",
                    tint = if (itemCount > 0) {
                        textColor
                    } else {
                        textColor.copy(alpha = 0.38f)
                    }
                )
            }

            // Backspace 按鈕
            IconButton(
                onClick = onBackspaceClick,
                modifier = Modifier.size(48.dp)
            ) {
                Icon(
                    imageVector = Icons.AutoMirrored.Outlined.Backspace,
                    contentDescription = "刪除",
                    tint = textColor
                )
            }
        }
    }
}

/**
 * 剪貼簿項目卡片（參考 FlorisBoard ClipItemView）
 */
@OptIn(androidx.compose.foundation.ExperimentalFoundationApi::class)
@Composable
private fun ClipboardItemCard(
    item: ClipboardItem,
    backgroundColor: ComposeColor,
    textColor: ComposeColor,
    currentLanguage: com.siansiansu.taigikeyboard.settings.DisplayLanguage,
    onItemClick: () -> Unit,
    onDeleteClick: () -> Unit
) {
    var showDeleteConfirm by remember { mutableStateOf(false) }

    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(12.dp))
            .combinedClickable(
                interactionSource = remember { MutableInteractionSource() },
                indication = ripple(),
                onLongClick = { showDeleteConfirm = true },
                onClick = onItemClick
            ),
        color = backgroundColor,
        tonalElevation = 0.dp,
        shape = RoundedCornerShape(12.dp)
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(12.dp)
        ) {
            // 文字內容
            Text(
                text = item.getPreview(),
                style = MaterialTheme.typography.bodyMedium,
                color = textColor,
                maxLines = 3,
                overflow = TextOverflow.Ellipsis
            )

            Spacer(modifier = Modifier.height(8.dp))

            // 時間戳
            Text(
                text = formatTimestamp(item.timestampMs, currentLanguage),
                style = MaterialTheme.typography.labelSmall,
                color = textColor.copy(alpha = 0.6f)
            )
        }
    }

    // 刪除確認對話框
    if (showDeleteConfirm) {
        AlertDialog(
            onDismissRequest = { showDeleteConfirm = false },
            icon = {
                Icon(
                    imageVector = Icons.Default.Delete,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.error
                )
            },
            title = {
                Text(
                    text = "刪除項目",
                    style = MaterialTheme.typography.titleMedium
                )
            },
            text = {
                Text(
                    text = "確定要刪除此剪貼簿項目嗎？",
                    style = MaterialTheme.typography.bodyMedium
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        onDeleteClick()
                        showDeleteConfirm = false
                    }
                ) {
                    Text(
                        text = "刪除",
                        color = MaterialTheme.colorScheme.error
                    )
                }
            },
            dismissButton = {
                TextButton(onClick = { showDeleteConfirm = false }) {
                    Text("取消")
                }
            },
            containerColor = MaterialTheme.colorScheme.surface,
            tonalElevation = 6.dp
        )
    }
}

/**
 * 空狀態視圖（參考 FlorisBoard HistoryEmptyView）
 */
@Composable
private fun EmptyState(textColor: ComposeColor) {
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .fillMaxHeight()
            .padding(32.dp),
        contentAlignment = Alignment.Center
    ) {
        Icon(
            imageVector = Icons.Outlined.Inbox,
            contentDescription = "剪貼簿為空",
            modifier = Modifier.size(72.dp),
            tint = textColor.copy(alpha = 0.3f)
        )
    }
}

/**
 * 計算鍵盤高度（與 text_input 一致）
 * text_input = Smartbar (40dp × factor) + KeyboardView (4行按鍵 × factor)
 * clipboard = ClipboardHeader (48dp 固定) + Grid
 *
 * 目標：clipboard 總高度 = text_input 總高度
 */
@Composable
private fun calculateKeyboardHeight(): androidx.compose.ui.unit.Dp {
    val context = LocalContext.current
    val configuration = LocalConfiguration.current
    val taigikeyboard = TaigiKeyboard.getInstance()

    // 讀取使用者偏好的高度係數
    val heightFactor = taigikeyboard.prefs.heightFactor

    // 基礎按鍵高度
    val baseKeyHeight = 42.dp

    // 計算高度係數（與 KeyboardView 一致）
    val keyHeightFactor = when (configuration.orientation) {
        android.content.res.Configuration.ORIENTATION_LANDSCAPE -> 0.85f
        else -> 1.0f
    } * when (heightFactor) {
        "extra_short" -> 0.85f
        "short" -> 0.90f
        "mid_short" -> 0.95f
        "normal" -> 1.00f
        "mid_tall" -> 1.05f
        "tall" -> 1.10f
        "extra_tall" -> 1.15f
        else -> 1.00f
    }

    // 計算實際按鍵高度
    val actualKeyHeight = baseKeyHeight * keyHeightFactor

    // 按鍵垂直間距
    val keyMarginV = 5.dp

    // text_input 的 Smartbar 高度（會被 factor 影響）
    val smartbarHeight = 40.dp * keyHeightFactor

    // KeyboardView 高度（3 行按鍵 + 間距）
    val numRows = 5
    val totalKeyHeight = actualKeyHeight * numRows
    val totalMargin = keyMarginV * 2 * numRows
    val keyboardViewHeight = totalKeyHeight + totalMargin

    // text_input 總高度 = Smartbar + KeyboardView
    val textInputHeight = smartbarHeight + keyboardViewHeight

    // clipboard 總高度應該等於 text_input 總高度
    // 但要確保有足夠空間給 ClipboardHeader (48dp)
    return textInputHeight
}

/**
 * 格式化時間戳（使用本地化文字）
 */
private fun formatTimestamp(
    timestampMs: Long,
    currentLanguage: com.siansiansu.taigikeyboard.settings.DisplayLanguage
): String {
    val now = System.currentTimeMillis()
    val diffMs = now - timestampMs
    val diffMinutes = diffMs / (1000 * 60)
    val diffHours = diffMs / (1000 * 60 * 60)
    val diffDays = diffMs / (1000 * 60 * 60 * 24)

    return when {
        diffMinutes < 1 -> com.siansiansu.taigikeyboard.settings.AppTexts.justNow.text(currentLanguage)
        diffMinutes < 60 -> "${diffMinutes} ${com.siansiansu.taigikeyboard.settings.AppTexts.minutesAgo.text(currentLanguage)}"
        diffHours < 24 -> "${diffHours} ${com.siansiansu.taigikeyboard.settings.AppTexts.hoursAgo.text(currentLanguage)}"
        diffDays < 7 -> "${diffDays} ${com.siansiansu.taigikeyboard.settings.AppTexts.daysAgo.text(currentLanguage)}"
        else -> {
            val date = java.util.Date(timestampMs)
            java.text.SimpleDateFormat("MM/dd HH:mm", java.util.Locale.getDefault()).format(date)
        }
    }
}
