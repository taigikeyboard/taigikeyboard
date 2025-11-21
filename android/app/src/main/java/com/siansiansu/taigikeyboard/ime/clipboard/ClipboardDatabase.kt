package com.siansiansu.taigikeyboard.ime.clipboard

import android.content.Context
import androidx.lifecycle.LiveData
import androidx.room.*

/**
 * DAO (Data Access Object) 提供剪貼簿資料庫操作
 */
@Dao
interface ClipboardDao {
    /**
     * 取得所有剪貼簿項目（依時間降序排列）
     */
    @Query("SELECT * FROM clipboard_history ORDER BY timestampMs DESC")
    fun getAllLive(): LiveData<List<ClipboardItem>>

    /**
     * 取得所有剪貼簿項目（同步版本）
     */
    @Query("SELECT * FROM clipboard_history ORDER BY timestampMs DESC")
    suspend fun getAll(): List<ClipboardItem>

    /**
     * 插入新的剪貼簿項目
     */
    @Insert
    suspend fun insert(item: ClipboardItem): Long

    /**
     * 更新剪貼簿項目
     */
    @Update
    suspend fun update(item: ClipboardItem)

    /**
     * 刪除指定 ID 的項目
     */
    @Query("DELETE FROM clipboard_history WHERE id = :id")
    suspend fun deleteById(id: Long)

    /**
     * 刪除所有項目
     */
    @Query("DELETE FROM clipboard_history")
    suspend fun deleteAll()

    /**
     * 取得項目總數
     */
    @Query("SELECT COUNT(*) FROM clipboard_history")
    suspend fun getCount(): Int

    /**
     * 檢查是否已存在相同文字內容
     */
    @Query("SELECT * FROM clipboard_history WHERE text = :text LIMIT 1")
    suspend fun findByText(text: String): ClipboardItem?
}

/**
 * Room 資料庫定義
 */
@Database(
    entities = [ClipboardItem::class],
    version = 1,
    exportSchema = false
)
abstract class ClipboardDatabase : RoomDatabase() {
    abstract fun clipboardDao(): ClipboardDao

    companion object {
        @Volatile
        private var INSTANCE: ClipboardDatabase? = null

        /**
         * 取得資料庫實例（單例模式）
         */
        fun getInstance(context: Context): ClipboardDatabase {
            return INSTANCE ?: synchronized(this) {
                val instance = Room.databaseBuilder(
                    context.applicationContext,
                    ClipboardDatabase::class.java,
                    "clipboard_database"
                )
                    .fallbackToDestructiveMigration()
                    .build()
                INSTANCE = instance
                instance
            }
        }
    }
}
