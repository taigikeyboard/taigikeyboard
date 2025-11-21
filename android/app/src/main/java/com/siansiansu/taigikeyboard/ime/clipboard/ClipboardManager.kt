package com.siansiansu.taigikeyboard.ime.clipboard

import android.content.ClipData
import android.content.ClipboardManager as SystemClipboardManager
import android.content.Context
import android.util.Log
import androidx.lifecycle.LiveData
import com.siansiansu.taigikeyboard.BuildConfig
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

/**
 * 管理剪貼簿歷史記錄，監聽系統剪貼簿變化
 * 只處理純文字剪貼簿項目
 */
class ClipboardManager private constructor(
    private val context: Context
) : SystemClipboardManager.OnPrimaryClipChangedListener {

    companion object {
        @Volatile
        private var instance: ClipboardManager? = null

        fun getInstance(context: Context): ClipboardManager {
            return instance ?: synchronized(this) {
                instance ?: ClipboardManager(context.applicationContext).also { instance = it }
            }
        }
    }

    private val systemClipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as SystemClipboardManager
    private val database = ClipboardDatabase.getInstance(context)
    private val dao = database.clipboardDao()
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    var isEnabled: Boolean = true
        set(value) {
            field = value
            if (value) {
                systemClipboard.addPrimaryClipChangedListener(this)
            } else {
                systemClipboard.removePrimaryClipChangedListener(this)
            }
        }

    init {
        systemClipboard.addPrimaryClipChangedListener(this)
    }

    /**
     * 取得所有剪貼簿項目（LiveData，自動更新）
     */
    fun getAllItemsLive(): LiveData<List<ClipboardItem>> {
        return dao.getAllLive()
    }

    /**
     * 取得所有剪貼簿項目（同步版本）
     */
    suspend fun getAllItems(): List<ClipboardItem> {
        return dao.getAll()
    }

    /**
     * 系統剪貼簿變化時被呼叫
     */
    override fun onPrimaryClipChanged() {
        if (!isEnabled) return

        val clipData = systemClipboard.primaryClip
        if (clipData == null || clipData.itemCount == 0) {
            return
        }

        val item = clipData.getItemAt(0)
        val text = item.text?.toString()

        if (text.isNullOrBlank()) {
            return
        }

        // 插入新項目到資料庫
        scope.launch {
            // 檢查是否已存在相同文字
            val existing = dao.findByText(text)
            if (existing == null) {
                // 不存在，插入新項目
                val newItem = ClipboardItem.fromText(text)
                dao.insert(newItem)
            } else {
                // 已存在，更新時間戳（刪除後重新插入以移到最前面）
                dao.deleteById(existing.id)
                val updatedItem = existing.copy(
                    id = 0, // 重新產生 ID
                    timestampMs = System.currentTimeMillis()
                )
                dao.insert(updatedItem)
            }
        }
    }

    /**
     * 貼上指定的剪貼簿項目到當前輸入框
     */
    fun pasteItem(item: ClipboardItem) {
        scope.launch(Dispatchers.Main) {
            try {
                val taigikeyboard = com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard.getInstance()
                val ic = taigikeyboard.currentInputConnection
                ic?.commitText(item.text, 1)

                // 更新系統剪貼簿
                val clipData = ClipData.newPlainText("text", item.text)
                systemClipboard.setPrimaryClip(clipData)
            } catch (e: Exception) {
                Log.e("ClipboardManager", "Failed to paste item", e)
            }
        }
    }

    /**
     * 刪除指定項目
     */
    fun deleteItem(id: Long) {
        scope.launch {
            dao.deleteById(id)
        }
    }

    /**
     * 清除所有項目
     */
    fun clearAll() {
        scope.launch {
            dao.deleteAll()
        }
    }

    /**
     * 清理資源
     */
    fun cleanup() {
        systemClipboard.removePrimaryClipChangedListener(this)
    }
}
