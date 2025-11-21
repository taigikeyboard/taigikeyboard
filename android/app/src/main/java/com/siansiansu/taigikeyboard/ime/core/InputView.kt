
package com.siansiansu.taigikeyboard.ime.core

import android.content.Context
import android.util.AttributeSet
import android.util.Log
import android.view.WindowInsets
import android.widget.FrameLayout
import android.widget.ViewFlipper
import androidx.core.view.WindowInsetsCompat
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.R

class InputView : FrameLayout {

    private var taigikeyboard: TaigiKeyboard = TaigiKeyboard.getInstance()

    var mainViewFlipper: ViewFlipper? = null
        private set

    constructor(context: Context) : this(context, null)
    constructor(context: Context, attrs: AttributeSet?) : this(context, attrs, 0)
    constructor(context: Context, attrs: AttributeSet?, defStyleAttr: Int) : super(context, attrs, defStyleAttr)

    override fun onAttachedToWindow() {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onAttachedToWindow()")

        super.onAttachedToWindow()

        mainViewFlipper = findViewById(R.id.main_view_flipper)

        taigikeyboard.registerInputView(this)

        // Force request insets to ensure onApplyWindowInsets is called
        // This is critical on API 35+ (Android 15)
        requestApplyInsets()

        if (BuildConfig.DEBUG) {
            Log.d(this::class.simpleName, "Requested apply insets")
        }
    }

    override fun onApplyWindowInsets(insets: WindowInsets): WindowInsets {
        if (BuildConfig.DEBUG) {
            val compat = WindowInsetsCompat.toWindowInsetsCompat(insets)
            val navBars = compat.getInsets(WindowInsetsCompat.Type.navigationBars())
            Log.d(this::class.simpleName, "onApplyWindowInsets called - navBars.bottom: ${navBars.bottom}")
        }
        return super.onApplyWindowInsets(insets)
    }
}
