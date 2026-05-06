
package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.content.Context
import android.util.AttributeSet

/**
 * Quick action button 元件
 * 用於 smartbar 的快捷操作按鈕（settings）
 */
class SmartbarQuickActionButton : androidx.appcompat.widget.AppCompatImageButton {
    constructor(context: Context) : this(context, null)
    constructor(context: Context, attrs: AttributeSet?) : this(context, attrs, 0)
    constructor(context: Context, attrs: AttributeSet?, defStyleAttr: Int) : super(context, attrs, defStyleAttr)
}
